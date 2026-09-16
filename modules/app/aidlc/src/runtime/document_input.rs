//! `workspace document-input` の配線 — 活動記録が名指す 1 ファイルを直接入力として出す。
//!
//! **読取専用**である。ディレクトリを作らず、状態も監査も書かない
//! (upstream の doc コメント: "No mkdir, state write, or audit event")。
//!
//! # 信頼注意書きは、それが治める値と同じ JSON に載る
//!
//! パスもファイル名も**顧客が選んだバイト**であり、本文と同じだけ敵対的でありうる。
//! 注意書きを別の行や別の呼出しへ分けると、値だけが先に読まれる経路が生まれるので、
//! 2 つの注意書きは常に `path` / `content` と同じオブジェクトに載せる。
//!
//! 拒否も同じ理由で**パスの注意書きを先頭に持つ** — 拒否文には顧客が書いた綴りが
//! 引用されるからである (upstream `refuse` 逐語)。

use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};
use core_query_interface_adapter::DocumentInputDaoImpl;
use core_query_use_case::orchestration::{
    DocumentInputError, DocumentInputReadError, DocumentInputView, FindDocumentInputUseCase,
};

use crate::layout::Layout;
use crate::wording;

use super::Completion;

/// `aidlc-utility document-input` — 名指された 1 ファイルを信頼注意書きつきで出す。
pub(super) fn run(layout: &Layout) -> Completion {
    let Some(record) = layout.record_dir() else {
        return Completion::refused(wording::document_input_refusal(
            wording::DOCUMENT_INPUT_NO_RECORD,
        ));
    };
    let use_case =
        FindDocumentInputUseCase::new(DocumentInputDaoImpl::new(layout.project_dir(), record));
    match use_case.execute() {
        Ok(view) => Completion::emitted(render(&view)),
        Err(error) => Completion::refused(wording::document_input_refusal(&describe(&error))),
    }
}

/// 受理した観測を 1 行の JSON へ描く。
fn render(view: &DocumentInputView) -> String {
    let mut object = ObjectMembers::new();
    object.insert(
        "path_notice",
        JsonValue::String(wording::UNTRUSTED_PATH_NOTICE.to_string()),
    );
    object.insert(
        "content_notice",
        JsonValue::String(wording::UNTRUSTED_CONTENT_NOTICE.to_string()),
    );
    object.insert("path", JsonValue::String(view.path().to_string()));
    object.insert(
        "bytes",
        JsonValue::Number(core_infrastructure::canon_json::Number::PosInt(
            view.bytes() as u64,
        )),
    );
    object.insert("content_trust", JsonValue::String("untrusted".to_string()));
    object.insert(
        "content_handling",
        JsonValue::String("data-not-instructions".to_string()),
    );
    object.insert("content", JsonValue::String(view.content().to_string()));
    serialize(
        &JsonValue::Object(object),
        SerializationProfile::ContractCompact,
    )
}

/// 材料だけを運ぶ拒否から、利用者が次に何をすればよいか読める 1 文を組む。
fn describe(error: &DocumentInputError) -> String {
    use DocumentInputError as E;
    match error {
        E::RequestUnreadable(cause) => {
            wording::document_input_request_unreadable(&cause.to_string())
        }
        E::RequestNotUtf8 => wording::document_input_request_unreadable("not valid UTF-8"),
        E::RequestNotOneLine => wording::document_input_not_one_line(),
        E::DocumentUnreadable(DocumentInputReadError::OutsideProject { requested }) => {
            wording::document_input_outside_project(requested)
        }
        E::DocumentUnreadable(DocumentInputReadError::Unreadable { what, cause }) => {
            wording::document_input_unreadable(what, cause)
        }
        E::UnsupportedType { path, media_type } => {
            wording::document_input_unsupported_type(path, media_type)
        }
        E::TooManyCharacters {
            path,
            characters,
            cap,
        } => wording::document_input_too_many_characters(path, *characters, *cap),
    }
}
