//! このクローンを名指すトークン — 監査シャード名の接尾辞（`aidlc/.aidlc-clone-id`）。
//!
//! **マシンローカルで、`.gitignore` 済みでなければならない。** コミットすると、その
//! コミットから作られた全クローンが同じシャードを共有し、監査行が毎回 git 衝突する
//! （`.gitignore` の逐語コメント: 「it MUST stay machine-local (gitignored) or every clone
//! from a commit would share a shard and git-conflict」）。
//!
//! 無ければ鋳造して置く。鍵（[`crate::steering`]）と同じ「マシンローカルな遅延鋳造」だが、
//! **秘密ではない**ので `secret_file` の 0600・排他の作法は要らない。要るのは
//! 「一度決まったら変わらない」ことだけである。

use std::fs;
use std::path::Path;

use core_command_domain::workspace::CloneId;
use core_infrastructure::atomic::write_file_atomic;

/// トークンのファイル名。
const CLONE_ID_FILE: &str = ".aidlc-clone-id";

/// `aidlc/` 配下のトークンを読み、無ければ鋳造して置く。
///
/// 鋳造した直後に**読み戻してから**返す。同時に 2 つのプロセスが鋳造しても、両方が
/// ディスク上の勝者を読むので、以後の監査シャードは 1 つに収束する。
///
/// # Errors
///
/// ディレクトリを作れない・書けない・読めない場合の I/O エラー、および読み戻した値が
/// トークンの文法に合わない場合（`InvalidData`）。
pub fn load_or_mint(aidlc_root: &Path) -> std::io::Result<CloneId> {
    let path = aidlc_root.join(CLONE_ID_FILE);
    if let Some(id) = read(&path) {
        return Ok(id);
    }
    fs::create_dir_all(aidlc_root)?;
    write_file_atomic(&path, format!("{}\n", mint()).as_bytes())?;
    // 競争に負けていても勝者の値を読む — シャードは 1 つに収束する。
    read(&path).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "the clone id on disk is not a valid token",
        )
    })
}

fn read(path: &Path) -> Option<CloneId> {
    CloneId::parse(fs::read_to_string(path).ok()?.trim()).ok()
}

/// 32 桁の 16 進（`[a-f0-9]` — `CloneId` の `[a-z0-9]` に収まる）。
///
/// v7 を使うのは採番済みの feature がそれだけだからで、時刻成分に意味は持たせていない。
fn mint() -> String {
    uuid::Uuid::now_v7().simple().to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn root() -> tempfile::TempDir {
        tempfile::tempdir().expect("一時ディレクトリ")
    }

    #[test]
    fn minting_creates_a_token_file_and_returns_a_valid_token() {
        let dir = root();
        let aidlc = dir.path().join("aidlc");

        let id = load_or_mint(&aidlc).expect("鋳造できる");

        assert!(aidlc.join(CLONE_ID_FILE).exists());
        assert_eq!(id.as_str().len(), 32);
        assert!(id.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// 一度決まったら変わらない — これが監査シャードが 1 本に留まる条件である。
    #[test]
    fn the_token_is_stable_across_calls() {
        let dir = root();
        let aidlc = dir.path().join("aidlc");

        let first = load_or_mint(&aidlc).expect("鋳造できる");
        let second = load_or_mint(&aidlc).expect("読み直せる");

        assert_eq!(first, second);
    }

    #[test]
    fn an_existing_token_is_read_rather_than_replaced() {
        let dir = root();
        let aidlc = dir.path().join("aidlc");
        fs::create_dir_all(&aidlc).expect("aidlc");
        fs::write(aidlc.join(CLONE_ID_FILE), "abc123\n").expect("既存トークン");

        assert_eq!(load_or_mint(&aidlc).expect("読める").as_str(), "abc123");
    }

    /// 文法に合わない中身は鋳造し直す（大文字や記号は `CloneId` に存在しない）。
    #[test]
    fn a_malformed_token_is_replaced_by_a_fresh_one() {
        let dir = root();
        let aidlc = dir.path().join("aidlc");
        fs::create_dir_all(&aidlc).expect("aidlc");
        fs::write(aidlc.join(CLONE_ID_FILE), "NOT A TOKEN\n").expect("壊れたトークン");

        let id = load_or_mint(&aidlc).expect("鋳造し直せる");

        assert_eq!(id.as_str().len(), 32);
    }
}
