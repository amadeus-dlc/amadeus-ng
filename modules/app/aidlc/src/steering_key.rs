//! 継続トークンの封緘鍵 — **置き場と鋳造方針**（機構は
//! [`core_infrastructure::secret_file`]）。
//!
//! 鍵の作り方（32 バイト・base64url・0600・排他）は言語拡張であって方針を持たない。
//! ここが持つのは upstream `aidlc-orchestrate.ts:2286-2352` が定める 2 つの方針である。
//!
//! # 方針 1: どこに置くか
//!
//! 状態ファイルがあれば**その隣**（= record ディレクトリ直下）、無ければクローンローカルの
//! `aidlc/.aidlc-sessions/` に置く（upstream `steeringTokenKeyPath`）。intent が生まれる前は
//! record が存在しないので、後者が受け皿になる。
//!
//! どちらも `.gitignore` 済みである（`aidlc/spaces/*/intents/*/.aidlc-*` と
//! `aidlc/.aidlc-sessions/`）。鍵はマシンローカルなランタイム状態であって共有された仕事では
//! ないので、コミットされてはならない。
//!
//! # 方針 2: いつ鋳造してよいか
//!
//! **`next` は鋳造し、`continue` は読むだけである**（upstream の `encodeSteeringToken` は
//! `steeringTokenKey(projectDir, true)`、`decodeSteeringToken` は `false` を渡す）。
//!
//! これは I8（`next` は読み取り専用）の**明示された 2 つの例外の 1 つ**であり、鍵の鋳造は
//! ワークフロー状態を変えないので読み取り専用性を破らない。逆に `continue` が鋳造できて
//! しまうと、鍵を失った継続が**新しい鍵で新しい封緘を検証してしまい**、fail-closed
//! （I12「fresh `next` からやり直し」）が成立しなくなる。だから読むだけなのである。

use std::path::Path;

use core_infrastructure::secret_file::{SecretFile, SecretFileError};

/// 鍵の長さ（upstream `STEERING_TOKEN_KEY_BYTES`）。
const KEY_BYTES: usize = 32;

/// 鍵ファイルの名前（upstream `STEERING_TOKEN_KEY_FILE`）。
const KEY_FILE: &str = ".aidlc-steering-token-key";

/// 状態ファイルの名前（置き場の判定に使う）。
const STATE_FILE: &str = "aidlc-state.md";

/// 鍵の置き場を決めて読み書きする、合成ルートの方針オブジェクト。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteeringKey {
    secret: SecretFile,
}

impl SteeringKey {
    /// ワークスペース根と（あれば）record ディレクトリから置き場を決める。
    ///
    /// `record_dir` を渡してもそこに状態ファイルが無ければセッションランタイム側へ倒す —
    /// upstream の判定が「`existsSync(stateFilePath(projectDir))`」であり、ディレクトリの
    /// 存在ではなく**状態ファイルの存在**を見ているためである。
    #[must_use]
    pub fn resolve(project_dir: &Path, record_dir: Option<&Path>) -> SteeringKey {
        let path = match record_dir {
            Some(record) if record.join(STATE_FILE).exists() => record.join(KEY_FILE),
            _ => project_dir
                .join("aidlc")
                .join(".aidlc-sessions")
                .join(KEY_FILE),
        };
        SteeringKey {
            secret: SecretFile::new(path, KEY_BYTES),
        }
    }

    /// 保管先（診断・テスト用）。
    #[must_use]
    pub fn path(&self) -> &Path {
        self.secret.path()
    }

    /// **`next` 用** — 鍵を読み、無ければ鋳造する。
    ///
    /// # Errors
    ///
    /// 読取・鋳造の失敗（[`SecretFileError`]）。
    pub fn mint_for_next(&self) -> Result<Vec<u8>, SecretFileError> {
        self.secret.load_or_mint()
    }

    /// **`continue` 用** — 鍵を読むだけ。無ければ `Ok(None)`。
    ///
    /// 呼出側は `None` を「この継続は検証できない」= fail-closed として扱う（I12）。
    ///
    /// # Errors
    ///
    /// 読取の失敗（[`SecretFileError`]）。
    pub fn read_for_continue(&self) -> Result<Option<Vec<u8>>, SecretFileError> {
        self.secret.read()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use std::fs;

    fn workspace() -> tempfile::TempDir {
        tempfile::tempdir().expect("一時ディレクトリ")
    }

    #[test]
    fn without_a_state_file_the_key_lives_in_the_clone_local_session_runtime() {
        let root = workspace();
        let record = root.path().join("record");
        fs::create_dir_all(&record).expect("record ディレクトリ");

        let key = SteeringKey::resolve(root.path(), Some(&record));

        assert_eq!(
            key.path(),
            root.path()
                .join("aidlc")
                .join(".aidlc-sessions")
                .join(KEY_FILE)
        );
    }

    #[test]
    fn with_a_state_file_the_key_lives_beside_it_in_the_record_dir() {
        let root = workspace();
        let record = root.path().join("record");
        fs::create_dir_all(&record).expect("record ディレクトリ");
        fs::write(record.join(STATE_FILE), "# state").expect("状態ファイル");

        let key = SteeringKey::resolve(root.path(), Some(&record));

        assert_eq!(key.path(), record.join(KEY_FILE));
    }

    #[test]
    fn with_no_record_at_all_the_key_lives_in_the_session_runtime() {
        let root = workspace();

        let key = SteeringKey::resolve(root.path(), None);

        assert_eq!(
            key.path(),
            root.path()
                .join("aidlc")
                .join(".aidlc-sessions")
                .join(KEY_FILE)
        );
    }

    /// `next` は鋳造し、`continue` は読むだけ。
    ///
    /// これが逆になると、鍵を失った継続が新しい鍵で新しい封緘を検証してしまい、I12 の
    /// fail-closed が成立しなくなる。
    #[test]
    fn continue_never_mints_but_next_does() {
        let root = workspace();
        let key = SteeringKey::resolve(root.path(), None);

        assert!(
            key.read_for_continue().expect("読める").is_none(),
            "まだ鍵は無い"
        );
        assert!(!key.path().exists(), "読むだけの経路は鋳造しない");

        let minted = key.mint_for_next().expect("鋳造できる");

        assert_eq!(minted.len(), KEY_BYTES);
        assert_eq!(
            key.read_for_continue().expect("読める"),
            Some(minted),
            "鋳造後は continue も同じ鍵を読む"
        );
    }

    /// 同じワークスペースで繰り返し `next` を叩いても鍵は変わらない。
    ///
    /// これがプロセスをまたいだ分割配信（`next` → load-steering → `continue`）が成立する
    /// 条件である — 別プロセスの `continue` は同じ鍵を読み直して封緘を検証する。
    #[test]
    fn the_key_is_stable_across_resolutions_of_the_same_workspace() {
        let root = workspace();

        let first = SteeringKey::resolve(root.path(), None)
            .mint_for_next()
            .expect("鋳造できる");
        // 別プロセスに相当する 2 回目の解決。
        let second = SteeringKey::resolve(root.path(), None)
            .mint_for_next()
            .expect("読み直せる");

        assert_eq!(first, second);
    }
}
