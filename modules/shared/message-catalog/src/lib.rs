//! 文言カタログ (ADR 0002 / A3) — 逐語文言とコマンド語彙写像表。emit 側と検出側を同居させる。純粋部品。
//!
//! 各エントリは upstream 出典 (`file:line` @ 3c3146cf) と採取状態を持つ:
//! - `captured`: ピン留めソース `3c3146cf` の実バイト、ないし stage-0 環境の実出力で確認済み
//! - `spec-quoted-only`: as-built 仕様の逐語引用が根拠 (ゴールデン採取待ち — ADR 0002 決定 5b)
//!
//! **2026-08-22 のゴールデン採取で、残っていた `SpecQuotedOnly` 4 件
//! (`state::field_not_found` / `state::file_not_found` / `lock::acquire_failed` /
//! `bolt::invalid_mode`) はピン留めソースに対し 4/4 バイト一致**することを確認した
//! (em dash U+2014・引用符・末尾ピリオドまで含む)。4 件とも `Captured` へ昇格し、出典行は
//! 関数の開始行ではなく**文言そのものの行**へピン留め実測で訂正してある
//! (`docs/specs/research/golden-3c3146cf-lib.md` §11-B)。

#![forbid(unsafe_code)]

/// 採取状態 (ADR 0002 決定 1)。フェーズ A 完了条件は契約経路の全数 `Captured`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoldenStatus {
    /// stage-0 環境の実出力で確認済み。
    Captured,
    /// as-built 仕様の逐語引用が根拠 (ゴールデン採取待ち)。
    SpecQuotedOnly,
}

/// 状態ファイル関連の逐語文言。
pub mod state {
    use super::GoldenStatus;

    /// 出典: `aidlc-lib.ts:6572` (upstream 03 §5.3)。ピン留めソース `3c3146cf` で逐語確認済み
    /// (`:6564` は `setFieldStrict` の関数開始行であって文言の行ではなかった — 実測で訂正)。
    pub const FIELD_NOT_FOUND_STATUS: GoldenStatus = GoldenStatus::Captured;

    /// `setFieldStrict` の拒否文言 — 「無言 no-op は検出不能なドリフト」の強制。
    #[must_use]
    pub fn field_not_found(field: &str) -> String {
        format!(
            "Field not found in state file: \"{field}\". Cannot update — refusing to silently no-op."
        )
    }

    /// 出典: `aidlc-lib.ts:6456` (upstream 03 §5.6)。ピン留めソース `3c3146cf` で逐語確認済み
    /// (`:6453` は `readStateFile` の関数開始行であって文言の行ではなかった — 実測で訂正)。
    pub const FILE_NOT_FOUND_STATUS: GoldenStatus = GoldenStatus::Captured;

    /// `readStateFile` の不在時文言。
    #[must_use]
    pub fn file_not_found(path: &str) -> String {
        format!("State file not found: {path}")
    }
}

/// 監査ロック関連の逐語文言。
pub mod lock {
    use super::GoldenStatus;

    /// 出典: `aidlc-audit.ts:543` (upstream 03 §6.8)。ピン留めソース `3c3146cf` で逐語確認済み。
    pub const ACQUIRE_FAILED_STATUS: GoldenStatus = GoldenStatus::Captured;

    /// acquire 予算超過の呼出側翻訳文言 (`acquireAuditLock` は `false` を返し、呼出側が
    /// この文言へ翻訳する — 11-workspace §4 の `AcquireError::Exhausted` に対応)。
    ///
    /// upstream ではこの文言が `aidlc-audit.ts` の 4 箇所 (`:543`, `:782`, `:897`, `:1150`)
    /// に現れ、うち `:897` / `:1150` は throw ではなく `jsonError(...)` — **例外経路と JSON
    /// エンベロープ経路で同一文言**という契約。
    #[must_use]
    pub const fn acquire_failed() -> &'static str {
        "Failed to acquire audit lock after retries"
    }

    /// 出典: `aidlc-lib.ts:7599` (`withAuditLock`)。ピン留めソース `3c3146cf` で逐語確認済み。
    pub const ACQUIRE_FAILED_FOR_KEY_STATUS: GoldenStatus = GoldenStatus::Captured;

    /// `withAuditLock` の acquire 失敗文言 — `acquire_failed()` とは**別系統**で、ロック
    /// identity (key) を埋め込む。カタログにこの形が無いと `withAuditLock` 経路の出力を
    /// 再現できない。
    ///
    /// upstream 逐語 (`aidlc-lib.ts:7599`):
    /// `` throw new Error(`Failed to acquire audit lock for ${key} after retries`); ``
    #[must_use]
    pub fn acquire_failed_for_key(key: &str) -> String {
        format!("Failed to acquire audit lock for {key} after retries")
    }

    /// 出典: `aidlc-audit.ts:1375` (`audit-merge`)。ピン留めソース `3c3146cf` で逐語確認済み。
    pub const MERGE_ACQUIRE_FAILED_STATUS: GoldenStatus = GoldenStatus::Captured;

    /// `audit-merge` の予算つき acquire 失敗文言 (`jsonError` 経路)。upstream 逐語
    /// (`aidlc-audit.ts:1375`):
    ///
    /// ```text
    /// `Failed to acquire audit lock after ${lockRetries} × ${lockRetryMs}ms = ${(lockRetries * lockRetryMs / 1000).toFixed(1)}s retries; another merge in flight?`
    /// ```
    ///
    /// 区切りの `×` は U+00D7 (MULTIPLICATION SIGN)。秒数は `Number.prototype.toFixed(1)`
    /// 相当の丸め (私有ヘルパ `to_fixed_1` を参照)。
    #[must_use]
    pub fn merge_acquire_failed(lock_retries: u32, lock_retry_ms: u32) -> String {
        let seconds = to_fixed_1(f64::from(lock_retries) * f64::from(lock_retry_ms) / 1000.0);
        format!(
            "Failed to acquire audit lock after {lock_retries} × {lock_retry_ms}ms = {seconds}s retries; another merge in flight?"
        )
    }

    /// `Number.prototype.toFixed(1)` 相当の 1 桁固定小数。
    ///
    /// ECMA-262 の `toFixed` は「`n/10 - x` を最小にする整数 `n`。同値の `n` が 2 つあるなら
    /// **大きい方**」= タイをゼロから遠い側へ丸める。Rust の `{:.1}` は最近接**偶数**丸めなので、
    /// 2 進で厳密なちょうど中間 (= 奇数分子の 1/4 — `0.25` / `0.75` / `1.25` …) のときだけ
    /// 結果が食い違う。その場合のみ手で切り上げる。
    fn to_fixed_1(x: f64) -> String {
        let quarters = x * 4.0;
        let is_tie = quarters.fract() == 0.0 && (quarters / 2.0).fract() != 0.0;
        if is_tie {
            let away = (x.abs() * 10.0).floor() + 1.0;
            return format!("{:.1}", away.copysign(x) / 10.0);
        }
        format!("{x:.1}")
    }
}

/// bolt / autonomy 関連の逐語文言。
pub mod bolt {
    use super::GoldenStatus;

    /// 出典: `aidlc-bolt.ts:808` (upstream 09 §5.6)。ピン留めソース `3c3146cf` で逐語確認済み。
    pub const INVALID_MODE_STATUS: GoldenStatus = GoldenStatus::Captured;

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
    fn acquire_failed_for_key_is_verbatim() {
        // aidlc-lib.ts:7599 — acquire_failed() とは別系統 (key を含む)
        assert_eq!(
            super::lock::acquire_failed_for_key("/ws\u{0}__workspace__"),
            "Failed to acquire audit lock for /ws\u{0}__workspace__ after retries"
        );
        assert_ne!(
            super::lock::acquire_failed_for_key("k"),
            super::lock::acquire_failed()
        );
    }

    #[test]
    fn merge_acquire_failed_is_verbatim_including_the_multiplication_sign() {
        // aidlc-audit.ts:1375 (既定値 50 × 100ms = 5.0s)
        assert_eq!(
            super::lock::merge_acquire_failed(50, 100),
            "Failed to acquire audit lock after 50 \u{00d7} 100ms = 5.0s retries; another merge in flight?"
        );
        assert!(
            super::lock::merge_acquire_failed(50, 100).contains(" \u{00d7} "),
            "区切りは U+00D7 (MULTIPLICATION SIGN) であり ASCII の 'x' ではない"
        );
    }

    #[test]
    fn merge_acquire_failed_rounds_seconds_like_js_to_fixed_1() {
        // 厳密なちょうど中間 (0.25 = 50 × 5ms) は JS `toFixed` の規約どおり切り上げ
        // (Rust の `{:.1}` の偶数丸めなら "0.2" になってしまう)。
        assert!(super::lock::merge_acquire_failed(50, 5).contains("= 0.3s retries"));
        assert!(super::lock::merge_acquire_failed(3, 250).contains("= 0.8s retries")); // 0.75
        // 二進で厳密に表せない 0.15 は double が 0.15 未満なので切り捨て (JS と同じ)。
        assert!(super::lock::merge_acquire_failed(3, 50).contains("= 0.1s retries"));
        assert!(super::lock::merge_acquire_failed(0, 100).contains("= 0.0s retries"));
    }

    #[test]
    fn field_not_found_is_verbatim() {
        assert_eq!(
            super::state::field_not_found("Construction Autonomy Mode"),
            "Field not found in state file: \"Construction Autonomy Mode\". Cannot update — refusing to silently no-op."
        );
    }
}
