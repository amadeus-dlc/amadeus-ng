//! plan-approval-guard — code-generation で、計画承認より前の生成を止める PreToolUse フック。
//!
//! 2.8.2 `hooks/aidlc-plan-approval-guard.ts` の native 版である。配布の TypeScript 版は
//! 自分の状態ファイル・受領の置き場を読むので、native の承認記録（イベントストア）を
//! 読めず、承認の後でも code-generation の書込と開発者の派遣を全部止める。
//!
//! 止めるのは 2 つだけである。
//!
//! - **開発者エージェントの派遣**（`Task` / `Agent` の `aidlc-developer-agent`）— 依頼文が
//!   承認対象の印（`AIDLC-STAGE: code-generation` または `AIDLC-UNIT: <unit>`）をちょうど
//!   1 つと、承認された `AIDLC-TESTING-CONTRACT: <hash>` を持ち、その対象の計画承認が現在の
//!   ものでなければ止める
//! - **ワークスペースの書換え**（`Write` / `Edit` / `MultiEdit` / `NotebookEdit` と、書込先を
//!   持つ `Bash`）— code-generation の記録ディレクトリの外へ書くのに、計画承認が現在の
//!   ものでなければ止める。記録ディレクトリの中（計画・テスト指示・質問・日誌）は承認を
//!   得るための作業なので通す
//!
//! 通すときは承認の受領を「生成開始」へ進める（2.8.2 `beginCodeGeneration`）。これで以降の
//! 書込でソースが変わっても承認は失効しない。
//!
//! code-generation の外、状態ファイルが無い、入力が読めない、読取専用の工具は、いずれも
//! 通す（fail-open）。`AIDLC_DISABLE_PLAN_APPROVAL_GUARD=1` で判定そのものを止める。
//!
//! # 2.8.2 との差（この build の範囲）
//!
//! - 書換えの承認対象は段階全体（zero-Unit）だけを見る。Unit ごとの書換えは Unit の
//!   計画承認を引かない（bugfix など Unit を切らない scope が対象）
//! - 書込先を特定できないシェル（`eval` など）と未知の工具は通す（2.8.2 は止める）
//! - 拒否の監査行 `PLAN_APPROVAL_BLOCKED` と無効化の `GUARD_DISABLED` は書かない
use super::testing_posture::{PlanApprovalState, approval, begin_generation};
use super::{Completion, Layout};
use core_command_domain::orchestration::PlanTarget;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// 稼働記録・drop の名前（フック名と同じ）。
const NAME: &str = "plan-approval-guard";
/// 守るステージ。
const GUARDED_STAGE: &str = "code-generation";
/// 守る派遣先。
const GUARDED_AGENT: &str = "aidlc-developer-agent";
/// 派遣の工具（2.8.2 `DISPATCH_TOOLS`）。
const DISPATCH_TOOLS: [&str; 2] = ["Task", "Agent"];
/// ファイルを直接書き換える工具（2.8.2 `WRITE_TOOLS`）。
const WRITE_TOOLS: [&str; 4] = ["Write", "Edit", "MultiEdit", "NotebookEdit"];

pub(super) async fn run(layout: &Layout, input: &str) -> Completion {
    if std::env::var("AIDLC_DISABLE_PLAN_APPROVAL_GUARD").as_deref() == Ok("1") {
        return Completion::silent();
    }
    // 稼働記録は判断より先で、失敗しても判断を変えない。
    let _ = super::observe_hook_health(layout, NAME).await;
    let Ok(value) = serde_json::from_str::<Value>(input) else {
        return Completion::silent();
    };
    let tool = value
        .get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let tool_input = value.get("tool_input").cloned().unwrap_or(Value::Null);
    let dispatch = DISPATCH_TOOLS.contains(&tool)
        && tool_input.get("subagent_type").and_then(Value::as_str) == Some(GUARDED_AGENT);
    let writes = tool == "Bash" || WRITE_TOOLS.contains(&tool);
    if !dispatch && !writes {
        return Completion::silent();
    }
    let Some(state) = layout
        .state_file()
        .and_then(|path| std::fs::read_to_string(path).ok())
    else {
        return Completion::silent();
    };
    let prompt = ["prompt", "description"]
        .iter()
        .filter_map(|key| tool_input.get(*key).and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    let markers = Markers::find(&prompt);
    let current = super::session_hooks::field(&state, "Current Stage")
        .map(|stage| normalize(&stage))
        .unwrap_or_default();
    if current != GUARDED_STAGE && !(dispatch && markers.any()) {
        return Completion::silent();
    }
    if dispatch {
        return guard_dispatch(layout, &markers).await;
    }
    guard_write(layout, input).await
}

/// 開発者エージェントの派遣を、依頼文の印と承認で判定する（2.8.2
/// `evaluatePlanApprovalDispatch`）。
async fn guard_dispatch(layout: &Layout, markers: &Markers) -> Completion {
    let mentioned = markers.mentioned();
    let target = match (markers.units.as_slice(), markers.stages.as_slice()) {
        ([unit], []) => PlanTarget::for_unit(unit).ok(),
        ([], [stage]) if stage == GUARDED_STAGE => Some(PlanTarget::stage_level()),
        _ => None,
    };
    let Some(target) = target else {
        return Completion::hook_denied(crate::wording::plan_dispatch_blocked(&mentioned, None));
    };
    let state = match approval(layout, &target).await {
        Ok(state) => state,
        Err(error) => return fail_closed(layout, &error).await,
    };
    let contract_matches = markers.contracts.len() == 1
        && state.contract_hash.as_deref() == markers.contracts.first().map(String::as_str);
    if !state.ok || !contract_matches {
        return Completion::hook_denied(crate::wording::plan_dispatch_blocked(
            &mentioned,
            detail(&state),
        ));
    }
    begin(layout, target).await
}

/// ワークスペースの書換えを、記録ディレクトリの内外と承認で判定する。
async fn guard_write(layout: &Layout, input: &str) -> Completion {
    let envelope = harness_claude::WriteToolEnvelope::parse(input, layout.project_dir());
    let Some(record) = layout.record_dir() else {
        return Completion::silent();
    };
    let approval_dir = record.join("construction").join(GUARDED_STAGE);
    let outside = envelope.targets().fold_left(None, |found, target| {
        found.or_else(|| {
            let path = PathBuf::from(target.as_str());
            (!within(&path, &approval_dir)).then(|| target.as_str().to_string())
        })
    });
    let Some(outside) = outside else {
        return Completion::silent();
    };
    let target = PlanTarget::stage_level();
    let state = match approval(layout, &target).await {
        Ok(state) => state,
        Err(error) => return fail_closed(layout, &error).await,
    };
    if !state.ok {
        return Completion::hook_denied(crate::wording::plan_mutation_blocked(
            &outside,
            detail(&state),
        ));
    }
    begin(layout, target).await
}

/// 通すと決めた呼出しで、承認の受領を生成開始へ進める。進められなければ止める。
async fn begin(layout: &Layout, target: PlanTarget) -> Completion {
    match begin_generation(layout, target).await {
        Ok(_) => Completion::silent(),
        Err(error) => Completion::hook_denied(crate::wording::plan_generation_unstartable(&error)),
    }
}

/// 評価そのものが失敗したら止める（2.8.2 も `failed closed`）。
async fn fail_closed(layout: &Layout, error: &str) -> Completion {
    let _ = super::record_hook_drop(layout, NAME, error).await;
    Completion::hook_denied(crate::wording::plan_authority_unavailable(error))
}

fn detail(state: &PlanApprovalState) -> Option<&str> {
    (!state.ok && !state.reason.is_empty()).then_some(state.reason.as_str())
}

/// `path` が `dir` の中か。
///
/// 字句で正規化（`..` を畳む）してから、在る先祖までを実体パスへ揃えて比べる — macOS の
/// `/var` → `/private/var` のように、同じ場所が 2 通りに綴られても取り違えない。
fn within(path: &Path, dir: &Path) -> bool {
    real(&crate::lexical_path::normalize(path))
        .starts_with(real(&crate::lexical_path::normalize(dir)))
}

/// 在る先祖を実体パスへ置き換えた綴り（在らない末尾はそのまま付け直す）。
fn real(path: &Path) -> PathBuf {
    let mut missing = Vec::new();
    let mut current = path.to_path_buf();
    loop {
        if let Ok(resolved) = std::fs::canonicalize(&current) {
            return missing
                .iter()
                .rev()
                .fold(resolved, |acc: PathBuf, part: &std::ffi::OsString| {
                    acc.join(part)
                });
        }
        let (Some(parent), Some(name)) = (current.parent(), current.file_name()) else {
            return path.to_path_buf();
        };
        missing.push(name.to_os_string());
        current = parent.to_path_buf();
    }
}

/// ステージ名の比較形（2.8.2 `normalizeStageName`）。
fn normalize(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

/// 依頼文の印（2.8.2 `promptUnitMarkers` / `promptStageMarkers` /
/// `promptTestingContractMarkers`）。同じ値の繰返しは 1 つに数える。
struct Markers {
    units: Vec<String>,
    stages: Vec<String>,
    contracts: Vec<String>,
}

impl Markers {
    fn find(text: &str) -> Markers {
        let mut markers = Markers {
            units: Vec::new(),
            stages: Vec::new(),
            contracts: Vec::new(),
        };
        for line in text.lines() {
            let Some((name, value)) = line.trim().split_once(':') else {
                continue;
            };
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
            let (list, value) = match name.trim() {
                "AIDLC-UNIT" => (&mut markers.units, value.to_string()),
                "AIDLC-STAGE" => (&mut markers.stages, normalize(value)),
                "AIDLC-TESTING-CONTRACT" => (&mut markers.contracts, value.to_string()),
                _ => continue,
            };
            if !list.contains(&value) {
                list.push(value);
            }
        }
        markers
    }

    const fn any(&self) -> bool {
        !(self.units.is_empty() && self.stages.is_empty() && self.contracts.is_empty())
    }

    /// 拒否文が名指す対象（Unit 名、または `stage:<slug>`）。
    fn mentioned(&self) -> Vec<String> {
        self.units
            .iter()
            .cloned()
            .chain(self.stages.iter().map(|stage| format!("stage:{stage}")))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_are_read_per_line_and_deduplicated() {
        let markers = Markers::find(
            "AIDLC-STAGE: Code Generation\nAIDLC-TESTING-CONTRACT: sha256:abc\n  AIDLC-STAGE: code-generation\nAIDLC-UNIT:\nnoise",
        );
        assert_eq!(markers.stages, vec!["code-generation"]);
        assert_eq!(markers.contracts, vec!["sha256:abc"]);
        assert!(markers.units.is_empty(), "空の値は印ではない");
        assert!(markers.any());
        assert_eq!(markers.mentioned(), vec!["stage:code-generation"]);
        assert!(!Markers::find("plain prompt").any());
    }

    #[test]
    fn only_paths_under_the_record_directory_are_inside() {
        let dir = Path::new("/w/aidlc/r/construction/code-generation");
        assert!(within(&dir.join("plan.md"), dir));
        assert!(!within(Path::new("/w/src/main.rs"), dir));
        assert!(!within(&dir.join("../../../../src/main.rs"), dir));
    }

    #[test]
    fn stage_names_compare_in_slug_form() {
        assert_eq!(normalize(" Code Generation "), "code-generation");
        assert_eq!(normalize("code-generation"), "code-generation");
    }
}
