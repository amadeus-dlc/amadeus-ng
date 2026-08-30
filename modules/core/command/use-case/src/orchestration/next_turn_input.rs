//! `NextTurnInput` — `next` 1 回分の観測 (Controller がパース済みの材料だけを運ぶ VO)。
//!
//! CLI フラグのパース・環境変数の読取・マシンローカルマーカーの観測は Controller (U7) の
//! 責務で、本 VO はその**結果**だけを運ぶ。ユースケースは本 VO + 読取専用ポートで 21 分岐
//! ラダーを畳む (use-case-rules §2b — execute 引数は ID + VO のみ)。

use core_command_domain::orchestration::{IntentExecutionId, IntentId, ReadOnlyVerb};
use core_command_domain::workflow_definition::WorkflowDefinitionId;

/// 名詞トークンの族 (分岐 1b/1c/1d — 先頭トークン意味論のみ)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NounFamily {
    /// `space` / `intent` (分岐 1b)。
    Workspace,
    /// `plugin` (分岐 1c)。
    Plugin,
    /// `knowledge` (分岐 1d)。
    Knowledge,
}

/// 名詞トークン (族 + 残りのトークン列は逐語で通す)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NounToken {
    family: NounFamily,
    tokens: Vec<String>,
}

impl NounToken {
    /// 族と残トークンを束ねる。
    #[must_use]
    pub const fn new(family: NounFamily, tokens: Vec<String>) -> NounToken {
        NounToken { family, tokens }
    }

    /// 族。
    #[must_use]
    pub const fn family(&self) -> NounFamily {
        self.family
    }

    /// 先頭トークンを含む残りのトークン列 (逐語)。
    #[must_use]
    pub fn tokens(&self) -> &[String] {
        &self.tokens
    }
}

/// 稼働中ワークフローの識別子束 (active-intent カーソルの解決結果 — Controller 供給)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveWorkflow {
    intent_id: IntentId,
    execution_id: IntentExecutionId,
}

impl ActiveWorkflow {
    /// 2 識別子を束ねる。
    #[must_use]
    pub const fn new(intent_id: IntentId, execution_id: IntentExecutionId) -> ActiveWorkflow {
        ActiveWorkflow {
            intent_id,
            execution_id,
        }
    }

    /// intent の識別子。
    #[must_use]
    pub const fn intent_id(&self) -> &IntentId {
        &self.intent_id
    }

    /// 実行の識別子。
    #[must_use]
    pub const fn execution_id(&self) -> &IntentExecutionId {
        &self.execution_id
    }
}

/// レコードとステージ資産の配置 (パス組み立ての材料 — Controller 供給)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceLayout {
    record_dir: String,
    stage_library_dir: String,
    agent_dir: String,
}

impl WorkspaceLayout {
    /// 3 つの配置を束ねる。
    #[must_use]
    pub const fn new(
        record_dir: String,
        stage_library_dir: String,
        agent_dir: String,
    ) -> WorkspaceLayout {
        WorkspaceLayout {
            record_dir,
            stage_library_dir,
            agent_dir,
        }
    }

    /// 稼働中 intent の record ディレクトリ (`aidlc/spaces/<space>/intents/<slug>-<id8>`)。
    #[must_use]
    pub fn record_dir(&self) -> &str {
        &self.record_dir
    }

    /// ステージ本体ファイルの置き場 (`.claude/aidlc-common/stages`)。
    #[must_use]
    pub fn stage_library_dir(&self) -> &str {
        &self.stage_library_dir
    }

    /// エージェントペルソナの置き場 (`.claude/agents`)。
    #[must_use]
    pub fn agent_dir(&self) -> &str {
        &self.agent_dir
    }
}

/// `next` 1 回分の観測。フィールドは private + アクセサ、構築はビルダー (材料が多いため)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NextTurnInput {
    parse_error: Option<String>,
    review: Option<String>,
    read_only: Option<ReadOnlyVerb>,
    noun_token: Option<NounToken>,
    stage: Option<String>,
    phase: Option<String>,
    scope: Option<String>,
    depth: Option<String>,
    test_strategy: Option<String>,
    freeform: Option<String>,
    resume: bool,
    single: bool,
    compose: bool,
    new_intent: Option<String>,
    env_default_scope: Option<String>,
    kiro_latch_bare_next: bool,
    records_exist_without_cursor: bool,
    active: Option<ActiveWorkflow>,
    layout: Option<WorkspaceLayout>,
    definition_id: Option<WorkflowDefinitionId>,
}

impl NextTurnInput {
    /// 何も観測していない素の 1 回 (ビルダーの起点)。
    #[must_use]
    pub fn new() -> NextTurnInput {
        NextTurnInput::default()
    }

    /// フラグのパース失敗 (逐語 stderr) を伴う。
    #[must_use]
    pub fn with_parse_error(mut self, message: impl Into<String>) -> NextTurnInput {
        self.parse_error = Some(message.into());
        self
    }

    /// `--review <class>` を伴う。
    #[must_use]
    pub fn with_review(mut self, class: impl Into<String>) -> NextTurnInput {
        self.review = Some(class.into());
        self
    }

    /// 読み取り専用動詞を伴う。
    #[must_use]
    pub const fn with_read_only(mut self, verb: ReadOnlyVerb) -> NextTurnInput {
        self.read_only = Some(verb);
        self
    }

    /// 名詞トークンを伴う。
    #[must_use]
    pub fn with_noun_token(mut self, token: NounToken) -> NextTurnInput {
        self.noun_token = Some(token);
        self
    }

    /// `--stage <slug>` を伴う。
    #[must_use]
    pub fn with_stage(mut self, stage: impl Into<String>) -> NextTurnInput {
        self.stage = Some(stage.into());
        self
    }

    /// `--phase <name>` を伴う。
    #[must_use]
    pub fn with_phase(mut self, phase: impl Into<String>) -> NextTurnInput {
        self.phase = Some(phase.into());
        self
    }

    /// 明示 `--scope <name>` を伴う。
    #[must_use]
    pub fn with_scope(mut self, scope: impl Into<String>) -> NextTurnInput {
        self.scope = Some(scope.into());
        self
    }

    /// `--depth <level>` を伴う。
    #[must_use]
    pub fn with_depth(mut self, depth: impl Into<String>) -> NextTurnInput {
        self.depth = Some(depth.into());
        self
    }

    /// `--test-strategy <level>` を伴う。
    #[must_use]
    pub fn with_test_strategy(mut self, level: impl Into<String>) -> NextTurnInput {
        self.test_strategy = Some(level.into());
        self
    }

    /// 位置引数の自由記述 (scope 名の可能性も含む) を伴う。
    #[must_use]
    pub fn with_freeform(mut self, text: impl Into<String>) -> NextTurnInput {
        self.freeform = Some(text.into());
        self
    }

    /// `--resume` を伴う。
    #[must_use]
    pub const fn with_resume(mut self) -> NextTurnInput {
        self.resume = true;
        self
    }

    /// `--single` を伴う。
    #[must_use]
    pub const fn with_single(mut self) -> NextTurnInput {
        self.single = true;
        self
    }

    /// `compose` / `--new-scope` / `--report` を伴う。
    #[must_use]
    pub const fn with_compose(mut self) -> NextTurnInput {
        self.compose = true;
        self
    }

    /// `--new-intent` (記述つき) を伴う。
    #[must_use]
    pub fn with_new_intent(mut self, description: impl Into<String>) -> NextTurnInput {
        self.new_intent = Some(description.into());
        self
    }

    /// `AWS_AIDLC_DEFAULT_SCOPE` の生値を伴う。
    #[must_use]
    pub fn with_env_default_scope(mut self, value: impl Into<String>) -> NextTurnInput {
        self.env_default_scope = Some(value.into());
        self
    }

    /// Kiro roll-forward ラッチの同一ターン観測 (分岐 0) を伴う。
    #[must_use]
    pub const fn with_kiro_latch_bare_next(mut self) -> NextTurnInput {
        self.kiro_latch_bare_next = true;
        self
    }

    /// 「records は存在するが active-intent カーソルなし」(分岐 7b の intent-pick) を伴う。
    #[must_use]
    pub const fn with_records_without_cursor(mut self) -> NextTurnInput {
        self.records_exist_without_cursor = true;
        self
    }

    /// 稼働中ワークフローの識別子束を伴う。
    #[must_use]
    pub fn with_active(mut self, active: ActiveWorkflow) -> NextTurnInput {
        self.active = Some(active);
        self
    }

    /// ワークスペース配置を伴う。
    #[must_use]
    pub fn with_layout(mut self, layout: WorkspaceLayout) -> NextTurnInput {
        self.layout = Some(layout);
        self
    }

    /// ハーネスの定義 id (state 不在時の birth / jump 用) を伴う。
    #[must_use]
    pub fn with_definition_id(mut self, id: WorkflowDefinitionId) -> NextTurnInput {
        self.definition_id = Some(id);
        self
    }

    // ---- 観測面 ----

    /// フラグのパース失敗。
    #[must_use]
    pub fn parse_error(&self) -> Option<&str> {
        self.parse_error.as_deref()
    }

    /// `--review`。
    #[must_use]
    pub fn review(&self) -> Option<&str> {
        self.review.as_deref()
    }

    /// 読み取り専用動詞。
    #[must_use]
    pub const fn read_only(&self) -> Option<ReadOnlyVerb> {
        self.read_only
    }

    /// 名詞トークン。
    #[must_use]
    pub const fn noun_token(&self) -> Option<&NounToken> {
        self.noun_token.as_ref()
    }

    /// `--stage`。
    #[must_use]
    pub fn stage(&self) -> Option<&str> {
        self.stage.as_deref()
    }

    /// `--phase`。
    #[must_use]
    pub fn phase(&self) -> Option<&str> {
        self.phase.as_deref()
    }

    /// 明示 `--scope`。
    #[must_use]
    pub fn scope(&self) -> Option<&str> {
        self.scope.as_deref()
    }

    /// `--depth`。
    #[must_use]
    pub fn depth(&self) -> Option<&str> {
        self.depth.as_deref()
    }

    /// `--test-strategy`。
    #[must_use]
    pub fn test_strategy(&self) -> Option<&str> {
        self.test_strategy.as_deref()
    }

    /// 位置引数の自由記述。
    #[must_use]
    pub fn freeform(&self) -> Option<&str> {
        self.freeform.as_deref()
    }

    /// `--resume` か。
    #[must_use]
    pub const fn is_resume(&self) -> bool {
        self.resume
    }

    /// `--single` か。
    #[must_use]
    pub const fn is_single(&self) -> bool {
        self.single
    }

    /// compose 系か。
    #[must_use]
    pub const fn is_compose(&self) -> bool {
        self.compose
    }

    /// `--new-intent` の記述。
    #[must_use]
    pub fn new_intent(&self) -> Option<&str> {
        self.new_intent.as_deref()
    }

    /// `AWS_AIDLC_DEFAULT_SCOPE` の生値。
    #[must_use]
    pub fn env_default_scope(&self) -> Option<&str> {
        self.env_default_scope.as_deref()
    }

    /// Kiro roll-forward ラッチの同一ターン観測か。
    #[must_use]
    pub const fn is_kiro_latch_bare_next(&self) -> bool {
        self.kiro_latch_bare_next
    }

    /// records はあるが active-intent カーソルが無いか。
    #[must_use]
    pub const fn records_exist_without_cursor(&self) -> bool {
        self.records_exist_without_cursor
    }

    /// 稼働中ワークフローの識別子束。
    #[must_use]
    pub const fn active(&self) -> Option<&ActiveWorkflow> {
        self.active.as_ref()
    }

    /// ワークスペース配置。
    #[must_use]
    pub const fn layout(&self) -> Option<&WorkspaceLayout> {
        self.layout.as_ref()
    }

    /// ハーネスの定義 id。
    #[must_use]
    pub const fn definition_id(&self) -> Option<&WorkflowDefinitionId> {
        self.definition_id.as_ref()
    }
}
