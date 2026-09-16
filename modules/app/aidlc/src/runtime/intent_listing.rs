//! `intent list` の配線 — 空間の依頼一覧を人間可読 / JSON で出す。
//!
//! **読取専用**である。カーソルを動かさず、記録も監査も書かない。切替 (`intent <name>` /
//! `intent switch`) はこの build が配線していないので、一覧と同じ入口に来ても**名指して
//! 拒否する** — 知らない依頼として落とすと、新しい依頼を始めよという誘いに読まれるからで
//! ある (upstream が同じ理由で拒否文を選んでいる)。

use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};
use core_query_interface_adapter::IntentListingDaoImpl;
use core_query_use_case::orchestration::{
    IntentListingRowView, IntentListingView, ListIntentsUseCase,
};

use crate::cli::IntentArgs;
use crate::layout::Layout;
use crate::wording;

use super::Completion;

/// `aidlc-utility intent [list] [--json]` — 空間の依頼一覧。
pub(super) fn run(layout: &Layout, args: &IntentArgs) -> Completion {
    if let Some(verb) = args.sub().filter(|verb| *verb != "list") {
        return Completion::refused(wording::intent_verb_not_wired(verb));
    }
    let active = layout
        .record_dir()
        .and_then(|record| record.file_name())
        .and_then(std::ffi::OsStr::to_str);
    let use_case = ListIntentsUseCase::new(IntentListingDaoImpl::new(&layout.intents_dir()));
    match use_case.execute(active) {
        Ok(view) if args.is_json() => Completion::emitted(render_json(layout.space(), &view)),
        Ok(view) => Completion::emitted(render_plain(layout.space(), &view)),
        Err(error) => Completion::refused(wording::intent_listing_unreadable(&error.to_string())),
    }
}

/// `{active, space, intents:[…]}` の 1 行 JSON（upstream の鍵と順序をそのまま写す）。
fn render_json(space: &str, view: &IntentListingView) -> String {
    let mut object = ObjectMembers::new();
    object.insert(
        "active",
        view.active().map_or(JsonValue::Null, |active| {
            JsonValue::String(active.to_string())
        }),
    );
    object.insert("space", JsonValue::String(space.to_string()));
    object.insert(
        "intents",
        JsonValue::Array(
            view.intents()
                .iter()
                .map(|row| row_json(view, row))
                .collect(),
        ),
    );
    serialize(
        &JsonValue::Object(object),
        SerializationProfile::ContractCompact,
    )
}

fn row_json(view: &IntentListingView, row: &IntentListingRowView) -> JsonValue {
    let mut object = ObjectMembers::new();
    object.insert("uuid", JsonValue::String(row.uuid().to_string()));
    object.insert("slug", JsonValue::String(row.slug().to_string()));
    object.insert("status", JsonValue::String(row.status().to_string()));
    object.insert(
        "repos",
        JsonValue::Array(
            row.repos()
                .iter()
                .map(|repo| JsonValue::String(repo.clone()))
                .collect(),
        ),
    );
    object.insert(
        "dirName",
        row.directory().map_or(JsonValue::Null, |directory| {
            JsonValue::String(directory.to_string())
        }),
    );
    object.insert("active", JsonValue::Bool(view.is_active(row)));
    JsonValue::Object(object)
}

/// 人間可読の一覧（末尾改行は `main.rs` の `writeln!` が付す）。
fn render_plain(space: &str, view: &IntentListingView) -> String {
    if view.intents().is_empty() {
        return wording::no_intents_in_space(space);
    }
    let mut lines = vec![wording::intents_in_space(space)];
    for row in view.intents() {
        let marker = if view.is_active(row) { '*' } else { ' ' };
        let name = row.directory().unwrap_or_else(|| row.slug());
        lines.push(format!("{marker} {name}  [{}]", row.status()));
    }
    if view.active().is_none() {
        lines.push(String::new());
        lines.push(wording::NO_ACTIVE_INTENT.to_string());
    }
    lines.join("\n")
}
