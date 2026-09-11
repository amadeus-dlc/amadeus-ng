//! 状態ファイル関連の**利用者向け逐語文言** — upstream 互換の綴りをバイト単位で固定する。
//!
//! 2026-08-29 オーナー裁定により独立クレート `modules/shared/message-catalog` を解体し、
//! 文言は**出す側に同居**させた（error-handling.md「利用者向け文言はアダプタ層」——
//! 状態ファイルを描くのは RMU なので、その文言は RMU が持つ）。各関数の出典
//! （固定コミット `a277af21` の upstream 関数名）と採取状態は doc コメントに随伴する。

/// `setFieldStrict` の拒否文言 — 「無言 no-op は検出不能なドリフト」の強制。
///
/// 出典: `aidlc-lib.ts` `setFieldStrict` (@a277af21)。採取状態: Captured — 2.7.1 コーパス
/// `cli/set-autonomy/state-field-absent/stderr` と逐語一致。
#[must_use]
pub(crate) fn field_not_found_message(field: &str) -> String {
    format!(
        "Field not found in state file: \"{field}\". Cannot update — refusing to silently no-op."
    )
}

/// `readStateFile` の不在時文言。
///
/// 出典: `aidlc-lib.ts` `readStateFile` (@a277af21)。採取状態: ソース読みのみ — この文言を
/// 含む 2.7.1 の採取ケースは無い。
#[must_use]
pub(crate) fn file_not_found_message(path: &str) -> String {
    format!("State file not found: {path}")
}

#[cfg(test)]
mod tests {
    // テストコードでは unwrap / expect を許可 (オーナー規約)。
    use super::*;

    #[test]
    fn field_not_found_message_is_verbatim() {
        assert_eq!(
            field_not_found_message("Current Stage"),
            "Field not found in state file: \"Current Stage\". Cannot update — refusing to silently no-op."
        );
    }

    #[test]
    fn file_not_found_message_is_verbatim() {
        assert_eq!(
            file_not_found_message("/ws/aidlc-state.md"),
            "State file not found: /ws/aidlc-state.md"
        );
    }
}
