//! 観測したタイムスタンプ — 生の綴りと、解釈できたときのエポックミリ秒。

/// 綴りは表示用、ミリ秒は比較用。解釈できない綴りは `None` を運ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampView {
    raw: String,
    millis: Option<i64>,
}

impl TimestampView {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(raw: String, millis: Option<i64>) -> Self {
        Self { raw, millis }
    }

    /// 観測した綴りそのもの。
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// エポックミリ秒 (解釈できなければ `None`)。
    #[must_use]
    pub const fn millis(&self) -> Option<i64> {
        self.millis
    }
}
