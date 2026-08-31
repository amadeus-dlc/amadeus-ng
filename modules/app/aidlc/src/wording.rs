//! 逐語文言 — **出す側が組む**（`coding-rules/error-handling.md`）。
//!
//! ドメインもポートも材料しか運ばない。利用者が読む文字列を組み立てるのはここだけである。
//!
//! # 逐語は 1 バイトも変えない
//!
//! ここの文字列は upstream の観測可能な契約（Published Language）であり、綴りが変われば
//! 互換が壊れる（`coding-rules/ubiquitous-language.md`「外に出る値は逐語で維持する」）。
//! 各定数の doc に upstream の出典を `ファイル:行` で書いてあるので、疑わしいときは
//! そちらを正とすること。

/// 未知サブコマンド（upstream `aidlc-orchestrate.ts:6155`）。
///
/// 引数が無いときの `(none)` まで含めて逐語である。
#[must_use]
pub fn unknown_orchestrate_subcommand(given: Option<&str>) -> String {
    format!(
        "Unknown subcommand: {}. Valid: next, continue, report, park",
        given.unwrap_or("(none)")
    )
}

/// 上限超過の emit 拒否（upstream `aidlc-orchestrate.ts:266`）。
///
/// upstream は定数 `DIRECTIVE_MAX_BYTES` を埋め込むので、こちらも同じ値を描く。
#[must_use]
pub fn refusing_oversize_directive(cap: usize) -> String {
    format!("aidlc-orchestrate: refusing to emit a directive larger than {cap} bytes")
}

/// 未捕捉の失敗（upstream `aidlc-orchestrate.ts:6167`）。
#[must_use]
pub fn orchestrate_failure(detail: &str) -> String {
    format!("aidlc-orchestrate: {detail}")
}

/// 継続トークンが検証できない（upstream `aidlc-orchestrate.ts:5999`）。
///
/// トークンの不正・鍵の不在・引数の個数違いを**区別しない** — fail-closed の指示は
/// どの原因でも同じ「fresh `next` からやり直せ」だからである（I12）。
pub const INVALID_CONTINUATION_TOKEN: &str = "Invalid steering continuation token: this stage's rules cannot be loaded from where they left off. Run a fresh `next` to restart delivery from part 1.";

/// 鍵ファイルが壊れている（upstream `aidlc-orchestrate.ts:2323`）。
#[must_use]
pub fn corrupt_key_file(path: &str) -> String {
    format!(
        "The local key file at \"{path}\" is corrupt, so this stage's rules cannot be loaded safely. \
         Delete that file and run a fresh `next`; a replacement is created automatically."
    )
}

/// 鍵ファイルが読めない（upstream `aidlc-orchestrate.ts:2331`）。
#[must_use]
pub fn unreadable_key_file(path: &str, cause: &str) -> String {
    format!(
        "Cannot read the local key file at \"{path}\", so this stage's rules cannot be loaded ({cause})."
    )
}

/// 鍵ファイルが作れない（upstream `aidlc-orchestrate.ts:2350`）。
#[must_use]
pub fn uncreatable_key_file(path: &str, cause: &str) -> String {
    format!(
        "Cannot create the local key file at \"{path}\", so this stage's rules cannot be loaded \
         ({cause}). Fix the directory permissions, then run a fresh `next`."
    )
}

/// 受理されない `--result`（upstream `aidlc-orchestrate.ts:5528`）。
#[must_use]
pub fn unknown_result(given: &str) -> String {
    format!(
        "Unknown --result \"{given}\". accepted outcomes: {}.",
        core_command_domain::orchestration::ACCEPTED_RESULTS.join(", ")
    )
}

/// 遷移が拒否された（upstream `aidlc-state.ts` 由来の拒否をエンジンが中継する形）。
#[must_use]
pub fn transition_rejected(detail: &str) -> String {
    format!("Transition rejected: {detail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 引数が無いときは `(none)` を描く（upstream の `?? "(none)"`）。
    #[test]
    fn the_unknown_subcommand_line_names_the_given_verb_or_none() {
        assert_eq!(
            unknown_orchestrate_subcommand(Some("frobnicate")),
            "Unknown subcommand: frobnicate. Valid: next, continue, report, park"
        );
        assert_eq!(
            unknown_orchestrate_subcommand(None),
            "Unknown subcommand: (none). Valid: next, continue, report, park"
        );
    }

    #[test]
    fn the_oversize_refusal_names_the_cap_in_bytes() {
        assert_eq!(
            refusing_oversize_directive(28 * 1024),
            "aidlc-orchestrate: refusing to emit a directive larger than 28672 bytes"
        );
    }

    #[test]
    fn the_failure_line_is_prefixed_with_the_tool_name() {
        assert_eq!(
            orchestrate_failure("missing graph"),
            "aidlc-orchestrate: missing graph"
        );
    }

    /// 鍵の 3 形は path を二重引用符で囲む（upstream の `"${path}"`）。
    #[test]
    fn the_key_file_wordings_quote_the_path() {
        assert!(
            corrupt_key_file("/tmp/k").starts_with("The local key file at \"/tmp/k\" is corrupt")
        );
        assert!(unreadable_key_file("/tmp/k", "EACCES").contains("\"/tmp/k\""));
        assert!(uncreatable_key_file("/tmp/k", "EACCES").contains("\"/tmp/k\""));
    }

    #[test]
    fn the_key_file_wordings_end_with_their_recovery_instruction() {
        assert!(corrupt_key_file("/tmp/k").ends_with("a replacement is created automatically."));
        assert!(unreadable_key_file("/tmp/k", "EACCES").ends_with("(EACCES)."));
        assert!(
            uncreatable_key_file("/tmp/k", "EACCES")
                .ends_with("Fix the directory permissions, then run a fresh `next`.")
        );
    }
}
