//! `IntentRow` — `read_intent` の 1 行 (intent 1 件の静的な材料)。

use chrono::SecondsFormat;
use core_command_domain::orchestration::Intent;

/// `read_intent` の 1 行。主キーは 1 列 `id` = intent の識別子 (集約そのものの表なので
/// 代理キーを作らない)。`definition_id` は `read_definition.id` を指す FK である。
///
/// 値はすべて [`Intent`] のアクセサの写しである。走査結果は 2 つの綴りを持つ
/// (`project_type` は状態ファイル面の `Greenfield` / `Brownfield`、`project_kind` は
/// `stage-graph.json` 面の小文字) ので、**両方を列にする** — どちらか一方に寄せると、
/// 読取側がもう一方の綴りを組み直すことになる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentRow {
    id: String,
    definition_id: String,
    definition_revision: String,
    scope: String,
    request: String,
    depth: Option<String>,
    test_strategy: Option<String>,
    review: Option<String>,
    created_at: String,
    project_type: String,
    project_kind: String,
    languages: String,
    frameworks: String,
    build_system: String,
}

impl IntentRow {
    /// intent 集約を 1 行へ写す (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn of(intent: &Intent) -> IntentRow {
        let scan = intent.scan();
        IntentRow {
            id: intent.id().as_str().to_string(),
            definition_id: intent.definition_id().as_str().to_string(),
            definition_revision: intent.definition_revision().as_str().to_string(),
            scope: intent.scope().to_string(),
            request: intent.request().to_string(),
            depth: intent.depth().map(str::to_string),
            test_strategy: intent.test_strategy().map(str::to_string),
            review: intent.review().map(str::to_string),
            created_at: intent
                .created_at()
                .to_rfc3339_opts(SecondsFormat::Secs, true),
            project_type: scan.project_type().to_string(),
            project_kind: scan.project_kind().as_str().to_string(),
            languages: scan.languages().to_string(),
            frameworks: scan.frameworks().to_string(),
            build_system: scan.build_system().to_string(),
        }
    }

    /// 主キー — intent の識別子 (UUIDv7)。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 依拠した定義の系譜 ID。
    #[must_use]
    pub fn definition_id(&self) -> &str {
        &self.definition_id
    }

    /// 依拠した定義の内容版。
    #[must_use]
    pub fn definition_revision(&self) -> &str {
        &self.definition_revision
    }

    /// 選ばれたスコープ名。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// 人が書いた依頼文 (逐語)。
    #[must_use]
    pub fn request(&self) -> &str {
        &self.request
    }

    /// 詳細度の上書き (無ければ NULL)。
    #[must_use]
    pub fn depth(&self) -> Option<&str> {
        self.depth.as_deref()
    }

    /// テスト戦略の上書き (無ければ NULL)。
    #[must_use]
    pub fn test_strategy(&self) -> Option<&str> {
        self.test_strategy.as_deref()
    }

    /// レビュー階級の上書き (無ければ NULL)。
    #[must_use]
    pub fn review(&self) -> Option<&str> {
        self.review.as_deref()
    }

    /// 誕生時刻 (RFC3339 / 秒精度 / `Z`)。
    #[must_use]
    pub fn created_at(&self) -> &str {
        &self.created_at
    }

    /// 状態ファイル面の種別綴り (`Greenfield` / `Brownfield`)。
    #[must_use]
    pub fn project_type(&self) -> &str {
        &self.project_type
    }

    /// `stage-graph.json` 面の種別綴り (小文字)。
    #[must_use]
    pub fn project_kind(&self) -> &str {
        &self.project_kind
    }

    /// 検出した言語 (未検出は `Unknown`)。
    #[must_use]
    pub fn languages(&self) -> &str {
        &self.languages
    }

    /// 検出したフレームワーク (未検出は `Unknown`)。
    #[must_use]
    pub fn frameworks(&self) -> &str {
        &self.frameworks
    }

    /// 検出したビルドシステム (未検出は `Unknown`)。
    #[must_use]
    pub fn build_system(&self) -> &str {
        &self.build_system
    }
}
