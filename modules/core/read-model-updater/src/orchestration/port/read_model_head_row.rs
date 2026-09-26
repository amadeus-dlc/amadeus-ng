//! `ReadModelHeadRow` — `amadeus_read_model_head` の唯一の行 (共有構造化面の公開位置と内容の記録)。

/// `amadeus_read_model_head` の唯一の行。
///
/// 構造化面 (`read_*` 20 表) は space で共有され、実行ごとの投影がそれぞれの位置まで進める。
/// この行は「共有面がいまどの位置の歴史を映しているか (`position`)」「何度描き直したか
/// (`generation`)」「どの変換で描いたか (`revision`)」「20 表の内容のダイジェスト
/// (`content_digest`)」「その記録を歴史と照合済みか (`verified`)」を持つ。
///
/// 値は保存されたとおりに運ぶ — 位置や世代が負・ゼロ (手で壊した行) でも、読めた値を
/// そのまま返す。壊れた記録を見分け、描き直すかどうかを決めるのは更新器である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadModelHeadRow {
    position: i64,
    generation: i64,
    revision: String,
    content_digest: String,
    verified: bool,
}

impl ReadModelHeadRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        position: i64,
        generation: i64,
        revision: String,
        content_digest: String,
        verified: bool,
    ) -> Self {
        Self {
            position,
            generation,
            revision,
            content_digest,
            verified,
        }
    }

    /// 共有面が映している歴史の位置 (ジャーナル上の位置)。
    #[must_use]
    pub const fn position(&self) -> i64 {
        self.position
    }

    /// 描き直した回数 (記録のたびに 1 つ進む)。
    #[must_use]
    pub const fn generation(&self) -> i64 {
        self.generation
    }

    /// 描いた変換の版 (`publication-1/read-<読み面の版>`)。
    #[must_use]
    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// 20 表の内容のダイジェスト。
    #[must_use]
    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }

    /// 記録が歴史と照合済みか (旧いストアから持ち越した記録は未照合)。
    #[must_use]
    pub const fn is_verified(&self) -> bool {
        self.verified
    }
}
