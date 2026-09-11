//! 規則受渡しフック — 指揮者から部下へ渡る brief へ、そのステージの規則束を逐語で付ける。
//!
//! 固定本家 2.7.1 `a277af21` の `hooks/aidlc-deliver-stage-rules.ts:76-165,193-221,266-358`
//! に対応する。`next` / `continue` が運ぶ `load-steering` とは**別の hop** であり、狙いは
//! 「エンジン→指揮者」と「指揮者→部下」で同じ規則が届くことである。
//!
//! # 何を判断し、何を判断しないか
//!
//! ここが持つのは 3 つだけである。
//!
//! 1. **どの stage の束か** — 明示パス → 現在 stage → 唯一の slug の順。
//! 2. **どのファイルを読むか** — その stage の `rules_in_context` を active space の
//!    memory 層へ解決し、中身のある規則だけを読み順に並べる。
//! 3. **出しても安全か** — 出力が 512KiB を超えるなら、部分的に書かずに拒否する。
//!
//! 封筒の読みと書き戻し（[`harness_claude::DispatchRulesEnvelope`]）と、束の描画
//! （[`harness_claude::StageRuleBundle`]）はここに無い。
use super::{Completion, Layout};
use crate::wording;
use core_command_domain::workflow_definition::{StageGraph, StageNode};
use core_command_interface_adapter::orchestration::CompiledDefinitionRepositoryImpl;
use core_command_use_case::orchestration::CompiledDefinitionRepository as _;
use core_read_model_updater::orchestration::SteeringSource;
use harness_claude::{DispatchRulesEnvelope, RuleFile, StageRuleBundle};
use std::path::Path;

/// 本家 `DISPATCH_HOOK_OUTPUT_MAX_BYTES` — 観測された最小の子プロセス捕捉上限の下に置く。
const OUTPUT_MAX_BYTES: usize = 512 * 1024;
/// 規則を自前で先読みするハーネスが、上限超過を停止ではなく助言にするための環境変数。
const PRELOAD_FALLBACK: &str = "AIDLC_DISPATCH_RULES_PRELOAD_FALLBACK";
/// ステージ本体ファイルが置かれる phase ディレクトリ（明示パスの照合に使う）。
const PHASE_DIRS: [&str; 5] = [
    "initialization",
    "ideation",
    "inception",
    "construction",
    "operation",
];
/// 規則パスの中で active space 相対部分が始まる目印（本家 `const marker = "/memory/"`）。
const MEMORY_MARKER: &str = "/memory/";

pub(super) async fn run(layout: &Layout, input: &str) -> Completion {
    let envelope = DispatchRulesEnvelope::parse(input, &layout.agent_dir());
    if envelope.briefs().is_empty() {
        return Completion::silent();
    }
    // グラフを読めないのは規則配送の判断材料が無いということであり、人間の作業は止めない。
    let Some(graph) = compiled_graph(layout).await else {
        return Completion::silent();
    };
    let fallback = current_stage(layout);
    let mut bundles = Vec::with_capacity(envelope.briefs().len());
    for brief in envelope.briefs() {
        match bundle_for(layout, &graph, brief, fallback.as_deref()) {
            Ok(bundle) => bundles.push(bundle),
            Err(message) => return Completion::hook_denied(message),
        }
    }
    let Some(output) = envelope.pre_tool_use_output(&bundles) else {
        return Completion::silent();
    };
    // 数えるのは stdout 全体である — 本家は末尾改行まで含めて上限と比べる。
    let bytes = output.len() + 1;
    if bytes > OUTPUT_MAX_BYTES {
        return if std::env::var(PRELOAD_FALLBACK).is_ok_and(|value| value == "1") {
            Completion::hook_advisory(wording::dispatch_rules_oversize_advisory(
                bytes,
                OUTPUT_MAX_BYTES,
            ))
        } else {
            Completion::hook_denied(wording::dispatch_rules_oversize(bytes, OUTPUT_MAX_BYTES))
        };
    }
    Completion::emitted(output)
}

/// 配布束のステージグラフ。読めなければ `None`（判断材料が無い）。
async fn compiled_graph(layout: &Layout) -> Option<StageGraph> {
    let id = super::compiled_definition_id(layout).ok()?;
    let compiled =
        CompiledDefinitionRepositoryImpl::new(layout.definition_data_dir(), layout.scopes_dir())
            .find_by_id(&id)
            .await
            .ok()?;
    Some(compiled.graph().clone())
}

/// 状態ファイルが名乗る `Current Stage`（無ければ `None`）。
fn current_stage(layout: &Layout) -> Option<String> {
    let text = std::fs::read_to_string(layout.state_file()?).ok()?;
    let stage = super::session_hooks::field(&text, "Current Stage")?;
    let stage = core_infrastructure::ecmascript::trim(&stage).to_string();
    (!stage.is_empty()).then_some(stage)
}

/// この brief に付ける束。stage が決まらなければ `Ok(None)`。
///
/// # Errors
///
/// 要る規則ファイルが読めない場合、逐語の拒否文言を返す。
fn bundle_for(
    layout: &Layout,
    graph: &StageGraph,
    brief: &str,
    fallback: Option<&str>,
) -> Result<Option<StageRuleBundle>, String> {
    let Some(node) = resolve_stage(graph, brief, fallback) else {
        return Ok(None);
    };
    let files = read_rules(layout, node)?;
    Ok(Some(StageRuleBundle::new(
        node.slug().as_str().to_string(),
        files,
    )))
}

/// どの stage の束かを決める — **明示パス → 現在 stage → 唯一の slug** の順。
///
/// 2 が 3 より強いのは、dispatch が stage の**最中**に起きるからである。brief が別の stage の
/// slug をついでに名乗っただけ（「scope-definition が終わったら…」）でその束を束ねてしまう
/// と、届く規則が実際の作業と食い違う。
fn resolve_stage<'a>(
    graph: &'a StageGraph,
    brief: &str,
    fallback: Option<&str>,
) -> Option<&'a StageNode> {
    let enabled = |node: &&StageNode| node.is_enabled();
    if let Some(slug) = explicit_stage_path(brief)
        && let Some(node) = graph
            .nodes()
            .iter()
            .find(|node| node.slug().as_str() == slug && enabled(node))
    {
        return Some(node);
    }
    if let Some(stage) = fallback {
        return graph
            .nodes()
            .iter()
            .find(|node| node.slug().as_str() == stage && enabled(node));
    }
    let mut named = graph
        .nodes()
        .iter()
        .filter(|node| enabled(node) && mentions_slug(brief, node.slug().as_str()));
    let first = named.next()?;
    named.next().is_none().then_some(first)
}

/// brief が名乗るステージ本体ファイルの slug。
///
/// 本家の正規表現
/// `/(?:^|[/\\])stages[/\\](?:<phase>)[/\\]([a-z0-9][a-z0-9-]*)\.md\b/i` を手で写したもの。
/// 最初に成立した位置の 1 件だけを返す（`String.match` と同じ）。
fn explicit_stage_path(brief: &str) -> Option<&str> {
    let bytes = brief.as_bytes();
    let separator = |index: usize| bytes.get(index).is_some_and(|b| *b == b'/' || *b == b'\\');
    let mut start = 0_usize;
    while let Some(offset) =
        find_ignore_ascii_case(bytes.get(start..).unwrap_or_default(), b"stages")
    {
        let at = start + offset;
        start = at + 1;
        if at != 0 && !separator(at - 1) {
            continue;
        }
        let after_stages = at + "stages".len();
        if !separator(after_stages) {
            continue;
        }
        let Some(phase) = PHASE_DIRS.iter().find(|phase| {
            bytes
                .get(after_stages + 1..after_stages + 1 + phase.len())
                .is_some_and(|found| found.eq_ignore_ascii_case(phase.as_bytes()))
                && separator(after_stages + 1 + phase.len())
        }) else {
            continue;
        };
        let slug_start = after_stages + 1 + phase.len() + 1;
        let slug_end = slug_start
            + bytes.get(slug_start..).map_or(0, |rest| {
                rest.iter()
                    .take_while(|byte| byte.is_ascii_alphanumeric() || **byte == b'-')
                    .count()
            });
        if !bytes.get(slug_start).is_some_and(u8::is_ascii_alphanumeric) {
            continue;
        }
        if bytes.get(slug_end..slug_end + 3) != Some(b".md".as_slice()) {
            continue;
        }
        // `\b` — `.md` の直後は語構成文字であってはならない。
        if bytes
            .get(slug_end + 3)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            continue;
        }
        return brief.get(slug_start..slug_end);
    }
    None
}

/// brief が slug を**語として**名乗っているか。
///
/// 本家 `(?:^|[^a-z0-9-])<slug>(?:$|[^a-z0-9-])` の /i 付き。`_` は境界であり、英数字と
/// ハイフンだけが境界にならない。
fn mentions_slug(brief: &str, slug: &str) -> bool {
    let bytes = brief.as_bytes();
    let inside = |index: usize| {
        bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
    };
    let mut start = 0_usize;
    while let Some(offset) =
        find_ignore_ascii_case(bytes.get(start..).unwrap_or_default(), slug.as_bytes())
    {
        let at = start + offset;
        start = at + 1;
        let before_is_boundary = at == 0 || !inside(at - 1);
        if before_is_boundary && !inside(at + slug.len()) {
            return true;
        }
    }
    false
}

/// ASCII の大小を無視した部分列探索（見つかった先頭のバイト位置）。
///
/// バイト列のまま探すのは、非 ASCII を含む brief でも境界を割らずに走査するためである
/// （`[a-z0-9-]` はすべて ASCII なので、判定に必要な情報はバイトで足りる）。
fn find_ignore_ascii_case(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (0..=haystack.len() - needle.len()).find(|start| {
        haystack
            .get(*start..*start + needle.len())
            .is_some_and(|found| found.eq_ignore_ascii_case(needle))
    })
}

/// stage の `rules_in_context` を active space の memory 層へ解決して読む。
///
/// 本家 `tools/aidlc-steering.ts` の `rulesContentEntries` + `readRuleBundle`。**在るのに
/// 読めない**のは配送の停止条件である（取得ループが欠損を正常として飛ばすのとは違う —
/// 部下へ渡す束が黙って痩せるのを許さないため）。
///
/// # Errors
///
/// 規則ファイルを UTF-8 として読めない場合、逐語の拒否文言を返す。
fn read_rules(layout: &Layout, node: &StageNode) -> Result<Vec<RuleFile>, String> {
    let memory = layout.memory_dir();
    let mut files: Vec<RuleFile> = Vec::new();
    for rule in node.rules_in_context() {
        let (relative, absolute) = match rule.path().find(MEMORY_MARKER) {
            Some(index) => {
                let subpath = &rule.path()[index + MEMORY_MARKER.len()..];
                (
                    format!("aidlc/spaces/{}/memory/{subpath}", layout.space(),),
                    memory.join(subpath),
                )
            }
            None => (
                rule.path().to_string(),
                layout.project_dir().join(rule.path()),
            ),
        };
        if files.iter().any(|file| file.path() == relative) {
            continue;
        }
        let text = read_utf8(&absolute)
            .map_err(|cause| wording::dispatch_rule_unreadable(&relative, &cause))?;
        if SteeringSource::text_is_substantive(&text) {
            files.push(RuleFile::new(relative, text));
        }
    }
    Ok(files)
}

/// UTF-8 として厳密に読む（不正バイトは読めなかったものとして扱う）。
///
/// 先頭の BOM（U+FEFF）は本文に含めない — 本家は `new TextDecoder("utf-8", { fatal: true })`
/// の既定（`ignoreBOM: false`）で復号するので、先頭の 1 つだけが落ち、2 つ目以降と途中の
/// ものは本文として残る（WHATWG Encoding の「BOM を 1 度だけ読み飛ばす」）。ここで落とさないと
/// 束のダイジェストが本家と食い違う。
fn read_utf8(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{error}"))?;
    let mut text = String::from_utf8(bytes).map_err(|error| format!("{error}"))?;
    if text.starts_with('\u{feff}') {
        text.drain(..'\u{feff}'.len_utf8());
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::explicit_stage_path;

    /// 本家の正規表現の境界: `stages` の直後は区切り、slug の先頭は英数字である。
    #[test]
    fn the_explicit_stage_path_needs_a_separator_and_an_alphanumeric_slug_start() {
        assert_eq!(
            explicit_stage_path("see x/stages/inception/domain-design.md now"),
            Some("domain-design")
        );
        assert_eq!(
            explicit_stage_path("see x/stagesy/inception/domain-design.md"),
            None,
            "`stages` の直後が区切りでない"
        );
        assert_eq!(
            explicit_stage_path("see x/stages/inception/-design.md"),
            None,
            "slug の先頭が英数字でない"
        );
        assert_eq!(
            explicit_stage_path(
                "see x/stages/inception/domain-design.md_x and x/stages/ideation/intent-capture.md"
            ),
            Some("intent-capture"),
            "`.md` の直後が語構成文字なら次の候補へ進む"
        );
        assert_eq!(explicit_stage_path("nothing here"), None);
    }
}
