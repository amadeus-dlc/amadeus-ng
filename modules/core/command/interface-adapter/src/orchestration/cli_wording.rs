//! CLI 境界の**利用者向け逐語文言** — upstream 互換の綴りをバイト単位で固定する。
//!
//! 2026-08-29 オーナー裁定により独立クレート `modules/shared/message-catalog` を解体し、
//! 文言は**出す側（境界）に同居**させた。ドメインは材料（与えられた値）だけを返し
//! （`InvalidModeArg::given`）、文言はここで組む（error-handling.md）。
//! 実配線（`set-autonomy --mode` の CLI 経路）は U7 ディスパッチャで行う。

use core_command_domain::orchestration::InvalidModeArg;

/// `set-autonomy --mode` の不正値拒否 (CLI 引数境界は 2 値厳密パース — 10 §2.2)。
///
/// 出典: `aidlc-bolt.ts:808` (upstream 09 §5.6)。ピン留めソース `3c3146cf` で逐語確認済み
/// （採取状態: Captured — 2026-08-22 のゴールデン採取で 4/4 バイト一致）。
#[must_use]
pub fn invalid_mode_message(rejected: &InvalidModeArg) -> String {
    format!(
        "Invalid --mode: {}. Must be 'autonomous' or 'gated'.",
        rejected.given()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_command_domain::orchestration::AutonomyMode;

    #[test]
    fn invalid_mode_message_is_verbatim() {
        let err = AutonomyMode::parse("turbo").expect_err("2 値以外は拒否");
        assert_eq!(
            invalid_mode_message(&err),
            "Invalid --mode: turbo. Must be 'autonomous' or 'gated'."
        );
    }
}
