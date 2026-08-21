//! 文言カタログ (ADR 0002 / A3) — 逐語文言とコマンド語彙写像表。emit 側と検出側を同居させる。純粋部品。
//!
//! 各エントリは upstream 出典 (`file:line` @ 3c3146cf) と採取状態を持つ:
//! - `captured`: stage-0 環境の実出力で確認済み
//! - `spec-quoted-only`: as-built 仕様の逐語引用が根拠 (ゴールデン採取待ち — ADR 0002 決定 5b)

#![forbid(unsafe_code)]

/// 採取状態 (ADR 0002 決定 1)。フェーズ A 完了条件は契約経路の全数 `Captured`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoldenStatus {
    Captured,
    SpecQuotedOnly,
}

/// 状態ファイル関連の逐語文言。
pub mod state {
    use super::GoldenStatus;

    /// 出典: `aidlc-lib.ts:6564` (upstream 03 §5.3)。
    pub const FIELD_NOT_FOUND_STATUS: GoldenStatus = GoldenStatus::SpecQuotedOnly;

    /// `setFieldStrict` の拒否文言 — 「無言 no-op は検出不能なドリフト」の強制。
    #[must_use]
    pub fn field_not_found(field: &str) -> String {
        format!(
            "Field not found in state file: \"{field}\". Cannot update — refusing to silently no-op."
        )
    }

    /// 出典: `aidlc-lib.ts:6453` (upstream 03 §5.6)。
    pub const FILE_NOT_FOUND_STATUS: GoldenStatus = GoldenStatus::SpecQuotedOnly;

    /// `readStateFile` の不在時文言。
    #[must_use]
    pub fn file_not_found(path: &str) -> String {
        format!("State file not found: {path}")
    }
}

/// 監査ロック関連の逐語文言。
pub mod lock {
    use super::GoldenStatus;

    /// 出典: `aidlc-audit.ts:543` (upstream 03 §6.8)。
    pub const ACQUIRE_FAILED_STATUS: GoldenStatus = GoldenStatus::SpecQuotedOnly;

    /// acquire 予算超過の呼出側翻訳文言 (`acquireAuditLock` は `false` を返し、呼出側が
    /// この文言へ翻訳する — 11-workspace §4 の `AcquireError::Exhausted` に対応)。
    #[must_use]
    pub const fn acquire_failed() -> &'static str {
        "Failed to acquire audit lock after retries"
    }
}

/// bolt / autonomy 関連の逐語文言。
pub mod bolt {
    use super::GoldenStatus;

    /// 出典: `aidlc-bolt.ts:808` (upstream 09 §5.6)。
    pub const INVALID_MODE_STATUS: GoldenStatus = GoldenStatus::SpecQuotedOnly;

    /// `set-autonomy --mode` の不正値拒否 (CLI 引数境界は 2 値厳密パース — 10 §2.2)。
    #[must_use]
    pub fn invalid_mode(mode: &str) -> String {
        format!("Invalid --mode: {mode}. Must be 'autonomous' or 'gated'.")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn invalid_mode_is_verbatim() {
        assert_eq!(
            super::bolt::invalid_mode("turbo"),
            "Invalid --mode: turbo. Must be 'autonomous' or 'gated'."
        );
    }

    #[test]
    fn file_not_found_is_verbatim() {
        assert_eq!(
            super::state::file_not_found("/ws/aidlc-state.md"),
            "State file not found: /ws/aidlc-state.md"
        );
    }

    #[test]
    fn acquire_failed_is_verbatim() {
        assert_eq!(
            super::lock::acquire_failed(),
            "Failed to acquire audit lock after retries"
        );
    }

    #[test]
    fn field_not_found_is_verbatim() {
        assert_eq!(
            super::state::field_not_found("Construction Autonomy Mode"),
            "Field not found in state file: \"Construction Autonomy Mode\". Cannot update — refusing to silently no-op."
        );
    }
}
