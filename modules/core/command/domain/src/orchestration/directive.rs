//! `Directive` — `next` / `continue` が放出する判別共用体 (公開言語 B14 のドメイン面)。
//!
//! upstream `validateDirective` 相当の検証は**型で**行う — kind ごとの typed variant なので
//! 未知キー・型違反・cross-field 違反は構成不能である (E1+E2)。ワイヤ JSON への直列化と
//! 28KiB 上限 (超過は emit 拒否 — half-emitted を出さない) は Presenter (U7) の責務で、
//! ここには持ち込まない (`coding-rules/domain-persistence-neutrality.md`)。
//!
//! placeholder 2 種 (`dispatch-subagent` / `present-gate`)、slice 2 の `invoke-swarm`、
//! B16 スライスの `load-steering` は variant を**持たない** — 「エンジンは今日これを
//! 構築しない」を構成不能で表す (投機実装禁止 — 02 §4.1)。[`DirectiveKind`] は 10 種の
//! 閉集合 (ワイヤ判別子のカタログ) のままで、この共用体は**構築できる部分集合**である。

use super::directive_schema::DirectiveKind;
use crate::workflow_definition::StageSlug;
use crate::workflow_definition::{PhaseId, ReviewClass, StageMode};

/// `run-stage` の `gate` フィールド — boolean か `"unresolved"` のみ (E2)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateField {
    /// 承認ゲート付き (`true`)。
    Gated,
    /// ゲートなし (`false` — 初期化ステージ)。
    Ungated,
    /// walking-skeleton 判定が要る非決定ケース (`"unresolved"`)。
    Unresolved,
}

/// 構造化質問の種別 — conductor の応答契約を選ぶ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AskKind {
    /// state ありでの `--resume` (分岐 6) — 再開メニュー。
    ResumeMenu,
    /// 稼働中の自由記述 (分岐 9c) — `new-work-routing`。回答は `next` 経由で、stage report に
    /// 記録してはならない (§4.5)。
    NewWorkRouting,
    /// state なし・キーワードヒットの scope 確認 (分岐 8)。
    ScopeConfirm,
    /// state なし・キーワード非ヒットの compose 提案 (分岐 8)。
    ComposeOffer,
    /// fresh clone の intent 選択 (分岐 7b — records はあるが active-intent カーソルなし)。
    IntentPick,
}

/// `ask` — 構造化質問の提示 (エンジンは人間ターンを conductor へ委ねる)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskDirective {
    kind: AskKind,
    question: String,
    proposed_scope: Option<String>,
    new_work_description: Option<String>,
}

impl AskDirective {
    /// 質問を組む (基本コンストラクタ)。
    #[must_use]
    pub const fn new(kind: AskKind, question: String) -> AskDirective {
        AskDirective {
            kind,
            question,
            proposed_scope: None,
            new_work_description: None,
        }
    }

    /// `new-work-routing` の材料 (提案 scope と新規作業の記述) を伴う。
    #[must_use]
    pub fn with_new_work(
        mut self,
        proposed_scope: impl Into<String>,
        description: impl Into<String>,
    ) -> AskDirective {
        self.proposed_scope = Some(proposed_scope.into());
        self.new_work_description = Some(description.into());
        self
    }

    /// 質問の種別。
    #[must_use]
    pub const fn ask_kind(&self) -> AskKind {
        self.kind
    }

    /// 質問文 (逐語)。
    #[must_use]
    pub fn question(&self) -> &str {
        &self.question
    }

    /// 提案 scope (`new-work-routing` / scope 確認)。
    #[must_use]
    pub fn proposed_scope(&self) -> Option<&str> {
        self.proposed_scope.as_deref()
    }

    /// 新規作業の記述 (`new-work-routing`)。
    #[must_use]
    pub fn new_work_description(&self) -> Option<&str> {
        self.new_work_description.as_deref()
    }
}

/// `run-stage` — ステージ本体の実行指示。
///
/// steering 由来のフィールド (`rules_in_context` / 束ダイジェスト) は B16 (load-steering
/// 連鎖) で載る。フィールドは private + アクセサ (`coding-rules/field-visibility.md`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStageDirective {
    stage: StageSlug,
    phase: PhaseId,
    lead_agent: String,
    support_agents: Vec<String>,
    mode: StageMode,
    gate: GateField,
    stage_file: String,
    memory_path: String,
    inline_context_paths: Vec<String>,
    consumes: Vec<String>,
    produces: Vec<String>,
    sensors_applicable: Vec<String>,
    next_stage: Option<String>,
    reviewer: Option<String>,
    review_class: Option<ReviewClass>,
    reviewer_max_iterations: Option<u32>,
    protocol_modules: Vec<String>,
    narration: Option<String>,
    single: bool,
}

/// [`RunStageDirective`] の組み立て器 — 必須 6 点を基本コンストラクタ相当で受け、残りは
/// `with_*` で伴わせる。`build()` だけが構造体リテラルを書く (factory-naming.md)。
#[derive(Debug, Clone)]
pub struct RunStageDirectiveBuilder {
    stage: StageSlug,
    phase: PhaseId,
    lead_agent: String,
    mode: StageMode,
    gate: GateField,
    stage_file: String,
    memory_path: String,
    support_agents: Vec<String>,
    inline_context_paths: Vec<String>,
    consumes: Vec<String>,
    produces: Vec<String>,
    sensors_applicable: Vec<String>,
    next_stage: Option<String>,
    reviewer: Option<String>,
    review_class: Option<ReviewClass>,
    reviewer_max_iterations: Option<u32>,
    protocol_modules: Vec<String>,
    narration: Option<String>,
    single: bool,
}

impl RunStageDirectiveBuilder {
    /// 必須材料 (ステージ・フェーズ・リード・モード・ゲート・本体ファイル・日誌) を束ねる。
    #[must_use]
    pub fn new(
        stage: StageSlug,
        phase: PhaseId,
        lead_agent: impl Into<String>,
        mode: StageMode,
        gate: GateField,
        stage_file: impl Into<String>,
        memory_path: impl Into<String>,
    ) -> RunStageDirectiveBuilder {
        RunStageDirectiveBuilder {
            stage,
            phase,
            lead_agent: lead_agent.into(),
            mode,
            gate,
            stage_file: stage_file.into(),
            memory_path: memory_path.into(),
            support_agents: Vec::new(),
            inline_context_paths: Vec::new(),
            consumes: Vec::new(),
            produces: Vec::new(),
            sensors_applicable: Vec::new(),
            next_stage: None,
            reviewer: None,
            review_class: None,
            reviewer_max_iterations: None,
            protocol_modules: Vec::new(),
            narration: None,
            single: false,
        }
    }

    /// 支援エージェント列を伴う。
    #[must_use]
    pub fn with_support_agents(mut self, agents: Vec<String>) -> RunStageDirectiveBuilder {
        self.support_agents = agents;
        self
    }

    /// 読み込み必須のコンテキストパス列を伴う。
    #[must_use]
    pub fn with_inline_context_paths(mut self, paths: Vec<String>) -> RunStageDirectiveBuilder {
        self.inline_context_paths = paths;
        self
    }

    /// 上流成果物 (consumes) を伴う。
    #[must_use]
    pub fn with_consumes(mut self, consumes: Vec<String>) -> RunStageDirectiveBuilder {
        self.consumes = consumes;
        self
    }

    /// 産出物 (produces) を伴う。
    #[must_use]
    pub fn with_produces(mut self, produces: Vec<String>) -> RunStageDirectiveBuilder {
        self.produces = produces;
        self
    }

    /// 発火センサー列を伴う。
    #[must_use]
    pub fn with_sensors(mut self, sensors: Vec<String>) -> RunStageDirectiveBuilder {
        self.sensors_applicable = sensors;
        self
    }

    /// 次ステージの表示名を伴う。
    #[must_use]
    pub fn with_next_stage(mut self, next_stage: impl Into<String>) -> RunStageDirectiveBuilder {
        self.next_stage = Some(next_stage.into());
        self
    }

    /// レビュアー構成 (名前・クラス・最大反復) を伴う。
    #[must_use]
    pub fn with_reviewer(
        mut self,
        reviewer: impl Into<String>,
        class: ReviewClass,
        max_iterations: u32,
    ) -> RunStageDirectiveBuilder {
        self.reviewer = Some(reviewer.into());
        self.review_class = Some(class);
        self.reviewer_max_iterations = Some(max_iterations);
        self
    }

    /// プロトコルモジュールのヒント列を伴う。
    #[must_use]
    pub fn with_protocol_modules(mut self, modules: Vec<String>) -> RunStageDirectiveBuilder {
        self.protocol_modules = modules;
        self
    }

    /// ユーザ向けのひとことを伴う。
    #[must_use]
    pub fn with_narration(mut self, narration: impl Into<String>) -> RunStageDirectiveBuilder {
        self.narration = Some(narration.into());
        self
    }

    /// 単一ステージ隔離モード (`--single`) を伴う。
    #[must_use]
    pub const fn with_single(mut self) -> RunStageDirectiveBuilder {
        self.single = true;
        self
    }

    /// 組み上げる (構造体リテラルはここだけ)。
    #[must_use]
    pub fn build(self) -> RunStageDirective {
        RunStageDirective {
            stage: self.stage,
            phase: self.phase,
            lead_agent: self.lead_agent,
            support_agents: self.support_agents,
            mode: self.mode,
            gate: self.gate,
            stage_file: self.stage_file,
            memory_path: self.memory_path,
            inline_context_paths: self.inline_context_paths,
            consumes: self.consumes,
            produces: self.produces,
            sensors_applicable: self.sensors_applicable,
            next_stage: self.next_stage,
            reviewer: self.reviewer,
            review_class: self.review_class,
            reviewer_max_iterations: self.reviewer_max_iterations,
            protocol_modules: self.protocol_modules,
            narration: self.narration,
            single: self.single,
        }
    }
}

impl RunStageDirective {
    /// 走らせるステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// フェーズ。
    #[must_use]
    pub const fn phase(&self) -> PhaseId {
        self.phase
    }

    /// リードエージェント。
    #[must_use]
    pub fn lead_agent(&self) -> &str {
        &self.lead_agent
    }

    /// 支援エージェント列。
    #[must_use]
    pub fn support_agents(&self) -> &[String] {
        &self.support_agents
    }

    /// 通信トポロジ。
    #[must_use]
    pub const fn mode(&self) -> StageMode {
        self.mode
    }

    /// 承認ゲートの有無 (`unresolved` は walking-skeleton 判定待ち)。
    #[must_use]
    pub const fn gate(&self) -> GateField {
        self.gate
    }

    /// ステージ本体ファイルのパス。
    #[must_use]
    pub fn stage_file(&self) -> &str {
        &self.stage_file
    }

    /// ステージ日誌のパス。
    #[must_use]
    pub fn memory_path(&self) -> &str {
        &self.memory_path
    }

    /// 読み込み必須のコンテキストパス列。
    #[must_use]
    pub fn inline_context_paths(&self) -> &[String] {
        &self.inline_context_paths
    }

    /// 上流成果物のパス列。
    #[must_use]
    pub fn consumes(&self) -> &[String] {
        &self.consumes
    }

    /// 産出物のパス列。
    #[must_use]
    pub fn produces(&self) -> &[String] {
        &self.produces
    }

    /// 発火センサー列。
    #[must_use]
    pub fn sensors_applicable(&self) -> &[String] {
        &self.sensors_applicable
    }

    /// 次ステージの表示名。
    #[must_use]
    pub fn next_stage(&self) -> Option<&str> {
        self.next_stage.as_deref()
    }

    /// レビュアー (実効)。
    #[must_use]
    pub fn reviewer(&self) -> Option<&str> {
        self.reviewer.as_deref()
    }

    /// レビュークラス。
    #[must_use]
    pub const fn review_class(&self) -> Option<ReviewClass> {
        self.review_class
    }

    /// レビュアーの最大反復。
    #[must_use]
    pub const fn reviewer_max_iterations(&self) -> Option<u32> {
        self.reviewer_max_iterations
    }

    /// プロトコルモジュールのヒント列。
    #[must_use]
    pub fn protocol_modules(&self) -> &[String] {
        &self.protocol_modules
    }

    /// ユーザ向けのひとこと。
    #[must_use]
    pub fn narration(&self) -> Option<&str> {
        self.narration.as_deref()
    }

    /// 単一ステージ隔離モードか。
    #[must_use]
    pub const fn is_single(&self) -> bool {
        self.single
    }
}

/// 放出できる directive の判別共用体。
#[allow(
    clippy::large_enum_variant,
    reason = "run-stage が最大の payload を持つのは公開言語の形そのもの — Box で包むと \
              消費側のパターンが崩れる。directive は 1 ターン 1 個しか生成しない"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    /// ステージ本体の実行。
    RunStage(RunStageDirective),
    /// 構造化質問の提示。
    Ask(AskDirective),
    /// 逐語で印字して停止、または名指しのコマンド実行 (print の 3 形)。
    Print {
        /// 逐語メッセージ (コマンドの名指しを含む)。
        message: String,
    },
    /// エラーで停止。`message` はユーザへ逐語で見せる。
    Error {
        /// 逐語メッセージ。
        message: String,
    },
    /// ループの停止 (完了・エピローグ・冪等な終端)。
    Done {
        /// 理由 (省略可 — 逐語)。
        reason: Option<String>,
    },
    /// park 済みワークフローでの停止。
    Parked {
        /// park している位置。
        stage: StageSlug,
        /// 逐語メッセージ (`Workflow parked at ...`)。
        message: String,
    },
}

impl Directive {
    /// ワイヤ判別子。
    #[must_use]
    pub const fn kind(&self) -> DirectiveKind {
        match self {
            Directive::RunStage(_) => DirectiveKind::RunStage,
            Directive::Ask(_) => DirectiveKind::Ask,
            Directive::Print { .. } => DirectiveKind::Print,
            Directive::Error { .. } => DirectiveKind::Error,
            Directive::Done { .. } => DirectiveKind::Done,
            Directive::Parked { .. } => DirectiveKind::Parked,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_definition::StageSlug;

    fn slug() -> StageSlug {
        StageSlug::parse("requirements-analysis").unwrap()
    }

    #[test]
    fn every_constructible_variant_names_its_wire_kind() {
        let run_stage = RunStageDirectiveBuilder::new(
            slug(),
            PhaseId::Inception,
            "aidlc-product-agent",
            StageMode::Inline,
            GateField::Gated,
            "stages/inception/requirements-analysis.md",
            "record/inception/requirements-analysis/memory.md",
        )
        .build();
        assert_eq!(
            Directive::RunStage(run_stage).kind(),
            DirectiveKind::RunStage
        );
        assert_eq!(
            Directive::Ask(AskDirective::new(
                AskKind::ResumeMenu,
                "How would you like to proceed?".to_string()
            ))
            .kind(),
            DirectiveKind::Ask
        );
        assert_eq!(
            Directive::Print {
                message: "aidlc-utility status".to_string()
            }
            .kind(),
            DirectiveKind::Print
        );
        assert_eq!(
            Directive::Error {
                message: "boom".to_string()
            }
            .kind(),
            DirectiveKind::Error
        );
        assert_eq!(Directive::Done { reason: None }.kind(), DirectiveKind::Done);
        assert_eq!(
            Directive::Parked {
                stage: slug(),
                message: "Workflow parked".to_string()
            }
            .kind(),
            DirectiveKind::Parked
        );
    }

    #[test]
    fn the_builder_carries_every_optional_face() {
        let directive = RunStageDirectiveBuilder::new(
            slug(),
            PhaseId::Inception,
            "aidlc-product-agent",
            StageMode::Inline,
            GateField::Unresolved,
            "stage.md",
            "memory.md",
        )
        .with_support_agents(vec!["aidlc-design-agent".to_string()])
        .with_inline_context_paths(vec!["agents/aidlc-product-agent.md".to_string()])
        .with_consumes(vec!["a.md".to_string()])
        .with_produces(vec!["b.md".to_string()])
        .with_sensors(vec!["traceability".to_string()])
        .with_next_stage("User Stories")
        .with_reviewer("aidlc-product-lead-agent", ReviewClass::Advisory, 1)
        .with_protocol_modules(vec!["reviewer".to_string()])
        .with_narration("Now working on requirements.")
        .with_single()
        .build();
        assert_eq!(directive.support_agents(), ["aidlc-design-agent"]);
        assert_eq!(directive.inline_context_paths().len(), 1);
        assert_eq!(directive.consumes(), ["a.md"]);
        assert_eq!(directive.produces(), ["b.md"]);
        assert_eq!(directive.sensors_applicable(), ["traceability"]);
        assert_eq!(directive.next_stage(), Some("User Stories"));
        assert_eq!(directive.reviewer(), Some("aidlc-product-lead-agent"));
        assert_eq!(directive.review_class(), Some(ReviewClass::Advisory));
        assert_eq!(directive.reviewer_max_iterations(), Some(1));
        assert_eq!(directive.protocol_modules(), ["reviewer"]);
        assert_eq!(directive.narration(), Some("Now working on requirements."));
        assert!(directive.is_single());
        assert_eq!(directive.gate(), GateField::Unresolved);
    }

    #[test]
    fn an_ask_can_carry_the_new_work_material() {
        let ask = AskDirective::new(AskKind::NewWorkRouting, "route?".to_string())
            .with_new_work("bugfix", "fix the login crash");
        assert_eq!(ask.ask_kind(), AskKind::NewWorkRouting);
        assert_eq!(ask.proposed_scope(), Some("bugfix"));
        assert_eq!(ask.new_work_description(), Some("fix the login crash"));
    }
}
