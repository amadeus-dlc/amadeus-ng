//! **純粋投影核** — ドメインイベントの列をリードモデルへ写す（C5 の投影規則）。
//!
//! ここが知っているのはドメインイベントとリードモデルの 2 つだけである。`JournalReader`・
//! SQLite 接続・チェックポイントは**署名にも本体にも現れない**（`coding-rules/cqrs-boundaries.md`
//! の禁止パターン「純粋投影核が取得の都合を知る」）。取得ループと投影核の二層を潰さないのは、
//! 投影の規則だけを単体でテストできるようにするためである。
//!
//! # 冪等（NFR3）
//!
//! 同じ入力からは常に同じバイトが出る — 壁時計を読まず（監査行の時刻はイベントの発生時刻）、
//! 乱数も環境変数も見ない。二度描かない保証はチェックポイントが与えるので、投影核自身は
//! 「渡された列を順に写す」だけでよい。

use core_domain::orchestration::{
    GateApproved, GateOpened, GateRejected, Jumped, Parked, Recomposed, StageRevised, Started,
    WorkflowExecutionEvent,
};
use core_domain::workspace::{
    AuditFieldKey, AuditFieldKeyError, AuditFields, CheckboxState, CheckboxUpdateError,
    with_checkbox_marker,
};

use audit_events::EventType;
use chrono::{DateTime, Utc};

use super::audit_block::render_audit_block;
use super::read_model::ReadModel;
use super::state_writers::{FieldNotFound, with_field};
use crate::orchestration::JournalEntry;

/// 状態ファイルのフィールド名（逐語 — upstream の bullet ラベル）。
const REVISION_COUNT_FIELD: &str = "Revision Count";
/// 同上。
const AUTONOMY_MODE_FIELD: &str = "Construction Autonomy Mode";

/// 監査行のフィールドキー（逐語 — upstream の `**<key>**:`）。
const STAGE_KEY: &str = "Stage";
/// 同上。監査行では小文字 c、状態ファイルでは大文字 C である（upstream の非対称）。
const REVISION_COUNT_KEY: &str = "Revision count";
/// 同上。
const FEEDBACK_KEY: &str = "Feedback";
/// 同上。
const DETAILS_KEY: &str = "Details";
/// 同上。**この 1 つだけ upstream ゴールデン未採取**である — `cli/set-autonomy` のゴールデンは
/// 失敗経路（`ERROR_LOGGED`）しか捉えておらず、成功時の `AUTONOMY_MODE_SET` 行が無い。
/// 状態ファイル側（`Construction Autonomy Mode`）はその失敗文言が逐語で固定しているが、
/// 監査行のフィールドキーは U1 の追加採取待ちである（報告書のドリフト欄に記載）。
const MODE_KEY: &str = "Mode";

/// 再入時の逐語（upstream `report --result revised`）。
const REENTRY_DETAILS: &str = "Re-entering gate after revision";

/// 投影の失敗（材料のみ — 文言はアダプタ層）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionError {
    /// 状態ファイルに書き換え先のフィールド行が無い。
    ///
    /// 無言 no-op は検出不能なドリフトなので、upstream 逐語の拒否文言を添えて止める。
    StateField(FieldNotFound),
    /// 状態ファイルに対象ステージのチェックボックス行が無い。
    Checkbox(CheckboxUpdateError),
    /// 監査行のフィールドキーが文法外だった（材料の綴りの誤り）。
    AuditFieldKey(AuditFieldKeyError),
    /// リードモデルがスコープを覚えていない（`**Scope**:` 行を描けない）。
    ScopeUnknown,
    /// 状態ファイルに park マーカーの置き場（`## Runtime State`）が無い。
    ParkSectionMissing,
    /// この行を描くにはワークフロー定義（ステージグラフ）が要る。
    ///
    /// `STAGE_STARTED` の `**Agent**:` は `StageNode::lead_agent()` の値であり、ドメイン
    /// イベントは定義を `definition_id` + `definition_revision` で間接参照するだけで詳細を
    /// 運ばない（ADR-008）。RMU に定義読取の口を与えるか、イベントへ焼き込むかは
    /// **未裁定**である（contract-summary §4 が U4 へ持ち越した項目）。裁定が降りるまで、
    /// 誤ったバイトを書くのではなくここで止める。
    DefinitionLookupRequired {
        /// 描けなかった `STAGE_STARTED` の対象ステージ。
        stage: String,
    },
}

impl core::fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProjectionError::StateField(inner) => write!(f, "state field: {}", inner.message()),
            ProjectionError::Checkbox(CheckboxUpdateError::MissingStage(slug)) => {
                write!(f, "checkbox: missing stage {slug}")
            }
            ProjectionError::AuditFieldKey(inner) => write!(f, "audit field key: {inner}"),
            ProjectionError::ScopeUnknown => f.write_str("scope unknown"),
            ProjectionError::ParkSectionMissing => f.write_str("park section missing"),
            ProjectionError::DefinitionLookupRequired { stage } => {
                write!(f, "definition lookup required: stage {stage}")
            }
        }
    }
}

impl std::error::Error for ProjectionError {}

impl From<FieldNotFound> for ProjectionError {
    fn from(inner: FieldNotFound) -> ProjectionError {
        ProjectionError::StateField(inner)
    }
}

impl From<CheckboxUpdateError> for ProjectionError {
    fn from(inner: CheckboxUpdateError) -> ProjectionError {
        ProjectionError::Checkbox(inner)
    }
}

impl From<AuditFieldKeyError> for ProjectionError {
    fn from(inner: AuditFieldKeyError) -> ProjectionError {
        ProjectionError::AuditFieldKey(inner)
    }
}

/// 監査行のフィールドキーを組む（綴りはこのファイルの定数が正本）。
fn key(raw: &str) -> Result<AuditFieldKey, ProjectionError> {
    AuditFieldKey::parse(raw).map_err(ProjectionError::from)
}

/// `Recomposed` の監査行の材料 — 状態面の裁定待ちで**まだ本体から呼ばれない**。
///
/// 行そのものはイベントとリードモデルの記憶だけで導けており、逐語をユニットテストで固定して
/// ある。裁定が降りたら `project_one` の分岐をここへ戻すだけで済むよう、消さずに置いておく。
mod recomposed_row {
    // 状態面（ステージ番号・EXECUTE/SKIP 接尾辞）の裁定が降りるまで、本体からの呼出は無い。
    // 逐語は下のユニットテストが固定しており、消すと採取済みのゴールデン知識が失われる。
    // `expect` ではなく `allow` なのは、テストビルドでは実際に使われるため
    // （`expect` は「未使用であること」を要求してしまい、test 側で unfulfilled になる）。
    #![allow(
        dead_code,
        reason = "Recomposed の状態面が未裁定のあいだ本体から呼ばれない (裁定後に project_one の分岐を戻す)"
    )]

    use super::{
        AuditFields, DateTime, EventType, ProjectionError, ReadModel, Recomposed, Utc, key,
        render_audit_block,
    };

    /// 監査行のフィールドキー（逐語 — upstream の `**<key>**:`）。
    pub(super) const SCOPE_KEY: &str = "Scope";
    /// 同上。
    pub(super) const STAGES_SKIPPED_KEY: &str = "Stages skipped";
    /// 同上。
    pub(super) const STAGES_ADDED_KEY: &str = "Stages added";
    /// 同上。
    pub(super) const STAGES_IN_SCOPE_KEY: &str = "Stages in Scope";
    /// 空のステージ集合を描く逐語（upstream `recompose` の `**Stages added**: none`）。
    pub(super) const NONE_LITERAL: &str = "none";

    /// ステージ集合を `**Stages ...**:` の値へ描く（空は逐語 `none`）。
    pub(super) fn stage_list(stages: &[core_domain::workflow_definition::StageSlug]) -> String {
        if stages.is_empty() {
            return NONE_LITERAL.to_string();
        }
        stages
            .iter()
            .map(|slug| slug.as_str().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// `Recomposed` の**監査行だけ**を描く（状態面は未裁定 — 上の分岐を参照）。
    ///
    /// 行そのものはイベントとリードモデルの記憶だけで導けるので、逐語をここで固定しておく。
    /// 状態面の裁定が降りたら、この関数を呼ぶ本体の分岐を戻す。
    ///
    /// # Errors
    ///
    /// リードモデルがスコープを覚えていなければ `ScopeUnknown`。
    pub(super) fn recomposed_audit_row(
        recomposed: &Recomposed,
        at: &DateTime<Utc>,
        read_model: &ReadModel,
    ) -> Result<String, ProjectionError> {
        let scope = read_model.scope().ok_or(ProjectionError::ScopeUnknown)?;
        let fields = AuditFields::new()
            .with(key(SCOPE_KEY)?, &scope)
            .with(key(STAGES_SKIPPED_KEY)?, &stage_list(recomposed.skipped()))
            .with(key(STAGES_ADDED_KEY)?, &stage_list(recomposed.added()))
            .with(
                key(STAGES_IN_SCOPE_KEY)?,
                &recomposed.stages_in_scope().len().to_string(),
            );
        Ok(render_audit_block(EventType::Recomposed, at, &fields))
    }
}

/// 純粋投影核 — 差分のジャーナル行をリードモデルへ写す。
///
/// 入口はドメインイベント 1 本である（`JournalEntry` が運ぶのはイベントと、その行が持つ
/// 材料だけ）。集約も Repository もストアのエラーもここには現れない。
///
/// # Errors
///
/// 状態ファイルに書き換え先が無い（`StateField` / `Checkbox`）、リードモデルがスコープを
/// 覚えていない（`ScopeUnknown`）、ワークフロー定義が要る行に当たった
/// （`DefinitionLookupRequired`）を返す。
pub fn project(
    entries: &[JournalEntry],
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    for entry in entries {
        project_one(entry.event(), entry.occurred_at(), read_model)?;
    }
    Ok(())
}

fn project_one(
    event: &WorkflowExecutionEvent,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    match event {
        WorkflowExecutionEvent::GateOpened(opened) => gate_opened(opened, at, read_model),
        WorkflowExecutionEvent::GateRejected(rejected) => gate_rejected(rejected, at, read_model),
        WorkflowExecutionEvent::StageRevised(revised) => stage_revised(revised, at, read_model),
        WorkflowExecutionEvent::Parked(parked) => parked_event(parked, at, read_model),
        WorkflowExecutionEvent::Unparked => {
            unparked(at, read_model);
            Ok(())
        }
        // 以下は `STAGE_STARTED` の `**Agent**:`（ワークフロー定義由来）を含むため未裁定。
        WorkflowExecutionEvent::Started(started) => Err(started_blocked(started)),
        WorkflowExecutionEvent::StageCompleted(completed) => Err(next_stage_blocked(
            completed.next_stage().map(|slug| slug.as_str()),
        )),
        WorkflowExecutionEvent::GateApproved(approved) => Err(gate_approved_blocked(approved)),
        WorkflowExecutionEvent::StageSkipped(skipped) => Err(next_stage_blocked(
            skipped.next_stage().map(|slug| slug.as_str()),
        )),
        WorkflowExecutionEvent::Jumped(jumped) => Err(jumped_blocked(jumped)),
        // 監査行は導けるが、状態面は `Stages to Execute` / `Stages to Skip` の**ステージ番号**
        // （`4.5 (incident-response)`）と各行の EXECUTE/SKIP 接尾辞の書き換えを要する。番号も
        // 接尾辞もワークフロー定義側の材料であり、同じ未裁定に当たる。
        WorkflowExecutionEvent::Recomposed(recomposed) => Err(recomposed_blocked(recomposed)),
        WorkflowExecutionEvent::AutonomyModeSet(mode) => {
            autonomy_mode_set(mode.mode(), at, read_model)
        }
    }
}

/// `GateOpened` → `STAGE_AWAITING_APPROVAL`、チェックボックス `[-]` → `[?]`。
fn gate_opened(
    opened: &GateOpened,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let fields = AuditFields::new().with(key(STAGE_KEY)?, opened.stage().as_str());
    read_model.append_audit(&render_audit_block(
        EventType::StageAwaitingApproval,
        at,
        &fields,
    ));
    set_checkbox(
        read_model,
        opened.stage().as_str(),
        CheckboxState::AwaitingApproval,
    )
}

/// `GateRejected` → `GATE_REJECTED` + `STAGE_REVISING`、`[?]` → `[R]`、`Revision Count`。
fn gate_rejected(
    rejected: &GateRejected,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let stage = rejected.stage().as_str();
    let feedback = rejected.feedback().unwrap_or_default();
    let revisions = rejected.revision_count().to_string();

    let mut gate = AuditFields::new().with(key(STAGE_KEY)?, stage);
    if rejected.feedback().is_some() {
        gate = gate.with(key(FEEDBACK_KEY)?, feedback);
    }
    read_model.append_audit(&render_audit_block(EventType::GateRejected, at, &gate));

    let mut revising = AuditFields::new()
        .with(key(STAGE_KEY)?, stage)
        .with(key(REVISION_COUNT_KEY)?, &revisions);
    if rejected.feedback().is_some() {
        revising = revising.with(key(FEEDBACK_KEY)?, feedback);
    }
    read_model.append_audit(&render_audit_block(EventType::StageRevising, at, &revising));

    set_checkbox(read_model, stage, CheckboxState::Revising)?;
    set_field(read_model, REVISION_COUNT_FIELD, &revisions)
}

/// `StageRevised` → `STAGE_AWAITING_APPROVAL`（再入の逐語つき）、`[R]` → `[?]`。
fn stage_revised(
    revised: &StageRevised,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let fields = AuditFields::new()
        .with(key(STAGE_KEY)?, revised.stage().as_str())
        .with(key(DETAILS_KEY)?, REENTRY_DETAILS);
    read_model.append_audit(&render_audit_block(
        EventType::StageAwaitingApproval,
        at,
        &fields,
    ));
    set_checkbox(
        read_model,
        revised.stage().as_str(),
        CheckboxState::AwaitingApproval,
    )
}

/// `Parked` → `WORKFLOW_PARKED`、park マーカーの設置。
fn parked_event(
    parked: &Parked,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let fields = AuditFields::new().with(key(STAGE_KEY)?, parked.stage().as_str());
    read_model.append_audit(&render_audit_block(EventType::WorkflowParked, at, &fields));
    park_marker::set(read_model, parked.stage().as_str(), at)
}

/// `Unparked` → `WORKFLOW_UNPARKED`（フィールド無し）、park マーカーの除去。
fn unparked(at: &DateTime<Utc>, read_model: &mut ReadModel) {
    read_model.append_audit(&render_audit_block(
        EventType::WorkflowUnparked,
        at,
        &AuditFields::new(),
    ));
    park_marker::clear(read_model);
}

/// 同上（状態面が定義を要するため未裁定）。
fn recomposed_blocked(recomposed: &Recomposed) -> ProjectionError {
    ProjectionError::DefinitionLookupRequired {
        stage: recomposed
            .skipped()
            .first()
            .map_or_else(|| "-".to_string(), |slug| slug.as_str().to_string()),
    }
}

/// `AutonomyModeSet` → `AUTONOMY_MODE_SET`、`Construction Autonomy Mode`。
fn autonomy_mode_set(
    mode: core_domain::orchestration::AutonomyMode,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let fields = AuditFields::new().with(key(MODE_KEY)?, mode.as_state_field());
    read_model.append_audit(&render_audit_block(EventType::AutonomyModeSet, at, &fields));
    set_field(read_model, AUTONOMY_MODE_FIELD, mode.as_state_field())
}

/// 未裁定の理由を材料つきで返す（`Started` は init 3 ステージの `STAGE_STARTED` を含む）。
fn started_blocked(started: &Started) -> ProjectionError {
    ProjectionError::DefinitionLookupRequired {
        stage: started.stages().first().map_or_else(
            || "-".to_string(),
            |entry| entry.slug().as_str().to_string(),
        ),
    }
}

/// 同上（次ステージの `STAGE_STARTED` を描けない）。
fn next_stage_blocked(next: Option<&str>) -> ProjectionError {
    ProjectionError::DefinitionLookupRequired {
        stage: next.unwrap_or("-").to_string(),
    }
}

/// 同上。
fn gate_approved_blocked(approved: &GateApproved) -> ProjectionError {
    next_stage_blocked(approved.next_stage().map(|slug| slug.as_str()))
}

/// 同上。
fn jumped_blocked(jumped: &Jumped) -> ProjectionError {
    ProjectionError::DefinitionLookupRequired {
        stage: jumped.target().as_str().to_string(),
    }
}

/// チェックボックスのマーカーだけを書き換える（接尾辞には触れない）。
fn set_checkbox(
    read_model: &mut ReadModel,
    slug: &str,
    state: CheckboxState,
) -> Result<(), ProjectionError> {
    let next = with_checkbox_marker(read_model.state(), slug, state)?;
    read_model.replace_state(next);
    Ok(())
}

/// 状態ファイルのフィールド行を書き換える（不在は拒否 — 無言 no-op は検出不能なドリフト）。
fn set_field(read_model: &mut ReadModel, field: &str, value: &str) -> Result<(), ProjectionError> {
    let next = with_field(read_model.state(), field, value)?;
    read_model.replace_state(next);
    Ok(())
}

/// park マーカー — `## Runtime State` セクションの**末尾**（次の見出しの直前）に 2 行、
/// unpark で 2 行とも消す。
///
/// # なぜ `with_field_or_insert` を使わないのか
///
/// あちらは挿入位置をセクション末尾の**空行より前**へ巻き戻す（upstream `setOrInsertField`
/// の挙動）。park マーカーはそうならない — ゴールデン `cli/park/park/state.diff` の実バイトは
/// 空行の**あと**、次の `## ` 見出しの直前に 2 行が入っている。別の書き手なので別に実装する。
mod park_marker {
    use super::{ProjectionError, ReadModel};
    use chrono::{DateTime, SecondsFormat, Utc};

    /// マーカーを差し込むセクション見出し。
    const HEADING: &str = "## Runtime State";
    /// 停止時刻のフィールド行の接頭辞。
    const PARKED_PREFIX: &str = "- **Parked**:";
    /// 停止したステージのフィールド行の接頭辞。
    const PARKED_AT_STAGE_PREFIX: &str = "- **Parked At Stage**:";

    /// park マーカーを設置する（既にあれば置き直す — 冪等）。
    pub(super) fn set(
        read_model: &mut ReadModel,
        stage: &str,
        at: &DateTime<Utc>,
    ) -> Result<(), ProjectionError> {
        let cleared = removed(read_model.state());
        let lines: Vec<&str> = cleared.lines().collect();
        let start = lines
            .iter()
            .position(|line| line.trim_end() == HEADING)
            .ok_or(ProjectionError::ParkSectionMissing)?;
        // セクション末尾 = 次の `## ` 見出しの直前（末尾の空行は**そのまま**残す）。
        let end = lines
            .iter()
            .enumerate()
            .skip(start + 1)
            .find(|(_, line)| line.starts_with("## "))
            .map_or(lines.len(), |(index, _)| index);

        let stamp = at.to_rfc3339_opts(SecondsFormat::Secs, true);
        let mut out: Vec<String> = lines.iter().map(|line| (*line).to_string()).collect();
        out.splice(
            end..end,
            [
                format!("{PARKED_PREFIX} {stamp}"),
                format!("{PARKED_AT_STAGE_PREFIX} {stage}"),
            ],
        );
        read_model.replace_state(rejoin(&out, &cleared));
        Ok(())
    }

    /// park マーカーを除去する（不在は no-op — 二重 unpark で落ちない）。
    pub(super) fn clear(read_model: &mut ReadModel) {
        let next = removed(read_model.state());
        read_model.replace_state(next);
    }

    /// マーカー 2 行を落とした本文。
    fn removed(content: &str) -> String {
        let out: Vec<String> = content
            .lines()
            .filter(|line| {
                !line.starts_with(PARKED_PREFIX) && !line.starts_with(PARKED_AT_STAGE_PREFIX)
            })
            .map(|line| line.to_string())
            .collect();
        rejoin(&out, content)
    }

    /// 末尾改行の有無を元の本文に合わせて行を綴じ直す。
    fn rejoin(lines: &[String], original: &str) -> String {
        let mut joined = lines.join("\n");
        if original.ends_with('\n') {
            joined.push('\n');
        }
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::orchestration::{AutonomyMode, GateApproved, StageCompleted, StageSkipped};
    use core_domain::workflow_definition::StageSlug;

    const STATE: &str = "\
## Project Information
- **Scope**: classic

## Scope Configuration
- **Total Stages**: 25

## Runtime State
- **Revision Count**: 0

## Stage Progress

### INCEPTION PHASE
- [-] practices-discovery — EXECUTE
- [ ] requirements-analysis — EXECUTE
";

    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-08-21T09:14:07Z")
            .expect("固定の ISO 8601")
            .with_timezone(&Utc)
    }

    fn slug(value: &str) -> StageSlug {
        StageSlug::parse(value).expect("テストの slug は文法内")
    }

    fn model() -> ReadModel {
        ReadModel::new(STATE)
    }

    fn entry(event: WorkflowExecutionEvent) -> JournalEntry {
        JournalEntry::new(
            crate::orchestration::GlobalSeqNr::new(1),
            core_domain::orchestration::IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6")
                .expect("UUIDv7"),
            1,
            at(),
            event,
        )
    }

    /// 未裁定分岐の材料に使う `Started`（集約を通さず直接組む — ここで見たいのは投影の
    /// 分岐であって、集約のガードではない）。
    fn started_event(stages: Vec<core_domain::orchestration::StageEntry>) -> Started {
        use core_domain::orchestration::StartRequest;
        use core_domain::workflow_definition::{DefinitionRevision, WorkflowDefinitionId};
        Started::new(
            WorkflowDefinitionId::parse("claude").expect("定義 id"),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
            &StartRequest::new("classic", "unit"),
            stages,
        )
    }

    #[test]
    fn a_started_event_without_stages_names_no_stage_in_the_refusal() {
        // `stages()` が空なら材料が無いので `-` を置く（材料の欠落で失敗そのものを潰さない）。
        assert_eq!(
            started_blocked(&started_event(Vec::new())),
            ProjectionError::DefinitionLookupRequired {
                stage: "-".to_string()
            }
        );
    }

    #[test]
    fn every_projection_refusal_renders_its_material() {
        assert_eq!(ProjectionError::ScopeUnknown.to_string(), "scope unknown");
        assert_eq!(
            ProjectionError::DefinitionLookupRequired {
                stage: "domain-design".to_string()
            }
            .to_string(),
            "definition lookup required: stage domain-design"
        );
        // 綴りを取り違えたキーは投影の材料として拒否される（本体の定数は全て文法内なので、
        // この経路は写像そのものを直接固定しておく）。
        let malformed = AuditFieldKey::parse("1x").expect_err("文法外");
        assert_eq!(
            ProjectionError::from(malformed).to_string(),
            "audit field key: malformed audit field key: 1x"
        );
        let boxed: Box<dyn std::error::Error> = Box::new(ProjectionError::ScopeUnknown);
        assert_eq!(boxed.to_string(), "scope unknown");
    }

    #[test]
    fn the_recomposed_row_lists_the_stages_and_falls_back_to_the_none_literal() {
        let recomposed = Recomposed::new(
            vec![slug("incident-response")],
            Vec::new(),
            vec![slug("practices-discovery"), slug("requirements-analysis")],
        );
        let row =
            recomposed_row::recomposed_audit_row(&recomposed, &at(), &model()).expect("行は導ける");
        assert_eq!(
            row,
            "\n## Plan Recomposed\n\
             **Timestamp**: 2026-08-21T09:14:07Z\n\
             **Event**: RECOMPOSED\n\
             **Scope**: classic\n\
             **Stages skipped**: incident-response\n\
             **Stages added**: none\n\
             **Stages in Scope**: 2\n\
             \n---\n"
        );
    }

    #[test]
    fn several_stages_are_joined_with_a_comma() {
        use recomposed_row::stage_list;
        assert_eq!(stage_list(&[]), "none");
        assert_eq!(stage_list(&[slug("a-b")]), "a-b");
        assert_eq!(stage_list(&[slug("a-b"), slug("c-d")]), "a-b, c-d");
    }

    #[test]
    fn the_recomposed_row_needs_the_scope_the_read_model_remembers() {
        let recomposed = Recomposed::new(Vec::new(), Vec::new(), Vec::new());
        let without_scope = ReadModel::new("# nothing\n");
        assert_eq!(
            recomposed_row::recomposed_audit_row(&recomposed, &at(), &without_scope),
            Err(ProjectionError::ScopeUnknown)
        );
    }

    #[test]
    fn the_autonomy_mode_row_and_field_use_the_same_spelling() {
        // 状態ファイル側の綴りは `AutonomyMode::from_state_field` の逆写像である。
        let mut read_model =
            ReadModel::new("## Runtime State\n- **Construction Autonomy Mode**: gated\n");
        project(
            &[entry(WorkflowExecutionEvent::AutonomyModeSet(
                core_domain::orchestration::AutonomyModeSet::new(AutonomyMode::Autonomous),
            ))],
            &mut read_model,
        )
        .expect("投影");
        assert!(
            read_model
                .state()
                .contains("- **Construction Autonomy Mode**: autonomous\n"),
            "実際: {}",
            read_model.state()
        );
        assert!(
            read_model
                .appended_audit()
                .contains("**Mode**: autonomous\n"),
            "実際: {}",
            read_model.appended_audit()
        );
    }

    #[test]
    fn a_gate_rejection_without_feedback_omits_the_feedback_line() {
        let mut read_model = ReadModel::new(
            "## Runtime State\n- **Revision Count**: 0\n\n## Stage Progress\n- [?] s — EXECUTE\n",
        );
        project(
            &[entry(WorkflowExecutionEvent::GateRejected(
                GateRejected::new(slug("s"), None, 2),
            ))],
            &mut read_model,
        )
        .expect("投影");
        assert!(
            !read_model.appended_audit().contains("**Feedback**"),
            "実際: {}",
            read_model.appended_audit()
        );
        assert!(
            read_model
                .appended_audit()
                .contains("**Revision count**: 2\n")
        );
        assert!(read_model.state().contains("- **Revision Count**: 2\n"));
    }

    #[test]
    fn a_missing_state_field_is_refused_rather_than_silently_skipped() {
        // 無言 no-op は検出不能なドリフトである。
        let mut read_model = ReadModel::new("## Stage Progress\n- [?] s — EXECUTE\n");
        let error = project(
            &[entry(WorkflowExecutionEvent::GateRejected(
                GateRejected::new(slug("s"), None, 1),
            ))],
            &mut read_model,
        )
        .expect_err("Revision Count 行が無い");
        assert!(
            matches!(error, ProjectionError::StateField(_)),
            "実際: {error}"
        );
        assert!(error.to_string().starts_with("state field: "));
    }

    #[test]
    fn a_missing_checkbox_row_is_refused() {
        let mut read_model = ReadModel::new("## Stage Progress\n- [ ] other — EXECUTE\n");
        let error = project(
            &[entry(WorkflowExecutionEvent::GateOpened(GateOpened::new(
                slug("s"),
                Vec::new(),
            )))],
            &mut read_model,
        )
        .expect_err("対象ステージの行が無い");
        assert_eq!(error.to_string(), "checkbox: missing stage s");
    }

    #[test]
    fn parking_without_a_runtime_section_is_refused() {
        let mut read_model = ReadModel::new("## Project Information\n- **Scope**: classic\n");
        let error = project(
            &[entry(WorkflowExecutionEvent::Parked(Parked::new(slug(
                "s",
            ))))],
            &mut read_model,
        )
        .expect_err("置き場が無い");
        assert_eq!(error, ProjectionError::ParkSectionMissing);
        assert_eq!(error.to_string(), "park section missing");
    }

    #[test]
    fn unparking_twice_is_a_no_op_the_second_time() {
        let mut read_model = ReadModel::new(
            "## Runtime State\n- **Parked**: x\n- **Parked At Stage**: s\n\n## Next\n",
        );
        let unpark = || entry(WorkflowExecutionEvent::Unparked);
        project(&[unpark()], &mut read_model).expect("1 回目");
        let once = read_model.state().to_string();
        project(&[unpark()], &mut read_model).expect("2 回目");
        assert_eq!(read_model.state(), once, "状態面は動かない");
    }

    #[test]
    fn parking_twice_replaces_the_marker_rather_than_doubling_it() {
        let mut read_model =
            ReadModel::new("## Runtime State\n- **Revision Count**: 1\n\n## Next\n");
        let park = || entry(WorkflowExecutionEvent::Parked(Parked::new(slug("s"))));
        project(&[park()], &mut read_model).expect("1 回目");
        project(&[park()], &mut read_model).expect("2 回目");
        assert_eq!(
            read_model.state().matches("- **Parked**:").count(),
            1,
            "実際: {}",
            read_model.state()
        );
    }

    #[test]
    fn the_events_that_need_the_workflow_definition_stop_instead_of_writing_wrong_bytes() {
        // `STAGE_STARTED` の `**Agent**:` はワークフロー定義側の材料であり、投影核だけでは
        // 描けない（contract-summary §4 が U4 へ持ち越した未裁定項目）。誤ったバイトを
        // 書くくらいなら止まる、という選択をここで固定しておく。
        let blocked = [
            WorkflowExecutionEvent::Started(started_event(vec![
                core_domain::orchestration::StageEntry::new(
                    slug("state-init"),
                    core_domain::workflow_definition::PhaseId::Initialization,
                    core_domain::workflow_definition::PlanAction::Execute,
                    false,
                ),
            ])),
            WorkflowExecutionEvent::Jumped(Jumped::new(
                core_domain::orchestration::JumpDirection::Forward,
                slug("a"),
                slug("b"),
                Vec::new(),
                Vec::new(),
            )),
            WorkflowExecutionEvent::StageCompleted(StageCompleted::new(slug("a"), Some(slug("b")))),
            WorkflowExecutionEvent::GateApproved(GateApproved::new(
                slug("a"),
                None,
                Some(slug("b")),
                None,
            )),
            WorkflowExecutionEvent::StageSkipped(StageSkipped::new(
                slug("a"),
                "why".to_string(),
                Some(slug("b")),
            )),
            WorkflowExecutionEvent::Recomposed(Recomposed::new(
                vec![slug("a")],
                Vec::new(),
                Vec::new(),
            )),
        ];
        for event in blocked {
            let mut read_model = model();
            let error = project(&[entry(event.clone())], &mut read_model)
                .expect_err("定義が要るので描けない");
            assert!(
                matches!(error, ProjectionError::DefinitionLookupRequired { .. }),
                "{event:?}: 実際 {error}"
            );
            assert_eq!(
                read_model.state(),
                STATE,
                "止まったなら状態面に手を付けていない"
            );
        }
    }
}
