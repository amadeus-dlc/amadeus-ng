//! `FindDocumentInputUseCase` — 活動記録が名指す 1 ファイルを直接入力として引く
//! (upstream `handleDocumentInput`)。

use crate::orchestration::{
    DocumentInputBytes, DocumentInputDao, DocumentInputError, DocumentInputView,
};

/// 直接入力の文字数上限 (upstream `EXTRACT_OUTPUT_CHAR_CAP`)。
///
/// DocumentKB の受渡上限と同じ値を使う — 直接入力は DocumentKB を迂回する近道であって、
/// 別の上限を持つ別の面ではない。
pub const EXTRACT_OUTPUT_CHAR_CAP: usize = 200_000;

/// 直接扱える 2 つの種別 (upstream `detectMimeType` の text 分岐)。
const TEXT_PLAIN: &str = "text/plain";
const TEXT_MARKDOWN: &str = "text/markdown";

/// PDF の magic とその探索窓 (upstream `PDF_MAGIC` / `PDF_SEARCH_WINDOW`)。
const PDF_MAGIC: &[u8] = b"%PDF-";
const PDF_SEARCH_WINDOW: usize = 1024;

/// 先頭固定オフセットの binary magic (upstream `BINARY_MAGICS_FIXED_OFFSET`)。
const BINARY_MAGICS_FIXED_OFFSET: [&[u8]; 5] = [
    b"PK\x03\x04",
    b"PK\x05\x06",
    b"\xff\xd8\xff",
    b"\x89PNG",
    b"\x1f\x8b",
];

/// 制御バイトがこの割合を超えたら binary と見なす (upstream 逐語 `> 0.3`)。
const NON_PRINTABLE_LIMIT: f64 = 0.3;

/// 転送ファイルが名指す 1 ファイルを、封じ込め・種別・上限の 3 つの関門に通して引く。
///
/// **検索も fallback もしない**のがこの引当の要である — 顧客が選んだ綴りは 1 つの正確な
/// パスとしてだけ解釈し、見つからなければ近い名前を探さずに拒む (upstream の doc コメント:
/// "filenames are not searched recursively")。
///
/// 3 つの関門を鍵の順にたどって View を組むのは、規則 6 (2026-09-03) がクエリ側ユースケース
/// に認めた「鍵をたどって面ごとに引き、View を組む」に当たる
/// (`coding-rules/cqrs-boundaries.md`)。逐語文言は組まない — 拒否は材料だけを運ぶ
/// [`DocumentInputError`] で返し、文言は出す側が組む。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindDocumentInputUseCase<D: DocumentInputDao> {
    document_input_dao: D,
}

impl<D: DocumentInputDao> FindDocumentInputUseCase<D> {
    /// 引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(document_input_dao: D) -> FindDocumentInputUseCase<D> {
        FindDocumentInputUseCase { document_input_dao }
    }

    /// 直接入力を引く。
    ///
    /// # Errors
    ///
    /// 転送ファイルが 1 行でない、名指された先を引けない、種別が違う、または上限を超えて
    /// いる ([`DocumentInputError`])。
    pub fn execute(&self) -> Result<DocumentInputView, DocumentInputError> {
        let raw = self
            .document_input_dao
            .find_request()
            .map_err(DocumentInputError::RequestUnreadable)?;
        let requested = requested_path(&raw)?;
        let document = self
            .document_input_dao
            .find_document(&requested)
            .map_err(DocumentInputError::DocumentUnreadable)?;
        accept(&document)
    }
}

/// 転送ファイルの生バイトから、1 本の非空パス行を取り出す。
///
/// 末尾の改行 1 つ (`\r\n` または `\n`) だけを落とす — upstream の
/// `raw.replace(/\r?\n$/, "")` と同じで、行の途中に現れる改行は「1 行でない」証拠として
/// 残す。
fn requested_path(raw: &[u8]) -> Result<String, DocumentInputError> {
    let text = core::str::from_utf8(raw).map_err(|_| DocumentInputError::RequestNotUtf8)?;
    let value = text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .unwrap_or(text);
    if value.is_empty() || value.contains(['\r', '\n']) {
        return Err(DocumentInputError::RequestNotOneLine);
    }
    Ok(value.to_string())
}

/// 引けたバイトを、種別と文字数上限の関門に通して View へ組む。
fn accept(document: &DocumentInputBytes) -> Result<DocumentInputView, DocumentInputError> {
    let media_type = media_type(document.path(), document.bytes());
    if media_type != TEXT_PLAIN && media_type != TEXT_MARKDOWN {
        return Err(DocumentInputError::UnsupportedType {
            path: document.path().to_string(),
            media_type,
        });
    }
    // 種別判定が UTF-8 を確かめた後なので、ここで失われるバイトは無い。
    let content = core::str::from_utf8(document.bytes()).map_err(|_| {
        DocumentInputError::UnsupportedType {
            path: document.path().to_string(),
            media_type: "application/octet-stream".to_string(),
        }
    })?;
    // 上限は JS の `String.length` と同じ UTF-16 単位で数える — 同じ文書が harness に
    // よって通ったり落ちたりしないようにするため。
    let characters = content.encode_utf16().count();
    if characters > EXTRACT_OUTPUT_CHAR_CAP {
        return Err(DocumentInputError::TooManyCharacters {
            path: document.path().to_string(),
            characters,
            cap: EXTRACT_OUTPUT_CHAR_CAP,
        });
    }
    Ok(DocumentInputView::new(
        document.path().to_string(),
        document.bytes().len(),
        content.to_string(),
    ))
}

/// バイトと拡張子から種別を決める (upstream `detectMimeType`)。
fn media_type(path: &str, bytes: &[u8]) -> String {
    if has_pdf_magic_in_window(bytes) {
        return "application/pdf".to_string();
    }
    if looks_binary(bytes) {
        return "application/octet-stream".to_string();
    }
    if path.to_ascii_lowercase().ends_with(".md") {
        TEXT_MARKDOWN.to_string()
    } else {
        TEXT_PLAIN.to_string()
    }
}

/// 先頭 1024 バイトのどこかに PDF の magic があるか (upstream `hasPdfMagicInWindow`)。
fn has_pdf_magic_in_window(bytes: &[u8]) -> bool {
    let window = bytes.len().min(PDF_SEARCH_WINDOW);
    bytes.get(..window).is_some_and(|window| {
        window
            .windows(PDF_MAGIC.len())
            .any(|slice| slice == PDF_MAGIC)
    })
}

/// binary か (upstream `looksBinary` — 窓を持たず**全バイト**を見る)。
fn looks_binary(bytes: &[u8]) -> bool {
    if BINARY_MAGICS_FIXED_OFFSET
        .iter()
        .any(|magic| bytes.starts_with(magic))
    {
        return true;
    }
    if bytes.contains(&0) {
        return true;
    }
    if core::str::from_utf8(bytes).is_err() {
        return true;
    }
    // 妥当な UTF-8 でも制御バイトの列は binary である (0x01 の連なりは復号できてしまう)。
    let non_printable = bytes
        .iter()
        .filter(|byte| **byte < 0x09 || (**byte > 0x0d && **byte < 0x20))
        .count();
    !bytes.is_empty() && ratio(non_printable, bytes.len()) > NON_PRINTABLE_LIMIT
}

#[expect(
    clippy::cast_precision_loss,
    reason = "本家は Number 同士の除算で割合を出す。桁落ちは同じ振る舞いの一部である"
)]
fn ratio(part: usize, whole: usize) -> f64 {
    part as f64 / whole as f64
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn a_single_trailing_newline_is_dropped_but_an_inner_one_is_not() {
        assert_eq!(
            requested_path(b"docs/request.md\n"),
            Ok("docs/request.md".to_string())
        );
        assert_eq!(
            requested_path(b"docs/request.md\r\n"),
            Ok("docs/request.md".to_string())
        );
        assert_eq!(
            requested_path(b"docs/request.md"),
            Ok("docs/request.md".to_string())
        );
        assert_eq!(
            requested_path(b"a.md\nb.md\n"),
            Err(DocumentInputError::RequestNotOneLine)
        );
        assert_eq!(
            requested_path(b"\n"),
            Err(DocumentInputError::RequestNotOneLine)
        );
        assert_eq!(
            requested_path(b""),
            Err(DocumentInputError::RequestNotOneLine)
        );
    }

    #[test]
    fn a_request_that_is_not_utf8_is_refused_before_the_one_line_check() {
        assert_eq!(
            requested_path(&[0xff, 0xfe]),
            Err(DocumentInputError::RequestNotUtf8)
        );
    }

    #[test]
    fn the_media_type_separates_text_from_markdown_and_from_binary() {
        assert_eq!(media_type("docs/a.txt", b"hello"), TEXT_PLAIN);
        assert_eq!(media_type("docs/a.md", b"hello"), TEXT_MARKDOWN);
        assert_eq!(media_type("docs/A.MD", b"hello"), TEXT_MARKDOWN);
        assert_eq!(media_type("docs/a.md", b"%PDF-1.7\n"), "application/pdf");
        assert_eq!(
            media_type("docs/a.md", b"PK\x03\x04rest"),
            "application/octet-stream"
        );
        assert_eq!(
            media_type("docs/a.md", b"one\0two"),
            "application/octet-stream"
        );
        assert_eq!(
            media_type("docs/a.md", &[0xff, 0xfe, 0x41]),
            "application/octet-stream"
        );
        assert_eq!(
            media_type("docs/a.md", &[0x01; 16]),
            "application/octet-stream"
        );
    }

    /// 空のファイルは binary ではない — 割合の分母が 0 のとき upstream も false を返す。
    #[test]
    fn an_empty_file_is_text() {
        assert_eq!(media_type("docs/a.txt", b""), TEXT_PLAIN);
    }

    #[test]
    fn the_character_cap_is_counted_in_the_same_units_as_the_harness() {
        // 補助対 1 つは UTF-16 で 2 単位、UTF-8 で 4 バイト。上限は文字数側で効く。
        let content = "\u{1F600}".repeat(EXTRACT_OUTPUT_CHAR_CAP / 2 + 1);
        let bytes = DocumentInputBytes::new("docs/a.md".to_string(), content.into_bytes());
        let error = accept(&bytes).expect_err("上限超過");
        assert!(matches!(
            error,
            DocumentInputError::TooManyCharacters { characters, cap, .. }
                if characters == EXTRACT_OUTPUT_CHAR_CAP + 2 && cap == EXTRACT_OUTPUT_CHAR_CAP
        ));
    }

    #[test]
    fn an_accepted_document_carries_its_portable_path_byte_count_and_content() {
        let bytes = DocumentInputBytes::new("docs/a.md".to_string(), "本文".as_bytes().to_vec());
        let view = accept(&bytes).expect("受理");
        assert_eq!(view.path(), "docs/a.md");
        assert_eq!(view.bytes(), 6);
        assert_eq!(view.content(), "本文");
    }

    #[test]
    fn a_binary_document_is_refused_by_its_type() {
        let bytes = DocumentInputBytes::new("docs/a.md".to_string(), b"%PDF-1.7\n".to_vec());
        assert_eq!(
            accept(&bytes),
            Err(DocumentInputError::UnsupportedType {
                path: "docs/a.md".to_string(),
                media_type: "application/pdf".to_string(),
            })
        );
    }
}
