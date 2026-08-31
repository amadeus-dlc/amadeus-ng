//! このクローンを名指すトークン — 監査シャード名の接尾辞（`aidlc/.aidlc-clone-id`）。
//!
//! **マシンローカルで、`.gitignore` 済みでなければならない。** コミットすると、その
//! コミットから作られた全クローンが同じシャードを共有し、監査行が毎回 git 衝突する
//! （`.gitignore` の逐語コメント: 「it MUST stay machine-local (gitignored) or every clone
//! from a commit would share a shard and git-conflict」）。
//!
//! 無ければ鋳造して置く。鍵（[`crate::steering`]）と同じ「マシンローカルな遅延鋳造」で、
//! **秘密ではない**ので 0600 は要らないが、**排他は要る** — 要件は「一度決まったら
//! 変わらない」ことであり、上書きを許す書込ではそれが崩れるからである。
//!
//! # なぜ atomic な上書きでは足りないか
//!
//! `write_file_atomic` の rename は既存の名前を**上書きする**。同時に 2 つが鋳造すると、
//! 先に読み戻して返った側の値を後から別の値が置き換えられるので、同じ起動の中で監査
//! シャードが 2 本に割れる。名前を主張できるのは 1 つだけ、という排他が要る。
//!
//! # なぜ `secret_file` と機構を共有しないか
//!
//! 共通なのは「中身を揃えてから名前を主張する」という骨だけで、
//! [`core_infrastructure::secret_file`] 側はそこに 0600 と「一時ファイル名に秘密を
//! 漏らさない」という秘密固有の関心を重ねている。1 つの汎用 API に畳むと、その関心が
//! 秘密でない利用者の側へ滲む。消費者が 2 つの段階では早すぎる抽象化と判断し、ここには
//! 12 行の最小実装を置く。3 つ目が現れたら `core_infrastructure::atomic` へ
//! `claim_new_file` として括り出すこと。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use core_command_domain::workspace::CloneId;

/// トークンのファイル名。
const CLONE_ID_FILE: &str = ".aidlc-clone-id";

/// `aidlc/` 配下のトークンを読み、無ければ鋳造して置く。
///
/// 鋳造は**名前の排他的な主張**で行い、鋳造した直後に読み戻してから返す。競争に負けた側は
/// 勝者の値を読むので、同時に何個が走っても監査シャードは 1 つに収束する。
///
/// # Errors
///
/// ディレクトリを作れない・書けない場合の I/O エラー、および読み戻した値がトークンの
/// 文法に合わない場合（`InvalidData`）。
pub fn load_or_mint(aidlc_root: &Path) -> io::Result<CloneId> {
    let path = aidlc_root.join(CLONE_ID_FILE);
    match observe(&path) {
        Observed::Valid(id) => return Ok(id),
        // **不在と「壊れている」を混ぜてはいけない。** 「名前が埋まっていれば壊れている」と
        // 見なして削除すると、同時に鋳造している相手が張ったばかりの正しいトークンを消して
        // しまう（実測で割れた）。削除してよいのは、読んで壊れていた場合だけである。
        Observed::Malformed => {
            let _ = fs::remove_file(&path);
        }
        Observed::Absent => {}
    }
    fs::create_dir_all(aidlc_root)?;
    claim(&path, &mint())?;
    // 競争に負けていても勝者の値を読む — シャードは 1 つに収束する。
    read(&path).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "the clone id on disk is not a valid token",
        )
    })
}

/// ディスク上のトークンの観測。**不在**（まだ誰も鋳造していない）と**壊れている**
/// （鋳造し直しが要る）を混ぜないための 3 値である。
enum Observed {
    /// 読めてトークンとして成立した。
    Valid(CloneId),
    /// 在るが読めない・文法に合わない。
    Malformed,
    /// まだ無い。
    Absent,
}

fn observe(path: &Path) -> Observed {
    match fs::read_to_string(path) {
        Ok(text) => CloneId::parse(text.trim()).map_or(Observed::Malformed, Observed::Valid),
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => Observed::Absent,
        // 在るのに読めない（権限など）— 鋳造し直して名前を取り戻す。
        Err(_) => Observed::Malformed,
    }
}

/// 名前を排他的に主張する。**負けは失敗ではない**（勝者の値を読めばよい）ので `Ok` を返す。
///
/// 一時ファイルへ中身を書き切ってから `hard_link` で名前を張る。`hard_link` は既存の名前に
/// 対して `AlreadyExists` で失敗するので `O_EXCL` と同じ排他性を持ち、しかも**リンクが
/// 見えた時点で中身は完成している**（`create_new` してから書くと、その間に空のトークンが
/// 読まれうる）。
fn claim(path: &Path, token: &str) -> io::Result<()> {
    let staged = staging_path(path, token);
    let outcome =
        fs::write(&staged, format!("{token}\n")).and_then(|()| fs::hard_link(&staged, path));
    let _ = fs::remove_file(&staged);
    match outcome {
        Ok(()) => Ok(()),
        // 名前は既に他の鋳造者のものである（その中身は完成している）。
        Err(cause) if cause.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(cause) => Err(cause),
    }
}

/// 鋳造中の一時ファイル（`hard_link` は同一ファイルシステム内でしか張れないので同じ
/// ディレクトリに置く）。接尾辞に鋳造値そのものを使う — 秘密ではないので隠す必要がなく、
/// 一意なので同一プロセス内の並行鋳造ともぶつからない。
fn staging_path(path: &Path, token: &str) -> PathBuf {
    let staged = format!(".{CLONE_ID_FILE}.mint-{token}");
    path.parent()
        .map_or_else(|| PathBuf::from(&staged), |parent| parent.join(&staged))
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

    /// **同時に鋳造しても 1 つの値へ収束する** — 上書き書込だとここで割れる。
    ///
    /// 割れたトークンは監査シャードを 2 本に分ける（`<host>-<cloneId>.md` の接尾辞が
    /// 食い違う）ので、これは「速いか」ではなく「証跡が 1 本か」のテストである。
    #[test]
    fn concurrent_minting_converges_on_one_token() {
        let dir = root();
        let aidlc = dir.path().join("aidlc");
        fs::create_dir_all(&aidlc).expect("aidlc");

        let minted: std::collections::BTreeSet<String> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| scope.spawn(|| load_or_mint(&aidlc).expect("鋳造できる")))
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("スレッド"))
                .map(|id| id.as_str().to_string())
                .collect()
        });

        assert_eq!(minted.len(), 1, "鋳造が割れた: {minted:?}");
    }

    /// 名前をディレクトリが塞いでいると鋳造は成立しない — 「読めない」を不在と混ぜず、
    /// かつ主張もできないので、`InvalidData` で**素直に失敗する**（黙って別名へ逃げない）。
    #[test]
    fn a_name_taken_by_a_directory_fails_loudly() {
        let dir = root();
        let aidlc = dir.path().join("aidlc");
        fs::create_dir_all(aidlc.join(CLONE_ID_FILE)).expect("同名のディレクトリ");

        let error = load_or_mint(&aidlc).expect_err("鋳造できない");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("not a valid token"), "{error}");
    }

    /// 親が書けなければ主張の失敗を**そのまま返す**（負け＝`AlreadyExists` とは区別する）。
    #[cfg(unix)]
    #[test]
    fn a_read_only_parent_surfaces_the_write_failure() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = root();
        let aidlc = dir.path().join("aidlc");
        fs::create_dir_all(&aidlc).expect("aidlc");
        fs::set_permissions(&aidlc, fs::Permissions::from_mode(0o555)).expect("読取専用へ");

        let error = load_or_mint(&aidlc).expect_err("書けない");

        fs::set_permissions(&aidlc, fs::Permissions::from_mode(0o755)).expect("戻す");
        assert_ne!(error.kind(), io::ErrorKind::AlreadyExists, "{error}");
        assert_ne!(error.kind(), io::ErrorKind::InvalidData, "{error}");
    }

    /// 鋳造の一時ファイルは残らない（`aidlc/` に見えるのはトークン 1 本だけ）。
    #[test]
    fn minting_leaves_no_staging_file_behind() {
        let dir = root();
        let aidlc = dir.path().join("aidlc");

        load_or_mint(&aidlc).expect("鋳造できる");

        let entries: Vec<String> = fs::read_dir(&aidlc)
            .expect("読める")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(entries, vec![CLONE_ID_FILE.to_string()], "{entries:?}");
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
