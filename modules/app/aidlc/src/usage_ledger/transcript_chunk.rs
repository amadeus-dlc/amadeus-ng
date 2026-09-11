//! 会話履歴ファイルの、前回畳んだ位置より後ろのバイト列。
//!
//! 本家 `tools/aidlc-usage.ts` の `readChunkFromOffset`。位置はバイトであって文字ではない
//! （多バイト文字で位置がずれないように、生のバイト列を LF で切ってから復号する）。
//! 末尾の改行で終わらない断片は書きかけとして次回へ残す。ただし flush のときだけ、
//! 完全な JSON 値なら改行なしでも 1 行として受け入れる。
use harness_claude::TranscriptUsageRow;
use std::path::Path;

/// 1 回の読取り結果。完全な行と、その行の絶対バイト位置、次回の開始位置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TranscriptChunk {
    lines: Vec<(u64, String)>,
    new_byte_offset: u64,
    reset: bool,
    trailing_partial: bool,
}

impl TranscriptChunk {
    const fn new(
        lines: Vec<(u64, String)>,
        new_byte_offset: u64,
        reset: bool,
        trailing_partial: bool,
    ) -> Self {
        Self {
            lines,
            new_byte_offset,
            reset,
            trailing_partial,
        }
    }

    /// `[byte_offset, size)` を読む。読めない・新しいバイトが無いときは `None`。
    /// ファイルが位置より短ければ切り詰めと見なして先頭から読み直す（`reset`）。
    pub(crate) fn read(path: &Path, byte_offset: u64, flush: bool) -> Option<Self> {
        use std::io::{Read as _, Seek as _, SeekFrom};
        let mut file = std::fs::File::open(path).ok()?;
        let size = file.metadata().ok()?.len();
        if size == byte_offset {
            return None;
        }
        let (reset, start) = if size < byte_offset {
            (true, 0)
        } else {
            (false, byte_offset)
        };
        file.seek(SeekFrom::Start(start)).ok()?;
        let mut chunk = Vec::new();
        file.by_ref()
            .take(size - start)
            .read_to_end(&mut chunk)
            .ok()?;
        // 最後の LF までが完全な行。その後ろは書きかけとして次回へ残す。
        let last_newline = chunk.iter().rposition(|byte| *byte == b'\n');
        let mut lines = Vec::new();
        let mut line_start = 0usize;
        if let Some(last_newline) = last_newline {
            for (index, byte) in chunk.iter().enumerate().take(last_newline + 1) {
                if *byte == b'\n' {
                    lines.push((start + line_start as u64, decode(&chunk, line_start, index)));
                    line_start = index + 1;
                }
            }
        }
        let mut new_byte_offset = last_newline.map_or(start, |index| start + index as u64 + 1);
        let mut trailing_partial = line_start < chunk.len();
        // flush のときだけ、改行の無い末尾でも完全な JSON 値なら 1 行として受け入れる。
        if flush && trailing_partial {
            let trailing = decode(&chunk, line_start, chunk.len());
            if serde_json::from_str::<serde_json::Value>(&trailing).is_ok() {
                lines.push((start + line_start as u64, trailing));
                new_byte_offset = start + chunk.len() as u64;
                trailing_partial = false;
            }
        }
        Some(Self::new(lines, new_byte_offset, reset, trailing_partial))
    }

    /// 課金対象の行だけを、その行の絶対バイト位置とともに返す。
    pub(crate) fn parsed_rows(
        &self,
        fallback_agent_id: Option<&str>,
    ) -> Vec<(u64, TranscriptUsageRow)> {
        self.lines
            .iter()
            .filter_map(|(start, line)| {
                TranscriptUsageRow::parse(line, fallback_agent_id).map(|row| (*start, row))
            })
            .collect()
    }

    /// 次回の読取り開始位置（完全な行の直後）。
    pub(crate) const fn new_byte_offset(&self) -> u64 {
        self.new_byte_offset
    }

    /// 切り詰め・ローテーションを検出して先頭から読み直したか。
    pub(crate) const fn reset(&self) -> bool {
        self.reset
    }

    /// 末尾に改行で終わらない断片が残っているか。
    pub(crate) const fn trailing_partial(&self) -> bool {
        self.trailing_partial
    }
}

/// `[from, to)` のバイト列を UTF-8 として（壊れた列は置換文字で）復号する。
fn decode(chunk: &[u8], from: usize, to: usize) -> String {
    chunk
        .get(from..to)
        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(bytes: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
        let directory = tempfile::tempdir().expect("一時ディレクトリ");
        let path = directory.path().join("t.jsonl");
        std::fs::write(&path, bytes).expect("テストのファイル");
        (directory, path)
    }

    #[test]
    fn reading_from_an_offset_yields_the_complete_lines_with_their_byte_starts() {
        let (_directory, path) = file(b"aaa\nbb\nc");
        let chunk = TranscriptChunk::read(&path, 0, false).expect("新しいバイトがある");
        assert_eq!(
            chunk,
            TranscriptChunk::new(vec![(0, "aaa".into()), (4, "bb".into())], 7, false, true)
        );
        let later = TranscriptChunk::read(&path, 4, false).expect("新しいバイトがある");
        assert_eq!(
            later,
            TranscriptChunk::new(vec![(4, "bb".into())], 7, false, true)
        );
    }

    #[test]
    fn no_new_bytes_and_a_missing_file_yield_nothing() {
        let (directory, path) = file(b"aaa\nbb\nc");
        assert_eq!(TranscriptChunk::read(&path, 8, false), None);
        assert_eq!(TranscriptChunk::read(&path, 8, true), None);
        assert_eq!(
            TranscriptChunk::read(&directory.path().join("missing"), 0, true),
            None
        );
    }

    #[test]
    fn a_shorter_file_than_the_cursor_resets_to_the_top() {
        let (_directory, path) = file(b"xy\n");
        let chunk = TranscriptChunk::read(&path, 10, false).expect("切り詰めは読み直す");
        assert_eq!(
            chunk,
            TranscriptChunk::new(vec![(0, "xy".into())], 3, true, false)
        );
    }

    #[test]
    fn a_flush_accepts_a_complete_trailing_json_value_but_keeps_a_fragment() {
        let (_directory, complete) = file(
            br#"{"a":1}
{"b":2}"#,
        );
        let flushed = TranscriptChunk::read(&complete, 0, true).expect("読める");
        assert_eq!(
            flushed,
            TranscriptChunk::new(
                vec![(0, r#"{"a":1}"#.into()), (8, r#"{"b":2}"#.into())],
                15,
                false,
                false
            )
        );
        let held = TranscriptChunk::read(&complete, 0, false).expect("読める");
        assert_eq!(
            held,
            TranscriptChunk::new(vec![(0, r#"{"a":1}"#.into())], 8, false, true)
        );
        let (_directory, fragment) = file(
            br#"{"a":1}
{"b":"#,
        );
        let kept = TranscriptChunk::read(&fragment, 0, true).expect("読める");
        assert_eq!(
            kept,
            TranscriptChunk::new(vec![(0, r#"{"a":1}"#.into())], 8, false, true)
        );
    }

    #[test]
    fn lines_are_decoded_lossily_and_split_on_lf_only() {
        let (_directory, path) = file(b"\xff\xfe\n{\"x\":1}\r\n");
        let chunk = TranscriptChunk::read(&path, 0, false).expect("読める");
        assert_eq!(
            chunk,
            TranscriptChunk::new(
                vec![(0, "\u{FFFD}\u{FFFD}".into()), (3, "{\"x\":1}\r".into())],
                12,
                false,
                false
            )
        );
    }

    #[test]
    fn parsing_keeps_each_priced_rows_byte_start() {
        let (_directory, path) = file(
            br#"{"uuid":"a","message":{"role":"user","content":"hi"}}
{"uuid":"b","message":{"id":"m","role":"assistant","model":"claude-opus-4-8","usage":{"input_tokens":5}}}
not json
"#,
        );
        let chunk = TranscriptChunk::read(&path, 0, false).expect("読める");
        let rows = chunk.parsed_rows(Some("agent-x"));
        assert_eq!(rows.len(), 1);
        let (start, row) = rows.first().expect("1 行");
        assert_eq!(*start, 54);
        assert_eq!(row.uuid(), "b");
        assert_eq!(row.agent_id(), Some("agent-x"));
    }
}
