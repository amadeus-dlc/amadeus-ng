//! `aidlc engine review-brief <verb>` — レビュー判断の文脈（読取専用の提示器）。
//!
//! upstream 2.8.2 は `review` / `context` / `summary` の 3 動詞を独立したツール
//! `.claude/tools/aidlc-review-brief.ts` に置き、ROUTES 表の `review-brief` はそこへ委譲する
//! （`TOOLS.reviewBrief`）。この build も同じ形を採り、`aidlc-review-brief` を **9 つ目の
//! multi-call 面**として持つ。`statusline` のように engine 自身が受ける形（`routeOnly`）を
//! 採らないのは、本家がこの noun を面へ委譲しているからである。
//!
//! # この build のレビュー証跡は成果物末尾の `## Review` 節である
//!
//! upstream 2.8.2 はレビューを独立した「レビュー記録」へ書き、成果物の `## Review` 節は
//! 退役した旧形として移行のためだけに読む。この build はレビューを
//! [`ReviewBinding`](core_command_domain::orchestration::ReviewBinding) が指す**追記先成果物
//! の終端 `## Review` 節**として記録する（`review_appendix.rs` が綴じ方を検証する）。
//! したがってここが読むのは後者だけである。前者に当たる記録はこの build に存在しない。
//!
//! # 記録が無いので描かない行
//!
//! upstream の `review` は次の 2 群を足すことがある。この build はどちらも描かない。
//!
//! - **受領後に変わった内容の告知**（`**Reviewed content differs:**` /
//!   `**Changed after review:**`）は `CHANGE_ACCEPTED` の `Checkpoint: review-receipt` 行から
//!   採る。この build の監査語彙（`workspace/audit_events.rs`）に `CHANGE_ACCEPTED` は無い。
//! - **`--why stale` の上流変更と再確認対象**（`**Changed upstream:**` ほか 2 行）は監査
//!   シャードの試行境界を辿って採る。この build は同じ 3 集合を `Jumped` イベントが持ち
//!   `STAGE_JUMPED` の監査行へ投影しているが、構造化リードモデル（`read_*` 表）へは
//!   投影していないため、クエリ側から読める形が無い。
//!
//! 無い記録から行を作らない。`--why stale` は `**Why now:**` の文言だけが変わる。
use super::{Completion, Layout, review_documents};
use crate::wording;
use core_infrastructure::ecmascript::trim;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

/// 定義グラフが綴るレビュー対象の段。
struct Stage {
    slug: String,
    name: String,
    phase: String,
}

/// `### Findings` 表の 1 行。
#[derive(Debug)]
struct Finding {
    id: String,
    severity: String,
    location: String,
    finding: String,
    required_action: String,
    status: String,
}

/// 1 つのレビュー成果物から読み取った文脈。
struct Context {
    artifact: String,
    verdict: Option<String>,
    findings: Vec<Finding>,
}

/// 開いたままの所見の状態（upstream `finding.status === "New" || … "Unresolved"`）。
const OPEN: [&str; 2] = ["New", "Unresolved"];

pub(super) fn run(layout: &Layout, args: &[String]) -> Completion {
    match render(layout, args) {
        Ok(text) => Completion::emitted(text),
        Err(detail) => Completion::refused(wording::review_brief_failure(&detail)),
    }
}

/// 動詞を解決して本文を組む。
///
/// upstream と同じ順で確かめる — フラグ文法 → `--stage` の有無 → 段の解決 → 動詞。
/// `--stage` が無ければ動詞が何であれそこで止まる。
fn render(layout: &Layout, args: &[String]) -> Result<String, String> {
    let verb = args.first().map(String::as_str);
    let flags = flags(args.get(1..).unwrap_or_default())?;
    let slug = flags
        .get("stage")
        .ok_or_else(|| wording::REVIEW_BRIEF_MISSING_STAGE.to_string())?;
    let stage = stage(layout, slug)?;
    if flags.contains_key("unit") {
        return Err(wording::REVIEW_BRIEF_UNIT_NOT_WIRED.to_string());
    }
    match verb {
        Some("review") => review(layout, &stage, &flags),
        Some("context") => Ok(findings_context(&contexts(layout, &stage)?)),
        Some("summary") => summary(layout, &stage, &flags),
        given => Err(wording::unknown_review_brief_subcommand(given)),
    }
}

/// `--flag value` の対だけを受ける（upstream `parseCliFlags` と同じ厳しさ）。
///
/// 値を伴わないフラグも、フラグでない位置引数も、そこで止める。緩めると `--why` の値が
/// 次のフラグ名に化けるような取り違えが黙って通る。
fn flags(args: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut found = BTreeMap::new();
    let mut rest = args;
    while let Some((flag, tail)) = rest.split_first() {
        let Some(name) = flag.strip_prefix("--") else {
            return Err(wording::review_brief_flag_pair(flag));
        };
        let Some((value, next)) = tail.split_first() else {
            return Err(wording::review_brief_flag_pair(flag));
        };
        found.insert(name.to_string(), value.clone());
        rest = next;
    }
    Ok(found)
}

/// 定義グラフから段を解決する（upstream `findStageBySlug`）。
fn stage(layout: &Layout, slug: &str) -> Result<Stage, String> {
    let path = layout.definition_data_dir().join("stage-graph.json");
    let raw =
        std::fs::read(&path).map_err(|error| wording::review_brief_graph(&error.to_string()))?;
    let graph: Vec<Value> = serde_json::from_slice(&raw)
        .map_err(|error| wording::review_brief_graph(&error.to_string()))?;
    let node = graph
        .iter()
        .find(|node| node.get("slug").and_then(Value::as_str) == Some(slug))
        .ok_or_else(|| wording::unknown_review_stage(slug))?;
    let field = |name: &str| {
        node.get(name)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| wording::review_stage_field(slug, name))
    };
    Ok(Stage {
        slug: slug.to_string(),
        name: field("name")?,
        phase: field("phase")?,
    })
}

/// レビュー判定の文脈（upstream `renderReviewBrief`）。
fn review(
    layout: &Layout,
    stage: &Stage,
    flags: &BTreeMap<String, String>,
) -> Result<String, String> {
    let why = match flags.get("why").map(String::as_str) {
        Some("first") => wording::REVIEW_BRIEF_WHY_FIRST,
        Some("revision") => wording::REVIEW_BRIEF_WHY_REVISION,
        Some("stale") => wording::REVIEW_BRIEF_WHY_STALE,
        _ => return Err(wording::REVIEW_BRIEF_WHY_REQUIRED.to_string()),
    };
    let mut found = contexts(layout, stage)?;
    if found.is_empty()
        && let Some(fallback) = flags.get("fallback-finding")
    {
        found = vec![fallback_context(layout, stage, fallback)?];
    }
    Ok(wording::review_brief(
        &stage.name,
        outcome(&found),
        why,
        &findings_context(&found),
    ))
}

/// レビューの結び（upstream の 4 分岐を同じ順で選ぶ）。
fn outcome(contexts: &[Context]) -> &'static str {
    let total: usize = contexts.iter().map(|context| context.findings.len()).sum();
    let open = contexts
        .iter()
        .flat_map(|context| context.findings.iter())
        .filter(|finding| OPEN.contains(&finding.status.as_str()))
        .count();
    if open > 0 {
        wording::REVIEW_BRIEF_CONCERNS
    } else if total > 0 {
        wording::REVIEW_BRIEF_NO_OPEN
    } else if contexts
        .iter()
        .any(|context| context.verdict.as_deref() == Some("NOT-READY"))
    {
        wording::REVIEW_BRIEF_INCOMPLETE
    } else {
        wording::REVIEW_BRIEF_CLEAR
    }
}

/// レビューが完了しなかったときに人へ渡す 1 件の所見（upstream の `fallbackFinding`）。
///
/// 「所見が無かった」と名乗らせないための明示の差し込みであって、既定値ではない — 呼び手が
/// `--fallback-finding` を綴ったときにだけ立つ。
fn fallback_context(layout: &Layout, stage: &Stage, finding: &str) -> Result<Context, String> {
    let artifact = review_documents::artifacts(layout, &stage.slug)?
        .first()
        .map_or_else(
            || format!("{}/{}", stage.phase, stage.slug),
            |entry| display(layout, entry.path()),
        );
    Ok(Context {
        findings: vec![Finding {
            id: "R-01".to_string(),
            severity: "Major".to_string(),
            location: wording::review_brief_fallback_location(&artifact),
            finding: finding.to_string(),
            required_action: wording::REVIEW_BRIEF_FALLBACK_ACTION.to_string(),
            status: "Unresolved".to_string(),
        }],
        artifact,
        verdict: Some("NOT-READY".to_string()),
    })
}

/// 要約確認の文脈（upstream `renderSummaryConfirmationBrief`）。
fn summary(
    layout: &Layout,
    stage: &Stage,
    flags: &BTreeMap<String, String>,
) -> Result<String, String> {
    let questions = flags
        .get("questions-file")
        .ok_or_else(|| wording::SUMMARY_BRIEF_QUESTIONS_REQUIRED.to_string())?;
    let outside = || wording::summary_questions_outside_record(questions);
    let record = layout.record_dir().ok_or_else(outside)?;
    let absolute = normalize(&layout.project_dir().join(questions));
    if (absolute != record && !absolute.starts_with(record)) || !absolute.is_file() {
        return Err(outside());
    }
    let listed: Vec<String> = review_documents::artifacts(layout, &stage.slug)?
        .iter()
        .map(|entry| format!("`{}`", display(layout, entry.path())))
        .collect();
    let generated = if listed.is_empty() {
        wording::SUMMARY_BRIEF_GENERIC_ARTIFACTS.to_string()
    } else {
        listed.join(", ")
    };
    let shown = absolute
        .strip_prefix(layout.project_dir())
        .map_or_else(|_| questions.clone(), posix);
    Ok(wording::summary_confirmation_brief(
        &stage.name,
        &shown,
        &generated,
    ))
}

/// 宣言された成果物のうち、末尾に `## Review` 節を持つものの文脈。
fn contexts(layout: &Layout, stage: &Stage) -> Result<Vec<Context>, String> {
    let mut found = Vec::new();
    for entry in review_documents::artifacts(layout, &stage.slug)? {
        let Some(body) = entry.body() else { continue };
        let artifact = display(layout, entry.path());
        let text =
            std::str::from_utf8(body).map_err(|_| wording::review_artifact_not_text(&artifact))?;
        let Some(section) = review_section(text) else {
            continue;
        };
        let (verdict, findings) = parse_section(&section, &artifact)?;
        found.push(Context {
            artifact,
            verdict,
            findings,
        });
    }
    Ok(found)
}

/// 終端の `## Review` 節（upstream `extractMarkdownSection(content, "## Review")`）。
///
/// フェンス内の見出しは数えない — 手順を引用したコードブロックを本物の節と取り違えない
/// ためである。節が空なら「無い」と同じに扱う（upstream の falsy 判定と同じ）。
fn review_section(content: &str) -> Option<String> {
    let stripped = strip_fenced_code_blocks(content);
    let mut offset = 0usize;
    let mut start = None;
    for line in stripped.split_inclusive('\n') {
        let text = line.strip_suffix('\n').unwrap_or(line);
        if text
            .strip_prefix("## Review")
            .is_some_and(|rest| rest.chars().all(|char| char == ' ' || char == '\t'))
        {
            start = Some(offset + line.len());
            break;
        }
        offset += line.len();
    }
    let rest = stripped.get(start?..)?;
    let mut end = rest.len();
    let mut cursor = 0usize;
    for line in rest.split_inclusive('\n') {
        if line.starts_with("## ") {
            end = cursor;
            break;
        }
        cursor += line.len();
    }
    let body = rest.get(..end)?.to_string();
    (!body.is_empty()).then_some(body)
}

/// フェンス付きコードブロックの中身を空行へ置き換える（upstream `stripFencedCodeBlocks`）。
fn strip_fenced_code_blocks(content: &str) -> String {
    let mut inside = false;
    content
        .split('\n')
        .map(|line| {
            if line.starts_with("```") {
                inside = !inside;
                return "";
            }
            if inside { "" } else { line }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `## Review` 節の判定と所見表（upstream `parseReviewSection`）。
///
/// 表の形が宣言と食い違ったら黙って読み飛ばさずに止める — 欠けたセルは後ろの値を 1 つずつ
/// 左へずらすので、どの列のつもりだったかを取り戻せないからである。
fn parse_section(section: &str, artifact: &str) -> Result<(Option<String>, Vec<Finding>), String> {
    let normalized = section.replace("\r\n", "\n");
    let verdict = normalized.split('\n').find_map(|line| {
        let value = trim(line.strip_prefix("**Verdict:**")?);
        matches!(value, "READY" | "NOT-READY").then(|| value.to_string())
    });
    let lines: Vec<&str> = normalized.split('\n').collect();
    let Some(heading) = lines.iter().position(|line| {
        line.strip_prefix("### Findings")
            .is_some_and(|rest| trim(rest).is_empty())
    }) else {
        return Ok((verdict, Vec::new()));
    };
    let end = lines
        .iter()
        .enumerate()
        .skip(heading.saturating_add(1))
        .find(|(_, line)| line.starts_with("### "))
        .map_or(lines.len(), |(index, _)| index);
    let table: Vec<&str> = lines
        .get(heading.saturating_add(1)..end)
        .unwrap_or_default()
        .iter()
        .copied()
        .filter(|line| trim(line).starts_with('|'))
        .collect();
    let Some((header, body)) = table.split_first() else {
        return Ok((verdict, Vec::new()));
    };
    let headers = split_row(header);
    if wording::REVIEW_FINDING_COLUMNS
        .iter()
        .any(|name| !headers.iter().any(|found| found == name))
    {
        return Ok((verdict, Vec::new()));
    }
    let mut findings = Vec::new();
    for line in body.iter().skip(1) {
        findings.push(row(&split_row(line), &headers, artifact)?);
    }
    Ok((verdict, findings))
}

/// 表の 1 行を所見へ写す。
fn row(cells: &[String], headers: &[String], artifact: &str) -> Result<Finding, String> {
    let at = |name: &str| {
        headers
            .iter()
            .position(|header| header == name)
            .and_then(|index| cells.get(index))
            .map_or_else(String::new, |value| trim(value).to_string())
    };
    if cells.len() != headers.len() {
        let named = at("ID");
        let named = if named.is_empty() {
            "?".to_string()
        } else {
            named
        };
        if cells.len() > headers.len() {
            return Err(wording::review_row_extra_cells(
                artifact,
                &named,
                cells.len(),
                headers.len(),
            ));
        }
        let last = cells.last().map_or("", |value| value.as_str());
        let hint = if headers.last().is_some_and(|name| name == "Status") && is_finding_status(last)
        {
            wording::review_row_status_hint(last)
        } else {
            wording::REVIEW_ROW_MISSING_HINT.to_string()
        };
        return Err(wording::review_row_missing_cells(
            artifact,
            &named,
            cells.len(),
            headers,
            &hint,
        ));
    }
    let id = at("ID");
    if !is_finding_id(&id) {
        return Err(wording::invalid_finding_id(artifact, &id));
    }
    let status = at("Status");
    if !is_finding_status(&status) {
        return Err(wording::invalid_finding_status(artifact, &id, &status));
    }
    Ok(Finding {
        severity: at("Severity"),
        location: at("Location"),
        finding: at("Finding"),
        required_action: at("Required action"),
        id,
        status,
    })
}

/// `R-` と 1 桁以上の数字だけ（upstream `/^R-[0-9]+$/`）。
fn is_finding_id(value: &str) -> bool {
    value.strip_prefix("R-").is_some_and(|digits| {
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    })
}

/// 閉じた状態語彙（upstream `validReviewFindingStatus`）。
fn is_finding_status(value: &str) -> bool {
    matches!(value, "New" | "Unresolved" | "Resolved" | "Accepted risk")
        || value
            .strip_prefix("Rejected: ")
            .is_some_and(|reason| reason.starts_with(|char: char| !char.is_whitespace()))
}

/// Markdown 表の 1 行をセルへ割る（upstream `splitMarkdownRow`）。
fn split_row(line: &str) -> Vec<String> {
    let trimmed = trim(line);
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let body = inner.strip_suffix('|').unwrap_or(inner);
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut escaped = false;
    for char in body.chars() {
        if escaped {
            cell.push(char);
            escaped = false;
        } else if char == '\\' {
            escaped = true;
        } else if char == '|' {
            cells.push(trim(&cell).to_string());
            cell.clear();
        } else {
            cell.push(char);
        }
    }
    if escaped {
        cell.push('\\');
    }
    cells.push(trim(&cell).to_string());
    cells
}

/// 所見表（upstream `renderFindingsContext`）。
fn findings_context(contexts: &[Context]) -> String {
    if contexts.is_empty() {
        return wording::REVIEW_BRIEF_EMPTY_CONTEXT.to_string();
    }
    contexts.iter().map(block).collect::<Vec<_>>().join("\n\n")
}

/// 1 つのレビュー成果物ぶんの表。
fn block(context: &Context) -> String {
    let mut lines = vec![
        wording::review_artifact_heading(&context.artifact),
        String::new(),
        wording::REVIEW_FINDINGS_COLUMNS.to_string(),
        wording::REVIEW_FINDINGS_SEPARATOR.to_string(),
    ];
    for finding in &context.findings {
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            cell(&finding.id),
            cell(&finding.severity),
            cell(&finding.location),
            cell(&finding.finding),
            cell(&finding.required_action),
            cell(&finding.status),
        ));
    }
    if context.findings.is_empty() {
        lines.push(wording::REVIEW_FINDINGS_EMPTY_ROW.to_string());
    }
    lines.join("\n")
}

/// 表の 1 セルとして安全な綴り（upstream `markdownCell`）。
fn cell(value: &str) -> String {
    let folded = value
        .replace("\r\n", " ")
        .replace('\n', " ")
        .replace('|', "\\|");
    trim(&folded).to_string()
}

/// 成果物の表示パス（upstream `workspaceArtifactPath` — プロジェクト根からの相対）。
fn display(layout: &Layout, logical: &str) -> String {
    layout
        .record_dir()
        .and_then(|record| record.strip_prefix(layout.project_dir()).ok())
        .map_or_else(
            || logical.to_string(),
            |relative| format!("{}/{logical}", posix(relative)),
        )
}

/// パスを POSIX 綴りへ（upstream `toPosix`）。
fn posix(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// `..` と `.` を字句のまま畳む（upstream `resolve` の正規化に当たる）。
///
/// 畳まないと `<record>/../../secret` が記録の中に見えてしまう。
fn normalize(path: &Path) -> PathBuf {
    let mut folded = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                folded.pop();
            }
            Component::CurDir => (),
            other => folded.push(other.as_os_str()),
        }
    }
    folded
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    fn owned(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_string()).collect()
    }

    fn found(id: &str, status: &str) -> Finding {
        Finding {
            id: id.to_string(),
            severity: "Major".to_string(),
            location: "a.md > §1".to_string(),
            finding: "壊れている".to_string(),
            required_action: "直す".to_string(),
            status: status.to_string(),
        }
    }

    /// 値を伴わないフラグも、フラグでない位置引数も、対として受けない。
    #[test]
    fn the_flag_grammar_takes_pairs_and_nothing_else() {
        let parsed = flags(&owned(&[
            "--stage",
            "requirements-analysis",
            "--why",
            "first",
        ]))
        .expect("対として読める");
        assert_eq!(
            parsed.get("stage").map(String::as_str),
            Some("requirements-analysis")
        );
        assert_eq!(parsed.get("why").map(String::as_str), Some("first"));
        assert!(
            flags(&owned(&["--stage"])).is_err(),
            "値の無いフラグは受けない"
        );
        assert!(
            flags(&owned(&["stage", "x"])).is_err(),
            "位置引数は受けない"
        );
        assert!(flags(&[]).expect("空は空").is_empty());
    }

    /// フェンスの中の見出しは節の始まりにならない。
    #[test]
    fn a_review_heading_inside_a_fence_is_not_the_section() {
        let quoted = "# a\n\n```\n## Review\n**Verdict:** READY\n```\n";
        assert_eq!(review_section(quoted), None);
        let real = "# a\n\n## Review\n\n**Verdict:** READY\n";
        assert_eq!(
            review_section(real).as_deref(),
            Some("\n**Verdict:** READY\n")
        );
        // 次の見出しの手前で切れる。
        let bounded = "## Review\nx\n## Next\ny\n";
        assert_eq!(review_section(bounded).as_deref(), Some("x\n"));
        // 中身の無い節は「無い」と同じ。
        assert_eq!(review_section("## Review\n## Next\n"), None);
    }

    /// 判定と所見表を読む。表の宣言に無い列があれば所見は空になる。
    #[test]
    fn the_section_yields_the_verdict_and_the_findings_table() {
        let section = "\n**Verdict:** NOT-READY\n\n### Findings\n\n\
             | ID | Severity | Location | Finding | Required action | Status |\n\
             |---|---|---|---|---|---|\n\
             | R-01 | Major | a.md | 欠落 | 足す | New |\n\
             | R-02 | Minor | b.md | 誤り | 直す | Resolved |\n";
        let (verdict, findings) = parse_section(section, "a.md").expect("読める");
        assert_eq!(verdict.as_deref(), Some("NOT-READY"));
        assert_eq!(findings.len(), 2);
        assert_eq!(findings.first().map(|f| f.id.as_str()), Some("R-01"));
        assert_eq!(findings.get(1).map(|f| f.status.as_str()), Some("Resolved"));
        // 宣言に無い列がある表は所見として読まない。
        let (_, none) = parse_section(
            "### Findings\n| ID | Status |\n|---|---|\n| R-01 | New |\n",
            "a.md",
        )
        .expect("読める");
        assert!(none.is_empty(), "列が足りない表から所見を作らない");
        // `### Findings` が無ければ判定だけを返す。
        let (only, empty) = parse_section("**Verdict:** READY\n", "a.md").expect("読める");
        assert_eq!(only.as_deref(), Some("READY"));
        assert!(empty.is_empty());
    }

    /// セル数・ID・状態のいずれかが宣言と食い違えば、その行を名指して止める。
    #[test]
    fn a_malformed_row_names_the_artifact_and_the_row() {
        let header = "### Findings\n| ID | Severity | Location | Finding | Required action | Status |\n|---|---|---|---|---|---|\n";
        let short = format!("{header}| R-01 | Major | a.md | 欠落 | New |\n");
        let error = parse_section(&short, "a.md").expect_err("セル数が合わない");
        assert!(
            error.contains("a.md#R-01") && error.contains("header declares 6"),
            "{error}"
        );
        let extra = format!("{header}| R-01 | Major | a.md | 欠落 | 足す | New | 余り |\n");
        let error = parse_section(&extra, "a.md").expect_err("セルが多い");
        assert!(error.contains("unexpected extra cell"), "{error}");
        let bad_id = format!("{header}| 1 | Major | a.md | 欠落 | 足す | New |\n");
        let error = parse_section(&bad_id, "a.md").expect_err("ID が違う");
        assert!(error.contains("invalid finding ID"), "{error}");
        let bad_status = format!("{header}| R-01 | Major | a.md | 欠落 | 足す | Maybe |\n");
        let error = parse_section(&bad_status, "a.md").expect_err("状態が違う");
        assert!(error.contains("invalid finding status"), "{error}");
    }

    /// 状態語彙は閉じている。`Rejected:` だけが自由文を伴う。
    #[test]
    fn the_status_vocabulary_is_closed_except_for_a_stated_rejection() {
        for value in [
            "New",
            "Unresolved",
            "Resolved",
            "Accepted risk",
            "Rejected: 別案を採る",
        ] {
            assert!(is_finding_status(value), "{value}");
        }
        for value in ["", "Rejected", "Rejected: ", "rejected: x", "Done"] {
            assert!(!is_finding_status(value), "{value}");
        }
        assert!(is_finding_id("R-07") && !is_finding_id("R-") && !is_finding_id("X-1"));
    }

    /// セルは改行を畳み、区切りを逃がす。
    #[test]
    fn a_cell_folds_newlines_and_escapes_the_separator() {
        assert_eq!(cell(" a\r\nb\nc | d "), "a b c \\| d");
        assert_eq!(split_row("| a | b\\|c | |"), owned(&["a", "b|c", ""]));
        assert_eq!(split_row("a | b"), owned(&["a", "b"]));
    }

    /// 結びは、開いた所見 → 所見はあるが閉じている → 判定が NOT-READY → 何も無い、の順で決まる。
    #[test]
    fn the_outcome_follows_the_state_of_the_findings() {
        let with = |verdict: Option<&str>, findings: Vec<Finding>| {
            vec![Context {
                artifact: "a.md".to_string(),
                verdict: verdict.map(str::to_string),
                findings,
            }]
        };
        assert_eq!(
            outcome(&with(Some("NOT-READY"), vec![found("R-01", "New")])),
            wording::REVIEW_BRIEF_CONCERNS
        );
        assert_eq!(
            outcome(&with(Some("READY"), vec![found("R-01", "Resolved")])),
            wording::REVIEW_BRIEF_NO_OPEN
        );
        assert_eq!(
            outcome(&with(Some("NOT-READY"), Vec::new())),
            wording::REVIEW_BRIEF_INCOMPLETE
        );
        assert_eq!(
            outcome(&with(Some("READY"), Vec::new())),
            wording::REVIEW_BRIEF_CLEAR
        );
        assert_eq!(outcome(&[]), wording::REVIEW_BRIEF_CLEAR);
    }

    /// 文脈が無ければそう名乗り、所見の無い成果物は「所見なし」の 1 行を持つ。
    #[test]
    fn the_findings_context_states_an_empty_record_rather_than_showing_nothing() {
        assert_eq!(findings_context(&[]), wording::REVIEW_BRIEF_EMPTY_CONTEXT);
        let rendered = findings_context(&[
            Context {
                artifact: "a.md".to_string(),
                verdict: Some("READY".to_string()),
                findings: Vec::new(),
            },
            Context {
                artifact: "b.md".to_string(),
                verdict: Some("NOT-READY".to_string()),
                findings: vec![found("R-01", "New")],
            },
        ]);
        assert!(
            rendered.starts_with("**Review artifact:** `a.md`\n\n|"),
            "{rendered}"
        );
        assert!(
            rendered.contains(wording::REVIEW_FINDINGS_EMPTY_ROW),
            "{rendered}"
        );
        assert!(
            rendered.contains("| R-01 | Major | a.md > §1 | 壊れている | 直す | New |"),
            "{rendered}"
        );
        assert!(
            !rendered.ends_with('\n'),
            "末尾の空行は残さない: {rendered:?}"
        );
    }

    /// `..` を含む綴りは記録の外を指すものとして畳まれる。
    #[test]
    fn a_traversing_path_is_folded_before_it_is_compared() {
        assert_eq!(
            normalize(Path::new("/w/aidlc/intents/r/../../../etc/passwd")),
            PathBuf::from("/w/etc/passwd")
        );
        assert_eq!(normalize(Path::new("/w/./a/b")), PathBuf::from("/w/a/b"));
        assert_eq!(posix(Path::new("a/b")), "a/b");
    }
}
