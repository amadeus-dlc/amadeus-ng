//! `LockIdentity` — `<realpath>\x00<space>\x00<intent>`。intent 省略時は
//! **`<realpath>\x00__workspace__` の 2 成分形 (space 成分ごと落ちる)** — 全 space の
//! `intents.json` 変更が単一バケットに集約される。keying に `activeIntent()` を使わない
//! (並行 birth の二重化防止 — upstream `aidlc-lib.ts:6757-6799`, 03 §6.8, 11-workspace §2.2)。
//! ロック dir 名は md5(identity) 先頭 8 文字から導出されるため、このバイト列は
//! stage-0/1 併用期の相互排他互換に直結する。

use super::space_name::SpaceName;

/// workspace センチネルロックの第 2 成分。space 名の位置に置かれる語であり、これにより
/// 全 space の `intents.json` 変更が単一バケットに集約される。
pub const WORKSPACE_LOCK_SENTINEL: &str = "__workspace__";

/// NUL 区切りのロック識別バイト列。ロック dir 名 (md5 先頭 8 文字) の唯一の材料であり、
/// 2 つの構成関数以外からは作れない (W13 の E1 装置)。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LockIdentity(String);

impl LockIdentity {
    /// intent スコープのロック識別子 (3 成分)。
    #[must_use]
    pub fn for_intent(
        realpath_project_dir: &str,
        space: &SpaceName,
        intent_dir_name: &str,
    ) -> LockIdentity {
        LockIdentity(format!(
            "{realpath_project_dir}\u{0}{}\u{0}{intent_dir_name}",
            space.as_str()
        ))
    }

    /// workspace センチネルロック (2 成分 — space 成分は落ちる)。
    /// `intents.json` の全変更はこのバケットを取る。
    #[must_use]
    pub fn for_workspace(realpath_project_dir: &str) -> LockIdentity {
        LockIdentity(format!(
            "{realpath_project_dir}\u{0}{WORKSPACE_LOCK_SENTINEL}"
        ))
    }

    /// md5 に食わせる識別バイト列そのもの。この並びが 1 バイトでも変われば別バケットになり、
    /// stage-0/1 併用期の相互排他が壊れる。
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_sentinel_is_two_components_and_drops_the_space() {
        let id = LockIdentity::for_workspace("/repo");
        assert_eq!(id.as_bytes(), b"/repo\x00__workspace__");
    }

    #[test]
    fn intent_identity_is_three_components() {
        let space = SpaceName::parse("team-a").unwrap();
        let id = LockIdentity::for_intent("/repo", &space, "260822-auth");
        assert_eq!(id.as_bytes(), b"/repo\x00team-a\x00260822-auth");
    }

    #[test]
    fn sentinel_bucket_is_shared_across_spaces() {
        // space が違っても workspace ロックは同一バケット (二重 birth 防止の要)
        let a = LockIdentity::for_workspace("/repo");
        let b = LockIdentity::for_workspace("/repo");
        assert_eq!(a, b);
    }
}
