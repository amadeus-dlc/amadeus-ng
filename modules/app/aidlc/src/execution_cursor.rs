//! 実行カーソル — この record がどの実行を指しているか（`<record>/.aidlc-execution`）。
//!
//! **リードモデルは実行の識別子を記録していない**（`aidlc-state.md` にも `intents.json` にも
//! 欄が無い — 実測）。かつて `report` はその穴をジャーナル先頭の実行行で埋めていたが、それは
//! 「実行はワークスペースにただ 1 つ」という仮定に乗っていて、2 本目が生まれた瞬間に静かに
//! 別の実行へ報告する。record が指す実行を**record 自身に書いておく**のがここである。
//!
//! # マシンローカルで、`.gitignore` 済みである
//!
//! カーソルは `aidlc/spaces/*/intents/*/.aidlc-*` に合致するので、`.gitignore` の既存の
//! 1 行がそのまま効く（active-intent / clone-id と同じ扱い — どの実行を握っているかは
//! クローンごとの navigation であって、共有される記録ではない）。
//!
//! # 置き場を決めるのは [`crate::layout::Layout`]、名前を決めるのはここ
//!
//! 先行する 2 つの機構（[`crate::clone_identity`] / [`crate::steering`]）と同じ流儀である —
//! 呼出側は record ディレクトリを渡すだけで、ファイル名を知らない。名前を 2 箇所に書くと
//! 片方だけが動いたときに静かにずれる。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use core_command_domain::orchestration::{IntentExecutionId, IntentId};
use core_infrastructure::atomic::write_file_atomic;

mod execution_cursor_error;

pub use execution_cursor_error::ExecutionCursorError;

/// カーソルのファイル名。
const EXECUTION_CURSOR_FILE: &str = ".aidlc-execution";

/// この record が指す実行（1 行目 = 実行の識別子、2 行目 = その実行が属する intent）。
///
/// intent の識別子を一緒に持つのは、record・状態ファイル・イベントが同じ intent を指して
/// いるかを**照合できる**ようにするためである（実行の識別子だけでは、どの intent の実行か
/// を知るのにジャーナルを読み直すことになる）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionCursor {
    execution_id: IntentExecutionId,
    intent_id: IntentId,
}

impl ExecutionCursor {
    /// 基本コンストラクタ — 構造体リテラルはここ 1 箇所だけに現れる
    /// （`coding-rules/factory-naming.md`）。
    #[must_use]
    pub const fn new(execution_id: IntentExecutionId, intent_id: IntentId) -> ExecutionCursor {
        ExecutionCursor {
            execution_id,
            intent_id,
        }
    }

    /// この record が指す実行。
    #[must_use]
    pub const fn execution_id(&self) -> &IntentExecutionId {
        &self.execution_id
    }

    /// その実行が属する intent。
    #[must_use]
    pub const fn intent_id(&self) -> &IntentId {
        &self.intent_id
    }

    /// カーソルを `<record_dir>/.aidlc-execution` へ**不可分に**据える。
    ///
    /// tmp + rename で書く理由は active-intent カーソルと同じである — `fs::write` は
    /// 切り詰めてから書くので、その隙に読んだ側が「1 行しか無い」カーソルを見て
    /// [`ExecutionCursorError::Malformed`] と判断してしまう。
    ///
    /// # Errors
    ///
    /// record ディレクトリが無い・書けない場合の I/O。
    pub fn write(&self, record_dir: &Path) -> Result<(), ExecutionCursorError> {
        let path = ExecutionCursor::path_in(record_dir);
        write_file_atomic(&path, self.render().as_bytes()).map_err(|cause| {
            ExecutionCursorError::Io {
                kind: cause.kind(),
                path,
            }
        })
    }

    /// `<record_dir>/.aidlc-execution` を読む。
    ///
    /// **不在は `Ok(None)`** — カーソルがまだ据わっていないのは intent 未鋳造の正常な姿で
    /// あり、失敗ではない。「在るのに読めない」「在るが 2 つの識別子として読めない」だけを
    /// `Err` で分ける（`clone_identity` の 3 値観測と同じ分け方 — 不在と破損を混ぜない）。
    ///
    /// # Errors
    ///
    /// 在るのに読めない場合の I/O、および中身が 2 つの識別子として読めない場合。
    pub fn read(record_dir: &Path) -> Result<Option<ExecutionCursor>, ExecutionCursorError> {
        let path = ExecutionCursor::path_in(record_dir);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(cause) if cause.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(cause) => {
                return Err(ExecutionCursorError::Io {
                    kind: cause.kind(),
                    path,
                });
            }
        };
        ExecutionCursor::parse(&text)
            .map(Some)
            .ok_or(ExecutionCursorError::Malformed { path })
    }

    /// ディスク上の綴り（末尾改行つき — 行指向のファイルの家風）。
    fn render(&self) -> String {
        format!("{}\n{}\n", self.execution_id, self.intent_id)
    }

    /// 2 行を識別子の対として読む（補助コンストラクタ — 基本コンストラクタへ委譲する）。
    ///
    /// 空行は読み飛ばす（末尾改行の有無・行区切りの揺れに耐える）。**3 つ目の中身があれば
    /// 拒む** — 読める分だけ拾って進むと、書き手が変わったことに気付けないまま古い解釈で
    /// 動き続ける。
    fn parse(text: &str) -> Option<ExecutionCursor> {
        let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
        let execution_id = IntentExecutionId::parse(lines.next()?).ok()?;
        let intent_id = IntentId::parse(lines.next()?).ok()?;
        if lines.next().is_some() {
            return None;
        }
        Some(ExecutionCursor::new(execution_id, intent_id))
    }

    /// record ディレクトリの中のカーソルの所在。
    fn path_in(record_dir: &Path) -> PathBuf {
        record_dir.join(EXECUTION_CURSOR_FILE)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use core_command_domain::orchestration::{IntentExecutionId, IntentId};
    use std::fs;

    const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";
    const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

    fn cursor() -> ExecutionCursor {
        ExecutionCursor::new(
            IntentExecutionId::parse(EXECUTION).expect("UUIDv7"),
            IntentId::parse(INTENT).expect("UUIDv7"),
        )
    }

    #[test]
    fn a_written_cursor_reads_back_as_the_same_pair() {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");

        cursor().write(dir.path()).expect("書ける");

        assert_eq!(
            ExecutionCursor::read(dir.path()).expect("読める"),
            Some(cursor())
        );
    }

    #[test]
    fn an_absent_cursor_is_none_rather_than_an_error() {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");

        assert_eq!(
            ExecutionCursor::read(dir.path()).expect("不在は失敗ではない"),
            None
        );
    }

    #[test]
    fn a_cursor_whose_lines_are_not_identifiers_is_malformed() {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        fs::write(dir.path().join(".aidlc-execution"), "not-an-id\nalso-not\n").expect("壊れた行");

        let error = ExecutionCursor::read(dir.path()).expect_err("拒む");

        assert!(
            matches!(error, ExecutionCursorError::Malformed { .. }),
            "{error:?}"
        );
    }

    /// 3 つ目の中身は**読み飛ばさず拒む** — 書式が変わったことに気付けないまま進まない。
    #[test]
    fn a_cursor_with_a_third_line_is_malformed() {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        fs::write(
            dir.path().join(".aidlc-execution"),
            format!("{EXECUTION}\n{INTENT}\nsomething-else\n"),
        )
        .expect("余分な行");

        assert!(
            matches!(
                ExecutionCursor::read(dir.path()),
                Err(ExecutionCursorError::Malformed { .. })
            ),
            "余分な行は拒む"
        );
    }

    /// 1 行しか無いカーソルも拒む（対で意味を成すので、半分だけでは読めない）。
    #[test]
    fn a_cursor_with_only_one_identifier_is_malformed() {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        fs::write(
            dir.path().join(".aidlc-execution"),
            format!("{EXECUTION}\n"),
        )
        .expect("片方だけ");

        assert!(
            matches!(
                ExecutionCursor::read(dir.path()),
                Err(ExecutionCursorError::Malformed { .. })
            ),
            "対でなければ読めない"
        );
    }

    /// **在るのに読めない**は不在ではない — `Ok(None)` に畳むと「まだ鋳造していない」と
    /// 誤読され、壊れた record の上で新しい実行が始まってしまう。
    #[test]
    fn an_unreadable_cursor_is_an_io_failure_rather_than_absent() {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        fs::create_dir(dir.path().join(".aidlc-execution")).expect("同名のディレクトリ");

        let error = ExecutionCursor::read(dir.path()).expect_err("読めない");

        assert!(
            matches!(error, ExecutionCursorError::Io { .. }),
            "{error:?}"
        );
    }

    /// 書けなければ分類とパスを材料として返す（文言を組むのは出す側）。
    #[test]
    fn an_absent_record_directory_surfaces_the_io_kind_and_path() {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let record = dir.path().join("never-created");

        let error = cursor().write(&record).expect_err("書けない");

        assert_eq!(
            error,
            ExecutionCursorError::Io {
                kind: std::io::ErrorKind::NotFound,
                path: record.join(".aidlc-execution"),
            }
        );
    }

    /// 据え直しは上書きになる（同じ record が別の実行を指すことは無いが、再鋳造が
    /// 半端な状態を残さないことを固定する）。
    #[test]
    fn writing_twice_leaves_the_latest_pair() {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let other = ExecutionCursor::new(
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0001").expect("UUIDv7"),
            IntentId::parse(INTENT).expect("UUIDv7"),
        );

        cursor().write(dir.path()).expect("1 回目");
        other.write(dir.path()).expect("2 回目");

        assert_eq!(
            ExecutionCursor::read(dir.path()).expect("読める"),
            Some(other)
        );
    }

    /// ディスク上の綴りは「1 行目 = 実行、2 行目 = intent、末尾改行」である。
    #[test]
    fn the_written_bytes_are_two_lines_in_a_fixed_order() {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");

        cursor().write(dir.path()).expect("書ける");

        assert_eq!(
            fs::read_to_string(dir.path().join(".aidlc-execution")).expect("読める"),
            format!("{EXECUTION}\n{INTENT}\n")
        );
    }

    #[test]
    fn the_pair_is_readable_through_its_accessors() {
        let cursor = cursor();

        assert_eq!(cursor.execution_id().as_str(), EXECUTION);
        assert_eq!(cursor.intent_id().as_str(), INTENT);
    }

    #[test]
    fn a_trailing_newline_is_optional() {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        fs::write(
            dir.path().join(".aidlc-execution"),
            format!("{EXECUTION}\n{INTENT}"),
        )
        .expect("改行なし");

        assert_eq!(
            ExecutionCursor::read(dir.path()).expect("読める"),
            Some(cursor())
        );
    }
}
