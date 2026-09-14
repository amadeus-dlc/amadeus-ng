//! `ReScopeView` — reverse-engineering ストアが記録した「走査範囲」1 件の写し。
//!
//! `reverse-engineering-timestamp.md` の中の fenced yaml ブロック (`scope_version: 1`) は
//! RE ステージが人間のゲートの向こうで書いたリードモデルであり、クエリ側はこれを**読むだけ**
//! である (`coding-rules/cqrs-boundaries.md` 規則 7)。媒体 (Markdown 中の yaml) は DAO 実装の
//! 内部詳細であり、この写しには現れない。
//!
//! 表現は隠し、契約 (アクセサ) だけを公開する (`coding-rules/field-visibility.md`)。

/// ストアが記録した走査範囲。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReScopeView {
    kind: String,
    intent: String,
    fingerprint: Option<String>,
    analyzed_paths: Vec<String>,
    analyzed_components: Vec<String>,
    shallow_paths: Vec<String>,
}

impl ReScopeView {
    /// 6 列をそのまま束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        kind: String,
        intent: String,
        fingerprint: Option<String>,
        analyzed_paths: Vec<String>,
        analyzed_components: Vec<String>,
        shallow_paths: Vec<String>,
    ) -> ReScopeView {
        ReScopeView {
            kind,
            intent,
            fingerprint,
            analyzed_paths,
            analyzed_components,
            shallow_paths,
        }
    }

    /// 網羅の別 (`full` / `partial`)。
    ///
    /// パーサがこの 2 綴りのどちらかであることを保証してから写す — 行の綴りのままで、表示の
    /// 読み替えは出す側である ([`super::StageGraphEntryView`] の `phase` / `mode` と同じ流儀)。
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// この走査を行った intent の slug（記録が無ければ空）。
    #[must_use]
    pub fn intent(&self) -> &str {
        &self.intent
    }

    /// 記録された内容指紋（`unknown` / 空欄は `None`）。
    #[must_use]
    pub fn fingerprint(&self) -> Option<&str> {
        self.fingerprint.as_deref()
    }

    /// 深く読んだと主張するパス。
    #[must_use]
    pub fn analyzed_paths(&self) -> &[String] {
        &self.analyzed_paths
    }

    /// 深く読んだと主張する構成要素の名前。
    #[must_use]
    pub fn analyzed_components(&self) -> &[String] {
        &self.analyzed_components
    }

    /// 浅くしか見ていないパス。
    #[must_use]
    pub fn shallow_paths(&self) -> &[String] {
        &self.shallow_paths
    }
}
