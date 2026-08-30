//! `StoreVersion` — ストアが採番する楽観 version の不透明トークン (steering 束縛用)。
//!
//! 我々は解釈も比較も算術もしない — 読んだ値を運ぶだけである (BR5.3)。
//!
//! **適用範囲**: Repository ポート面 (`store` の `expected_version` /
//! `RehydratedIntentExecution::version`) は本家 event-store-adapter-rs v3 の語彙どおり
//! `usize` のまま — newtype で包む案は Conformist 方針違反として**却下済み**である
//! (オーナー裁定 2026-08-29、`coding-rules/upstream-contracts.md`)。本型はその裁定の射程外
//! にある**自前の VO** ([`super::state_position::StatePosition`]) の中でだけ使う: そこでは
//! `seq_nr: usize` と隣接して取り違えがコンパイルを通るため、不透明トークンを newtype で
//! 運ぶ (オーナー裁定 2026-08-30)。

/// ストア採番の楽観 version (不透明トークン)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreVersion(usize);

impl StoreVersion {
    /// ストア (アダプタ層) が読み取った採番値を包む。
    #[must_use]
    pub const fn new(value: usize) -> StoreVersion {
        StoreVersion(value)
    }

    /// ストア境界へ返す生値 (アダプタ層専用 — ユースケースは呼ばない)。
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_token_round_trips_and_compares_by_value() {
        assert_eq!(StoreVersion::new(4).as_usize(), 4);
        assert_eq!(StoreVersion::new(4), StoreVersion::new(4));
        assert_ne!(StoreVersion::new(4), StoreVersion::new(5));
    }
}
