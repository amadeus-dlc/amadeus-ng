//! **純粋投影核** — ドメインイベントの列をリードモデルへ写す（C5 の投影規則）。
//!
//! ここが知っているのはドメインイベント・解決済み計画・リードモデルの 3 つだけである。
//! `JournalReader`・SQLite 接続・チェックポイントは**署名にも本体にも現れない**
//! （`coding-rules/cqrs-boundaries.md` の禁止パターン「純粋投影核が取得の都合を知る」）。
//! 取得ループと投影核の二層を潰さないのは、投影の規則だけを単体でテストできるようにするため
//! である。
//!
//! # 計画が引数なのはなぜか
//!
//! 表示属性（ステージ番号・表題・担当エージェント）と走査結果の正本は intent 自身の
//! ジャーナルの `Created`（誕生の材料 — issue #50 / #56）である。差分投影のバッチにその
//! 記録が入っているとは限らないので、計画は [`ResolvedPlan`] として**渡される** —
//! リードモデルと同じ「渡されるデータ」である。取ってくるのは取得ループの仕事であり、
//! 二層は保たれる。
//!
//! # 冪等（NFR3）
//!
//! 同じ入力からは常に同じバイトが出る — 壁時計を読まず（監査行の時刻はイベントの発生時刻）、
//! 乱数も環境変数も見ず、ワークフロー定義も引かない（引くと過去のイベントを今の定義で描くこと
//! になり、再構成が当時と一致しない）。二度描かない保証はチェックポイントが与えるので、投影核
//! 自身は「渡された列を順に写す」だけでよい。

use core_command_domain::orchestration::{
    AutonomyMode, CapturedLearnings, IntentExecutionEvent, JumpDirection, Jumped, LearningScope,
    LearningsCaptured, Parked, PhaseBoundary, PracticesAffirmed, Recomposed, ReviewCompleted,
    ReviewRequested, SingleStageRunCommitted, SkeletonStanceRecorded, StageSlugSet,
};
use core_command_domain::workflow_definition::{PhaseId, PlanAction, StageSlug};
use core_command_domain::workspace::{
    AuditFieldKey, AuditFieldKeyError, AuditFields, CheckboxState, CheckboxUpdateError, Checkboxes,
    PromotedSection, append_under_heading, ensure_heading, replace_section,
};

use chrono::{DateTime, Utc};
use core_command_domain::workspace::EventType;

use super::audit_block::{iso8601_seconds, render_audit_block};
use super::memory_faces::MemoryFaces;
use super::read_model::ReadModel;
use super::resolved_plan::{PlannedStage, ResolvedPlan};
use super::state_writers::{FieldNotFound, find_field, with_field, with_field_or_insert};
use crate::orchestration::JournalEntry;

// ---------------------------------------------------------------------------
// 逐語の綴り（upstream 実バイト — `tests/golden/` が正本）
// ---------------------------------------------------------------------------

/// 監査行のフィールドキー。
mod key {
    /// `**Stage**:`。
    pub(super) const STAGE: &str = "Stage";
    /// `**Agent**:`。
    pub(super) const AGENT: &str = "Agent";
    /// `**Details**:`。
    pub(super) const DETAILS: &str = "Details";
    /// `**Scope**:`。
    pub(super) const SCOPE: &str = "Scope";
    /// `**Request**:`。
    pub(super) const REQUEST: &str = "Request";
    /// `**Phase**:`。
    pub(super) const PHASE: &str = "Phase";
    /// `**Stage count**:`（genesis の `PHASE_STARTED` だけが持つ）。
    pub(super) const STAGE_COUNT: &str = "Stage count";
    /// `**Reason**:`。
    pub(super) const REASON: &str = "Reason";
    /// `**From phase**:`。
    pub(super) const FROM_PHASE: &str = "From phase";
    /// `**To phase**:`。
    pub(super) const TO_PHASE: &str = "To phase";
    /// `**Stages completed**:`。
    pub(super) const STAGES_COMPLETED: &str = "Stages completed";
    /// `**Phase boundary**:`。
    pub(super) const PHASE_BOUNDARY: &str = "Phase boundary";
    /// `**Project Type**:`。
    pub(super) const PROJECT_TYPE: &str = "Project Type";
    /// `**Languages**:`。
    pub(super) const LANGUAGES: &str = "Languages";
    /// `**Frameworks**:`。
    pub(super) const FRAMEWORKS: &str = "Frameworks";
    /// `**Build System**:`。
    pub(super) const BUILD_SYSTEM: &str = "Build System";
    /// `**User Input**:`。
    pub(super) const USER_INPUT: &str = "User Input";
    /// `**Direction**:`。
    pub(super) const DIRECTION: &str = "Direction";
    /// `**Source**:`。
    pub(super) const SOURCE: &str = "Source";
    /// `**Target**:`。
    pub(super) const TARGET: &str = "Target";
    /// `**Revision count**:`（状態ファイル側は大文字 C の `Revision Count` — upstream の非対称）。
    pub(super) const REVISION_COUNT: &str = "Revision count";
    /// `**Feedback**:`。
    pub(super) const FEEDBACK: &str = "Feedback";
    /// `**Stages skipped**:`。
    pub(super) const STAGES_SKIPPED: &str = "Stages skipped";
    /// 読み飛ばしが条件判定かジャンプかを示す本家2.7.1の分類。
    pub(super) const SKIP_KIND: &str = "Skip Kind";
    /// `**Stages added**:`。
    pub(super) const STAGES_ADDED: &str = "Stages added";
    /// `**Stages in Scope**:`。
    pub(super) const STAGES_IN_SCOPE: &str = "Stages in Scope";
    /// `**Workflow**:`（隔離実行の疑似ワークフロー ID — ピン `3c3146cf` `:5326-5343`）。
    pub(super) const WORKFLOW: &str = "Workflow";
    /// `**Reviewer**:`（レビュー受領証 — ピン `3c3146cf` `aidlc-log.ts:916-919`）。
    pub(super) const REVIEWER: &str = "Reviewer";
    /// `**Iteration**:`（同 `:988` / `:1128`）。
    pub(super) const ITERATION: &str = "Iteration";
    /// `**Verdict**:`（同 `:1135`）。
    pub(super) const VERDICT: &str = "Verdict";
    /// `**Retry**:`（判定待ちの依頼の呼び直し — 同 `:1058`）。
    pub(super) const RETRY: &str = "Retry";
    /// `**Affirming User**:`（昇格の承認者 — ピン `3c3146cf` `aidlc-state.ts:3734`）。
    pub(super) const AFFIRMING_USER: &str = "Affirming User";
    /// `**Candidate-ID**:`（学びの儀式 — `aidlc-learnings.ts:853`）。
    pub(super) const CANDIDATE_ID: &str = "Candidate-ID";
    /// `**Content-Hash**:`（同 `:854`。学びの同一性）。
    pub(super) const CONTENT_HASH: &str = "Content-Hash";
    /// `**Destination**:`（同 `:855`。`<project-dir>` へ伏せたメモリ層のパス）。
    pub(super) const DESTINATION: &str = "Destination";
    /// `**Heading**:`（同 `:856`。`## ` を含む完全形）。
    pub(super) const HEADING: &str = "Heading";
    /// `**Sections Written**:`（同 `:3735`。空でも欄は描く）。
    pub(super) const SECTIONS_WRITTEN: &str = "Sections Written";
    /// `**Mandated Rules Appended**:`（同 `:3736`）。
    pub(super) const MANDATED_RULES_APPENDED: &str = "Mandated Rules Appended";
    /// `**Forbidden Rules Appended**:`（同 `:3737`）。
    pub(super) const FORBIDDEN_RULES_APPENDED: &str = "Forbidden Rules Appended";
    /// `**Mode**:`（本家 `aidlc-bolt.ts` `handleSetAutonomy` の `AUTONOMY_MODE_SET` 監査行）。
    /// 2.7.1 コーパスに成功経路の採取は無い（`cli/set-autonomy/state-field-absent` は拒否側
    /// のみ）ので、綴りの根拠はソース読みである。自律実行は今回の必須範囲外であり、この
    /// 綴りを互換性の証明として使わない。
    pub(super) const MODE: &str = "Mode";
}

/// 状態ファイルの bullet ラベル。
mod field {
    /// `- **Active Agent**:`。
    pub(super) const ACTIVE_AGENT: &str = "Active Agent";
    /// `- **Completed**:`。
    pub(super) const COMPLETED: &str = "Completed";
    /// `- **In Progress**:`。
    pub(super) const IN_PROGRESS: &str = "In Progress";
    /// `- **Current Stage**:`。
    pub(super) const CURRENT_STAGE: &str = "Current Stage";
    /// `- **Lifecycle Phase**:`（値は**大文字**の フェーズ名 — `INCEPTION`）。
    pub(super) const LIFECYCLE_PHASE: &str = "Lifecycle Phase";
    /// `- **Next Stage**:`。
    pub(super) const NEXT_STAGE: &str = "Next Stage";
    /// `- **Last Completed Stage**:`。
    pub(super) const LAST_COMPLETED_STAGE: &str = "Last Completed Stage";
    /// `- **Next Action**:`。
    pub(super) const NEXT_ACTION: &str = "Next Action";
    /// `- **Revision Count**:`。
    pub(super) const REVISION_COUNT: &str = "Revision Count";
    /// `- **Total Stages**:`。
    pub(super) const TOTAL_STAGES: &str = "Total Stages";
    /// `- **Construction Autonomy Mode**:`。
    pub(super) const AUTONOMY_MODE: &str = "Construction Autonomy Mode";
    /// `- **Skeleton Stance**:`（骨格テンプレートに無いランタイム欄 — 無ければ挿入する）。
    pub(super) const SKELETON_STANCE: &str = "Skeleton Stance";
    /// `- **Stages to Execute**:`。
    pub(super) const STAGES_TO_EXECUTE: &str = "Stages to Execute";
    /// `- **Stages to Skip**:`。
    pub(super) const STAGES_TO_SKIP: &str = "Stages to Skip";
    /// `- **Status**:`（`Running` / `Completed` — 集約の `Status` と同じ 2 値）。
    pub(super) const STATUS: &str = "Status";
    /// `- **Last Updated**:`（値はイベントの発生時刻 — 投影は壁時計を読まない）。
    pub(super) const LAST_UPDATED: &str = "Last Updated";
    /// `- **Practices Affirmed Timestamp**:`（骨格テンプレートに無いランタイム欄 —
    /// 無ければ `## Project Information` の末尾へ挿入する。upstream `:3743-3748`）。
    pub(super) const PRACTICES_AFFIRMED_TIMESTAMP: &str = "Practices Affirmed Timestamp";

    /// `## Phase Progress` の 1 行のラベル（`- **Inception**:`）。
    ///
    /// upstream は `phase.charAt(0).toUpperCase() + phase.slice(1)` で作る — フェーズ slug の
    /// 先頭 1 文字だけを大文字にしたものである。5 つとも ASCII 小文字始まりなので、この
    /// 単純な変換で `Initialization` / `Ideation` / `Inception` / `Construction` / `Operation`
    /// になる。
    pub(super) fn phase_row(phase: super::PhaseId) -> String {
        let slug = phase.as_str();
        let mut label = String::with_capacity(slug.len());
        for (index, ch) in slug.chars().enumerate() {
            if index == 0 {
                label.extend(ch.to_uppercase());
            } else {
                label.push(ch);
            }
        }
        label
    }
}

/// ランタイム metadata の置き場（park マーカーと `Skeleton Stance` が入るセクション）。
///
/// `with_field_or_insert` は `## ` を自分で前置するので、ここは見出し名だけを持つ。
const RUNTIME_STATE_HEADING: &str = "Runtime State";

/// 昇格のタイムスタンプが入るセクション見出し（upstream `:3745`）。
const PROJECT_INFORMATION_HEADING: &str = "Project Information";

/// メモリ層 team.md の 5 節（`## ` を前置してから置き換える）。
const MEMORY_HEADING_PREFIX: &str = "## ";

/// `## Mandated` / `## Forbidden` の綴り（project.md の追記先）。
const MANDATED_HEADING: &str = "## Mandated";
const FORBIDDEN_HEADING: &str = "## Forbidden";

/// 診断が名指す書込先のファイル名。
const TEAM_MD: &str = "team.md";
const PROJECT_MD: &str = "project.md";
const STATE_FILE: &str = "aidlc-state.md";

/// `**Retry**:` の唯一の値（upstream `fields.Retry = "pending-request"` — `:1058`）。
const RETRY_PENDING_REQUEST: &str = "pending-request";

/// 空のステージ集合を描く逐語（`**Stages added**: none`）。
const NONE_LITERAL: &str = "none";
/// 再入時の逐語（`report --result revised`）。
const REENTRY_DETAILS: &str = "Re-entering gate after revision";
/// フェーズ境界の区切り（U+2192）。
const BOUNDARY_ARROW: &str = " → ";
/// Skip 行の項目に付く注釈の区切り（U+2014）。`2.1 (reverse-engineering — greenfield)`。
const SKIP_ANNOTATION: &str = " — ";
/// 一覧の区切り。
const LIST_SEPARATOR: &str = ", ";

/// `## Phase Progress` の行がとる 4 値（`<!-- Status values: … -->` が正本）。
mod phase_status {
    /// まだ来ていない。
    pub(super) const PENDING: &str = "Pending";
    /// いま走っている。
    pub(super) const ACTIVE: &str = "Active";
    /// 通過して検証済み。
    pub(super) const VERIFIED: &str = "Verified";
    /// スコープ内ステージが 1 つも無い、または飛び越えた。
    pub(super) const SKIPPED: &str = "Skipped";
}

/// ジャンプがフェーズ境界をまたいだときの `PHASE_VERIFIED` の逐語
/// （`cli/jump/execute-backward` / `execute-forward-across-phases`）。
const JUMP_BOUNDARY_VERIFICATION: &str = "Traceability verification on jump";

/// 走査して 1 つも完了ステージが見つからなかったときの `- **Last Completed Stage**:`
/// （upstream がジャンプ経路にだけ置いている既定値）。
const NO_EARLIER_COMPLETION: &str = "state-init";

/// ワークフロー完了後の `- **Status**:`（upstream `complete-workflow` `:2473`）。
const STATUS_COMPLETED: &str = "Completed";
/// ワークフロー完了後の `- **Next Action**:`（同 `:2478`）。
const WORKFLOW_COMPLETE_ACTION: &str = "Workflow complete";
/// 到達点の無いフェーズ境界の `**To phase**:`（同 `:2507` / skip 経路 `:3216`）。
const END_PHASE: &str = "(end)";
/// 到達点の無いフェーズ境界の `**Phase boundary**:` の右辺（同 `:2511` / `:3117`）。
const END_BOUNDARY: &str = "end";

/// 初期化 3 ステージが描く固有行の対応（upstream の出荷グラフに固定）。
const INITIALIZATION_ROWS: [(&str, EventType); 3] = [
    ("workspace-scaffold", EventType::WorkspaceScaffolded),
    ("workspace-detection", EventType::WorkspaceScanned),
    ("state-init", EventType::WorkspaceInitialised),
];

// ---------------------------------------------------------------------------
// 失敗
// ---------------------------------------------------------------------------

/// 投影の失敗（材料のみ — 文言はアダプタ層）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionError {
    /// 状態ファイルに書き換え先のフィールド行が無い。
    ///
    /// 無言 no-op は検出不能なドリフトなので、upstream 逐語の拒否文言を添えて止める。
    StateField(FieldNotFound),
    /// 状態ファイルに対象ステージのチェックボックス行が無い、または行末トークンが無い。
    Checkbox(CheckboxUpdateError),
    /// 監査行のフィールドキーが文法外だった（材料の綴りの誤り）。
    AuditFieldKey(AuditFieldKeyError),
    /// イベントが名指したステージが解決済み計画に無い。
    ///
    /// 計画とジャーナルが同じワークフローのものでなければ起きる — 読み替えずに止める。
    UnknownStage {
        /// 計画に無かったステージ。
        stage: String,
    },
    /// 状態ファイルに park マーカーの置き場（`## Runtime State`）が無い。
    ParkSectionMissing,
    /// メモリ層の 2 ファイルが載っていないのに `PracticesAffirmed` を描けと言われた。
    ///
    /// fail-closed である — 動詞側が 2 本の存在を確かめた後に消された場合だけ到達する。
    /// 読めないものを黙って飛ばすと、受領証だけが立って正本が古いままになる。
    MemoryFilesMissing,
    /// メモリ層のファイルに置換先・追記先の見出しが無い。
    MemoryHeadingMissing {
        /// 見出しが無かったファイル（`team.md` / `project.md`）。
        file: &'static str,
        /// 見つからなかった見出しの綴り（`## ` を含む完全形）。
        heading: String,
    },
    /// 状態ファイルの**骨格が無い** — 投影の前提違反である。
    ///
    /// # 骨格を書くのは投影ではない（オーナー裁定 2026-08-29）
    ///
    /// 投影の責務は**既存本文への差分適用に徹する**ことであり、本文そのもの — 9 セクションの
    /// 骨格と 31 のフィールド行 — を起こすことは含まれない。骨格は intent-create の時点で
    /// **合成ルート**が書く（環境と両側を知ってよい唯一の場所。実装は U7）。
    ///
    /// これは「導出の工夫が足りない」のではなく、**構造から従う**裁定である。骨格には
    /// `- **Project Root**:` があり、これはワークツリーの絶対パス — すなわち**環境の値**で、
    /// ジャーナルに存在しない。投影がこれを書けるようになる道は「環境を読む」か「環境パスを
    /// ドメインイベントへ載せる」かの 2 つしかなく、前者は投影核の定義を壊し、後者は ADR-008
    /// と NFR3 の趣旨に反する。書けないのではなく、**書く場所がここではない**。
    ///
    /// # NFR3 の適用範囲
    ///
    /// 冪等な再構成が保証するのは**差分適用**である — 同じジャーナルを同じ本文へ当てれば
    /// 常に同じバイトが出る。骨格はその保証の対象ではなく**環境成果物**であり、全損したら
    /// 再生成ではなく upstream 同様 archive & recreate で復旧する運用に載る。
    ///
    /// # 骨格の実バイトはある
    ///
    /// `cli/intent-create/classic-scope/state-full.md` が全文（102 行）で、U7 が骨格を書く
    /// ときの正本になる。upstream 側の正本は `aidlc-utility.ts` の template literal である
    /// （`knowledge/aidlc-shared/state-template.md` は LLM 向けの契約文書でツールは読まない）。
    ScaffoldMissing,
}

impl core::fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProjectionError::StateField(inner) => write!(f, "state field: {}", inner.message()),
            ProjectionError::Checkbox(CheckboxUpdateError::MissingStage(slug)) => {
                write!(f, "checkbox: missing stage {slug}")
            }
            ProjectionError::Checkbox(CheckboxUpdateError::MissingSuffix(slug)) => {
                write!(f, "checkbox: missing suffix {slug}")
            }
            ProjectionError::AuditFieldKey(inner) => write!(f, "audit field key: {inner}"),
            ProjectionError::UnknownStage { stage } => write!(f, "unknown stage: {stage}"),
            ProjectionError::ParkSectionMissing => f.write_str("park section missing"),
            ProjectionError::MemoryFilesMissing => f.write_str("memory files missing"),
            ProjectionError::MemoryHeadingMissing { file, heading } => {
                write!(f, "memory heading missing: {heading} in {file}")
            }
            ProjectionError::ScaffoldMissing => f.write_str("scaffold missing"),
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

/// 監査行のフィールドキーを組む（綴りはこのファイルの `key` モジュールが正本）。
fn key(raw: &str) -> Result<AuditFieldKey, ProjectionError> {
    AuditFieldKey::parse(raw).map_err(ProjectionError::from)
}

// ---------------------------------------------------------------------------
// 投影核
// ---------------------------------------------------------------------------

/// 純粋投影核 — 差分のジャーナル行をリードモデルへ写す。
///
/// 入口はドメインイベントと解決済み計画である。集約も Repository もストアのエラーも
/// ここには現れない。
///
/// # Errors
///
/// 状態ファイルに書き換え先が無い（`StateField` / `Checkbox`）、計画に無いステージを
/// 名指された（`UnknownStage`）、park マーカーの置き場が無い（`ParkSectionMissing`）、
/// 骨格の無い状態ファイルへ `Started` の状態面を求められた（`ScaffoldMissing`）を返す。
pub fn project(
    entries: &[JournalEntry],
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    for entry in entries {
        project_one(entry.event(), entry.occurred_at(), plan, read_model)?;
    }
    Ok(())
}

fn project_one(
    event: &IntentExecutionEvent,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    match event {
        IntentExecutionEvent::PlanAnswerLogged(answer) => {
            let input = answer.input();
            let evidence = input.decision().evidence();
            let authority = evidence.authority();
            let mut fields = AuditFields::new()
                .with(key("Stage")?, input.stage())
                .with(key("Details")?, input.choice().as_str());
            for (name, value) in [
                ("Checkpoint", "Code Generation Plan Approval"),
                ("Plan Target", authority.target_id()),
                ("Intent", authority.intent_id()),
                ("Directive Epoch", authority.directive_epoch()),
                ("Run floor", authority.run_floor()),
                ("Approval Fingerprint", evidence.fingerprint()),
                ("Questions File", evidence.questions_file()),
                ("Questions SHA-256", evidence.questions_sha256()),
                ("Prompt SHA-256", evidence.prompt_sha256()),
                ("Session", input.decision().session().raw()),
            ] {
                fields = fields.with(key(name)?, value);
            }
            let kind = match input.choice() {
                core_command_domain::orchestration::PlanChoice::ApprovePlan => {
                    EventType::PlanApprovalRecorded
                }
                core_command_domain::orchestration::PlanChoice::RequestChanges => {
                    EventType::QuestionAnswered
                }
            };
            read_model.append_audit(&render_audit_block(kind, at, &fields));
            Ok(())
        }
        IntentExecutionEvent::SingleStageRunStarted(event) => {
            let agent = plan
                .display_of(event.stage())
                .ok_or_else(|| unknown(event.stage()))?
                .lead_agent();
            let fields = AuditFields::new()
                .with(key("Stage")?, event.stage().as_str())
                .with(key("Agent")?, agent)
                .with(
                    key("Workflow")?,
                    &format!("single-stage:{}", event.stage().as_str()),
                );
            read_model.append_audit(&render_audit_block(EventType::StageStarted, at, &fields));
            Ok(())
        }
        IntentExecutionEvent::PipelineLinkCompleted(event) => {
            let receipt = event.receipt();
            let mut fields = AuditFields::new()
                .with(key("Stage")?, receipt.stage())
                .with(key("Link")?, receipt.link())
                .with(
                    key("Position")?,
                    &format!("{}/{}", receipt.position(), receipt.total()),
                );
            if let Some(handoff) = receipt.handoff() {
                fields = fields
                    .with(key("Artifact Path")?, handoff.path())
                    .with(key("Artifact SHA256")?, handoff.sha256())
                    .with(key("Artifact Mtime Ms")?, handoff.mtime_ms());
            }
            if let Some(repo) = receipt.repo() {
                fields = fields.with(key("Repo")?, repo);
            }
            if receipt.is_single() {
                fields = fields.with(
                    key("Workflow")?,
                    &format!("single-stage:{}", receipt.stage()),
                );
            }
            read_model.append_audit(&render_audit_block(
                EventType::PipelineLinkCompleted,
                at,
                &fields,
            ));
            Ok(())
        }
        IntentExecutionEvent::ArtifactReused(event) => {
            let receipt = event.receipt();
            let mut fields = AuditFields::new()
                .with(key("Stage")?, receipt.stage())
                .with(key("Decision")?, receipt.decision())
                .with(key("Artifacts")?, receipt.artifacts());
            if let Some(repo) = receipt.repo() {
                fields = fields.with(key("Repo")?, repo);
            }
            if receipt.is_single() {
                fields = fields.with(
                    key("Workflow")?,
                    &format!("single-stage:{}", receipt.stage()),
                );
            }
            read_model.append_audit(&render_audit_block(EventType::ArtifactReused, at, &fields));
            Ok(())
        }
        IntentExecutionEvent::DirectiveContextInvalidated(event) => {
            read_model.apply_directive_issue(event.directive());
            Ok(())
        }
        IntentExecutionEvent::DirectiveIssued(event) => {
            read_model.apply_directive_issue(event.directive());
            Ok(())
        }
        IntentExecutionEvent::AnswerRecorded(answer) => {
            use core_command_domain::orchestration::AnswerDisposition;
            let mut fields = AuditFields::new()
                .with(key("Stage")?, answer.stage())
                .with(key("Details")?, answer.details());
            let event_type = match answer.disposition() {
                AnswerDisposition::Recorded => Some(EventType::QuestionAnswered),
                AnswerDisposition::ApprovalGateReportOwned => None,
                AnswerDisposition::SummaryConfirmed(evidence) => {
                    fields = fields
                        .with(key("Checkpoint")?, "Consolidated Summary Confirmation")
                        .with(key("Questions File")?, evidence.questions_file())
                        .with(key("Questions SHA-256")?, evidence.questions_sha256())
                        .with(key("Hash Scope")?, "confirmed-content-v1");
                    Some(EventType::SummaryConfirmationRecorded)
                }
            };
            if let Some(event_type) = event_type {
                read_model.append_audit(&render_audit_block(event_type, at, &fields));
            }
            Ok(())
        }
        IntentExecutionEvent::PromptObserved(prompt) => {
            if !prompt.unattended() {
                let mut fields = AuditFields::new();
                if !prompt.session().is_empty() {
                    fields = fields.with(key("Session")?, prompt.session());
                }
                read_model.append_audit(&render_audit_block(EventType::HumanTurn, at, &fields));
            }
            Ok(())
        }
        IntentExecutionEvent::CommandFailed(event) => {
            let failure = event.failure();
            let fields = AuditFields::new()
                .with(key("Tool")?, failure.tool())
                .with(key("Command")?, failure.command())
                .with(key("Error")?, failure.error());
            read_model.append_audit(&render_audit_block(EventType::ErrorLogged, at, &fields));
            Ok(())
        }
        IntentExecutionEvent::MemoryJournalsObserved(event) => {
            // compile が判定した位置ぶんの `MEMORY_EMPTY` を描く。判定のやり直しはしない —
            // 承認済みか・この承認について記録済みかは集約の状態でしか決まらないからである
            // (`coding-rules/cqrs-boundaries.md`)。状態ファイルは動かさない。
            event.empty_stages().fold_left(Ok(()), |written, stage| {
                written?;
                let fields = AuditFields::new().with(key(key::STAGE)?, stage.as_str());
                read_model.append_audit(&render_audit_block(EventType::MemoryEmpty, at, &fields));
                Ok(())
            })
        }
        IntentExecutionEvent::HealthChecked(event) => {
            let result = event.result();
            let fields = AuditFields::new()
                .with(key("Request")?, "/aidlc --doctor")
                .with(
                    key("Details")?,
                    &format!("{} passed, {} failed", result.passed(), result.failed()),
                );
            read_model.append_audit(&render_audit_block(EventType::HealthChecked, at, &fields));
            Ok(())
        }
        IntentExecutionEvent::TaskSynchronized(event) => {
            let stage = plan
                .find(event.stage())
                .ok_or_else(|| unknown(event.stage()))?;
            set_field(
                read_model,
                field::LIFECYCLE_PHASE,
                &stage.phase().as_str().to_uppercase(),
            )?;
            set_field(
                read_model,
                field::ACTIVE_AGENT,
                stage.display().lead_agent(),
            )?;
            set_field(read_model, field::STATUS, "Running")?;
            set_field(read_model, field::LAST_UPDATED, &iso8601_seconds(at))?;
            set_field(read_model, field::CURRENT_STAGE, event.stage().as_str())?;
            set_field(read_model, field::IN_PROGRESS, event.stage().as_str())?;
            set_checkbox(
                read_model,
                event.stage().as_str(),
                CheckboxState::InProgress,
            )
        }
        IntentExecutionEvent::DecisionRecorded(recorded) => {
            let prompt = recorded.prompt();
            let mut fields = AuditFields::new()
                .with(key("Stage")?, prompt.stage())
                .with(key("Decision")?, prompt.decision());
            if let Some(options) = prompt.options() {
                fields = fields.with(key("Options")?, options);
            }
            if let Some(rationale) = prompt.rationale() {
                fields = fields.with(key("Rationale")?, rationale);
            }
            if let Some(file) = prompt.summary_file() {
                fields = fields
                    .with(key("Checkpoint")?, "Consolidated Summary Confirmation")
                    .with(key("Questions File")?, file);
            }
            if let Some(plan) = prompt.plan_approval() {
                let evidence = plan.evidence();
                let authority = evidence.authority();
                for (name, value) in [
                    ("Checkpoint", "Code Generation Plan Approval"),
                    ("Plan Target", authority.target_id()),
                    ("Intent", authority.intent_id()),
                    ("Directive Epoch", authority.directive_epoch()),
                    ("Run floor", authority.run_floor()),
                    ("Approval Fingerprint", evidence.fingerprint()),
                    ("Questions File", evidence.questions_file()),
                    ("Questions SHA-256", evidence.questions_sha256()),
                    ("Prompt SHA-256", evidence.prompt_sha256()),
                    ("Session", plan.session().raw()),
                ] {
                    fields = fields.with(key(name)?, value);
                }
            }
            read_model.append_audit(&render_audit_block(
                EventType::DecisionRecorded,
                at,
                &fields,
            ));
            Ok(())
        }
        IntentExecutionEvent::Reported(reported) => {
            use core_command_domain::orchestration::{ReportResult, ReportTransition};
            match reported.result() {
                ReportResult::NoOp { .. } => Ok(()),
                ReportResult::Committed {
                    stage, transition, ..
                } => match transition {
                    ReportTransition::GateOpened { .. } => gate_opened(stage, at, read_model),
                    ReportTransition::GateApproved { user_input } => gate_approved(
                        stage,
                        user_input.as_deref(),
                        reported.validation(),
                        reported.source_baseline(),
                        at,
                        plan,
                        read_model,
                    ),
                    ReportTransition::GateRejected { feedback } => {
                        gate_rejected(stage, feedback.as_deref(), at, read_model)
                    }
                    ReportTransition::StageRevised => stage_revised(stage, at, read_model),
                    ReportTransition::StageSkipped { reason } => stage_skipped(
                        stage,
                        reason,
                        reported.source_baseline(),
                        at,
                        plan,
                        read_model,
                    ),
                },
            }
        }
        IntentExecutionEvent::Started(_) => started(at, plan, read_model),
        IntentExecutionEvent::GateOpened(opened) => gate_opened(opened.stage(), at, read_model),
        IntentExecutionEvent::GateApproved(approved) => gate_approved(
            approved.stage(),
            approved.user_input(),
            None,
            None,
            at,
            plan,
            read_model,
        ),
        IntentExecutionEvent::GateRejected(rejected) => {
            gate_rejected(rejected.stage(), rejected.feedback(), at, read_model)
        }
        IntentExecutionEvent::StageRevised(revised) => {
            stage_revised(revised.stage(), at, read_model)
        }
        IntentExecutionEvent::StageSkipped(skipped) => stage_skipped(
            skipped.stage(),
            skipped.reason(),
            None,
            at,
            plan,
            read_model,
        ),
        IntentExecutionEvent::Jumped(jumped) => jumped_event(jumped, at, plan, read_model),
        IntentExecutionEvent::Parked(parked) => parked_event(parked, at, read_model),
        IntentExecutionEvent::Unparked(_) => {
            unparked(at, read_model);
            Ok(())
        }
        IntentExecutionEvent::Recomposed(recomposed) => {
            recomposed_event(recomposed, at, plan, read_model)
        }
        IntentExecutionEvent::AutonomyModeSet(mode) => {
            autonomy_mode_set(mode.mode(), at, read_model)
        }
        IntentExecutionEvent::SingleStageRunCommitted(committed) => {
            single_stage_run_committed(committed, at, plan, read_model)
        }
        IntentExecutionEvent::SkeletonStanceRecorded(recorded) => {
            skeleton_stance_recorded(recorded, read_model)
        }
        IntentExecutionEvent::ReviewRequested(requested) => {
            review_requested(requested, at, read_model)
        }
        IntentExecutionEvent::ReviewCompleted(completed) => {
            review_completed(completed, at, read_model)
        }
        IntentExecutionEvent::PracticesAffirmed(affirmed) => {
            practices_affirmed(affirmed, at, read_model)
        }
        IntentExecutionEvent::LearningsCaptured(captured) => {
            learnings_captured(captured, at, read_model)
        }
    }
}

// ---------------------------------------------------------------------------
// `Started` — 初期化 3 ステージの 16 行（`cli/intent-create/classic-scope`）
// ---------------------------------------------------------------------------

/// `Started` → 監査行 16 本と、状態ファイルの初期化。
///
/// 状態ファイルの**骨格が無ければ**（本文が空）`ScaffoldMissing` で止まる — 骨格を書くのは
/// 合成ルートであって投影ではない（オーナー裁定 2026-08-29、[`ProjectionError::ScaffoldMissing`]）。
/// 骨格があるなら、初期化 3 ステージの完了・最初のゲート付きステージへの着地・総数を書く —
/// いずれも他のイベントで逐語検収済みの writer と導出をそのまま使う。
///
/// **書かないもの**: `- **Stages to Execute**: ` / `- **Stages to Skip**: ` の 2 つ。どちらも
/// 骨格の行であり、書くのは合成ルートである。`- **Stages to Skip**:` の実バイトは
/// `2.1 (reverse-engineering — greenfield)` のように**畳まれた理由**を括弧内に持つが、その理由は
/// 素のグリッド値と調整後の値の区別を要し、`Started` の材料からは導けない — これも骨格生成が
/// 投影の仕事ではないことの傍証である。
fn started(
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    append_started_rows(at, plan, read_model)?;
    if read_model.state().trim().is_empty() {
        return Err(ProjectionError::ScaffoldMissing);
    }
    for stage in plan
        .stages()
        .iter()
        .filter(|stage| stage.is_in_scope() && stage.phase() == PhaseId::Initialization)
    {
        set_checkbox(read_model, stage.slug().as_str(), CheckboxState::Completed)?;
    }
    let completed = Checkboxes::parse(read_model.state())
        .count_completed()
        .to_string();
    set_field(read_model, field::COMPLETED, &completed)?;
    if let Some(last) = plan
        .stages()
        .iter()
        .rfind(|stage| stage.is_in_scope() && stage.phase() == PhaseId::Initialization)
    {
        set_field(
            read_model,
            field::LAST_COMPLETED_STAGE,
            last.slug().as_str(),
        )?;
    }
    set_field(
        read_model,
        field::TOTAL_STAGES,
        &plan.in_scope_count().to_string(),
    )?;
    match first_gated_in_scope(plan).map(|stage| stage.slug().clone()) {
        Some(slug) => {
            enter_stage_without_row(read_model, plan, &slug)?;
            set_field(
                read_model,
                field::NEXT_ACTION,
                &format!("Execute {}", slug.as_str()),
            )
        }
        None => Ok(()),
    }
}

/// `Started` の監査行 16 本（順序は upstream の emit 順）。
fn append_started_rows(
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let scope = plan.scope();

    let mut fields = AuditFields::new()
        .with(key(key::SCOPE)?, scope)
        .with(key(key::REQUEST)?, &plan.request_line());
    if let Some(baseline) = plan.source_baseline() {
        fields = fields.with(key("Source Baseline")?, &baseline.fingerprint());
    }
    read_model.append_audit(&render_audit_block(EventType::WorkflowStarted, at, &fields));

    read_model.append_audit(&render_audit_block(
        EventType::PhaseStarted,
        at,
        &AuditFields::new()
            .with(key(key::PHASE)?, PhaseId::Initialization.as_str())
            .with(
                key(key::STAGE_COUNT)?,
                &plan.in_scope_count_of(PhaseId::Initialization).to_string(),
            )
            .with(key(key::SCOPE)?, scope),
    ));

    for phase in plan.phases_out_of_scope() {
        read_model.append_audit(&render_audit_block(
            EventType::PhaseSkipped,
            at,
            &AuditFields::new()
                .with(key(key::PHASE)?, phase.as_str())
                .with(key(key::SCOPE)?, scope)
                .with(
                    key(key::REASON)?,
                    &format!("scope {scope} excludes {}", phase.as_str()),
                ),
        ));
    }

    let routing_to = first_gated_in_scope(plan);
    for stage in plan
        .stages()
        .iter()
        .filter(|stage| stage.is_in_scope() && stage.phase() == PhaseId::Initialization)
    {
        read_model.append_audit(&stage_started_row(stage, at, None)?);
        if let Some(row) = initialization_row(stage, at, plan, routing_to)? {
            read_model.append_audit(&row);
        }
        read_model.append_audit(&render_audit_block(
            EventType::StageCompleted,
            at,
            &AuditFields::new()
                .with(key(key::STAGE)?, stage.slug().as_str())
                .with(
                    key(key::DETAILS)?,
                    &initialization_completion_details(stage, plan, routing_to),
                ),
        ));
    }

    let phases = plan.phases_in_scope();
    if let Some(to_phase) = phases.get(1).copied() {
        append_phase_boundary(
            read_model,
            at,
            plan,
            PhaseBoundary::new(PhaseId::Initialization, to_phase),
            &plan.in_scope_count_of(PhaseId::Initialization).to_string(),
        )?;
    }

    if let Some(stage) = routing_to {
        read_model.append_audit(&stage_started_row(stage, at, None)?);
    }
    Ok(())
}

/// 最初のゲート付きスコープ内ステージ（`routing to …` の材料）。
fn first_gated_in_scope(plan: &ResolvedPlan) -> Option<&PlannedStage> {
    plan.stages()
        .iter()
        .find(|stage| stage.is_in_scope() && stage.phase() != PhaseId::Initialization)
}

/// initialization ステージ固有の行（`WORKSPACE_*`）。
fn initialization_row(
    stage: &PlannedStage,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    routing_to: Option<&PlannedStage>,
) -> Result<Option<String>, ProjectionError> {
    let Some((_, event)) = INITIALIZATION_ROWS
        .iter()
        .find(|(slug, _)| *slug == stage.slug().as_str())
    else {
        return Ok(None);
    };
    let scan = plan.scan();
    let fields = match *event {
        EventType::WorkspaceScaffolded => AuditFields::new()
            .with(key(key::REQUEST)?, &plan.request_line())
            .with(
                key(key::DETAILS)?,
                &format!(
                    "{} in-scope phase dirs + verification/ + space-level knowledge/ ensured \
                     (shell shipped by SEED)",
                    plan.phases_in_scope().len()
                ),
            ),
        EventType::WorkspaceScanned => AuditFields::new()
            .with(key(key::PROJECT_TYPE)?, scan.project_type())
            .with(key(key::LANGUAGES)?, scan.languages())
            .with(key(key::FRAMEWORKS)?, scan.frameworks())
            .with(key(key::BUILD_SYSTEM)?, scan.build_system())
            .with(key(key::DETAILS)?, "Deterministic rule-based scan"),
        _ => AuditFields::new()
            .with(key(key::REQUEST)?, &plan.request_line())
            .with(key(key::PROJECT_TYPE)?, scan.project_type())
            .with(key(key::SCOPE)?, plan.scope())
            .with(key(key::LANGUAGES)?, scan.languages())
            .with(key(key::FRAMEWORKS)?, scan.frameworks())
            .with(key(key::BUILD_SYSTEM)?, scan.build_system())
            .with(
                key(key::DETAILS)?,
                &format!(
                    "{} stages in scope, routing to {}",
                    plan.in_scope_count(),
                    routing_to.map_or("-", |stage| stage.slug().as_str())
                ),
            ),
    };
    Ok(Some(render_audit_block(*event, at, &fields)))
}

/// initialization ステージの `STAGE_COMPLETED` の `**Details**:`（ステージごとに逐語が違う）。
fn initialization_completion_details(
    stage: &PlannedStage,
    plan: &ResolvedPlan,
    routing_to: Option<&PlannedStage>,
) -> String {
    let scan = plan.scan();
    let routing = routing_to.map_or("-", |stage| stage.slug().as_str());
    match stage.slug().as_str() {
        "workspace-scaffold" => format!(
            "{} in-scope phase dirs + verification/ + space-level knowledge/ ensured",
            plan.phases_in_scope().len()
        ),
        "workspace-detection" => format!(
            "Classified {}; languages={}; frameworks={}",
            scan.project_type(),
            scan.languages(),
            scan.frameworks()
        ),
        _ => format!(
            "State initialized: {} scope, {} stages, routing to {routing}",
            plan.scope(),
            plan.in_scope_count()
        ),
    }
}

// ---------------------------------------------------------------------------
// ゲートまわり
// ---------------------------------------------------------------------------

/// `GateOpened` → `STAGE_AWAITING_APPROVAL`、チェックボックス `[-]` → `[?]`。
fn gate_opened(
    stage: &StageSlug,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let fields = AuditFields::new().with(key(key::STAGE)?, stage.as_str());
    read_model.append_audit(&render_audit_block(
        EventType::StageAwaitingApproval,
        at,
        &fields,
    ));
    set_checkbox(read_model, stage.as_str(), CheckboxState::AwaitingApproval)
}

/// `GateRejected` → `GATE_REJECTED` + `STAGE_REVISING`、`[?]` → `[R]`、`Revision Count`。
fn gate_rejected(
    stage: &StageSlug,
    feedback: Option<&str>,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let stage = stage.as_str();
    let feedback_text = feedback.unwrap_or_default();
    // 改訂回数はイベントに載らない — upstream `aidlc-state.ts` と同じく、リードモデルの
    // `Revision Count` を読んで +1 する (非数値・欠落は 0 に畳む — 正本互換の導出)。
    let prior = find_field(read_model.state(), field::REVISION_COUNT)
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(0);
    let revisions = prior.saturating_add(1).to_string();

    let mut gate = AuditFields::new().with(key(key::STAGE)?, stage);
    if feedback.is_some() {
        gate = gate.with(key(key::FEEDBACK)?, feedback_text);
    }
    read_model.append_audit(&render_audit_block(EventType::GateRejected, at, &gate));

    let mut revising = AuditFields::new()
        .with(key(key::STAGE)?, stage)
        .with(key(key::REVISION_COUNT)?, &revisions);
    if feedback.is_some() {
        revising = revising.with(key(key::FEEDBACK)?, feedback_text);
    }
    read_model.append_audit(&render_audit_block(EventType::StageRevising, at, &revising));

    set_checkbox(read_model, stage, CheckboxState::Revising)?;
    set_field(read_model, field::REVISION_COUNT, &revisions)
}

/// `StageRevised` → `STAGE_AWAITING_APPROVAL`（再入の逐語つき）、`[R]` → `[?]`。
fn stage_revised(
    stage: &StageSlug,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let fields = AuditFields::new()
        .with(key(key::STAGE)?, stage.as_str())
        .with(key(key::DETAILS)?, REENTRY_DETAILS);
    read_model.append_audit(&render_audit_block(
        EventType::StageAwaitingApproval,
        at,
        &fields,
    ));
    set_checkbox(read_model, stage.as_str(), CheckboxState::AwaitingApproval)
}

/// `GateApproved` → `GATE_APPROVED` + `STAGE_COMPLETED` + (フェーズ境界) + 次ステージの開始。
fn gate_approved(
    stage: &StageSlug,
    user_input: Option<&str>,
    validation: Option<&core_command_domain::orchestration::StageValidation>,
    baseline: Option<&core_command_domain::orchestration::SourceBaseline>,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let title = title_of(plan, stage)?;

    let mut gate = AuditFields::new().with(key(key::STAGE)?, stage.as_str());
    if let Some(input) = user_input {
        gate = gate.with(key(key::USER_INPUT)?, input);
    }
    read_model.append_audit(&render_audit_block(EventType::GateApproved, at, &gate));
    let mut completed = AuditFields::new().with(key(key::STAGE)?, stage.as_str());
    if let Some(validation) = validation {
        use core_command_domain::orchestration::StageValidation;
        let (name, value) = match validation {
            StageValidation::Basis(value) => ("Validation Basis", value),
            StageValidation::Warning(value) => ("Validation Warning", value),
        };
        completed = completed.with(key(name)?, value);
    }
    completed = completed.with(
        key(key::DETAILS)?,
        &format!("Stage {title} approved by gate"),
    );
    read_model.append_audit(&render_audit_block(
        EventType::StageCompleted,
        at,
        &completed,
    ));
    // 境界行の `**Stages completed**:` は**倒したあとの**チェックボックスを数えた値なので、
    // 先に完了させる（`cli/report/approved-across-phases` は 2 — 計画上の inception 内
    // スコープ件数 8 とは一致しない）。監査行の順序はここでは動かない — 下のローカルヘルパは
    // 状態面だけを触り、行を描かないからである。
    complete_stage(read_model, stage)?;
    // 次カーソルとフェーズ境界はイベントに載らない — リードモデルの実効プランと計画から
    // 導く (`Jumped` の境界導出と同じ理由 — 材料が足りているうちはイベントを太らせない)。
    let next = next_in_effective_scope(read_model, plan, stage);
    if let Some(next) = &next
        && let Some(boundary) = crossed_phase_boundary(plan, stage, next)?
    {
        let completed = completed_count(read_model);
        append_phase_boundary(read_model, at, plan, boundary, &completed)?;
        set_phase_progress_for_advance(read_model, boundary)?;
    }
    leave_for(
        read_model,
        at,
        plan,
        next.as_ref(),
        stage,
        Completion::Approved,
        baseline,
    )
}

// ---------------------------------------------------------------------------
// 進行
// ---------------------------------------------------------------------------

/// `StageSkipped` → `STAGE_SKIPPED` + 次ステージの開始。完了数と最終完了ステージは動かさない。
fn stage_skipped(
    stage: &StageSlug,
    reason: &str,
    baseline: Option<&core_command_domain::orchestration::SourceBaseline>,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    read_model.append_audit(&render_audit_block(
        EventType::StageSkipped,
        at,
        &AuditFields::new()
            .with(key(key::STAGE)?, stage.as_str())
            .with(key(key::REASON)?, reason)
            .with(key(key::SKIP_KIND)?, "conditional-runtime"),
    ));
    set_checkbox(read_model, stage.as_str(), CheckboxState::Skipped)?;
    let next = next_in_effective_scope(read_model, plan, stage);
    leave_for(
        read_model,
        at,
        plan,
        next.as_ref(),
        stage,
        Completion::Skipped { reason },
        baseline,
    )
}

/// `Jumped` → 読み飛ばした各ステージの `STAGE_SKIPPED` + (フェーズ境界) + `STAGE_JUMPED`
/// + 目標の開始。
///
/// # フェーズ境界はイベントに載らず、計画から導く
///
/// `Jumped` は出発点と到達点の slug しか運ばない。だが計画は両方のフェーズを知っているので、
/// またいだかどうかは**渡された計画から導ける** — `GateApproved` のように `PhaseBoundary` を
/// イベントへ足す必要は無い。導出であって推測ではないので、材料が足りているうちはイベントを
/// 太らせない（`resolved_plan.rs` の「正本は 1 つでよい」と同じ理由）。
///
/// 境界 3 行はゲート経由のものと**同型ではない**。ジャンプ側だけが `**Details**:` を持ち
/// （`Phase boundary crossed via <方向> jump` / `Traceability verification on jump`）、
/// `**Stages completed**:` は計画上のフェーズ内件数ではなく**チェックボックスの数え直し**で
/// ある（`cli/jump/execute-backward` は 0、`execute-forward-across-phases` は 1 — どちらも
/// 直前の書き換え後に数えた値でしか説明が付かない）。
fn jumped_event(
    jumped: &Jumped,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let target = jumped.target();
    // イベントは到達点しか運ばない — 出発点は自分の `Current Stage` 行、方向は計画上の
    // 位置の大小、読み飛ばし・巻き戻しの列は跳躍規則 (BR1.6) をリードモデルの行へ適用して
    // 導く (オーナー裁定 2026-08-30「イベントに状態は含めるな」)。
    let source_raw = find_field(read_model.state(), field::CURRENT_STAGE).unwrap_or_default();
    let source =
        StageSlug::parse(source_raw.trim()).map_err(|_| ProjectionError::UnknownStage {
            stage: source_raw.trim().to_string(),
        })?;
    let position = |slug: &StageSlug| -> Result<usize, ProjectionError> {
        plan.stages()
            .iter()
            .position(|stage| stage.slug() == slug)
            .ok_or_else(|| unknown(slug))
    };
    let (src_at, tgt_at) = (position(&source)?, position(target)?);
    let direction = jumped.direction();
    let spelling = direction_spelling(direction);
    let lowered = spelling.to_lowercase();

    let checkboxes = Checkboxes::parse(read_model.state());
    let state_of = |slug: &StageSlug| {
        checkboxes
            .find(slug.as_str())
            .map(core_command_domain::workspace::CheckboxEntry::state)
    };
    let mut reset_stages = Vec::new();
    // 読み飛ばし・巻き戻しの対象は跳躍に使う計画で決まる — `--scope` を名指した直接
    // execute は別 scope の静的な列で導く (裁定 2026-09-10: jump-contract Q1 = A。
    // 集約の `jump_plan` と同じ規則)。
    let in_jump_plan = |stage: &PlannedStage| match jumped.scope() {
        Some(scope) => scope.contains(stage.slug()),
        None => effective_action(read_model, stage) == PlanAction::Execute,
    };
    match direction {
        JumpDirection::Forward => {
            // 中間の未了 + 出発点で稼働中のものを skipped にする。行の並びは upstream の
            // emit 順 — **中間ステージを計画順に並べたあと、最後に出発点そのもの**が来る
            // (`jump/execute-forward-across-phases` の実バイト)。
            let mut skipped: Vec<&StageSlug> = plan
                .stages()
                .get(src_at + 1..tgt_at)
                .unwrap_or_default()
                .iter()
                .filter(|stage| {
                    // 実効 SKIP の中間は触らない (`SKIP` 行はそのまま — upstream 実バイト)。
                    in_jump_plan(stage)
                        && state_of(stage.slug()).is_some_and(CheckboxState::is_in_flight)
                })
                .map(PlannedStage::slug)
                .collect();
            if &source != target && state_of(&source).is_some_and(CheckboxState::is_active) {
                skipped.push(&source);
            }
            for slug in skipped {
                read_model.append_audit(&render_audit_block(
                    EventType::StageSkipped,
                    at,
                    &AuditFields::new()
                        .with(key(key::STAGE)?, slug.as_str())
                        .with(
                            key(key::REASON)?,
                            &format!("Skipped by jump to {} ({lowered})", target.as_str()),
                        )
                        .with(key(key::SKIP_KIND)?, "jump"),
                ));
                set_checkbox(read_model, slug.as_str(), CheckboxState::Skipped)?;
            }
        }
        JumpDirection::Backward => {
            // 到達点**以降**の in-scope 既着手を pending へ戻す — upstream は到達点自身も
            // 一度 pending へ戻してから開始し直す (`jump/execute-backward` の
            // `**Stages completed**: 0` は到達点の [x] を戻した後の数え直しでしか説明が
            // 付かない)。
            let resets: Vec<StageSlug> = plan
                .stages()
                .get(tgt_at..)
                .unwrap_or_default()
                .iter()
                .filter(|stage| {
                    in_jump_plan(stage)
                        && state_of(stage.slug())
                            .is_some_and(|marker| marker != CheckboxState::Pending)
                })
                .map(|stage| stage.slug().clone())
                .collect();
            for slug in resets {
                reset_stages.push(slug.clone());
                set_checkbox(read_model, slug.as_str(), CheckboxState::Pending)?;
            }
        }
        JumpDirection::Redo => {}
    }
    // 完了数はジャンプでも**数え直す**。後方ジャンプは `[x]` を `[ ]` へ戻すので減る
    // （`cli/jump/execute-backward` は 4 → 0）。境界をまたがないジャンプでも upstream は
    // 同じ書き換えを打つ（値が動かないだけ）。
    set_field(read_model, field::COMPLETED, &completed_count(read_model))?;

    if let Some(boundary) = crossed_phase_boundary(plan, &source, target)? {
        append_jump_phase_boundary(read_model, at, plan, boundary, &lowered)?;
    }

    let number = number_of(plan, target)?;
    let mut fields = AuditFields::new()
        .with(key(key::DIRECTION)?, spelling)
        .with(key(key::SOURCE)?, source.as_str())
        .with(key(key::TARGET)?, target.as_str())
        .with(key(key::SCOPE)?, plan.scope())
        .with(
            key(key::DETAILS)?,
            &format!(
                "{spelling} jump from {} to {} ({number}). Scope: {}.",
                source.as_str(),
                target.as_str(),
                plan.scope()
            ),
        );
    if let Some(observation) = jumped
        .observation()
        .filter(|_| direction == JumpDirection::Backward)
    {
        let (mut changed, mut invalidated, mut reviews) = observation.fold_artifacts(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut changed, mut invalidated, mut reviews), artifact| {
                if artifact.stage() == target {
                    changed.push(artifact.path().to_string());
                }
                if artifact.stage() != target
                    && reset_stages.contains(artifact.stage())
                    && artifact.exists()
                {
                    invalidated.push(artifact.path().to_string());
                    if artifact.has_review() {
                        reviews.push(format!("{}#Review", artifact.path()));
                    }
                }
                (changed, invalidated, reviews)
            },
        );
        for values in [&mut changed, &mut invalidated, &mut reviews] {
            values.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            values.dedup();
        }
        fields = fields
            .with(
                key("Changed Upstream Artifacts")?,
                &core_infrastructure::canon_json::serialize(
                    &core_infrastructure::canon_json::JsonValue::Array(
                        changed
                            .into_iter()
                            .map(core_infrastructure::canon_json::JsonValue::String)
                            .collect(),
                    ),
                    core_infrastructure::canon_json::SerializationProfile::ContractCompact,
                ),
            )
            .with(
                key("Invalidated Downstream Artifacts")?,
                &core_infrastructure::canon_json::serialize(
                    &core_infrastructure::canon_json::JsonValue::Array(
                        invalidated
                            .into_iter()
                            .map(core_infrastructure::canon_json::JsonValue::String)
                            .collect(),
                    ),
                    core_infrastructure::canon_json::SerializationProfile::ContractCompact,
                ),
            )
            .with(
                key("Invalidated Downstream Reviews")?,
                &core_infrastructure::canon_json::serialize(
                    &core_infrastructure::canon_json::JsonValue::Array(
                        reviews
                            .into_iter()
                            .map(core_infrastructure::canon_json::JsonValue::String)
                            .collect(),
                    ),
                    core_infrastructure::canon_json::SerializationProfile::ContractCompact,
                ),
            );
    }
    if let Some(baseline) = jumped.baseline() {
        fields = fields.with(key("Source Baseline")?, &baseline.fingerprint());
    }
    read_model.append_audit(&render_audit_block(EventType::StageJumped, at, &fields));
    let stage = plan.find(target).ok_or_else(|| unknown(target))?;
    read_model.append_audit(&stage_started_row(stage, at, jumped.baseline())?);
    enter_stage_without_row(read_model, plan, target)?;
    set_field(
        read_model,
        field::LAST_COMPLETED_STAGE,
        &last_completion_before(read_model, plan, target),
    )
}

/// 出発点と到達点のフェーズが違えば境界を返す（どちらも計画に居ることが前提）。
fn crossed_phase_boundary(
    plan: &ResolvedPlan,
    source: &StageSlug,
    target: &StageSlug,
) -> Result<Option<PhaseBoundary>, ProjectionError> {
    let from = plan.find(source).ok_or_else(|| unknown(source))?.phase();
    let to = plan.find(target).ok_or_else(|| unknown(target))?.phase();
    Ok((from != to).then(|| PhaseBoundary::new(from, to)))
}

/// ジャンプの境界 3 行と、`## Phase Progress` の行の付け替え。
fn append_jump_phase_boundary(
    read_model: &mut ReadModel,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    boundary: PhaseBoundary,
    direction: &str,
) -> Result<(), ProjectionError> {
    let from = boundary.from_phase();
    let to = boundary.to_phase();
    read_model.append_audit(&render_audit_block(
        EventType::PhaseCompleted,
        at,
        &AuditFields::new()
            .with(key(key::FROM_PHASE)?, from.as_str())
            .with(key(key::TO_PHASE)?, to.as_str())
            .with(key(key::STAGES_COMPLETED)?, &completed_count(read_model))
            .with(
                key(key::DETAILS)?,
                &format!("Phase boundary crossed via {direction} jump"),
            ),
    ));
    read_model.append_audit(&render_audit_block(
        EventType::PhaseVerified,
        at,
        &AuditFields::new()
            .with(
                key(key::PHASE_BOUNDARY)?,
                &format!("{}{BOUNDARY_ARROW}{}", from.as_str(), to.as_str()),
            )
            .with(key(key::DETAILS)?, JUMP_BOUNDARY_VERIFICATION),
    ));
    read_model.append_audit(&render_audit_block(
        EventType::PhaseStarted,
        at,
        &AuditFields::new()
            .with(key(key::PHASE)?, to.as_str())
            .with(key(key::SCOPE)?, plan.scope()),
    ));
    set_phase_progress_for_jump(read_model, plan, from, to)
}

/// ジャンプ後の `## Phase Progress`。
///
/// 前方は「出発フェーズは通過したので `Verified`、飛び越えたフェーズは `Skipped`」、後方は
/// 「到達フェーズより後ろでスコープ内ステージを持つものは `Pending` へ戻す」。どちらも最後に
/// 到達フェーズを `Active` にする。スコープ内ステージが 1 つも無いフェーズには触れない —
/// genesis が置いた `Skipped` のままでよく、触ると `- **Ideation**: Skipped` が動いてしまう。
fn set_phase_progress_for_jump(
    read_model: &mut ReadModel,
    plan: &ResolvedPlan,
    from: PhaseId,
    to: PhaseId,
) -> Result<(), ProjectionError> {
    let (from_at, to_at) = (phase_order(from), phase_order(to));
    if from_at < to_at {
        set_field(read_model, &field::phase_row(from), phase_status::VERIFIED)?;
        for phase in PhaseId::ALL
            .iter()
            .copied()
            .skip(from_at.saturating_add(1))
            .take(to_at.saturating_sub(from_at).saturating_sub(1))
        {
            set_field(read_model, &field::phase_row(phase), phase_status::SKIPPED)?;
        }
    } else {
        for phase in PhaseId::ALL
            .iter()
            .copied()
            .skip(to_at.saturating_add(1))
            .filter(|phase| plan.in_scope_count_of(*phase) > 0)
        {
            set_field(read_model, &field::phase_row(phase), phase_status::PENDING)?;
        }
    }
    set_field(read_model, &field::phase_row(to), phase_status::ACTIVE)
}

/// ジャンプ後の `- **Last Completed Stage**:`。
///
/// 到達点より**手前**にある最後の `[x]` のステージを書く。1 つも無ければ upstream の既定値
/// `state-init` を書く（到達点が先頭ステージのときに起きる — `cli/jump/execute-backward` が
/// その実測である）。
///
/// 前から辿って**最後に当たったもの**を残すのは、後ろから探して最初に当たったものと同じで
/// ある。`take_while` は逆順に辿れないので、こちらの向きで書いた。到達点そのものは見ない。
fn last_completion_before(
    read_model: &ReadModel,
    plan: &ResolvedPlan,
    target: &StageSlug,
) -> String {
    let checkboxes = Checkboxes::parse(read_model.state());
    let mut found = NO_EARLIER_COMPLETION;
    for stage in plan
        .stages()
        .iter()
        .take_while(|stage| stage.slug() != target)
    {
        let slug = stage.slug().as_str();
        if checkboxes.has_completed(slug) {
            found = slug;
        }
    }
    found.to_string()
}

/// `Recomposed` → `RECOMPOSED`、計画一覧・総数・行末トークンの更新。
fn recomposed_event(
    recomposed: &Recomposed,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    // 適用後の in-scope 数はイベントに載らない — 行末トークンを反転してから自分の行を
    // 数える (オーナー裁定 2026-08-30)。監査行の位置は従来と同じ (このイベントで 1 行)。
    // 反転集合は**辞書順**なので、行末トークンの書き替えも監査行の綴りも、計画の位置で
    // 文書順へ並べ直してから使う (監査行の逐語一致は U4 / U7 の NFR1 要求)。
    let skipped = in_document_order(plan, recomposed.skipped());
    let added = in_document_order(plan, recomposed.added());
    for slug in &skipped {
        set_suffix(read_model, slug, PlanAction::Skip)?;
    }
    for slug in &added {
        set_suffix(read_model, slug, PlanAction::Execute)?;
    }
    let in_scope = plan
        .stages()
        .iter()
        .filter(|stage| effective_action(read_model, stage) == PlanAction::Execute)
        .count()
        .to_string();
    read_model.append_audit(&render_audit_block(
        EventType::Recomposed,
        at,
        &AuditFields::new()
            .with(key(key::SCOPE)?, plan.scope())
            .with(key(key::STAGES_SKIPPED)?, &stage_list(&skipped))
            .with(key(key::STAGES_ADDED)?, &stage_list(&added))
            .with(key(key::STAGES_IN_SCOPE)?, &in_scope),
    ));
    rebuild_plan_rows(read_model, plan)
}

/// `- **Stages to Execute**: ` / `- **Stages to Skip**: ` / `- **Total Stages**: ` を
/// 行末トークンから組み直す。
///
/// # 2 行は同じ規則で作られていない（upstream 実バイト）
///
/// **Execute 行は毎回 graph 順に組み直す**。`cli/recompose/add-restores-conditional` で
/// `2.1` が末尾ではなく `0.3` と `2.2` の**間**へ入るのがその実測である。
///
/// **Skip 行は既存項目をその位置のまま保つ**。項目は `<番号> (<slug>)` の形だが、genesis が
/// 書いた `2.1 (reverse-engineering — greenfield)` のように**注釈**を持つものがあり、slug から
/// 組み直すと注釈が消えてしまうためである。まだ skip のままの項目を逐語で残し、EXECUTE へ
/// 戻った項目を落とし、新しく skip になった項目を graph 順で末尾へ足す。
/// `cli/recompose/skip-two-appends-in-graph-order` が「既存の 4.5 の**後ろ**に 4.3, 4.7 が
/// 並ぶ」ことを、`add-restores-conditional` が「注釈ごと消える」ことを固定している。
///
/// # 実効計画をどこから読むか
///
/// upstream の `eff` は「recompose のオーバレイ ?? スコープグリッド」である。投影にとっての
/// オーバレイはチェックボックス行の行末トークンで、`set_suffix` がそれを保っている。行が
/// 無ければ（差分の断片しか無いテストなど）genesis の計画へ落とす — これは upstream の
/// `?? scopeDef.stages[slug]` と同じ既定である。
fn rebuild_plan_rows(
    read_model: &mut ReadModel,
    plan: &ResolvedPlan,
) -> Result<(), ProjectionError> {
    let checkboxes = Checkboxes::parse(read_model.state());
    let effective = |stage: &PlannedStage| -> PlanAction {
        checkboxes
            .find(stage.slug().as_str())
            .and_then(|entry| match entry.rest().trim() {
                "EXECUTE" => Some(PlanAction::Execute),
                "SKIP" => Some(PlanAction::Skip),
                _ => None,
            })
            .unwrap_or_else(|| stage.plan_action())
    };

    let mut skips: Vec<String> = Vec::new();
    let mut preserved: Vec<String> = Vec::new();
    for token in list_of(read_model, field::STAGES_TO_SKIP)? {
        let slug = slug_of_skip_token(&token);
        let still_skipped = plan
            .stages()
            .iter()
            .find(|stage| stage.slug().as_str() == slug)
            .is_some_and(|stage| effective(stage) == PlanAction::Skip);
        if still_skipped {
            preserved.push(slug.to_string());
            skips.push(token);
        }
    }

    let mut executes: Vec<String> = Vec::new();
    for stage in plan.stages() {
        let slug = stage.slug().as_str();
        if effective(stage) == PlanAction::Execute {
            executes.push(stage.display().number().as_str().to_string());
        } else if !preserved.iter().any(|kept| kept == slug) {
            skips.push(format!("{} ({slug})", stage.display().number().as_str()));
        }
    }

    let total = executes.len().to_string();
    set_field(
        read_model,
        field::STAGES_TO_EXECUTE,
        &executes.join(LIST_SEPARATOR),
    )?;
    set_field(
        read_model,
        field::STAGES_TO_SKIP,
        &if skips.is_empty() {
            NONE_LITERAL.to_string()
        } else {
            skips.join(LIST_SEPARATOR)
        },
    )?;
    set_field(read_model, field::TOTAL_STAGES, &total)
}

/// Skip 行の 1 項目から slug を取り出す。
///
/// 項目は `<番号> (<slug>)`、注釈付きなら `<番号> (<slug> — <理由>)`。括弧の中を取り、
/// em dash があればその手前までが slug である（upstream の `slugOfSkipToken` と同じ規則）。
/// 形に合わない項目は丸ごと slug として扱い、既知の slug に一致しないので保存されない。
fn slug_of_skip_token(token: &str) -> &str {
    let inner = token
        .split_once(" (")
        .and_then(|(_, rest)| rest.strip_suffix(')'))
        .unwrap_or(token);
    inner.split(SKIP_ANNOTATION).next().unwrap_or(inner)
}

// ---------------------------------------------------------------------------
// park / autonomy
// ---------------------------------------------------------------------------

/// `Parked` → `WORKFLOW_PARKED`、park マーカーの設置。
fn parked_event(
    parked: &Parked,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let fields = AuditFields::new().with(key(key::STAGE)?, parked.stage().as_str());
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

/// `AutonomyModeSet` → `AUTONOMY_MODE_SET`、`Construction Autonomy Mode`。
fn autonomy_mode_set(
    mode: AutonomyMode,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let fields = AuditFields::new().with(key(key::MODE)?, mode.as_state_field());
    read_model.append_audit(&render_audit_block(EventType::AutonomyModeSet, at, &fields));
    set_field(read_model, field::AUTONOMY_MODE, mode.as_state_field())
}

// ---------------------------------------------------------------------------
// 隔離実行 (`--single`) と walking-skeleton stance (b47 / #73)
// ---------------------------------------------------------------------------

/// 隔離実行の疑似ワークフロー ID（監査行の `**Workflow**:` の値 — ピン `:5017-5019`）。
///
/// これは本物の `WORKFLOW_STARTED` の id では**ない**。監査台帳の対を「本流ではなく隔離実行の
/// もの」と名乗らせるためだけに存在し、`<slug>` の並置で出所が読めるようにしてある。
fn synthetic_workflow_id(slug: &StageSlug) -> String {
    format!("single-stage:{}", slug.as_str())
}

/// `SingleStageRunCommitted` → `STAGE_COMPLETED` の監査1行。開始は専用イベントが描く。
///
/// 状態ファイルのフィールドもチェックボックスも `read_*` 表も**一切動かさない** — 適用が
/// フレーム空だからである（仕様 I10、オーナー裁定 2026-09-04）。`Last Updated` も触らない。
/// 行の並びと各行のフィールド順は upstream `handleSingleReport` の
/// `spawnAuditAppendBatch`（ピン `:5326-5343`）が正本である。
fn single_stage_run_committed(
    committed: &SingleStageRunCommitted,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let slug = committed.stage();
    plan.find(slug).ok_or_else(|| unknown(slug))?;
    let workflow = synthetic_workflow_id(slug);
    read_model.append_audit(&render_audit_block(
        EventType::StageCompleted,
        at,
        &AuditFields::new()
            .with(key(key::STAGE)?, slug.as_str())
            .with(
                key(key::DETAILS)?,
                &format!("Single-stage run of {} completed", slug.as_str()),
            )
            .with(key(key::WORKFLOW)?, &workflow),
    ));
    Ok(())
}

/// `SkeletonStanceRecorded` → `## Runtime State` の `Skeleton Stance` 欄（**監査行なし**）。
///
/// upstream の `aidlc-state.ts set-skeleton-stance`（ピン `:703-733`）は監査行を出さず
/// `Last Updated` も触らない — stance は状態機械の遷移ではなく、次の `next` が読む
/// ランタイム metadata だからである。欄は骨格テンプレートに無いので
/// **有れば置換・無ければセクション末尾へ挿入**する（upstream の `setOrInsertField`）。
///
/// # Errors
///
/// `## Runtime State` セクションが無ければ `ParkSectionMissing`（park マーカーと同じ
/// 置き場なので、不在の意味も同じである）。
fn skeleton_stance_recorded(
    recorded: &SkeletonStanceRecorded,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let next = with_field_or_insert(
        read_model.state(),
        RUNTIME_STATE_HEADING,
        field::SKELETON_STANCE,
        recorded.stance().as_str(),
    )
    .map_err(|_| ProjectionError::ParkSectionMissing)?;
    *read_model = read_model.clone().with_state(next);
    Ok(())
}

/// `ReviewRequested` → `REVIEW_REQUESTED` の**監査 1 行だけ**。
///
/// フィールドの並びは upstream `handleReview` の `fields` 構築順が正本である（ピン
/// `3c3146cf` `aidlc-log.ts:916-919` の `Stage` / `Reviewer` → `:988` の `Iteration` →
/// `:1058` の `Retry`）。`Unit` / `Workflow` は本 build では繰延（`--unit` / `--single` の
/// 受領証は未配線）なので描かない。
///
/// 状態ファイル・`Last Updated`・チェックボックス・`read_*` 表は**一切動かさない** —
/// upstream の `aidlc-log review` は `emitAudit` しか呼ばないからである。
fn review_requested(
    requested: &ReviewRequested,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let mut fields = AuditFields::new()
        .with(key(key::STAGE)?, requested.stage().as_str())
        .with(key(key::REVIEWER)?, requested.reviewer())
        .with(key(key::ITERATION)?, &requested.iteration().to_string());
    if requested.is_retry() {
        fields = fields.with(key(key::RETRY)?, RETRY_PENDING_REQUEST);
    }
    let fields = review_binding_fields(fields, requested.evidence())?;
    read_model.append_audit(&render_audit_block(EventType::ReviewRequested, at, &fields));
    Ok(())
}

/// `ReviewCompleted` → `REVIEW_COMPLETED` の**監査 1 行だけ**。
///
/// フィールドの並びは upstream `:916-919` / `:1128` / `:1135` の構築順である。
/// 要求・完成文書・追記境界・ソースの結合も、本家2.7.1の欄順で描く。
fn review_completed(
    completed: &ReviewCompleted,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let fields = AuditFields::new()
        .with(key(key::STAGE)?, completed.stage().as_str())
        .with(key(key::REVIEWER)?, completed.reviewer())
        .with(key(key::ITERATION)?, &completed.iteration().to_string())
        .with(key(key::VERDICT)?, completed.verdict().as_str())
        .with(
            key("Request Fingerprint")?,
            completed.evidence().request().fingerprint(),
        )
        .with(
            key("Artifact Fingerprint")?,
            completed.evidence().fingerprint(),
        );
    let fields = review_appendix_fields(fields, completed.evidence().request())?;
    let fields = if let Some(source) = completed.evidence().request().source() {
        fields
            .with(key("Request Source Fingerprint")?, source)
            .with(key("Source Fingerprint")?, source)
    } else {
        fields
    };
    read_model.append_audit(&render_audit_block(EventType::ReviewCompleted, at, &fields));
    Ok(())
}

fn review_binding_fields(
    fields: AuditFields,
    binding: &core_command_domain::orchestration::ReviewBinding,
) -> Result<AuditFields, ProjectionError> {
    let fields = fields.with(key("Artifact Fingerprint")?, binding.fingerprint());
    let fields = review_appendix_fields(fields, binding)?;
    Ok(if let Some(source) = binding.source() {
        fields.with(key("Source Fingerprint")?, source)
    } else {
        fields
    })
}
fn review_appendix_fields(
    fields: AuditFields,
    binding: &core_command_domain::orchestration::ReviewBinding,
) -> Result<AuditFields, ProjectionError> {
    let fields = fields
        .with(
            key("Review Appendix Artifact")?,
            binding.appendix_artifact(),
        )
        .with(
            key("Review Appendix Offset")?,
            &binding.appendix_offset().to_string(),
        )
        .with(key("Review Appendix Prior Digest")?, binding.prior_digest())
        .with(
            key("Review Appendix Prior Length")?,
            &binding.prior_length().to_string(),
        );
    Ok(if let Some(challenge) = binding.challenge() {
        fields.with(key("Review Challenge")?, challenge)
    } else {
        fields
    })
}

/// `LearningsCaptured` → メモリ層への実践行の追記と監査行 `RULE_LEARNED`。
///
/// 状態ファイルは動かさない — 学びは進行でも承認でもない。
///
/// # Errors
///
/// 実践行を書く学びが在るのにメモリ層が載っていない（`MemoryFilesMissing`）、監査キーの
/// 綴り違反。
fn learnings_captured(
    captured: &LearningsCaptured,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let practice_lines = captured
        .learnings()
        .filter(|learning| learning.disposition().writes_practice_line());
    if !practice_lines.is_empty() {
        let memory = read_model
            .memory()
            .ok_or(ProjectionError::MemoryFilesMissing)?;
        let (team, project) = append_practice_lines(
            memory.team(),
            memory.project(),
            captured,
            &practice_lines,
            at.date_naive(),
        )?;
        *read_model = read_model.clone().with_rewritten_memory(team, project);
    }
    let rows = captured
        .learnings()
        .filter(|learning| learning.disposition().writes_audit_row());
    let blocks = rows.fold_left(
        Ok(String::new()),
        |blocks: Result<String, ProjectionError>, entry| {
            let mut blocks = blocks?;
            let learning = entry.learning();
            blocks.push_str(&render_audit_block(
                EventType::RuleLearned,
                at,
                &AuditFields::new()
                    .with(key(key::STAGE)?, captured.stage().as_str())
                    .with(key(key::CANDIDATE_ID)?, learning.candidate_id().as_str())
                    .with(key(key::CONTENT_HASH)?, learning.content_hash().as_str())
                    .with(
                        key(key::DESTINATION)?,
                        &captured.provenance().destination(learning.scope()),
                    )
                    .with(key(key::HEADING)?, learning.heading().as_str())
                    .with(key(key::SOURCE)?, learning.source().as_str()),
            ));
            Ok(blocks)
        },
    )?;
    read_model.append_audit(&blocks);
    Ok(())
}

/// メモリ層 2 本の新しい本文を組む（純粋 — ディスクは触らない）。
///
/// 選ばれた見出しは**足りなければ作ってから**追記する — orchestrator は配布の正本が持たない
/// 見出しへも learnings を振れる（`aidlc-learnings.ts:836-841`）。
fn append_practice_lines(
    team: &str,
    project: &str,
    captured: &LearningsCaptured,
    practice_lines: &CapturedLearnings,
    learned_on: chrono::NaiveDate,
) -> Result<(String, String), ProjectionError> {
    practice_lines.fold_left(
        Ok((team.to_string(), project.to_string())),
        |faces: Result<(String, String), ProjectionError>, entry| {
            let (team, project) = faces?;
            let learning = entry.learning();
            let heading = learning.heading().as_str();
            let line = learning.practice_line(captured.provenance(), captured.stage(), learned_on);
            let (file, before) = match learning.scope() {
                LearningScope::Team => (TEAM_MD, team.as_str()),
                LearningScope::Project => (PROJECT_MD, project.as_str()),
            };
            let with_heading = ensure_heading(before, heading);
            let after = append_under_heading(&with_heading, heading, &line).map_err(|error| {
                ProjectionError::MemoryHeadingMissing {
                    file,
                    heading: error.as_str().to_string(),
                }
            })?;
            match learning.scope() {
                LearningScope::Team => Ok((after, project)),
                LearningScope::Project => Ok((team, after)),
            }
        },
    )
}

/// `PracticesAffirmed` → メモリ層 2 本の書き替え・状態ファイルの 2 欄・監査 1 行。
///
/// 書く順は upstream の Step 4〜7 の写しである: team.md の 5 節を置換 → project.md へ
/// 印付きの規則行を追記 → 状態ファイルの `Practices Affirmed Timestamp` と `Last Updated`
/// → 監査行 `PRACTICES_AFFIRMED`（ピン `3c3146cf` `aidlc-state.ts:3619-3757`）。
/// ディスクへ落とす順序（project.md が先）は取得ループが持つ。
///
/// # 重複除去は**ここでも**効く
///
/// 集約が載せた行は「昇格を計画した時点の正本」に対して重複除去済みだが、at-least-once の
/// 再投影ではその行が既に入っている。trim 一致で既在の行を飛ばすので、同じイベントを二度
/// 描いても project.md は 1 行しか増えない（upstream の `existingGuardrailLines` と同じ形）。
///
/// # Errors
///
/// メモリ層が載っていない（`MemoryFilesMissing`）、置換先・追記先の見出しが無い
/// （`MemoryHeadingMissing`）、状態ファイルの `## Project Information` が無い
/// （`ParkSectionMissing` と同じ材料 — 見出し不在）、監査キーの綴り違反。
fn practices_affirmed(
    affirmed: &PracticesAffirmed,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let memory = read_model
        .memory()
        .ok_or(ProjectionError::MemoryFilesMissing)?;
    let (team, project) = rewrite_memory(memory, affirmed)?;
    *read_model = read_model.clone().with_rewritten_memory(team, project);

    let stamp = iso8601_seconds(at);
    let next = with_field_or_insert(
        read_model.state(),
        PROJECT_INFORMATION_HEADING,
        field::PRACTICES_AFFIRMED_TIMESTAMP,
        &stamp,
    )
    // `with_field_or_insert` の拒否は `## ` を含む完全形の綴りを運ぶので、そのまま載せる。
    .map_err(|error| ProjectionError::MemoryHeadingMissing {
        file: STATE_FILE,
        heading: error.as_str().to_string(),
    })?;
    *read_model = read_model.clone().with_state(next);
    set_field(read_model, field::LAST_UPDATED, &stamp)?;

    let sections = affirmed
        .sections()
        .fold_left(Vec::new(), |mut headings, section| {
            headings.push(PromotedSection::heading(section));
            headings
        })
        .join(LIST_SEPARATOR);
    read_model.append_audit(&render_audit_block(
        EventType::PracticesAffirmed,
        at,
        &AuditFields::new()
            .with(key(key::AFFIRMING_USER)?, affirmed.affirming_user())
            .with(key(key::SECTIONS_WRITTEN)?, &sections)
            .with(
                key(key::MANDATED_RULES_APPENDED)?,
                &affirmed.mandated().len().to_string(),
            )
            .with(
                key(key::FORBIDDEN_RULES_APPENDED)?,
                &affirmed.forbidden().len().to_string(),
            ),
    ));
    Ok(())
}

/// メモリ層 2 本の新しい本文を組む（純粋 — ディスクは触らない）。
fn rewrite_memory(
    memory: &MemoryFaces,
    affirmed: &PracticesAffirmed,
) -> Result<(String, String), ProjectionError> {
    let team = affirmed.sections().fold_left(
        Ok(memory.team().to_string()),
        |team: Result<String, ProjectionError>, section| {
            let heading = format!("{MEMORY_HEADING_PREFIX}{}", section.heading());
            replace_section(&team?, &heading, section.body()).map_err(|error| {
                ProjectionError::MemoryHeadingMissing {
                    file: TEAM_MD,
                    heading: error.as_str().to_string(),
                }
            })
        },
    )?;
    let mut project = memory.project().to_string();
    for (heading, rules) in [
        (MANDATED_HEADING, affirmed.mandated()),
        (FORBIDDEN_HEADING, affirmed.forbidden()),
    ] {
        project = rules.fold_left(
            Ok(project),
            |project: Result<String, ProjectionError>, rule| {
                let project = project?;
                if project.split('\n').any(|line| line.trim() == rule) {
                    return Ok(project);
                }
                append_under_heading(&project, heading, &format!("{rule}\n")).map_err(|error| {
                    ProjectionError::MemoryHeadingMissing {
                        file: PROJECT_MD,
                        heading: error.as_str().to_string(),
                    }
                })
            },
        )?;
    }
    Ok((team, project))
}

// ---------------------------------------------------------------------------
// 共有の断片
// ---------------------------------------------------------------------------

/// フェーズ境界の 3 行（完了 → 検証 → 次フェーズ開始）。
///
/// `**Stages completed**:` は**呼出側が決める**。genesis は計画上の initialization 件数
/// （まだ 1 つも倒れていない時点で描くため）、ゲート承認は倒したあとのチェックボックスの
/// 数え直しで、値が一致しない（`cli/report/approved-across-phases` は 2、計画上の inception 内
/// スコープ件数は 8）。ジャンプの境界 3 行は `**Details**:` を持つので別関数である。
fn append_phase_boundary(
    read_model: &mut ReadModel,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    boundary: PhaseBoundary,
    stages_completed: &str,
) -> Result<(), ProjectionError> {
    let from = boundary.from_phase();
    let to = boundary.to_phase();
    read_model.append_audit(&render_audit_block(
        EventType::PhaseCompleted,
        at,
        &AuditFields::new()
            .with(key(key::FROM_PHASE)?, from.as_str())
            .with(key(key::TO_PHASE)?, to.as_str())
            .with(key(key::STAGES_COMPLETED)?, stages_completed),
    ));
    read_model.append_audit(&render_audit_block(
        EventType::PhaseVerified,
        at,
        &AuditFields::new().with(
            key(key::PHASE_BOUNDARY)?,
            &format!("{}{BOUNDARY_ARROW}{}", from.as_str(), to.as_str()),
        ),
    ));
    read_model.append_audit(&render_audit_block(
        EventType::PhaseStarted,
        at,
        &AuditFields::new()
            .with(key(key::PHASE)?, to.as_str())
            .with(key(key::SCOPE)?, plan.scope()),
    ));
    Ok(())
}

/// 通過したフェーズを `Verified`、入ったフェーズを `Active` にする
/// （`cli/report/approved-across-phases`）。
///
/// [`append_phase_boundary`] とは分けてある — あちらは genesis の**監査行だけを描く**段からも
/// 呼ばれ、その時点では状態ファイルの本文がまだ無いことがあるからである。genesis 自身は
/// 骨格が既にこの 2 行を正しい値で持っているので、書き換える必要がない。
fn set_phase_progress_for_advance(
    read_model: &mut ReadModel,
    boundary: PhaseBoundary,
) -> Result<(), ProjectionError> {
    set_field(
        read_model,
        &field::phase_row(boundary.from_phase()),
        phase_status::VERIFIED,
    )?;
    set_field(
        read_model,
        &field::phase_row(boundary.to_phase()),
        phase_status::ACTIVE,
    )
}

/// `STAGE_STARTED` 行 1 本。
fn stage_started_row(
    stage: &PlannedStage,
    at: &DateTime<Utc>,
    baseline: Option<&core_command_domain::orchestration::SourceBaseline>,
) -> Result<String, ProjectionError> {
    let mut fields = AuditFields::new()
        .with(key(key::STAGE)?, stage.slug().as_str())
        .with(key(key::AGENT)?, stage.display().lead_agent());
    if let Some(baseline) = baseline {
        fields = fields.with(key("Source Baseline")?, &baseline.fingerprint());
    }
    Ok(render_audit_block(EventType::StageStarted, at, &fields))
}

/// ワークフローを畳んだ経路（`WORKFLOW_COMPLETED` の材料が違う）。
///
/// upstream では完了の後始末が 2 か所にある — ゲート承認は `handleApprove` が
/// `handleCompleteWorkflow` へ自己委譲し（`aidlc-state.ts:2839-2844`）、ルーティングされた
/// 読み飛ばしは `handleSkip --route` が自分で書く（同 `:3167-3232`）。書く欄は同じだが、
/// `WORKFLOW_COMPLETED` の `Details` と `Reason` が違うので、経路を型で運ぶ。
#[derive(Debug, Clone, Copy)]
enum Completion<'a> {
    /// 承認が最後の in-scope ステージを畳んだ（`complete-workflow`）。
    Approved,
    /// 読み飛ばしが最後の in-scope ステージを畳んだ（`skip --route`）。
    Skipped {
        /// 読み飛ばしの理由（`WORKFLOW_COMPLETED` の `Reason` になる）。
        reason: &'a str,
    },
}

/// 次ステージへ移る（次が無ければワークフロー完了）。
fn leave_for(
    read_model: &mut ReadModel,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    next: Option<&StageSlug>,
    completed: &StageSlug,
    completion: Completion<'_>,
    baseline: Option<&core_command_domain::orchestration::SourceBaseline>,
) -> Result<(), ProjectionError> {
    match next {
        Some(slug) => enter_stage(read_model, at, plan, slug, baseline),
        None => complete_workflow(read_model, at, plan, completed, completion),
    }
}

/// ワークフロー完了の投影 — 状態 7 欄 + フェーズ行 + 監査 3 行。
///
/// upstream `aidlc-state.ts handleCompleteWorkflow`（`:2415-2560`）と
/// `handleSkip --route` の完了枝（`:3167-3232`）を写したものである。
///
/// # `STAGE_COMPLETED` はここでは描かない
///
/// upstream の `complete-workflow` は `alreadyMarkedCompleted && stageCompletedAlreadyAudited`
/// のとき `STAGE_COMPLETED` を再 emit しない（`:2498`）。承認経路は直前の `approve` が
/// `Stage <Name> approved by gate` を既に書いており、読み飛ばし経路は `STAGE_SKIPPED` しか
/// 書かない（読み飛ばしは完了ではない）。したがって `Final stage <Name> completed` の行は
/// **report からは到達しない** — 到達するのは `complete-workflow` を直に叩く経路だけで、
/// それはこの build に無い（b42 で撤去）。
fn complete_workflow(
    read_model: &mut ReadModel,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    completed: &StageSlug,
    completion: Completion<'_>,
) -> Result<(), ProjectionError> {
    let stage = plan.find(completed).ok_or_else(|| unknown(completed))?;
    let phase = stage.phase();
    // 完了数はチェックボックスの数え直しである（読み飛ばしは `[S]` なので増えない）。
    let completed_count = completed_count(read_model);

    set_field(read_model, field::COMPLETED, &completed_count)?;
    set_field(read_model, field::STATUS, STATUS_COMPLETED)?;
    set_field(read_model, field::LAST_UPDATED, &iso8601_seconds(at))?;
    set_field(read_model, field::IN_PROGRESS, NONE_LITERAL)?;
    set_field(read_model, field::NEXT_STAGE, NONE_LITERAL)?;
    set_field(read_model, field::NEXT_ACTION, WORKFLOW_COMPLETE_ACTION)?;
    // 最終フェーズの行は前進側の境界処理では倒れない（ステージ間の境界でしか発火しない）
    // ので、ここで `Verified` にする（upstream `:2483` / `:3172`）。
    set_field(read_model, &field::phase_row(phase), phase_status::VERIFIED)?;

    read_model.append_audit(&render_audit_block(
        EventType::PhaseCompleted,
        at,
        &AuditFields::new()
            .with(key(key::FROM_PHASE)?, phase.as_str())
            .with(key(key::TO_PHASE)?, END_PHASE)
            .with(key(key::STAGES_COMPLETED)?, &completed_count),
    ));
    read_model.append_audit(&render_audit_block(
        EventType::PhaseVerified,
        at,
        &AuditFields::new().with(
            key(key::PHASE_BOUNDARY)?,
            &format!("{}{BOUNDARY_ARROW}{END_BOUNDARY}", phase.as_str()),
        ),
    ));
    let scope = plan.scope();
    let mut fields = AuditFields::new().with(key(key::SCOPE)?, scope);
    fields = match completion {
        Completion::Approved => fields.with(
            key(key::DETAILS)?,
            &format!("Scope: {scope}, {completed_count} stages completed"),
        ),
        Completion::Skipped { reason } => fields
            .with(
                key(key::DETAILS)?,
                &format!("Scope: {scope}, final stage {completed} skipped"),
            )
            .with(key(key::REASON)?, reason),
    };
    read_model.append_audit(&render_audit_block(
        EventType::WorkflowCompleted,
        at,
        &fields,
    ));
    Ok(())
}

/// ステージを開始する — `STAGE_STARTED` 行と、現在位置まわりの状態フィールド。
fn enter_stage(
    read_model: &mut ReadModel,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    slug: &StageSlug,
    baseline: Option<&core_command_domain::orchestration::SourceBaseline>,
) -> Result<(), ProjectionError> {
    let stage = plan.find(slug).ok_or_else(|| unknown(slug))?;
    read_model.append_audit(&stage_started_row(stage, at, baseline)?);
    enter_stage_without_row(read_model, plan, slug)
}

/// 現在位置まわりの状態フィールドだけを書く（`STAGE_STARTED` 行は描かない）。
///
/// `Started` は初期化 3 ステージの行を先に描き終えており、最初のゲート付きステージの
/// `STAGE_STARTED` もその列の最後に既に入っている。ここで二度描かないための分割である。
fn enter_stage_without_row(
    read_model: &mut ReadModel,
    plan: &ResolvedPlan,
    slug: &StageSlug,
) -> Result<(), ProjectionError> {
    let stage = plan.find(slug).ok_or_else(|| unknown(slug))?;
    set_checkbox(read_model, slug.as_str(), CheckboxState::InProgress)?;
    set_field(
        read_model,
        field::ACTIVE_AGENT,
        stage.display().lead_agent(),
    )?;
    // 値は大文字。同じフェーズへ移るケースでは書き換えても値が変わらないので、フェーズを
    // またぐジャンプ (`cli/jump/execute-backward` ほか) だけがこの行の差分を見せる。
    set_field(
        read_model,
        field::LIFECYCLE_PHASE,
        &stage.phase().as_str().to_uppercase(),
    )?;
    set_field(read_model, field::IN_PROGRESS, slug.as_str())?;
    set_field(read_model, field::CURRENT_STAGE, slug.as_str())?;
    set_field(
        read_model,
        field::NEXT_STAGE,
        plan.next_in_scope_after(slug)
            .map_or("", |next| next.slug().as_str()),
    )?;
    set_field(
        read_model,
        field::NEXT_ACTION,
        &format!("Execute {}", stage.display().name()),
    )
}

/// ステージを完了させる — チェックボックス `[x]`、完了数の同期、最終完了ステージ。
fn complete_stage(read_model: &mut ReadModel, slug: &StageSlug) -> Result<(), ProjectionError> {
    set_checkbox(read_model, slug.as_str(), CheckboxState::Completed)?;
    set_field(read_model, field::COMPLETED, &completed_count(read_model))?;
    set_field(read_model, field::LAST_COMPLETED_STAGE, slug.as_str())
}

/// いま `[x]` のチェックボックスの数（`- **Completed**:` と境界行の材料）。
fn completed_count(read_model: &ReadModel) -> String {
    Checkboxes::parse(read_model.state())
        .count_completed()
        .to_string()
}

/// リードモデルの行末トークンから実効プランを読む (無ければ静的計画の値)。
///
/// イベントは事実だけを運ぶ (オーナー裁定 2026-08-30) ので、次カーソル・in-scope 数の材料は
/// **リードモデル自身**である — `rebuild_plan_rows` と同じ読み方 (状態の正本は自分の行)。
fn effective_action(read_model: &ReadModel, stage: &PlannedStage) -> PlanAction {
    Checkboxes::parse(read_model.state())
        .find(stage.slug().as_str())
        .and_then(|entry| match entry.rest().trim() {
            "EXECUTE" => Some(PlanAction::Execute),
            "SKIP" => Some(PlanAction::Skip),
            _ => None,
        })
        .unwrap_or_else(|| stage.plan_action())
}

/// 名指しステージの後で実効 EXECUTE の最初のステージ (無ければ `None` = ワークフロー完了)。
fn next_in_effective_scope(
    read_model: &ReadModel,
    plan: &ResolvedPlan,
    after: &StageSlug,
) -> Option<StageSlug> {
    plan.stages()
        .iter()
        .skip_while(|stage| stage.slug() != after)
        .skip(1)
        .find(|stage| effective_action(read_model, stage) == PlanAction::Execute)
        .map(|stage| stage.slug().clone())
}

/// 計画上の表題を引く。
fn title_of(plan: &ResolvedPlan, slug: &StageSlug) -> Result<String, ProjectionError> {
    plan.display_of(slug)
        .map(|display| display.name().to_string())
        .ok_or_else(|| unknown(slug))
}

/// 計画上のステージ番号を引く。
fn number_of(plan: &ResolvedPlan, slug: &StageSlug) -> Result<String, ProjectionError> {
    plan.display_of(slug)
        .map(|display| display.number().as_str().to_string())
        .ok_or_else(|| unknown(slug))
}

fn unknown(slug: &StageSlug) -> ProjectionError {
    ProjectionError::UnknownStage {
        stage: slug.as_str().to_string(),
    }
}

/// フェーズの文書順の位置。
///
/// `PhaseId::ALL` を `position` で引くと「見つからない」枝が生まれるが、閉集合なので実際には
/// 起きない。網羅 `match` にすればその枝が消え、フェーズを増やしたときは**コンパイルエラー**で
/// ここを直すよう強制できる（順序が `PhaseId::ALL` と一致することは単体テストが見張る）。
const fn phase_order(phase: PhaseId) -> usize {
    match phase {
        PhaseId::Initialization => 0,
        PhaseId::Ideation => 1,
        PhaseId::Inception => 2,
        PhaseId::Construction => 3,
        PhaseId::Operation => 4,
    }
}

/// `JumpDirection` のワイヤ綴り（`**Direction**:` は大文字）。
const fn direction_spelling(direction: JumpDirection) -> &'static str {
    match direction {
        JumpDirection::Forward => "FORWARD",
        JumpDirection::Backward => "BACKWARD",
        JumpDirection::Redo => "REDO",
    }
}

/// ステージ集合を `**Stages ...**:` の値へ描く（空は逐語 `none`）。
fn stage_list(stages: &[String]) -> String {
    if stages.is_empty() {
        return NONE_LITERAL.to_string();
    }
    stages.join(LIST_SEPARATOR)
}

/// 反転した slug 集合を計画の**文書順**へ並べ直す。
///
/// [`StageSlugSet`] は辞書順である（集合として一意にするための順序であり、業務の順序では
/// ない）。監査行 `**Stages skipped**:` と行末トークンの書き替えはどちらも upstream が
/// 文書順で描くので、計画の位置で引き直してから使う — 型側の順序は変えない
/// （委任 2 の申し送り、NFR1）。計画に無い slug は写さない。
fn in_document_order(plan: &ResolvedPlan, slugs: &StageSlugSet) -> Vec<String> {
    plan.stages()
        .iter()
        .filter(|stage| slugs.contains(stage.slug()))
        .map(|stage| stage.slug().as_str().to_string())
        .collect()
}

/// チェックボックスのマーカーだけを書き換える（接尾辞には触れない）。
fn set_checkbox(
    read_model: &mut ReadModel,
    slug: &str,
    state: CheckboxState,
) -> Result<(), ProjectionError> {
    let next = Checkboxes::with_marker(read_model.state(), slug, state)?;
    *read_model = read_model.clone().with_state(next);
    Ok(())
}

/// チェックボックスの行末トークンだけを書き換える（マーカーには触れない）。
fn set_suffix(
    read_model: &mut ReadModel,
    slug: &str,
    action: PlanAction,
) -> Result<(), ProjectionError> {
    let next = Checkboxes::with_suffix(read_model.state(), slug, action)?;
    *read_model = read_model.clone().with_state(next);
    Ok(())
}

/// 状態ファイルのフィールド行を書き換える（不在は拒否 — 無言 no-op は検出不能なドリフト）。
fn set_field(read_model: &mut ReadModel, field: &str, value: &str) -> Result<(), ProjectionError> {
    let next = with_field(read_model.state(), field, value)?;
    *read_model = read_model.clone().with_state(next);
    Ok(())
}

/// 一覧フィールドの現在値を項目へ割る（空は 0 項目）。
fn list_of(read_model: &ReadModel, field: &str) -> Result<Vec<String>, ProjectionError> {
    let raw = find_field(read_model.state(), field).ok_or_else(|| {
        ProjectionError::StateField(FieldNotFound::new(super::wording::field_not_found_message(
            field,
        )))
    })?;
    Ok(if raw.is_empty() {
        Vec::new()
    } else {
        raw.split(LIST_SEPARATOR)
            .map(|entry| entry.trim().to_string())
            .collect()
    })
}

/// park マーカー — `## Runtime State` セクションの**末尾**（次の見出しの直前）に 2 行、
/// unpark で 2 行とも消す。
///
/// # なぜ `with_field_or_insert` を使わないのか
///
/// 挿入位置は同じ（どちらも次の `## ` 見出しの直前 — ゴールデン `cli/park/park/state.diff`
/// の実バイトがその形である）が、park マーカーは**2 行を対で置き直す**書き手である。
/// 既存のマーカーを 2 行とも落としてから同じ位置へ 2 行入れる（再 park の冪等）ので、
/// 「1 フィールドを置換 or 挿入」の口では表せない。
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
            .skip(start.saturating_add(1))
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
        *read_model = read_model.clone().with_state(rejoin(&out, &cleared));
        Ok(())
    }

    /// park マーカーを除去する（不在は no-op — 二重 unpark で落ちない）。
    pub(super) fn clear(read_model: &mut ReadModel) {
        let next = removed(read_model.state());
        *read_model = read_model.clone().with_state(next);
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
    use core_command_domain::orchestration::Created;
    use core_command_domain::orchestration::{GateApproved, GateRejected, StageSkipped};
    use core_command_domain::workspace::{PromotedSections, RuleLines};

    #[test]
    fn protected_plan_answer_renders_its_evidence_without_changing_public_state() {
        use core_command_domain::orchestration::{
            CodeGenerationAuthority, PlanAnswerInput, PlanAnswerLogged, PlanApprovalEvidence,
            PlanApprovalOperationId, PlanApprovalOrigin, PlanChoice, PlanDecisionEvidence,
            PlanSession, PlanTarget,
        };
        let authority = CodeGenerationAuthority::new(
            &PlanTarget::stage_level(),
            &IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001").unwrap(),
            format!("sha256:{}", "a".repeat(64)),
            "WORKFLOW_STARTED:2026-09-08T01:00:00Z#1".to_string(),
            "b".repeat(64),
            2,
        )
        .unwrap();
        let evidence = PlanApprovalEvidence::new(
            authority,
            format!("sha256:{}", "c".repeat(64)),
            "questions.md".to_string(),
            "d".repeat(64),
            "e".repeat(64),
        )
        .unwrap();
        for (choice, event_type) in [
            (PlanChoice::ApprovePlan, "PLAN_APPROVAL_RECORDED"),
            (PlanChoice::RequestChanges, "QUESTION_ANSWERED"),
        ] {
            let input = PlanAnswerInput::new(
                PlanApprovalOrigin::new(
                    core_command_domain::workspace::SpaceName::default(),
                    execution_id(),
                ),
                "code-generation".to_string(),
                PlanDecisionEvidence::new(
                    evidence.clone(),
                    PlanSession::new("session".to_string()).unwrap(),
                ),
                choice,
                Some("b".repeat(64)),
            );
            let mut actual = model();
            let before = actual.state().to_string();
            project(
                &[entry(IntentExecutionEvent::PlanAnswerLogged(Box::new(
                    PlanAnswerLogged::new(
                        event_id(),
                        execution_id(),
                        PlanApprovalOperationId::generate(),
                        input,
                    ),
                )))],
                &plan(),
                &mut actual,
            )
            .unwrap();
            assert_eq!(actual.state(), before);
            let fields = AuditFields::new()
                .with(key("Stage").unwrap(), "code-generation")
                .with(key("Details").unwrap(), choice.as_str())
                .with(key("Checkpoint").unwrap(), "Code Generation Plan Approval")
                .with(key("Plan Target").unwrap(), "stage:code-generation")
                .with(
                    key("Intent").unwrap(),
                    "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001",
                )
                .with(
                    key("Directive Epoch").unwrap(),
                    &format!("sha256:{}", "a".repeat(64)),
                )
                .with(
                    key("Run floor").unwrap(),
                    "WORKFLOW_STARTED:2026-09-08T01:00:00Z#1",
                )
                .with(
                    key("Approval Fingerprint").unwrap(),
                    &format!("sha256:{}", "c".repeat(64)),
                )
                .with(key("Questions File").unwrap(), "questions.md")
                .with(key("Questions SHA-256").unwrap(), &"d".repeat(64))
                .with(key("Prompt SHA-256").unwrap(), &"e".repeat(64))
                .with(key("Session").unwrap(), "session");
            let kind = EventType::parse(event_type).expect("本家2.7.1の監査語彙");
            assert_eq!(
                actual.appended_audit(),
                render_audit_block(kind, &at(), &fields)
            );
        }
    }

    /// b40 のテスト用固定イベント識別子 (同じ材料から組んだイベントを同値に保つため)。
    fn event_id() -> IntentExecutionEventId {
        IntentExecutionEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002").expect("UUIDv7")
    }

    /// b40 のテスト用集約識別子 (行の `aid` と payload の `aggregate_id` を揃える)。
    fn execution_id() -> IntentExecutionId {
        IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7")
    }

    /// b40 のテスト用固定イベント識別子 (intent 面)。
    fn intent_event_id() -> IntentEventId {
        IntentEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001").expect("UUIDv7")
    }

    #[test]
    fn a_missing_field_converts_into_the_projection_error() {
        // `set_field(...)?` が通る変換経路 (From) — 材料をそのまま包む。
        let inner = FieldNotFound::new("Field not found: Current Stage");
        assert_eq!(
            ProjectionError::from(inner.clone()),
            ProjectionError::StateField(inner)
        );
    }

    use core_command_domain::orchestration::{
        AutonomyModeSet, Intent, IntentEventId, IntentExecutionEventId, IntentExecutionId,
        IntentId, ReviewVerdict, SingleStageRunCommitted, SkeletonStance, SkeletonStanceRecorded,
        StageDisplay, StageEntries, StageEntry, StartRequest, Started, WorkspaceScan,
    };
    use core_command_domain::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, StageNumber, WorkflowDefinitionId,
    };

    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-08-21T09:14:07Z")
            .expect("固定の ISO 8601")
            .with_timezone(&Utc)
    }

    fn slug(value: &str) -> StageSlug {
        StageSlug::parse(value).expect("テストの slug は文法内")
    }

    fn stage(name: &str, number: &str, phase: PhaseId, action: PlanAction) -> StageEntry {
        StageEntry::new(
            slug(name),
            phase,
            action,
            false,
            StageDisplay::new(
                StageNumber::parse(number).expect("番号"),
                "Some Title",
                "orchestrator",
            )
            .expect("単一行"),
        )
    }

    /// initialization 1 + inception 2 + operation 1 の合成計画。
    fn genesis_intent() -> Intent {
        Intent::from((
            Created::new(
                intent_event_id(),
                IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7"),
                WorkflowDefinitionId::parse("claude").expect("定義 id"),
                DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
                StartRequest::new("classic", "build it"),
                StageEntries::new(vec![
                    stage(
                        "state-init",
                        "0.1",
                        PhaseId::Initialization,
                        PlanAction::Execute,
                    ),
                    stage("first", "2.1", PhaseId::Inception, PlanAction::Execute),
                    stage("second", "2.2", PhaseId::Inception, PlanAction::Execute),
                    stage("late", "4.1", PhaseId::Operation, PlanAction::Skip),
                ])
                .expect("フィクスチャの計画は不変条件を満たす"),
                WorkspaceScan::new(
                    BrownfieldGreenfield::Greenfield,
                    "Unknown",
                    "Unknown",
                    "Unknown",
                )
                .expect("単一行"),
            ),
            at(),
        ))
    }

    fn started() -> Started {
        let intent = genesis_intent();
        Started::new(
            event_id(),
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7"),
            intent.id().clone(),
            intent.stages().clone(),
        )
    }

    fn plan() -> ResolvedPlan {
        ResolvedPlan::of(&genesis_intent())
    }

    const SKELETON: &str = "\
## Project Information
- **Active Agent**: orchestrator

## Scope Configuration
- **Stages to Execute**: 0.1, 2.1, 2.2
- **Stages to Skip**: 4.1 (late)

## Execution Plan Summary
- **Total Stages**: 3
- **Completed**: 0
- **In Progress**: state-init

## Runtime State
- **Revision Count**: 0
- **Construction Autonomy Mode**: gated

## Stage Progress
- [-] state-init — EXECUTE
- [ ] first — EXECUTE
- [ ] second — EXECUTE
- [ ] late — SKIP

## Phase Progress
- **Initialization**: Active
- **Ideation**: Pending
- **Inception**: Pending
- **Construction**: Pending
- **Operation**: Pending

## Current Status
- **Lifecycle Phase**: INITIALIZATION
- **Current Stage**: state-init
- **Next Stage**: first
- **Status**: Running
- **Last Updated**: 2026-08-20T00:00:00Z

## Session Resume Point
- **Last Completed Stage**: 
- **Next Action**: Execute Stage
";

    fn model() -> ReadModel {
        ReadModel::new(SKELETON)
    }

    fn entry(event: IntentExecutionEvent) -> JournalEntry {
        JournalEntry::new(
            crate::orchestration::GlobalSeqNr::new(1),
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7"),
            1,
            at(),
            event,
        )
    }

    fn run(event: IntentExecutionEvent) -> ReadModel {
        let mut read_model = model();
        project(&[entry(event)], &plan(), &mut read_model).expect("投影");
        read_model
    }

    #[test]
    fn the_phase_order_agrees_with_the_declared_document_order() {
        // 網羅 match で書いた順序が `PhaseId::ALL` とずれていないことを見張る
        // （ずれると `## Phase Progress` の前方 / 後方の判定が逆になる）。
        for (index, phase) in PhaseId::ALL.into_iter().enumerate() {
            assert_eq!(phase_order(phase), index, "{}", phase.as_str());
        }
    }

    #[test]
    fn the_genesis_lands_on_the_first_gated_stage_when_the_skeleton_exists() {
        let read_model = run(IntentExecutionEvent::Started(started()));
        // initialization は完了、最初のゲート付きステージが in-flight。
        assert!(read_model.state().contains("- [x] state-init — EXECUTE"));
        assert!(read_model.state().contains("- [-] first — EXECUTE"));
        assert!(read_model.state().contains("- **Completed**: 1\n"));
        assert!(
            read_model
                .state()
                .contains("- **Last Completed Stage**: state-init\n")
        );
        // スコープ内は state-init / first / second の 3 つ。
        assert!(read_model.state().contains("- **Total Stages**: 3\n"));
        assert!(read_model.state().contains("- **Next Stage**: second\n"));
        assert!(
            read_model
                .state()
                .contains("- **Next Action**: Execute first\n")
        );
        // 計画一覧は触らない（畳まれた理由を導けないため）。
        assert!(
            read_model
                .state()
                .contains("- **Stages to Skip**: 4.1 (late)\n")
        );
        // `STAGE_STARTED` は 1 本だけ（行を二度描かない）。
        assert_eq!(
            read_model
                .appended_audit()
                .matches("**Event**: STAGE_STARTED")
                .count(),
            2,
            "初期化 1 本 + 最初のゲート付き 1 本"
        );
    }

    #[test]
    fn a_genesis_without_a_skeleton_stops_instead_of_inventing_one() {
        let mut read_model = ReadModel::new("   \n");
        let error = project(
            &[entry(IntentExecutionEvent::Started(started()))],
            &plan(),
            &mut read_model,
        )
        .expect_err("骨格が無い");
        assert_eq!(error, ProjectionError::ScaffoldMissing);
        assert_eq!(error.to_string(), "scaffold missing");
        // 監査行だけは描けている（骨格が無くても台帳は書ける）。
        assert!(
            read_model
                .appended_audit()
                .contains("**Event**: WORKFLOW_STARTED")
        );
    }

    #[test]
    fn switching_the_autonomy_mode_writes_the_row_and_the_field() {
        let read_model = run(IntentExecutionEvent::AutonomyModeSet(AutonomyModeSet::new(
            event_id(),
            execution_id(),
            AutonomyMode::Autonomous,
        )));
        assert!(
            read_model
                .appended_audit()
                .contains("**Event**: AUTONOMY_MODE_SET")
        );
        assert!(
            read_model
                .appended_audit()
                .contains("**Mode**: autonomous\n")
        );
        assert!(
            read_model
                .state()
                .contains("- **Construction Autonomy Mode**: autonomous\n")
        );
    }

    // ---- b47: 隔離実行 (`--single`) と walking-skeleton stance ----

    #[test]
    fn an_isolated_run_appends_the_two_audit_rows_verbatim_and_touches_nothing_else() {
        let before = model();
        let mut read_model = model();
        project(
            &[
                entry(IntentExecutionEvent::SingleStageRunStarted(
                    core_command_domain::orchestration::SingleStageRunStarted::new(
                        event_id(),
                        execution_id(),
                        slug("first"),
                    ),
                )),
                entry(IntentExecutionEvent::SingleStageRunCommitted(
                    SingleStageRunCommitted::new(event_id(), execution_id(), slug("first")),
                )),
            ],
            &plan(),
            &mut read_model,
        )
        .unwrap();
        // 監査 2 行が upstream の順序・フィールド順で並ぶ (ピン `:5326-5343`)。
        assert_eq!(
            audit_events(&read_model),
            ["STAGE_STARTED", "STAGE_COMPLETED"]
        );
        let appended = read_model.appended_audit();
        assert!(
            appended.contains(
                "**Stage**: first\n**Agent**: orchestrator\n**Workflow**: single-stage:first\n"
            ),
            "STAGE_STARTED の 3 フィールドはこの順: {appended}"
        );
        assert!(
            appended.contains(
                "**Stage**: first\n**Details**: Single-stage run of first completed\n**Workflow**: single-stage:first\n"
            ),
            "STAGE_COMPLETED の 3 フィールドはこの順: {appended}"
        );
        // 状態ファイルは 1 バイトも動かない (フレーム空 — 仕様 I10)。
        assert_eq!(
            read_model.state(),
            before.state(),
            "隔離実行は本流の状態ファイルを動かさない"
        );
    }

    #[test]
    fn an_isolated_completion_of_a_stage_outside_the_plan_is_refused() {
        let mut read_model = model();
        let error = project(
            &[entry(IntentExecutionEvent::SingleStageRunCommitted(
                SingleStageRunCommitted::new(event_id(), execution_id(), slug("nowhere")),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect_err("計画に無いステージ");
        assert_eq!(
            error,
            ProjectionError::UnknownStage {
                stage: "nowhere".to_string()
            }
        );
    }

    #[test]
    fn recording_a_skeleton_stance_inserts_the_runtime_field_without_an_audit_row() {
        let read_model = run(IntentExecutionEvent::SkeletonStanceRecorded(
            SkeletonStanceRecorded::new(event_id(), execution_id(), SkeletonStance::On),
        ));
        // 骨格に無い欄なので `## Runtime State` の末尾へ挿入される。
        assert!(
            read_model.state().contains("- **Skeleton Stance**: on\n"),
            "{}",
            read_model.state()
        );
        // 監査行は出さない (upstream `set-skeleton-stance` は台帳を触らない)。
        assert_eq!(audit_events(&read_model), Vec::<&str>::new());
        // `Last Updated` も触らない。
        assert!(
            read_model
                .state()
                .contains("- **Last Updated**: 2026-08-20T00:00:00Z\n")
        );
    }

    #[test]
    fn a_second_stance_replaces_the_field_instead_of_appending_another_line() {
        let mut read_model = model();
        project(
            &[entry(IntentExecutionEvent::SkeletonStanceRecorded(
                SkeletonStanceRecorded::new(event_id(), execution_id(), SkeletonStance::On),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect("1 回目");
        project(
            &[entry(IntentExecutionEvent::SkeletonStanceRecorded(
                SkeletonStanceRecorded::new(
                    event_id(),
                    execution_id(),
                    SkeletonStance::ScopeDependent,
                ),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect("2 回目");
        assert_eq!(
            read_model.state().matches("- **Skeleton Stance**:").count(),
            1,
            "行は 1 本のまま"
        );
        assert!(
            read_model
                .state()
                .contains("- **Skeleton Stance**: scope-dependent\n")
        );
    }

    // ---- b48: レビュー受領証 (#51 / B10) ----

    #[test]
    fn a_review_request_appends_one_audit_row_in_the_upstream_field_order() {
        let before = model();
        let read_model = run(IntentExecutionEvent::ReviewRequested(ReviewRequested::new(
            event_id(),
            execution_id(),
            slug("first"),
            "aidlc-quality-agent",
            1,
            false,
            crate::review_test_fixture::binding(),
        )));
        assert_eq!(audit_events(&read_model), ["REVIEW_REQUESTED"]);
        let appended = read_model.appended_audit();
        assert!(
            appended.contains(
                "**Stage**: first\n**Reviewer**: aidlc-quality-agent\n**Iteration**: 1\n"
            ),
            "フィールドはこの順 (upstream `:916-919` / `:988`): {appended}"
        );
        // 呼び直しでなければ `Retry` は載らない。
        assert!(!appended.contains("**Retry**:"), "{appended}");
        // 状態ファイルは 1 バイトも動かない (`aidlc-log review` は emitAudit だけである)。
        assert_eq!(read_model.state(), before.state());
    }

    #[test]
    fn a_retried_review_request_adds_the_retry_field_last() {
        let read_model = run(IntentExecutionEvent::ReviewRequested(ReviewRequested::new(
            event_id(),
            execution_id(),
            slug("first"),
            "aidlc-quality-agent",
            2,
            true,
            crate::review_test_fixture::binding(),
        )));
        assert!(
            read_model.appended_audit().contains(
                "**Stage**: first\n**Reviewer**: aidlc-quality-agent\n**Iteration**: 2\n**Retry**: pending-request\n"
            ),
            "{}",
            read_model.appended_audit()
        );
    }

    #[test]
    fn a_review_verdict_appends_one_audit_row_ending_with_the_verdict() {
        let before = model();
        let read_model = run(IntentExecutionEvent::ReviewCompleted(ReviewCompleted::new(
            event_id(),
            execution_id(),
            slug("second"),
            "aidlc-architecture-reviewer-agent",
            2,
            ReviewVerdict::NotReady,
            crate::review_test_fixture::completion(),
        )));
        assert_eq!(audit_events(&read_model), ["REVIEW_COMPLETED"]);
        assert!(
            read_model.appended_audit().contains(
                "**Stage**: second\n**Reviewer**: aidlc-architecture-reviewer-agent\n**Iteration**: 2\n**Verdict**: NOT-READY\n"
            ),
            "{}",
            read_model.appended_audit()
        );
        // 要求原文とReview節を含む完成原文を、別の固定指紋で監査へ束縛する。
        assert!(read_model.appended_audit().contains(
            "**Request Fingerprint**: sha256:40a450c7f1afe19930706ee78a898e60d9a4b93b81b308ed890cdafe8e74777d\n**Artifact Fingerprint**: sha256:e985de06efb4acc251ce219f41f822c0d3367e3e9ca13c8b2d0f2bb4541595f0\n**Review Appendix Artifact**: stage/artifact.md\n**Review Appendix Offset**: 11\n**Review Appendix Prior Digest**: none\n**Review Appendix Prior Length**: 0\n"
        ), "{}", read_model.appended_audit());
        assert_eq!(read_model.state(), before.state());
    }

    /// 受領証は**計画に無いステージ**でも描ける — 監査行に載るのは slug だけで、
    /// 担当エージェントのような計画由来の材料を要さないからである。
    #[test]
    fn a_receipt_for_a_stage_outside_the_plan_still_renders_its_row() {
        let mut read_model = model();
        project(
            &[entry(IntentExecutionEvent::ReviewRequested(
                ReviewRequested::new(
                    event_id(),
                    execution_id(),
                    slug("nowhere"),
                    "aidlc-quality-agent",
                    1,
                    false,
                    crate::review_test_fixture::binding(),
                ),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect("受領証の投影は計画を引かない");
        assert_eq!(audit_events(&read_model), ["REVIEW_REQUESTED"]);
    }

    #[test]
    fn a_state_file_without_the_runtime_section_refuses_the_stance() {
        let mut read_model = ReadModel::new("## Current Status\n- **Status**: Running\n");
        let error = project(
            &[entry(IntentExecutionEvent::SkeletonStanceRecorded(
                SkeletonStanceRecorded::new(event_id(), execution_id(), SkeletonStance::Off),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect_err("置き場が無い");
        assert_eq!(error, ProjectionError::ParkSectionMissing);
    }

    /// 監査シャードに現れたイベント名の列。
    fn audit_events(read_model: &ReadModel) -> Vec<&str> {
        read_model
            .appended_audit()
            .lines()
            .filter_map(|line| line.strip_prefix("**Event**: "))
            .collect()
    }

    #[test]
    fn approving_the_last_stage_completes_the_workflow_instead_of_starting_one() {
        // 次は導出 — second の後の実効 EXECUTE は無い (late は SKIP) ので完了行になる。
        let read_model = run(IntentExecutionEvent::GateApproved(GateApproved::new(
            event_id(),
            execution_id(),
            slug("second"),
            None,
        )));
        // 承認の 2 行に続いて、完了の 3 行が upstream の順序で並ぶ
        // (`aidlc-state.ts handleApprove` → `handleCompleteWorkflow` の委譲、`:2839-2844`)。
        assert_eq!(
            audit_events(&read_model),
            [
                "GATE_APPROVED",
                "STAGE_COMPLETED",
                "PHASE_COMPLETED",
                "PHASE_VERIFIED",
                "WORKFLOW_COMPLETED",
            ]
        );
        // `STAGE_COMPLETED` は承認が既に書いた 1 本だけ — 完了側は再 emit しない (`:2498`)。
        assert!(
            read_model
                .appended_audit()
                .contains("**Details**: Stage Some Title approved by gate")
        );
        assert!(
            !read_model
                .appended_audit()
                .contains("**Details**: Final stage"),
            "承認経路では `Final stage <Name> completed` は描かれない"
        );
        for row in [
            "**From phase**: inception",
            "**To phase**: (end)",
            "**Stages completed**: 1",
            "**Phase boundary**: inception → end",
            "**Scope**: classic",
            "**Details**: Scope: classic, 1 stages completed",
        ] {
            assert!(
                read_model.appended_audit().contains(row),
                "{row}:\n{}",
                read_model.appended_audit()
            );
        }
        // 完了は `Reason` を持たない (承認経路の `--reason` はこの build に無い)。
        assert!(!read_model.appended_audit().contains("**Reason**"));
        for field in [
            "- **Status**: Completed",
            "- **Last Updated**: 2026-08-21T09:14:07Z",
            "- **In Progress**: none",
            "- **Next Stage**: none",
            "- **Next Action**: Workflow complete",
            "- **Last Completed Stage**: second",
            "- **Completed**: 1",
            "- **Inception**: Verified",
        ] {
            assert!(
                read_model.state().contains(field),
                "{field}:\n{}",
                read_model.state()
            );
        }
        // 完了は `Current Stage` を書かない — 実運用では既に最終 slug を指しており、この
        // フィクスチャでは骨格の値 (`state-init`) のままである (upstream も書き換えない)。
        assert!(
            read_model
                .state()
                .contains("- **Current Stage**: state-init")
        );
        assert!(
            !read_model
                .appended_audit()
                .contains("**Event**: STAGE_STARTED"),
            "次が無ければステージは始まらない"
        );
        // `User Input` が無ければ行も出ない。
        assert!(!read_model.appended_audit().contains("**User Input**"));
    }

    #[test]
    fn a_phase_boundary_adds_the_three_boundary_rows_in_order() {
        // 境界はイベントに載らない — state-init (initialization) の次の実効 EXECUTE が
        // first (inception) なので、計画からの導出で境界が立つ。
        let read_model = run(IntentExecutionEvent::GateApproved(GateApproved::new(
            event_id(),
            execution_id(),
            slug("state-init"),
            Some("A".to_string()),
        )));
        let events: Vec<&str> = read_model
            .appended_audit()
            .lines()
            .filter_map(|line| line.strip_prefix("**Event**: "))
            .collect();
        assert_eq!(
            events,
            [
                "GATE_APPROVED",
                "STAGE_COMPLETED",
                "PHASE_COMPLETED",
                "PHASE_VERIFIED",
                "PHASE_STARTED",
                "STAGE_STARTED",
            ]
        );
        assert!(
            read_model
                .appended_audit()
                .contains("**Phase boundary**: initialization → inception\n")
        );
        // 数え直しである — 承認で `state-init` が `[x]` になった時点の 1 本。
        assert!(
            read_model
                .appended_audit()
                .contains("**Stages completed**: 1\n")
        );
    }

    #[test]
    fn a_backward_jump_resets_the_downstream_checkboxes() {
        // 出発点はイベントに載らない — 自分の `Current Stage` 行から導く。second で稼働中の
        // 状態を作り、first へ跳ぶ。
        let mut read_model = ReadModel::new(
            SKELETON
                .replace(
                    "- **Current Stage**: state-init",
                    "- **Current Stage**: second",
                )
                .replace("- [-] state-init — EXECUTE", "- [x] state-init — EXECUTE")
                .replace("- [ ] first — EXECUTE", "- [x] first — EXECUTE")
                .replace("- [ ] second — EXECUTE", "- [-] second — EXECUTE"),
        );
        project(
            &[entry(IntentExecutionEvent::Jumped(Jumped::new(
                event_id(),
                execution_id(),
                slug("first"),
                core_command_domain::orchestration::JumpDirection::Backward,
                None,
            )))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        assert!(
            read_model
                .appended_audit()
                .contains("**Direction**: BACKWARD\n")
        );
        assert!(
            read_model.appended_audit().contains(
                "**Details**: BACKWARD jump from second to first (2.1). Scope: classic.\n"
            )
        );
        assert!(
            read_model.state().contains("- [ ] second — EXECUTE"),
            "下流は pending へ"
        );
        assert!(read_model.state().contains("- [-] first — EXECUTE"));
    }

    #[test]
    fn every_jump_direction_has_an_audit_spelling() {
        assert_eq!(direction_spelling(JumpDirection::Forward), "FORWARD");
        assert_eq!(direction_spelling(JumpDirection::Backward), "BACKWARD");
        assert_eq!(direction_spelling(JumpDirection::Redo), "REDO");
    }

    #[test]
    fn skipping_the_last_effective_stage_completes_the_workflow_row() {
        // 導出 leave_for の None 腕 — second を skip すると次の実効 EXECUTE は無い
        // (late は SKIP) ので WORKFLOW_COMPLETED 行になる。
        let mut read_model = ReadModel::new(
            SKELETON
                .replace(
                    "- **Current Stage**: state-init",
                    "- **Current Stage**: second",
                )
                .replace("- [-] state-init — EXECUTE", "- [x] state-init — EXECUTE")
                .replace("- [ ] first — EXECUTE", "- [x] first — EXECUTE")
                .replace("- [ ] second — EXECUTE", "- [-] second — EXECUTE"),
        );
        project(
            &[entry(IntentExecutionEvent::StageSkipped(
                StageSkipped::new(
                    event_id(),
                    execution_id(),
                    slug("second"),
                    "not needed".to_string(),
                ),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        // 読み飛ばしの 1 行に続いて完了の 3 行 (`aidlc-state.ts handleSkip --route` の
        // 完了枝、`:3212-3232`)。`STAGE_COMPLETED` は描かれない — 読み飛ばしは完了ではない。
        assert_eq!(
            audit_events(&read_model),
            [
                "STAGE_SKIPPED",
                "PHASE_COMPLETED",
                "PHASE_VERIFIED",
                "WORKFLOW_COMPLETED",
            ]
        );
        for row in [
            "**From phase**: inception",
            "**To phase**: (end)",
            // `[S]` は完了数に入らない — 倒れているのは state-init と first の 2 本である。
            "**Stages completed**: 2",
            "**Phase boundary**: inception → end",
            "**Scope**: classic",
            "**Details**: Scope: classic, final stage second skipped",
            "**Reason**: not needed",
        ] {
            assert!(
                read_model.appended_audit().contains(row),
                "{row}:\n{}",
                read_model.appended_audit()
            );
        }
        assert!(read_model.state().contains("- [S] second — EXECUTE"));
        for field in [
            "- **Status**: Completed",
            "- **In Progress**: none",
            "- **Next Stage**: none",
            "- **Next Action**: Workflow complete",
            "- **Completed**: 2",
            "- **Inception**: Verified",
        ] {
            assert!(
                read_model.state().contains(field),
                "{field}:\n{}",
                read_model.state()
            );
        }
        // 読み飛ばしは最終完了ステージを動かさない (upstream も書かない)。
        assert!(
            read_model
                .state()
                .contains("- **Last Completed Stage**: \n")
        );
    }

    #[test]
    fn a_forward_jump_emits_skips_in_plan_order_with_the_source_last() {
        // upstream の emit 順 — 中間 (計画順) → 最後に出発点。実効 SKIP の介在 (late) は
        // 触らない (実バイトが正本)。first で稼働中の状態から second を跨いで…は 4 ステージ
        // 構成では作れないので、state-init 稼働中から second へ跳ぶ。
        let mut read_model = ReadModel::new(SKELETON.to_string());
        project(
            &[entry(IntentExecutionEvent::Jumped(Jumped::new(
                event_id(),
                execution_id(),
                slug("second"),
                core_command_domain::orchestration::JumpDirection::Forward,
                None,
            )))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        let skipped: Vec<&str> = read_model
            .appended_audit()
            .lines()
            .filter_map(|line| line.strip_prefix("**Stage**: "))
            .collect();
        // STAGE_SKIPPED は first (中間)、state-init (出発点) の順。STAGE_STARTED の
        // second が最後に混ざるので先頭 2 つを見る。
        assert_eq!(skipped.first(), Some(&"first"), "中間が先");
        assert_eq!(skipped.get(1), Some(&"state-init"), "出発点が最後");
        assert!(read_model.state().contains("- [S] state-init — EXECUTE"));
        assert!(read_model.state().contains("- [S] first — EXECUTE"));
        assert!(read_model.state().contains("- [-] second — EXECUTE"));
        assert!(
            read_model.state().contains("- [ ] late — SKIP"),
            "実効 SKIP の行は触らない"
        );
    }

    #[test]
    fn a_non_numeric_revision_count_is_coerced_to_zero_before_the_bump() {
        // upstream getField + 1 と同じ防御 — 非数値 (手編集・欠落) は 0 に畳んでから +1。
        let mut read_model = ReadModel::new(
            SKELETON.replace("- **Revision Count**: 0", "- **Revision Count**: abc"),
        );
        project(
            &[entry(IntentExecutionEvent::GateRejected(
                GateRejected::new(event_id(), execution_id(), slug("state-init"), None),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        assert!(read_model.state().contains("- **Revision Count**: 1\n"));
    }

    #[test]
    fn a_second_rejection_bumps_the_read_model_counter_again() {
        // read-modify-write の連続性 — 現値 1 の状態からの差し戻しは 2 を書く
        // (upstream getField + 1 と同じ)。
        let mut read_model =
            ReadModel::new(SKELETON.replace("- **Revision Count**: 0", "- **Revision Count**: 1"));
        project(
            &[entry(IntentExecutionEvent::GateRejected(
                GateRejected::new(event_id(), execution_id(), slug("state-init"), None),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        assert!(read_model.state().contains("- **Revision Count**: 2\n"));
    }

    #[test]
    fn an_unknown_plan_suffix_token_falls_back_to_the_static_plan() {
        // 行末トークンが閉集合外なら静的計画の値で読む (次の導出が止まらない)。
        let mut read_model =
            ReadModel::new(SKELETON.replace("- [ ] first — EXECUTE", "- [ ] first — WHAT"));
        project(
            &[entry(IntentExecutionEvent::GateApproved(
                GateApproved::new(event_id(), execution_id(), slug("state-init"), None),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        // first の静的計画は EXECUTE — 次の開始先として選ばれる。
        assert!(read_model.state().contains("- [-] first — WHAT"));
    }

    #[test]
    fn a_redo_jump_reopens_the_current_stage_without_touching_neighbours() {
        // 到達点 = 現在地 (redo)。checkbox の書き換えは到達点の [-] 化だけで、隣は触らない。
        let read_model = run(IntentExecutionEvent::Jumped(Jumped::new(
            event_id(),
            execution_id(),
            slug("state-init"),
            core_command_domain::orchestration::JumpDirection::Redo,
            None,
        )));
        assert!(
            read_model
                .appended_audit()
                .contains("**Direction**: REDO\n")
        );
        assert!(read_model.appended_audit().contains(
            "**Details**: REDO jump from state-init to state-init (0.1). Scope: classic.\n"
        ));
        assert!(read_model.state().contains("- [-] state-init — EXECUTE"));
        assert!(read_model.state().contains("- [ ] first — EXECUTE"));
    }

    #[test]
    fn a_jump_with_a_broken_current_stage_row_is_refused() {
        // 出発点の導出元 (`Current Stage` 行) が壊れていれば、読み替えずに止める (fail-closed)。
        let mut read_model = ReadModel::new(SKELETON.replace(
            "- **Current Stage**: state-init",
            "- **Current Stage**: NOT A SLUG",
        ));
        let error = project(
            &[entry(IntentExecutionEvent::Jumped(Jumped::new(
                event_id(),
                execution_id(),
                slug("first"),
                core_command_domain::orchestration::JumpDirection::Forward,
                None,
            )))],
            &plan(),
            &mut read_model,
        )
        .expect_err("出発点が導けない");
        assert_eq!(error.to_string(), "unknown stage: NOT A SLUG");
    }

    #[test]
    fn a_rejection_without_feedback_omits_the_feedback_rows() {
        // feedback 無しの差し戻し — 行は出ず、Revision Count は現値 +1 (0 → 1)。
        let read_model = run(IntentExecutionEvent::GateRejected(GateRejected::new(
            event_id(),
            execution_id(),
            slug("state-init"),
            None,
        )));
        assert!(!read_model.appended_audit().contains("**Feedback**"));
        assert!(
            read_model
                .appended_audit()
                .contains("**Revision count**: 1\n")
        );
        assert!(read_model.state().contains("- **Revision Count**: 1\n"));
        assert!(read_model.state().contains("- [R] state-init — EXECUTE"));
    }

    #[test]
    fn recomposing_back_into_scope_moves_the_entry_the_other_way() {
        // 適用後の in-scope 数は行末トークンの反転後に自分の行から導く (= 4)。
        let read_model = run(IntentExecutionEvent::Recomposed(Recomposed::new(
            event_id(),
            execution_id(),
            StageSlugSet::empty(),
            StageSlugSet::new([slug("late")]),
        )));
        // Execute 行は graph 順に組み直される（4.1 は末尾で、ここでは順序が変わらない）。
        assert!(
            read_model
                .state()
                .contains("- **Stages to Execute**: 0.1, 2.1, 2.2, 4.1\n"),
            "実際: {}",
            read_model.state()
        );
        // 空になった Skip 行は upstream と同じ逐語 `none` を書く。
        assert!(read_model.state().contains("- **Stages to Skip**: none\n"));
        assert!(read_model.state().contains("- [ ] late — EXECUTE"));
        assert!(read_model.state().contains("- **Total Stages**: 4\n"));
        assert!(
            read_model
                .appended_audit()
                .contains("**Stages skipped**: none\n")
        );
    }

    /// 反転集合の綴りは**文書順**であって、集合型の辞書順ではない (NFR1 の逐語一致)。
    ///
    /// [`StageSlugSet`] は一意化のために辞書順で並ぶ。監査行と行末トークンの書き替えを
    /// その順で書くと upstream の実バイトから外れるので、投影が計画の位置で並べ直す。
    /// ここでは文書順と辞書順が**逆になる**計画 (`zulu` が `alpha` より前) を組み、
    /// 並べ直しが効いていることを綴りの一致で固定する。
    #[test]
    fn the_recomposed_spelling_follows_the_document_order_not_the_alphabet() {
        let intent = Intent::from((
            Created::new(
                intent_event_id(),
                IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7"),
                WorkflowDefinitionId::parse("claude").expect("定義 id"),
                DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
                StartRequest::new("classic", "build it"),
                StageEntries::new(vec![
                    stage(
                        "state-init",
                        "0.1",
                        PhaseId::Initialization,
                        PlanAction::Execute,
                    ),
                    stage("zulu", "2.1", PhaseId::Inception, PlanAction::Execute),
                    stage("alpha", "2.2", PhaseId::Inception, PlanAction::Execute),
                ])
                .expect("フィクスチャの計画は不変条件を満たす"),
                WorkspaceScan::new(
                    BrownfieldGreenfield::Greenfield,
                    "Unknown",
                    "Unknown",
                    "Unknown",
                )
                .expect("単一行"),
            ),
            at(),
        ));
        let skeleton = "\
## Scope Configuration
- **Stages to Execute**: 0.1, 2.1, 2.2
- **Stages to Skip**: none

## Execution Plan Summary
- **Total Stages**: 3

## Stage Progress
- [-] state-init — EXECUTE
- [ ] zulu — EXECUTE
- [ ] alpha — EXECUTE
";
        let mut read_model = ReadModel::new(skeleton);
        project(
            &[entry(IntentExecutionEvent::Recomposed(Recomposed::new(
                event_id(),
                IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7"),
                // 集合の内部順は辞書順 (alpha, zulu) — 書き出しは文書順 (zulu, alpha)。
                StageSlugSet::new([slug("alpha"), slug("zulu")]),
                StageSlugSet::empty(),
            )))],
            &ResolvedPlan::of(&intent),
            &mut read_model,
        )
        .expect("投影");
        assert!(
            read_model
                .appended_audit()
                .contains("**Stages skipped**: zulu, alpha\n"),
            "実際: {}",
            read_model.appended_audit()
        );
        // Skip 行も同じ順で末尾へ足される (graph 順、`rebuild_plan_rows` の逐語規則)。
        assert!(
            read_model
                .state()
                .contains("- **Stages to Skip**: 2.1 (zulu), 2.2 (alpha)\n"),
            "実際: {}",
            read_model.state()
        );
    }

    #[test]
    fn a_missing_list_field_is_refused_with_the_verbatim_wording() {
        let mut read_model = ReadModel::new("## Empty\n");
        let error = rebuild_plan_rows(&mut read_model, &plan()).expect_err("一覧フィールドが無い");
        assert_eq!(
            error.to_string(),
            "state field: Field not found in state file: \"Stages to Skip\". \
             Cannot update — refusing to silently no-op."
        );
    }

    #[test]
    fn the_skip_token_parser_keeps_the_slug_and_drops_the_annotation() {
        // 注釈付き項目から slug を取れないと、EXECUTE へ戻した段でその項目が落ちずに残る。
        assert_eq!(
            slug_of_skip_token("4.5 (incident-response)"),
            "incident-response"
        );
        assert_eq!(
            slug_of_skip_token("2.1 (reverse-engineering — greenfield)"),
            "reverse-engineering"
        );
    }

    #[test]
    fn every_projection_refusal_renders_its_material() {
        assert_eq!(
            ProjectionError::UnknownStage {
                stage: "ghost".to_string()
            }
            .to_string(),
            "unknown stage: ghost"
        );
        assert_eq!(
            ProjectionError::from(CheckboxUpdateError::MissingSuffix("s".to_string())).to_string(),
            "checkbox: missing suffix s"
        );
        assert_eq!(
            ProjectionError::from(CheckboxUpdateError::MissingStage("s".to_string())).to_string(),
            "checkbox: missing stage s"
        );
        // 綴りを取り違えたキーは投影の材料として拒否される（本体の定数は全て文法内なので、
        // この写像は直接固定しておく）。
        let malformed = AuditFieldKey::parse("1x").expect_err("文法外");
        assert_eq!(
            ProjectionError::from(malformed).to_string(),
            "audit field key: malformed audit field key: 1x"
        );
        let boxed: Box<dyn std::error::Error> = Box::new(ProjectionError::ParkSectionMissing);
        assert_eq!(boxed.to_string(), "park section missing");
    }

    #[test]
    fn a_stage_the_plan_does_not_know_is_refused_everywhere_it_is_named() {
        let ghost = slug("ghost");
        assert_eq!(
            title_of(&plan(), &ghost),
            Err(ProjectionError::UnknownStage {
                stage: "ghost".to_string()
            })
        );
        assert_eq!(
            number_of(&plan(), &ghost),
            Err(ProjectionError::UnknownStage {
                stage: "ghost".to_string()
            })
        );
    }

    // ---- b49: 昇格受領証 (practices-discovery、段 12) ----

    const TEAM_MD: &str = "\
# Team

## Way of Working
old way.

## Walking Skeleton
old skeleton.

## Code Style
old style.
";

    const PROJECT_MD: &str = "\
# Project

## Mandated

## Forbidden

## Corrections
";

    fn affirmed(
        sections: Vec<PromotedSection>,
        mandated: &[&str],
        forbidden: &[&str],
    ) -> PracticesAffirmed {
        PracticesAffirmed::new(
            event_id(),
            execution_id(),
            slug("practices-discovery"),
            "owner",
            PromotedSections::new(sections).expect("フィクスチャの見出しは一意"),
            RuleLines::new(mandated.iter().map(|rule| (*rule).to_string()).collect()),
            RuleLines::new(forbidden.iter().map(|rule| (*rule).to_string()).collect()),
        )
    }

    // ---- 学びの儀式 (§13) ----

    const GOLDEN_TEXT: &str = "ALWAYS 採取用の検証結果を記録する。";
    const GOLDEN_HASH: &str = "f543ed24a72a9b57b8fac723a95fa0a2c04240a25c8a6a67229322acb3de9bd3";

    fn learning(
        text: &str,
        scope: core_command_domain::orchestration::LearningScope,
        heading: &str,
        source: core_command_domain::orchestration::LearningSource,
    ) -> core_command_domain::orchestration::Learning {
        core_command_domain::orchestration::Learning::new(
            core_command_domain::orchestration::LearningCandidateId::parse("fixture-1")
                .expect("候補番号"),
            scope,
            core_command_domain::orchestration::PracticeHeading::from_routed(heading),
            text,
            source,
        )
    }

    fn captured_event(
        learnings: Vec<core_command_domain::orchestration::CapturedLearning>,
    ) -> IntentExecutionEvent {
        IntentExecutionEvent::LearningsCaptured(Box::new(
            core_command_domain::orchestration::LearningsCaptured::new(
                event_id(),
                execution_id(),
                slug("requirements-analysis"),
                core_command_domain::orchestration::LearningProvenance::new(
                    core_command_domain::workspace::SpaceName::default(),
                    core_command_domain::workspace::IntentDirName::parse("260908-learnings")
                        .expect("記録名"),
                ),
                core_command_domain::orchestration::CapturedLearnings::new(learnings),
            ),
        ))
    }

    /// ゴールデン `learnings/persist-one` の実践行と監査行を逐語で固定する。
    #[test]
    fn a_fresh_learning_writes_the_practice_line_and_the_audit_row() {
        let read_model = run_with_memory(captured_event(vec![
            core_command_domain::orchestration::CapturedLearning::new(
                learning(
                    GOLDEN_TEXT,
                    core_command_domain::orchestration::LearningScope::Project,
                    "Corrections",
                    core_command_domain::orchestration::LearningSource::UserAddition,
                ),
                core_command_domain::orchestration::LearningDisposition::Fresh,
            ),
        ]));

        let memory = read_model.memory().expect("面は載っている");
        assert!(memory.is_dirty());
        assert_eq!(memory.team(), TEAM_MD, "team.md は触らない");
        assert_eq!(
            memory.project(),
            format!(
                "{PROJECT_MD}- {GOLDEN_TEXT} (learned 2026-08-21) <!-- cid:260908-learnings:requirements-analysis:{GOLDEN_HASH} -->\n"
            )
        );
        assert_eq!(
            read_model.appended_audit(),
            format!(
                "\n## Rule Learned\n**Timestamp**: 2026-08-21T09:14:07Z\n**Event**: RULE_LEARNED\n**Stage**: requirements-analysis\n**Candidate-ID**: fixture-1\n**Content-Hash**: {GOLDEN_HASH}\n**Destination**: <project-dir>/aidlc/spaces/default/memory/project.md\n**Heading**: ## Corrections\n**Source**: user_addition\n\n---\n"
            )
        );
        // 状態ファイルは動かない — 学びは進行でも承認でもない。
        assert_eq!(read_model.state(), model().state());
    }

    /// 監査行だけを補う復旧では実践行を二重に足さない。
    #[test]
    fn an_audit_row_only_repair_leaves_the_method_file_untouched() {
        let read_model = run_with_memory(captured_event(vec![
            core_command_domain::orchestration::CapturedLearning::new(
                learning(
                    GOLDEN_TEXT,
                    core_command_domain::orchestration::LearningScope::Project,
                    "Corrections",
                    core_command_domain::orchestration::LearningSource::Orchestrator,
                ),
                core_command_domain::orchestration::LearningDisposition::AuditRowOnly,
            ),
        ]));
        let memory = read_model.memory().expect("面は載っている");
        assert!(!memory.is_dirty(), "実践行は既に在るので書き替えない");
        assert_eq!(memory.project(), PROJECT_MD);
        assert!(
            read_model
                .appended_audit()
                .contains("**Event**: RULE_LEARNED\n")
        );
    }

    /// 実践行だけを補う復旧では監査行を二重に立てない。
    #[test]
    fn a_practice_line_only_repair_writes_no_audit_row() {
        let read_model = run_with_memory(captured_event(vec![
            core_command_domain::orchestration::CapturedLearning::new(
                learning(
                    GOLDEN_TEXT,
                    core_command_domain::orchestration::LearningScope::Project,
                    "Corrections",
                    core_command_domain::orchestration::LearningSource::Orchestrator,
                ),
                core_command_domain::orchestration::LearningDisposition::PracticeLineOnly,
            ),
        ]));
        assert!(
            read_model
                .memory()
                .expect("面")
                .project()
                .contains(GOLDEN_HASH)
        );
        assert_eq!(read_model.appended_audit(), "");
    }

    /// 選ばれた見出しが正本に無ければ作ってから足す（本家の ensure-exists）。
    #[test]
    fn a_routed_heading_the_method_file_lacks_is_created_first() {
        let read_model = run_with_memory(captured_event(vec![
            core_command_domain::orchestration::CapturedLearning::new(
                learning(
                    "ALWAYS run the suite",
                    core_command_domain::orchestration::LearningScope::Team,
                    "Testing Posture",
                    core_command_domain::orchestration::LearningSource::Orchestrator,
                ),
                core_command_domain::orchestration::LearningDisposition::Fresh,
            ),
        ]));
        let memory = read_model.memory().expect("面は載っている");
        assert_eq!(memory.project(), PROJECT_MD, "project.md は触らない");
        assert!(
            memory.team().ends_with(
                "\n## Testing Posture\n- ALWAYS run the suite (learned 2026-08-21) <!-- cid:260908-learnings:requirements-analysis:205b8933ecca515ff63f555e40b8fb01eeb738a8846bcbd0fef69a3113213b9e -->\n"
            ),
            "実測: {}",
            memory.team()
        );
        assert!(
            read_model
                .appended_audit()
                .contains("**Heading**: ## Testing Posture\n")
        );
    }

    /// 何も選ばれなかった回は 1 バイトも書かない（メモリ層が載っていなくても通る）。
    #[test]
    fn an_empty_capture_writes_nothing_and_needs_no_memory_face() {
        let mut read_model = model();
        project(
            &[entry(captured_event(Vec::new()))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        assert_eq!(read_model.appended_audit(), "");
        assert_eq!(read_model.state(), model().state());
        assert!(read_model.memory().is_none());
    }

    /// 実践行を書くのにメモリ層が載っていなければ fail-closed で止まる。
    #[test]
    fn a_practice_line_without_the_memory_face_is_refused() {
        let mut read_model = model();
        assert_eq!(
            project(
                &[entry(captured_event(vec![
                    core_command_domain::orchestration::CapturedLearning::new(
                        learning(
                            GOLDEN_TEXT,
                            core_command_domain::orchestration::LearningScope::Project,
                            "Corrections",
                            core_command_domain::orchestration::LearningSource::Orchestrator,
                        ),
                        core_command_domain::orchestration::LearningDisposition::Fresh,
                    ),
                ]))],
                &plan(),
                &mut read_model
            ),
            Err(ProjectionError::MemoryFilesMissing)
        );
    }

    /// メモリ層を載せたリードモデルへ 1 件だけ投影する。
    fn run_with_memory(event: IntentExecutionEvent) -> ReadModel {
        let mut read_model = model().with_memory(TEAM_MD, PROJECT_MD);
        project(&[entry(event)], &plan(), &mut read_model).expect("投影");
        read_model
    }

    /// 4 面すべてを描く — team.md の節置換・project.md の追記・状態ファイル 2 欄・監査 1 行。
    #[test]
    fn a_promotion_writes_all_four_faces() {
        let read_model = run_with_memory(IntentExecutionEvent::PracticesAffirmed(affirmed(
            vec![PromotedSection::new("Way of Working", "trunk-based.\n\n")],
            &["ALWAYS review. (affirmed 2026-09-05)"],
            &["NEVER force-push. (affirmed 2026-09-05)"],
        )));

        let memory = read_model.memory().expect("面は載っている");
        assert!(memory.is_dirty(), "書き替えたので dirty である");
        assert_eq!(
            memory.team(),
            "\
# Team

## Way of Working
trunk-based.

## Walking Skeleton
old skeleton.

## Code Style
old style.
"
        );
        // 追記は**次の `## ` 見出しの直前**に入る (upstream `appendUnderHeading` — 節末尾の
        // 空行の後ろ)。既存行との間に空行が残るのは upstream の実バイトどおりである。
        assert_eq!(
            memory.project(),
            "\
# Project

## Mandated

ALWAYS review. (affirmed 2026-09-05)
## Forbidden

NEVER force-push. (affirmed 2026-09-05)
## Corrections
"
        );

        // 状態ファイルは 2 欄 — `Practices Affirmed Timestamp` は骨格に無いので挿入される。
        assert!(
            read_model
                .state()
                .contains("- **Practices Affirmed Timestamp**: 2026-08-21T09:14:07Z\n"),
            "{}",
            read_model.state()
        );
        assert!(
            read_model
                .state()
                .contains("- **Last Updated**: 2026-08-21T09:14:07Z\n"),
            "{}",
            read_model.state()
        );

        // 監査 1 行 — 欄の並びは upstream `:3733-3738` の構築順である。
        assert_eq!(audit_events(&read_model), ["PRACTICES_AFFIRMED"]);
        assert!(
            read_model.appended_audit().contains(
                "**Affirming User**: owner\n**Sections Written**: Way of Working\n**Mandated Rules Appended**: 1\n**Forbidden Rules Appended**: 1\n"
            ),
            "{}",
            read_model.appended_audit()
        );
    }

    /// 空の昇格でも 4 欄を描く — `Sections Written` は**空値のまま欄を描く**。
    #[test]
    fn an_empty_promotion_still_renders_every_audit_field() {
        let read_model = run_with_memory(IntentExecutionEvent::PracticesAffirmed(affirmed(
            Vec::new(),
            &[],
            &[],
        )));
        assert!(
            read_model.appended_audit().contains(
                "**Affirming User**: owner\n**Sections Written**: \n**Mandated Rules Appended**: 0\n**Forbidden Rules Appended**: 0\n"
            ),
            "{}",
            read_model.appended_audit()
        );
        // 何も書き替えていなくても、昇格の事実そのものは面を触る (dirty)。
        assert!(read_model.memory().is_some_and(MemoryFaces::is_dirty));
    }

    /// 節は宣言順に置き換わる (5 節のうち 2 節だけを持つ昇格)。
    #[test]
    fn every_promoted_section_is_replaced_in_order() {
        let read_model = run_with_memory(IntentExecutionEvent::PracticesAffirmed(affirmed(
            vec![
                PromotedSection::new("Way of Working", "A\n\n"),
                PromotedSection::new("Code Style", "B\n"),
            ],
            &[],
            &[],
        )));
        let memory = read_model.memory().expect("面は載っている");
        assert!(
            memory
                .team()
                .contains("## Way of Working\nA\n\n## Walking Skeleton")
        );
        assert!(memory.team().ends_with("## Code Style\nB\n"));
        assert!(
            read_model
                .appended_audit()
                .contains("**Sections Written**: Way of Working, Code Style\n"),
            "{}",
            read_model.appended_audit()
        );
    }

    /// 既に同じ印付き行が在れば足さない (at-least-once の再投影で重複しない)。
    #[test]
    fn a_rule_already_present_is_not_appended_twice() {
        let live =
            "# Project\n\n## Mandated\nALWAYS review. (affirmed 2026-09-05)\n\n## Forbidden\n";
        let mut read_model = model().with_memory(TEAM_MD, live);
        project(
            &[entry(IntentExecutionEvent::PracticesAffirmed(affirmed(
                Vec::new(),
                &["ALWAYS review. (affirmed 2026-09-05)"],
                &[],
            )))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        let memory = read_model.memory().expect("面は載っている");
        assert_eq!(
            memory
                .project()
                .matches("ALWAYS review. (affirmed 2026-09-05)")
                .count(),
            1,
            "{}",
            memory.project()
        );
    }

    /// メモリ層が載っていなければ fail-closed で止まる。
    #[test]
    fn a_promotion_without_the_memory_faces_is_refused() {
        let mut read_model = model();
        let error = project(
            &[entry(IntentExecutionEvent::PracticesAffirmed(affirmed(
                Vec::new(),
                &[],
                &[],
            )))],
            &plan(),
            &mut read_model,
        )
        .expect_err("面が無ければ描けない");
        assert_eq!(error, ProjectionError::MemoryFilesMissing);
        assert_eq!(error.to_string(), "memory files missing");
    }

    /// 置換先・追記先の見出しが無ければ、どちらのファイルかを名指して止まる。
    #[test]
    fn a_missing_memory_heading_names_the_file_and_the_heading() {
        let mut read_model = model().with_memory("# Team\n", PROJECT_MD);
        let error = project(
            &[entry(IntentExecutionEvent::PracticesAffirmed(affirmed(
                vec![PromotedSection::new("Deployment", "none.\n")],
                &[],
                &[],
            )))],
            &plan(),
            &mut read_model,
        )
        .expect_err("見出しが無ければ描けない");
        assert_eq!(
            error,
            ProjectionError::MemoryHeadingMissing {
                file: "team.md",
                heading: "## Deployment".to_string(),
            }
        );
        assert_eq!(
            error.to_string(),
            "memory heading missing: ## Deployment in team.md"
        );

        let mut read_model = model().with_memory(TEAM_MD, "# Project\n\n## Corrections\n");
        let error = project(
            &[entry(IntentExecutionEvent::PracticesAffirmed(affirmed(
                Vec::new(),
                &["ALWAYS x. (affirmed 2026-09-05)"],
                &[],
            )))],
            &plan(),
            &mut read_model,
        )
        .expect_err("見出しが無ければ描けない");
        assert_eq!(
            error,
            ProjectionError::MemoryHeadingMissing {
                file: "project.md",
                heading: "## Mandated".to_string(),
            }
        );
    }

    /// 状態ファイルに `## Project Information` が無ければ、そのファイルを名指して止まる。
    #[test]
    fn a_state_file_without_the_project_information_section_is_refused() {
        let mut read_model = ReadModel::new("## Runtime State\n- **Revision Count**: 0\n")
            .with_memory(TEAM_MD, PROJECT_MD);
        let error = project(
            &[entry(IntentExecutionEvent::PracticesAffirmed(affirmed(
                Vec::new(),
                &[],
                &[],
            )))],
            &plan(),
            &mut read_model,
        )
        .expect_err("挿入先の見出しが無ければ描けない");
        assert_eq!(
            error,
            ProjectionError::MemoryHeadingMissing {
                file: "aidlc-state.md",
                heading: "## Project Information".to_string(),
            }
        );
    }

    /// メモリ層を触らないイベントは面を dirty にしない (mtime を動かさないため)。
    #[test]
    fn an_event_that_touches_no_memory_face_leaves_it_clean() {
        let mut read_model = model().with_memory(TEAM_MD, PROJECT_MD);
        project(
            &[entry(IntentExecutionEvent::ReviewRequested(
                ReviewRequested::new(
                    event_id(),
                    execution_id(),
                    slug("first"),
                    "aidlc-quality-agent",
                    1,
                    false,
                    crate::review_test_fixture::binding(),
                ),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        let memory = read_model.memory().expect("面は載っている");
        assert!(!memory.is_dirty());
        assert_eq!(memory.team(), TEAM_MD);
        assert_eq!(memory.project(), PROJECT_MD);
    }

    #[test]
    fn a_pipeline_link_row_carries_the_repo_and_the_isolated_workflow() {
        use core_command_domain::orchestration::{
            PipelineHandoff, PipelineLinkCompleted, PipelineReceipt,
        };
        let read_model = run(IntentExecutionEvent::PipelineLinkCompleted(
            PipelineLinkCompleted::new(
                event_id(),
                execution_id(),
                PipelineReceipt::new(
                    "first".to_string(),
                    "aidlc-architect-agent".to_string(),
                    Some("modules/app".to_string()),
                    true,
                    1,
                    2,
                    Some(
                        PipelineHandoff::new(
                            "aidlc/handoff.json".to_string(),
                            format!("sha256:{}", "9".repeat(64)),
                            "1700000000000".to_string(),
                        )
                        .expect("整合した受領"),
                    ),
                )
                .expect("整合した受領証"),
            ),
        ));
        let audit = read_model.appended_audit();
        assert!(
            audit.contains("**Event**: PIPELINE_LINK_COMPLETED\n"),
            "{audit}"
        );
        assert!(audit.contains("**Position**: 1/2\n"), "{audit}");
        assert!(
            audit.contains("**Artifact Path**: aidlc/handoff.json\n"),
            "{audit}"
        );
        assert!(audit.contains("**Repo**: modules/app\n"), "{audit}");
        assert!(
            audit.contains("**Workflow**: single-stage:first\n"),
            "{audit}"
        );
    }

    #[test]
    fn an_attended_prompt_without_a_session_is_a_human_turn_row_without_the_session_field() {
        use core_command_domain::orchestration::PromptObserved;
        let read_model = run(IntentExecutionEvent::PromptObserved(PromptObserved::new(
            event_id(),
            execution_id(),
            "",
            "A",
            false,
        )));
        let audit = read_model.appended_audit();
        assert!(audit.contains("**Event**: HUMAN_TURN\n"), "{audit}");
        assert!(!audit.contains("**Session**"), "{audit}");
        let unattended = run(IntentExecutionEvent::PromptObserved(PromptObserved::new(
            event_id(),
            execution_id(),
            "session-1",
            "A",
            true,
        )));
        assert!(
            unattended.appended_audit().is_empty(),
            "無人運転の応答は人間の在席証拠にならない"
        );
    }

    #[test]
    fn a_decision_row_carries_the_options_and_the_rationale() {
        use core_command_domain::orchestration::{DecisionPrompt, DecisionRecorded};
        let read_model = run(IntentExecutionEvent::DecisionRecorded(
            DecisionRecorded::new(
                event_id(),
                execution_id(),
                DecisionPrompt::new("first", "Pick one")
                    .with_options("A,B")
                    .with_rationale("because"),
            ),
        ));
        let audit = read_model.appended_audit();
        assert!(audit.contains("**Event**: DECISION_RECORDED\n"), "{audit}");
        assert!(audit.contains("**Options**: A,B\n"), "{audit}");
        assert!(audit.contains("**Rationale**: because\n"), "{audit}");
    }

    #[test]
    fn an_approval_reported_with_a_validation_warning_carries_it_on_the_completion_row() {
        use core_command_domain::orchestration::{
            ReportId, ReportResult, ReportTransition, Reported, StageValidation, TransitionStep,
            TransitionSteps,
        };
        let mut read_model = ReadModel::new(
            SKELETON
                .replace(
                    "- **Current Stage**: state-init",
                    "- **Current Stage**: first",
                )
                .replace("- [-] state-init — EXECUTE", "- [x] state-init — EXECUTE")
                .replace("- [ ] first — EXECUTE", "- [?] first — EXECUTE"),
        );
        project(
            &[entry(IntentExecutionEvent::Reported(
                Reported::new(
                    event_id(),
                    execution_id(),
                    ReportId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0555").expect("UUIDv7"),
                    ReportResult::Committed {
                        stage: slug("first"),
                        scope: "classic".to_string(),
                        steps: TransitionSteps::single(TransitionStep::Approve),
                        transition: ReportTransition::GateApproved {
                            user_input: Some("Approve".to_string()),
                        },
                    },
                    Some(StageValidation::Warning("receipt unavailable".to_string())),
                    None,
                )
                .expect("整合した報告"),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        let audit = read_model.appended_audit();
        assert!(
            audit.contains("**Validation Warning**: receipt unavailable\n"),
            "{audit}"
        );
        assert!(!audit.contains("**Validation Basis**"), "{audit}");
        assert!(
            read_model.state().contains("- [x] first — EXECUTE"),
            "{}",
            read_model.state()
        );
    }

    #[test]
    fn a_backward_jump_with_an_observation_lists_the_changed_and_invalidated_artifacts() {
        use core_command_domain::orchestration::{JumpArtifact, JumpObservation, SourceBaseline};
        let mut read_model = ReadModel::new(
            SKELETON
                .replace(
                    "- **Current Stage**: state-init",
                    "- **Current Stage**: second",
                )
                .replace("- [-] state-init — EXECUTE", "- [x] state-init — EXECUTE")
                .replace("- [ ] first — EXECUTE", "- [x] first — EXECUTE")
                .replace("- [ ] second — EXECUTE", "- [-] second — EXECUTE"),
        );
        project(
            &[entry(IntentExecutionEvent::Jumped(Jumped::new(
                event_id(),
                execution_id(),
                slug("first"),
                core_command_domain::orchestration::JumpDirection::Backward,
                Some(JumpObservation::new(
                    SourceBaseline::new(None).expect("束縛不能の基準"),
                    vec![
                        JumpArtifact::new(
                            slug("first"),
                            "inception/first/first.md".to_string(),
                            true,
                            true,
                        ),
                        JumpArtifact::new(
                            slug("second"),
                            "inception/second/second.md".to_string(),
                            false,
                            false,
                        ),
                    ],
                )),
            )))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        let audit = read_model.appended_audit();
        assert!(audit.contains("**Direction**: BACKWARD\n"), "{audit}");
        assert!(
            audit.contains("**Source Baseline**: unbindable\n"),
            "{audit}"
        );
        assert!(
            audit.contains("**Changed Upstream Artifacts**: "),
            "{audit}"
        );
        assert!(
            audit.contains("**Invalidated Downstream Artifacts**: "),
            "{audit}"
        );
        assert!(
            audit.contains("**Invalidated Downstream Reviews**: "),
            "{audit}"
        );
    }

    #[test]
    fn a_checkbox_row_with_an_unknown_action_falls_back_to_the_plan() {
        // `EXECUTE` / `SKIP` 以外の綴りは計画の値へ戻す（読み替えず、行の嘘に従わない）。
        let mut read_model = ReadModel::new(
            SKELETON
                .replace("- [ ] late — SKIP", "- [ ] late — MAYBE")
                .replace("- **Stages to Skip**: 4.1 (late)", "- **Stages to Skip**: "),
        );
        project(
            &[entry(IntentExecutionEvent::Recomposed(
                core_command_domain::orchestration::Recomposed::new(
                    event_id(),
                    execution_id(),
                    core_command_domain::orchestration::StageSlugSet::empty(),
                    core_command_domain::orchestration::StageSlugSet::empty(),
                ),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        assert!(
            read_model
                .state()
                .contains("- **Stages to Skip**: 4.1 (late)\n"),
            "{}",
            read_model.state()
        );
    }

    /// 状態ファイルから欄を 1 行抜いた出発点で投影し、欠けた欄の名前で拒否されることを見る。
    #[test]
    fn a_state_file_missing_the_field_an_event_writes_is_refused_by_that_field_name() {
        use core_command_domain::orchestration::{Recomposed, StageSlugSet, TaskSynchronized};
        let task = || {
            IntentExecutionEvent::TaskSynchronized(TaskSynchronized::new(
                event_id(),
                execution_id(),
                slug("second"),
            ))
        };
        let genesis = || IntentExecutionEvent::Started(started());
        let approve_first = || {
            IntentExecutionEvent::GateApproved(GateApproved::new(
                event_id(),
                execution_id(),
                slug("first"),
                None,
            ))
        };
        let recompose = || {
            IntentExecutionEvent::Recomposed(Recomposed::new(
                event_id(),
                execution_id(),
                StageSlugSet::empty(),
                StageSlugSet::empty(),
            ))
        };
        let cases: Vec<(&str, IntentExecutionEvent)> = vec![
            ("- **Lifecycle Phase**: INITIALIZATION\n", task()),
            ("- **Active Agent**: orchestrator\n", task()),
            ("- **Last Completed Stage**: \n", genesis()),
            ("- **Total Stages**: 3\n", genesis()),
            ("- **Active Agent**: orchestrator\n", approve_first()),
            ("- **Lifecycle Phase**: INITIALIZATION\n", approve_first()),
            ("- **Next Stage**: first\n", approve_first()),
            ("- **Stages to Execute**: 0.1, 2.1, 2.2\n", recompose()),
            ("- **Stages to Skip**: 4.1 (late)\n", recompose()),
        ];
        for (line, event) in cases {
            assert!(SKELETON.contains(line), "{line}");
            let mut read_model = ReadModel::new(SKELETON.replace(line, ""));
            let field = line
                .trim_start_matches("- **")
                .split("**")
                .next()
                .expect("欄の名前");
            let error = project(&[entry(event)], &plan(), &mut read_model)
                .expect_err("欠けた欄を黙って読み飛ばさない");
            assert_eq!(
                error,
                ProjectionError::StateField(FieldNotFound::new(
                    super::super::wording::field_not_found_message(field)
                )),
                "{line}"
            );
        }
    }

    /// 初期化以外がすべて SKIP の計画では、誕生は次の位置へ入らず `Next Action` も書かない。
    #[test]
    fn a_genesis_whose_plan_has_no_stage_after_initialization_routes_nowhere() {
        let intent = Intent::from((
            Created::new(
                intent_event_id(),
                IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7"),
                WorkflowDefinitionId::parse("claude").expect("定義 id"),
                DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
                StartRequest::new("classic", "build it"),
                StageEntries::new(vec![
                    stage(
                        "state-init",
                        "0.1",
                        PhaseId::Initialization,
                        PlanAction::Execute,
                    ),
                    stage("first", "2.1", PhaseId::Inception, PlanAction::Skip),
                ])
                .expect("フィクスチャの計画は不変条件を満たす"),
                WorkspaceScan::new(
                    BrownfieldGreenfield::Greenfield,
                    "Unknown",
                    "Unknown",
                    "Unknown",
                )
                .expect("単一行"),
            ),
            at(),
        ));
        let plan = ResolvedPlan::of(&intent);
        let started = Started::new(
            event_id(),
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7"),
            intent.id().clone(),
            intent.stages().clone(),
        );
        let mut read_model = model();
        project(
            &[entry(IntentExecutionEvent::Started(started))],
            &plan,
            &mut read_model,
        )
        .expect("投影");
        assert!(
            read_model.state().contains("- [x] state-init — EXECUTE"),
            "{}",
            read_model.state()
        );
        assert!(
            read_model
                .state()
                .contains("- **Next Action**: Execute Stage\n"),
            "次の位置が無ければ Next Action は出発点のまま: {}",
            read_model.state()
        );
        assert!(
            !read_model
                .appended_audit()
                .contains("**Event**: STAGE_STARTED\n**Stage**: first"),
            "{}",
            read_model.appended_audit()
        );
    }
}
