//! `NextTurnInput` — `next` 1 回分の観測 (Controller がパース済みの材料だけを運ぶ VO)。
//!
//! CLI フラグのパース・環境変数の読取・マシンローカルマーカーの観測は Controller (U7) の
//! 責務で、本 VO はその**結果**だけを運ぶ。ユースケースは本 VO + 読み終えたビューで
//! 21 分岐ラダーを畳む。
//!
//! **識別子は運ばない** — 旧コマンド側の `ActiveWorkflow` (intent / execution の uuid) と
//! `definition_id` は Repository を引くための材料だった。クエリ側は Repository を持たず、
//! リードモデルを読む DAO ポート ([`super::ExecutionStateDao`] /
//! [`super::WorkflowDefinitionDao`]) が**どこを読むかを構築時に決めている**ので、識別子は
//! observed 面から消える。

use super::noun_token::NounToken;
use super::read_only_verb::ReadOnlyVerb;

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
}

#[cfg(test)]
mod tests {
    use super::super::noun_family::NounFamily;
    use super::*;

    #[test]
    fn a_plain_turn_observes_nothing() {
        let input = NextTurnInput::new();
        assert_eq!(input.parse_error(), None);
        assert_eq!(input.review(), None);
        assert_eq!(input.read_only(), None);
        assert_eq!(input.noun_token(), None);
        assert_eq!(input.stage(), None);
        assert_eq!(input.phase(), None);
        assert_eq!(input.scope(), None);
        assert_eq!(input.depth(), None);
        assert_eq!(input.test_strategy(), None);
        assert_eq!(input.freeform(), None);
        assert!(!input.is_resume());
        assert!(!input.is_single());
        assert!(!input.is_compose());
        assert_eq!(input.new_intent(), None);
        assert_eq!(input.env_default_scope(), None);
        assert!(!input.is_kiro_latch_bare_next());
        assert!(!input.records_exist_without_cursor());
    }

    #[test]
    fn the_builder_carries_every_observation() {
        let input = NextTurnInput::new()
            .with_parse_error("boom")
            .with_review("advisory")
            .with_read_only(ReadOnlyVerb::Status)
            .with_noun_token(NounToken::new(
                NounFamily::Workspace,
                vec!["intent".to_string(), "list".to_string()],
            ))
            .with_stage("domain-design")
            .with_phase("inception")
            .with_scope("bugfix")
            .with_depth("standard")
            .with_test_strategy("minimal")
            .with_freeform("fix the login crash")
            .with_resume()
            .with_single()
            .with_compose()
            .with_new_intent("new work")
            .with_env_default_scope("mvp")
            .with_kiro_latch_bare_next()
            .with_records_without_cursor();
        assert_eq!(input.parse_error(), Some("boom"));
        assert_eq!(input.review(), Some("advisory"));
        assert_eq!(input.read_only(), Some(ReadOnlyVerb::Status));
        assert_eq!(
            input.noun_token().map(NounToken::family),
            Some(NounFamily::Workspace)
        );
        assert_eq!(
            input.noun_token().map(NounToken::tokens),
            Some(&["intent".to_string(), "list".to_string()][..])
        );
        assert_eq!(input.stage(), Some("domain-design"));
        assert_eq!(input.phase(), Some("inception"));
        assert_eq!(input.scope(), Some("bugfix"));
        assert_eq!(input.depth(), Some("standard"));
        assert_eq!(input.test_strategy(), Some("minimal"));
        assert_eq!(input.freeform(), Some("fix the login crash"));
        assert!(input.is_resume());
        assert!(input.is_single());
        assert!(input.is_compose());
        assert_eq!(input.new_intent(), Some("new work"));
        assert_eq!(input.env_default_scope(), Some("mvp"));
        assert!(input.is_kiro_latch_bare_next());
        assert!(input.records_exist_without_cursor());
    }
}
