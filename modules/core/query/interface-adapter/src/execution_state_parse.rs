//! 実行状態リードモデル `aidlc-state.md` の**純 parse** — 読み終えた本文をクエリモデル
//! [`ExecutionStateView`] へ写す (11-workspace §4 / 03 §5.3-5.6)。
//!
//! 状態ファイルは RMU が投影した**リードモデル**であり、読む・パースする責務はクエリ側に
//! ある (オーナー裁定 2026-08-30 — `coding-rules/cqrs-boundaries.md` 規則 6/7)。写す先は
//! **自前のクエリモデル**であって、コマンド側ドメインの集約ではない。
//!
//! ファイルの読取・パス解決は同クレートの reader ([`super::execution_state_reader`]) が行い、
//! 本モジュールは **fs 呼び出しゼロ**の変換だけを持つ (b25 の定義読取と同じ分業)。
//!
//! # 読む文法 (投影ライタの逆)
//!
//! | 取り出す値 | 行の形 |
//! |---|---|
//! | scope | `- **Scope**: <slug>` |
//! | カーソル | `- **Current Stage**: <slug>` |
//! | Status | `- **Status**: Running` \| `Completed` |
//! | park 位置 | `- **Parked At Stage**: <slug>` (無ければ未 park) |
//! | 最終更新 | `- **Last Updated**: <ISO 8601>` |
//! | ステージ行 | `### <PHASE> PHASE` 見出し配下の `- [<m>] <slug> — <…EXECUTE\|SKIP>` |
//!
//! フィールド行の文法は投影ライタと同じ `- **<Field>**:[ \t]*(.*)`、checkbox 行は
//! `- [<m>] <slug> — <rest>` である。**寛容パース**: 文法に一致しない行は黙って無視する
//! (upstream 同等)。閉集合 (`Status` / フェーズ見出し / マーカー / EXECUTE\|SKIP) だけは
//! 厳密に落とす — 未知値を `Unknown` 変種へ逃がさない (12 §10 表 #3 と同じ方針)。

use core_query_use_case::orchestration::{
    CheckboxState, ExecutionStateError, ExecutionStateView, ExecutionStatus, PhaseView,
    PlanActionView, ScopeSlugView, StageProgressView, StageSlugView,
};

/// `- **Scope**:` — scope 名。
const FIELD_SCOPE: &str = "Scope";
/// `- **Current Stage**:` — カーソル位置の slug。
const FIELD_CURRENT_STAGE: &str = "Current Stage";
/// `- **Status**:` — Running / Completed。
const FIELD_STATUS: &str = "Status";
/// `- **Parked At Stage**:` — park マーカーが記録する slug (無ければ未 park)。
const FIELD_PARKED_AT_STAGE: &str = "Parked At Stage";
/// `- **Last Updated**:` — 最終更新時刻 (逐語)。
const FIELD_LAST_UPDATED: &str = "Last Updated";

/// フェーズ見出しの接頭辞 (`### INITIALIZATION PHASE`)。
const PHASE_HEADING_PREFIX: &str = "### ";
/// フェーズ見出しの接尾辞。
const PHASE_HEADING_SUFFIX: &str = " PHASE";

/// 行末に書かれる計画側トークン 2 種 (upstream 逐語)。
const PLAN_SUFFIXES: [(&str, PlanActionView); 2] = [
    ("EXECUTE", PlanActionView::Execute),
    ("SKIP", PlanActionView::Skip),
];

/// 状態ファイルの parse 失敗。
///
/// 逐語文言そのものは持たず、**文言を組み立てる材料**を運ぶ
/// (`coding-rules/error-handling.md`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStateParseError {
    /// 必須のフィールド行が無い。
    MissingField {
        /// 見つからなかったフィールド名 (`- **<name>**:` の `<name>`)。
        field: String,
    },
    /// フィールドの値が閉集合・文法に合わない。
    InvalidField {
        /// 対象のフィールド名。
        field: String,
        /// 読めた生値 (逐語)。
        value: String,
    },
    /// Stage Progress 行が `### <PHASE> PHASE` 見出しより前に現れた
    /// (どのフェーズに属するか決まらない)。
    StageBeforePhaseHeading {
        /// 行が名指ししていた slug。
        stage: String,
    },
    /// Stage Progress 行の末尾が EXECUTE / SKIP のどちらでもない (実効プランが読めない)。
    MissingPlanSuffix {
        /// 行が名指ししていた slug。
        stage: String,
    },
    /// 読めたリードモデルがビューとして成立しない (行なし・カーソル不一致など)。
    NotAView(ExecutionStateError),
}

impl std::fmt::Display for ExecutionStateParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStateParseError::MissingField { field } => {
                write!(f, "missing field {field:?}")
            }
            ExecutionStateParseError::InvalidField { field, value } => {
                write!(f, "invalid field {field:?}: {value:?}")
            }
            ExecutionStateParseError::StageBeforePhaseHeading { stage } => {
                write!(f, "stage row {stage:?} before any phase heading")
            }
            ExecutionStateParseError::MissingPlanSuffix { stage } => {
                write!(f, "stage row {stage:?} has no EXECUTE/SKIP suffix")
            }
            ExecutionStateParseError::NotAView(inner) => write!(f, "{inner}"),
        }
    }
}

impl std::error::Error for ExecutionStateParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecutionStateParseError::NotAView(inner) => Some(inner),
            _ => None,
        }
    }
}

impl From<ExecutionStateError> for ExecutionStateParseError {
    fn from(inner: ExecutionStateError) -> ExecutionStateParseError {
        ExecutionStateParseError::NotAView(inner)
    }
}

/// 状態ファイルの本文をクエリモデルへ写す (**fs 呼び出しゼロ**)。
///
/// # Errors
///
/// 必須フィールドの欠落・閉集合違反・見出しのない Stage Progress 行・EXECUTE/SKIP の欠落・
/// ビューとして成立しない観測を [`ExecutionStateParseError`] で拒否する。
pub fn parse_execution_state(
    content: &str,
) -> Result<ExecutionStateView, ExecutionStateParseError> {
    let scope_raw = require_field(content, FIELD_SCOPE)?;
    let scope =
        ScopeSlugView::parse(&scope_raw).map_err(|_| ExecutionStateParseError::InvalidField {
            field: FIELD_SCOPE.to_string(),
            value: scope_raw.clone(),
        })?;
    let status_raw = require_field(content, FIELD_STATUS)?;
    let status = ExecutionStatus::parse(&status_raw).map_err(|_| {
        ExecutionStateParseError::InvalidField {
            field: FIELD_STATUS.to_string(),
            value: status_raw.clone(),
        }
    })?;
    let cursor = require_field(content, FIELD_CURRENT_STAGE)?;
    let parked_at = find_field(content, FIELD_PARKED_AT_STAGE).filter(|s| !s.is_empty());
    let last_updated = find_field(content, FIELD_LAST_UPDATED).unwrap_or_default();
    let stages = parse_stage_rows(content)?;
    Ok(ExecutionStateView::new(
        scope,
        status,
        &cursor,
        parked_at.as_deref(),
        last_updated,
        stages,
    )?)
}

/// Stage Progress の全行 (文書順)。フェーズは直前の `### <PHASE> PHASE` 見出しから取る。
fn parse_stage_rows(content: &str) -> Result<Vec<StageProgressView>, ExecutionStateParseError> {
    let mut phase: Option<PhaseView> = None;
    let mut rows = Vec::new();
    for line in content.lines() {
        if let Some(found) = parse_phase_heading(line) {
            phase = Some(found?);
            continue;
        }
        let Some((marker, slug, rest)) = split_checkbox_line(line) else {
            continue;
        };
        let Ok(slug) = StageSlugView::parse(slug) else {
            // slug 文法に合わない行は Stage Progress 行ではない (寛容パース)。
            continue;
        };
        let Some(phase) = phase else {
            return Err(ExecutionStateParseError::StageBeforePhaseHeading {
                stage: slug.as_str().to_string(),
            });
        };
        let plan = plan_of(rest).ok_or_else(|| ExecutionStateParseError::MissingPlanSuffix {
            stage: slug.as_str().to_string(),
        })?;
        rows.push(StageProgressView::new(slug, phase, marker, plan));
    }
    Ok(rows)
}

/// `### <PHASE> PHASE` 見出しの判定と復号。見出しでない行は `None`。
fn parse_phase_heading(line: &str) -> Option<Result<PhaseView, ExecutionStateParseError>> {
    let name = line
        .trim_end()
        .strip_prefix(PHASE_HEADING_PREFIX)?
        .strip_suffix(PHASE_HEADING_SUFFIX)?;
    Some(PhaseView::parse(&name.to_lowercase()).map_err(|_| {
        ExecutionStateParseError::InvalidField {
            field: "phase heading".to_string(),
            value: name.to_string(),
        }
    }))
}

/// `- [<m>] <slug> — <rest>` の 3 分割。文法に一致しない行は `None` (寛容パース)。
fn split_checkbox_line(line: &str) -> Option<(CheckboxState, &str, &str)> {
    let rest = line.strip_prefix("- [")?;
    let mut chars = rest.chars();
    let state = CheckboxState::from_marker(chars.next()?)?;
    let rest = chars.as_str().strip_prefix("] ")?;
    let dash = rest.find('—')?;
    let (slug_part, tail) = rest.split_at(dash);
    let slug = slug_part.trim_end_matches([' ', '\t']);
    if slug.is_empty() || slug.contains(char::is_whitespace) {
        return None;
    }
    let tail = tail.strip_prefix('—').unwrap_or(tail);
    Some((state, slug, tail.trim_start_matches([' ', '\t'])))
}

/// em dash 以降のテキストから実効プランを読む (先頭トークンが EXECUTE / SKIP)。
///
/// 投影ライタは `SKIP — <reason>` のように理由を続けて書くので、**接頭辞**で判定する
/// (`Checkboxes::with_suffix` が接尾辞で書き換えるのは行末が裸のトークンである場合のみ)。
fn plan_of(rest: &str) -> Option<PlanActionView> {
    let trimmed = rest.trim();
    PLAN_SUFFIXES.iter().find_map(|(token, action)| {
        let after = trimmed.strip_prefix(token)?;
        // 直後は行末か区切り (`:` / 空白) — `SKIPPED` のような別語に食い込ませない。
        (after.is_empty() || after.starts_with([':', ' ', '\t', '—', '-'])).then_some(*action)
    })
}

/// フィールド行の値 (`getField` 相当 — 最初に一致した行の値を trim して返す)。
fn find_field(content: &str, field: &str) -> Option<String> {
    let prefix = format!("- **{field}**:");
    content.lines().find_map(|line| {
        line.strip_prefix(&prefix)
            .map(|value| value.trim_matches([' ', '\t']).to_string())
    })
}

/// 必須フィールド行の値。
fn require_field(content: &str, field: &str) -> Result<String, ExecutionStateParseError> {
    find_field(content, field).ok_or_else(|| ExecutionStateParseError::MissingField {
        field: field.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_query_use_case::orchestration::StageIndex;

    /// 出荷テンプレートと同じ節構成の最小状態ファイル。
    fn state_file() -> String {
        [
            "# AI-DLC State Tracking",
            "",
            "## Project Information",
            "- **Project**: demo",
            "- **Scope**: classic",
            "- **State Version**: 8",
            "",
            "## Stage Progress",
            "<!-- Checkbox states: [ ] pending … -->",
            "",
            "### INITIALIZATION PHASE",
            "- [x] workspace-scaffold — EXECUTE",
            "- [x] state-init — EXECUTE",
            "",
            "### IDEATION PHASE",
            "- [ ] intent-capture — SKIP",
            "",
            "### INCEPTION PHASE",
            "- [-] domain-design — EXECUTE",
            "- [ ] contract-design — EXECUTE",
            "",
            "## Current Status",
            "- **Lifecycle Phase**: INCEPTION",
            "- **Current Stage**: domain-design",
            "- **Next Stage**: contract-design",
            "- **Status**: Running",
            "- **Last Updated**: 2026-08-29T16:36:24Z",
            "",
        ]
        .join("\n")
    }

    #[test]
    fn the_shipping_shape_reads_back_into_the_query_model() {
        let view = parse_execution_state(&state_file()).unwrap();
        assert_eq!(view.scope().as_str(), "classic");
        assert_eq!(view.status(), ExecutionStatus::Running);
        assert_eq!(view.stage_count(), 5);
        assert_eq!(view.cursor().to_usize(), 3, "domain-design は 4 行目");
        assert_eq!(view.parked_at(), None);
        assert_eq!(view.last_updated(), "2026-08-29T16:36:24Z");

        let at = |i: usize| view.stage_index(i).unwrap();
        assert_eq!(view.checkbox(at(0)), Some(CheckboxState::Completed));
        assert_eq!(view.checkbox(at(3)), Some(CheckboxState::InProgress));
        assert_eq!(view.effective_plan(at(2)), Some(PlanActionView::Skip));
        assert!(!view.is_gated(at(0)), "initialization は非ゲート");
        assert!(view.is_gated(at(2)), "ideation はゲート付き");
        assert_eq!(view.next_in_scope(at(3)).map(StageIndex::to_usize), Some(4));
    }

    #[test]
    fn a_park_marker_is_read_from_the_runtime_state_section() {
        let parked = state_file().replace(
            "## Current Status",
            "- **Parked**: 2026-08-30T00:00:00Z\n- **Parked At Stage**: domain-design\n\n## Current Status",
        );
        let view = parse_execution_state(&parked).unwrap();
        assert_eq!(view.parked_at().map(StageIndex::to_usize), Some(3));
        assert!(view.parked_active());
        assert!(!view.accepts_commands());
    }

    #[test]
    fn a_skip_row_may_carry_its_reason_after_the_token() {
        let annotated = state_file().replace(
            "- [ ] intent-capture — SKIP",
            "- [ ] intent-capture — SKIP: out of scope for classic",
        );
        let view = parse_execution_state(&annotated).unwrap();
        let at = view.stage_index(2).unwrap();
        assert_eq!(view.effective_plan(at), Some(PlanActionView::Skip));
    }

    #[test]
    fn lines_that_are_not_stage_rows_are_ignored() {
        // 見出し直下の自由記述 (`Per unit: [TBD]`) や checkbox 文法に合わない行は無視する。
        let noisy = state_file().replace(
            "### INCEPTION PHASE",
            "### INCEPTION PHASE\nPer unit: [TBD]\n- [z] not-a-marker — EXECUTE\n- [x] two words — EXECUTE",
        );
        let view = parse_execution_state(&noisy).unwrap();
        assert_eq!(view.stage_count(), 5, "無視された 3 行は数に入らない");
    }

    #[test]
    fn a_missing_required_field_is_refused_with_its_name() {
        for (field, line) in [
            (FIELD_SCOPE, "- **Scope**: classic"),
            (FIELD_STATUS, "- **Status**: Running"),
            (FIELD_CURRENT_STAGE, "- **Current Stage**: domain-design"),
        ] {
            let without = state_file().replace(line, "");
            assert_eq!(
                parse_execution_state(&without),
                Err(ExecutionStateParseError::MissingField {
                    field: field.to_string()
                })
            );
        }
    }

    #[test]
    fn a_closed_set_violation_is_refused_with_its_material() {
        let bad_status = state_file().replace("- **Status**: Running", "- **Status**: Parked");
        assert_eq!(
            parse_execution_state(&bad_status),
            Err(ExecutionStateParseError::InvalidField {
                field: "Status".to_string(),
                value: "Parked".to_string()
            })
        );
        let bad_scope = state_file().replace("- **Scope**: classic", "- **Scope**: Classic");
        assert_eq!(
            parse_execution_state(&bad_scope),
            Err(ExecutionStateParseError::InvalidField {
                field: "Scope".to_string(),
                value: "Classic".to_string()
            })
        );
        let bad_phase = state_file().replace("### IDEATION PHASE", "### DAYDREAMING PHASE");
        assert_eq!(
            parse_execution_state(&bad_phase),
            Err(ExecutionStateParseError::InvalidField {
                field: "phase heading".to_string(),
                value: "DAYDREAMING".to_string()
            })
        );
    }

    #[test]
    fn a_stage_row_without_a_phase_heading_is_refused() {
        let orphan = state_file().replace("### INITIALIZATION PHASE\n", "");
        assert_eq!(
            parse_execution_state(&orphan),
            Err(ExecutionStateParseError::StageBeforePhaseHeading {
                stage: "workspace-scaffold".to_string()
            })
        );
    }

    #[test]
    fn a_stage_row_without_a_plan_suffix_is_refused() {
        let bare = state_file().replace(
            "- [-] domain-design — EXECUTE",
            "- [-] domain-design — Domain Design",
        );
        assert_eq!(
            parse_execution_state(&bare),
            Err(ExecutionStateParseError::MissingPlanSuffix {
                stage: "domain-design".to_string()
            })
        );
    }

    #[test]
    fn a_cursor_outside_the_stage_rows_is_not_a_view() {
        let ghost = state_file().replace(
            "- **Current Stage**: domain-design",
            "- **Current Stage**: ghost",
        );
        assert_eq!(
            parse_execution_state(&ghost),
            Err(ExecutionStateParseError::NotAView(
                ExecutionStateError::UnknownCursor {
                    stage: "ghost".to_string()
                }
            ))
        );
    }

    #[test]
    fn a_file_without_any_stage_row_is_not_a_view() {
        let empty = [
            "- **Scope**: classic",
            "- **Status**: Running",
            "- **Current Stage**: domain-design",
        ]
        .join("\n");
        assert_eq!(
            parse_execution_state(&empty),
            Err(ExecutionStateParseError::NotAView(
                ExecutionStateError::NoStages
            ))
        );
    }

    #[test]
    fn the_rejections_carry_material_not_wording() {
        assert_eq!(
            ExecutionStateParseError::MissingField {
                field: "Scope".to_string()
            }
            .to_string(),
            "missing field \"Scope\""
        );
        assert_eq!(
            ExecutionStateParseError::InvalidField {
                field: "Status".to_string(),
                value: "Parked".to_string()
            }
            .to_string(),
            "invalid field \"Status\": \"Parked\""
        );
        assert_eq!(
            ExecutionStateParseError::StageBeforePhaseHeading {
                stage: "a".to_string()
            }
            .to_string(),
            "stage row \"a\" before any phase heading"
        );
        assert_eq!(
            ExecutionStateParseError::MissingPlanSuffix {
                stage: "a".to_string()
            }
            .to_string(),
            "stage row \"a\" has no EXECUTE/SKIP suffix"
        );
        let wrapped: ExecutionStateParseError = ExecutionStateError::NoStages.into();
        assert_eq!(wrapped.to_string(), "no stage progress rows");
        let boxed: Box<dyn std::error::Error> = Box::new(wrapped);
        assert!(boxed.source().is_some(), "内側の拒否は source で辿れる");
        let leaf: Box<dyn std::error::Error> = Box::new(ExecutionStateParseError::MissingField {
            field: "Scope".to_string(),
        });
        assert!(leaf.source().is_none());
    }

    #[test]
    fn a_tab_padded_field_value_is_trimmed() {
        let padded = state_file().replace("- **Scope**: classic", "- **Scope**:\tclassic\t");
        assert_eq!(
            parse_execution_state(&padded).unwrap().scope().as_str(),
            "classic"
        );
    }

    #[test]
    fn an_empty_parked_at_stage_line_means_not_parked() {
        let blank = state_file().replace(
            "## Current Status",
            "- **Parked At Stage**:\n\n## Current Status",
        );
        assert_eq!(parse_execution_state(&blank).unwrap().parked_at(), None);
    }
}
