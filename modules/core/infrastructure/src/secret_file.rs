//! マシンローカルな秘密鍵ファイル — 乱数で鋳造し、base64url で保管する機構。
//!
//! **言語拡張であって方針を持たない**（`coding-rules/infrastructure-layer.md`）。ここが知るのは
//! 「N バイトの乱数を作り、base64url + 改行で 0600 のファイルへ排他的に置き、読むときは
//! 長さを検査する」という機構だけである。**どこに置くか**（record ディレクトリか
//! セッションランタイムか）と**いつ鋳造してよいか**（`next` は鋳造する / `continue` は読む
//! だけ）は呼出側＝合成ルートの方針であり、ここには無い。
//!
//! 鍵が何に使われるか（HMAC の鍵か、別の秘密か）も知らない。封緘そのものは
//! [`codec`](crate::codec) が持ち、そちらは I/O を持たない — 秘密の**保管**と**利用**を
//! 別のモジュールに分けてある。
//!
//! # 正準性は厳格デコーダが担保する（upstream との実装差）
//!
//! 同じ鍵に 2 通りの綴りが存在すると、綴りを鍵の同一性の根拠にしている面が静かに割れる。
//! upstream（Node）の `Buffer.from(text, "base64url")` は**寛容**で、パディング付きも
//! 非正準な末尾ビットも同じバイト列へデコードしてしまうため、`aidlc-orchestrate.ts:2313` は
//! 「再エンコードが原文と一致すること」を明示的に検査している。
//!
//! こちらの `URL_SAFE_NO_PAD` は**厳格**で、パディングも非正準な末尾ビットも decode の時点で
//! 拒否する（2026-08-31 に実測確認）。したがって往復検査は書いても到達不能な死んだ分岐に
//! なるので置いていない。**観測される結末は upstream と同じ**（どちらの綴りも `Corrupt`）で
//! あり、違うのは拒否が起きる位置だけである。この依存はテスト
//! `a_padded_spelling_is_rejected_by_the_strict_decoder` が固定する — エンジンや設定を
//! 寛容なものへ替えたらそこが落ちるので、そのときは往復検査を足すこと。
//!
//! `SecretFileError` は独立ファイルに 1 型 1 ファイルで置く（`one-public-type`）。
//! 子モジュール `secret_file::secret_file_error`（`secret_file/secret_file_error.rs`）として
//! 所有し、ここから `pub use` で再輸出する — 兄弟ファイルを跨いだ利便再エクスポートではなく、
//! 通常のファサード（`mod.rs` と同型の所有連鎖）である
//! (`coding-rules/module-visibility.md`)。`core_infrastructure::secret_file::{SecretFile,
//! SecretFileError}` というクレート境界越しの直接参照はこの形のまま変わらない。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::atomic::create_new_file;

mod secret_file_error;

pub use secret_file_error::SecretFileError;

/// 一時ファイル名の nonce の長さ（衝突しなければよいので短くてよい）。
const NONCE_LEN: usize = 8;

/// 決まった長さの秘密を base64url で保持するマシンローカルなファイル。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretFile {
    path: PathBuf,
    len: usize,
}

impl SecretFile {
    /// 置き場と鍵の長さ（バイト数）を据える。どちらも呼出側の方針である。
    #[must_use]
    pub const fn new(path: PathBuf, len: usize) -> SecretFile {
        SecretFile { path, len }
    }

    /// 保管先のパス。
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 鍵を**読むだけ**。無ければ `Ok(None)` を返し、鋳造はしない。
    ///
    /// 「無いこと」を `Err` にしないのは、鍵の不在が失敗ではなく**まだ鋳造されていない**と
    /// いう正当な状態だからである。それを受けてどうするか（fail-closed に倒すか鋳造するか）は
    /// 呼出側の方針である。
    ///
    /// # Errors
    ///
    /// ファイルはあるが読めない（`Unreadable`）、読めたが鍵として成立しない（`Corrupt` —
    /// 長さ違い、または非正準な base64url）。
    pub fn read(&self) -> Result<Option<Vec<u8>>, SecretFileError> {
        let encoded = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(cause) if cause.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(cause) => {
                return Err(SecretFileError::Unreadable {
                    path: self.path.clone(),
                    cause,
                });
            }
        };
        self.decode(encoded.trim()).map(Some)
    }

    /// 鍵を読み、無ければ**鋳造してから**返す（遅延鋳造）。
    ///
    /// 同じ瞬間に複数の書き手が鋳造しようとしても、名前を主張できるのは 1 つだけで、負けた
    /// 側は勝者の鍵を**読み直して収束する** — そうしないとプロセスごとに違う鍵を持ち、
    /// 互いの封緘を検証できなくなる。排他性の張り方は [`SecretFile::mint`] を参照。
    ///
    /// # Errors
    ///
    /// 読取の失敗（`Unreadable` / `Corrupt`）、鋳造の失敗（`Uncreatable` — 親ディレクトリの
    /// 権限・ディスクなど）。
    pub fn load_or_mint(&self) -> Result<Vec<u8>, SecretFileError> {
        if let Some(key) = self.read()? {
            return Ok(key);
        }
        match self.mint()? {
            Some(key) => Ok(key),
            // 競争に負けた — 勝者が名前を主張した時点で中身は完成しているので、読み直せば
            // 必ず勝者の完全な鍵が得られる（`mint` の doc「見えるなら完全」）。
            None => self.read()?.ok_or(SecretFileError::Corrupt {
                path: self.path.clone(),
            }),
        }
    }

    /// 綴りをバイト列へ戻す。
    ///
    /// 正準性はデコーダが担保するので、ここで見るのは長さだけである（モジュール doc の
    /// 「正準性は厳格デコーダが担保する」を参照）。
    fn decode(&self, encoded: &str) -> Result<Vec<u8>, SecretFileError> {
        let corrupt = || SecretFileError::Corrupt {
            path: self.path.clone(),
        };
        let key = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| corrupt())?;
        if key.len() != self.len {
            return Err(corrupt());
        }
        Ok(key)
    }

    /// 乱数を鋳造し、**中身が揃った状態で**排他的に置く。
    ///
    /// # なぜ「作ってから書く」ではないのか
    ///
    /// `O_EXCL` で作ってから中身を書くと、作成と書込の間に**空のファイルが見える窓**が開く。
    /// 競争に負けた側はそこを読んで「長さ 0 の鍵」を掴み、`Corrupt` で倒れる（実測で再現 —
    /// `concurrent_minting_converges_on_one_key` が 8 スレッドで必ず捕まえる）。
    ///
    /// そこで**先に一時ファイルへ中身と権限を揃え**、それから `hard_link` で本来の名前を
    /// 主張する。`hard_link` は既存の名前に対して `AlreadyExists` で失敗するので `O_EXCL` と
    /// 同じ排他性を持ちながら、**リンクが張られた瞬間に中身は完成している**。したがって
    /// 「見えるなら完全」が成立し、負けた側が読むのは常に勝者の完全な鍵である。
    ///
    /// 勝てば鋳造した鍵、競争に負けたら `Ok(None)` を返す。
    fn mint(&self) -> Result<Option<Vec<u8>>, SecretFileError> {
        let uncreatable = |cause: io::Error| SecretFileError::Uncreatable {
            path: self.path.clone(),
            cause,
        };
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(uncreatable)?;
        }
        let mut material = vec![0u8; self.len + NONCE_LEN];
        getrandom::fill(&mut material)
            .map_err(|error| uncreatable(io::Error::other(format!("getrandom: {error}"))))?;
        let (key, nonce) = material.split_at(self.len);

        // 一時ファイル名の nonce は**鍵とは別の乱数**である。鍵のバイトを名前に使うと
        // ディレクトリ一覧に秘密が漏れる。
        let staged = self.staging_path(nonce);
        let file = create_new_file(&staged).map_err(uncreatable)?;
        let outcome = set_owner_only(&file)
            .and_then(|()| {
                write_all(
                    file,
                    format!("{}\n", URL_SAFE_NO_PAD.encode(key)).as_bytes(),
                )
            })
            // 中身が揃ってから名前を主張する。
            .and_then(|()| fs::hard_link(&staged, &self.path));
        let _ = fs::remove_file(&staged);
        match outcome {
            Ok(()) => Ok(Some(key.to_vec())),
            // 名前は既に他の鋳造者のものである（この時点で相手の中身は完成している）。
            Err(cause) if cause.kind() == io::ErrorKind::AlreadyExists => Ok(None),
            Err(cause) => Err(uncreatable(cause)),
        }
    }

    /// 鋳造中の一時ファイル（同じディレクトリに置く — `hard_link` は同一ファイルシステム内
    /// でしか張れない）。nonce で**同一プロセス内の並行鋳造とも**ぶつからないようにする
    /// （プロセス id だけではスレッド同士が同じ名前を取り合う）。
    fn staging_path(&self, nonce: &[u8]) -> PathBuf {
        let name = self.path.file_name().map_or_else(
            || String::from("secret"),
            |n| n.to_string_lossy().into_owned(),
        );
        let staged = format!(".{name}.mint-{}", URL_SAFE_NO_PAD.encode(nonce));
        self.path
            .parent()
            .map_or_else(|| PathBuf::from(&staged), |parent| parent.join(&staged))
    }
}

/// 所有者だけが読み書きできる権限（0600）に締める。
fn set_owner_only(file: &fs::File) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        file.set_permissions(fs::Permissions::from_mode(0o600))
    }
    // 非 Unix では POSIX モードが無い。作成できたこと自体は成功として扱う。
    #[cfg(not(unix))]
    {
        let _ = file;
        Ok(())
    }
}

fn write_all(mut file: fs::File, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write as _;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const LEN: usize = 32;

    fn secret_at(dir: &Path) -> SecretFile {
        SecretFile::new(dir.join("nested").join(".secret"), LEN)
    }

    #[test]
    fn minting_creates_a_key_of_the_requested_length() {
        let dir = tempdir().unwrap();
        let secret = secret_at(dir.path());

        let key = secret.load_or_mint().unwrap();

        assert_eq!(key.len(), LEN);
        assert!(secret.path().exists(), "親ディレクトリごと作られる");
    }

    #[test]
    fn a_minted_key_is_stored_as_canonical_base64url_with_a_trailing_newline() {
        let dir = tempdir().unwrap();
        let secret = secret_at(dir.path());

        let key = secret.load_or_mint().unwrap();

        let text = fs::read_to_string(secret.path()).unwrap();
        assert!(text.ends_with('\n'), "改行で終わる: {text:?}");
        assert_eq!(URL_SAFE_NO_PAD.encode(&key), text.trim());
    }

    #[test]
    fn minting_is_idempotent_so_repeated_calls_see_the_same_key() {
        // 同じチェックアウトでの繰り返し呼出が同じ鍵を見ることが、
        // 別プロセスをまたいだ封緘の検証が成立する条件である。
        let dir = tempdir().unwrap();
        let secret = secret_at(dir.path());

        let first = secret.load_or_mint().unwrap();
        let second = secret.load_or_mint().unwrap();

        assert_eq!(first, second);
    }

    /// 同時に鋳造しようとしても、全員が同じ鍵へ収束する。
    ///
    /// 排他作成に負けた側は勝者の鍵を読み直す（`load_or_mint` の `AlreadyExists` の腕）。
    /// これが無いとプロセスごとに違う鍵を持ち、互いの封緘を検証できなくなる。競争が
    /// 実際に起きるかはスケジューラ次第だが、**主張している不変条件は決定的**である。
    #[test]
    fn concurrent_minting_converges_on_one_key() {
        let dir = tempdir().unwrap();
        let secret = secret_at(dir.path());

        let keys: Vec<Vec<u8>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| scope.spawn(|| secret.load_or_mint().unwrap()))
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        assert_eq!(keys.len(), 8);
        let distinct: std::collections::BTreeSet<_> = keys.into_iter().collect();
        assert_eq!(distinct.len(), 1, "全員が同じ鍵を見る");
    }

    /// 鋳造は一時ファイルを残さない（勝っても負けても後始末する）。
    #[test]
    fn minting_leaves_no_staging_file_behind() {
        let dir = tempdir().unwrap();
        let secret = secret_at(dir.path());

        secret.load_or_mint().unwrap();
        secret.load_or_mint().unwrap();

        let entries: Vec<_> = fs::read_dir(secret.path().parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(entries, vec![".secret".to_string()], "{entries:?}");
    }

    #[test]
    fn reading_an_absent_secret_reports_absence_rather_than_failure() {
        let dir = tempdir().unwrap();

        assert!(secret_at(dir.path()).read().unwrap().is_none());
    }

    #[test]
    fn reading_never_mints() {
        let dir = tempdir().unwrap();
        let secret = secret_at(dir.path());

        let _ = secret.read().unwrap();

        assert!(!secret.path().exists(), "読むだけの経路は書かない");
    }

    #[test]
    fn a_key_of_the_wrong_length_is_corrupt() {
        let dir = tempdir().unwrap();
        let secret = secret_at(dir.path());
        fs::create_dir_all(secret.path().parent().unwrap()).unwrap();
        fs::write(
            secret.path(),
            format!("{}\n", URL_SAFE_NO_PAD.encode([1u8; 8])),
        )
        .unwrap();

        assert!(matches!(
            secret.read(),
            Err(SecretFileError::Corrupt { .. })
        ));
    }

    /// パディング付きの綴りは拒否される。
    ///
    /// これは**依存の固定**である。正準性を担保しているのは自前の検査ではなく
    /// `URL_SAFE_NO_PAD` の厳格さなので（モジュール doc 参照）、エンジンや設定を寛容なものへ
    /// 替えたらここが落ちる。落ちたら往復忠実性の検査を足すこと — 同じ鍵に 2 通りの綴りが
    /// 存在すると、綴りを同一性の根拠にしている面が静かに割れる。
    #[test]
    fn a_padded_spelling_is_rejected_by_the_strict_decoder() {
        let dir = tempdir().unwrap();
        let secret = secret_at(dir.path());
        fs::create_dir_all(secret.path().parent().unwrap()).unwrap();
        let padded = base64::engine::general_purpose::URL_SAFE.encode([7u8; LEN]);
        assert!(
            URL_SAFE_NO_PAD.decode(&padded).is_err(),
            "この検査が意味を持つ前提: 厳格デコーダはパディングを受け付けない"
        );
        fs::write(secret.path(), format!("{padded}\n")).unwrap();

        assert!(matches!(
            secret.read(),
            Err(SecretFileError::Corrupt { .. })
        ));
    }

    #[test]
    fn garbage_is_corrupt() {
        let dir = tempdir().unwrap();
        let secret = secret_at(dir.path());
        fs::create_dir_all(secret.path().parent().unwrap()).unwrap();
        fs::write(secret.path(), "not-base64url!!\n").unwrap();

        assert!(matches!(
            secret.read(),
            Err(SecretFileError::Corrupt { .. })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn a_minted_key_is_readable_only_by_its_owner() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempdir().unwrap();
        let secret = secret_at(dir.path());

        secret.load_or_mint().unwrap();

        let mode = fs::metadata(secret.path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "mode was {:o}", mode & 0o777);
    }

    /// 「読めない」は不在ではない — 名前がディレクトリに取られていれば `Unreadable` である
    /// （不在なら `Ok(None)` で鋳造へ進んでしまう）。
    #[test]
    fn a_name_taken_by_a_directory_is_unreadable_not_absent() {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let path = dir.path().join("key");
        fs::create_dir(&path).expect("同名のディレクトリ");

        let error = SecretFile::new(path, 32).read().expect_err("読めない");

        assert!(
            matches!(error, SecretFileError::Unreadable { .. }),
            "{error:?}"
        );
        assert!(error.to_string().contains("unreadable"), "{error}");
    }

    /// 親ディレクトリが書けなければ鋳造は `Uncreatable` で止まる（黙って諦めない）。
    #[cfg(unix)]
    #[test]
    fn minting_under_a_read_only_parent_is_uncreatable() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let parent = dir.path().join("locked");
        fs::create_dir(&parent).expect("親");
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o555)).expect("読取専用へ");

        let error = SecretFile::new(parent.join("key"), 32)
            .load_or_mint()
            .expect_err("書けない");

        // 後片付けのために戻す（tempdir の削除が失敗しないように）。
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).expect("戻す");
        assert!(
            matches!(error, SecretFileError::Uncreatable { .. }),
            "{error:?}"
        );
    }
}
