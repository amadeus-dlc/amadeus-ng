//! `CodekbScopeDao` の実 Gateway — `reverse-engineering-timestamp.md` の走査範囲ブロックを解く。
//!
//! 媒体は Markdown の中の fenced yaml であり、SQLite の `read_*` 表とは別の面である —
//! したがって [`super::ReadModelDaos`] (1 要求 1 接続) の住人ではなく、1 面の所在だけを握る。
//! 解くのは**版を固定した小さな行走査**で、YAML ライブラリは使わない (upstream `parseReScope`
//! も同じ理由で行走査である — 「Pure data - no model call. Same idiom as parseBoltDag」)。

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use core_query_use_case::orchestration::{
    CodekbScopeDao, ReScopeParseView, ReScopeView, ReadModelReadError,
};

/// この読み手が解ける唯一の版 (upstream `scope_version: 1`)。
const SUPPORTED_SCOPE_VERSION: &str = "1";

/// 走査範囲を記録した 1 面を読む実装。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodekbScopeDaoImpl {
    timestamp: PathBuf,
}

impl CodekbScopeDaoImpl {
    /// 走査範囲を記録したファイルの所在を受け取る (**この型の唯一の構築経路**)。
    ///
    /// 引く先は 2 通りある — durable な codekb ストアの `reverse-engineering-timestamp.md` と、
    /// 突合のために渡された取込側の走査記録。どちらも同じ綴りなので実装は 1 つである。
    #[must_use]
    pub fn new(timestamp: &Path) -> CodekbScopeDaoImpl {
        CodekbScopeDaoImpl {
            timestamp: timestamp.to_path_buf(),
        }
    }
}

impl CodekbScopeDao for CodekbScopeDaoImpl {
    fn find(&self) -> Result<Option<ReScopeParseView>, ReadModelReadError> {
        let body = match std::fs::read_to_string(&self.timestamp) {
            Ok(body) => body,
            // 不在は失敗ではない — ストアがまだ無い / 突合相手が渡されていない。
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(ReadModelReadError::new(
                    error.kind(),
                    Some(self.timestamp.clone()),
                ));
            }
        };
        Ok(Some(parse_re_scope(&body)))
    }
}

/// 走査範囲ブロックを解く (upstream `parseReScope`)。
///
/// 未知の版は **malformed** として扱う — 先の版の書き手が書いたものを古い読み手が半分だけ
/// 読んでしまわないためである。ブロックそのものが無いのは **absent** (scope 追跡より前の
/// legacy ストア) で、両者は呼び手が別の文言で報せる。
fn parse_re_scope(body: &str) -> ReScopeParseView {
    let Some(block) = extract_scope_block(body) else {
        return ReScopeParseView::Absent("no fenced yaml scope_version block found".to_string());
    };

    let mut kind = "partial".to_string();
    let mut intent = String::new();
    let mut fingerprint: Option<String> = None;
    let mut analyzed_paths: Vec<String> = Vec::new();
    let mut analyzed_components: Vec<String> = Vec::new();
    let mut shallow_paths: Vec<String> = Vec::new();
    let mut section: Option<Section> = None;
    let mut list: Option<List> = None;
    let mut saw_kind = false;

    for raw in block.split('\n') {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !raw.starts_with(char::is_whitespace) {
            // 字下げの無い行は必ず節の頭であり、直前の節と一覧を閉じる。
            section = None;
            list = None;
            if let Some(value) = trimmed.strip_prefix("scope_version:") {
                let value = value.trim();
                if value != SUPPORTED_SCOPE_VERSION {
                    return ReScopeParseView::Malformed(format!("unknown scope_version: {value}"));
                }
            } else if let Some(value) = trimmed.strip_prefix("kind:") {
                let value = value.trim();
                if value != "full" && value != "partial" {
                    return ReScopeParseView::Malformed(format!(
                        "kind must be full|partial, got: {value}"
                    ));
                }
                kind = value.to_string();
                saw_kind = true;
            } else if let Some(value) = trimmed.strip_prefix("intent:") {
                intent = value.trim().to_string();
            } else if let Some(value) = trimmed.strip_prefix("fingerprint:") {
                let value = value.trim();
                // 空欄と `unknown` はどちらも「記録していない」である。
                fingerprint = (!value.is_empty() && value != "unknown").then(|| value.to_string());
            } else if trimmed == "analyzed:" {
                section = Some(Section::Analyzed);
            } else if trimmed == "shallow:" {
                section = Some(Section::Shallow);
            }
        } else if section.is_some() && !trimmed.starts_with('-') && trimmed.ends_with(':') {
            list = match trimmed {
                "paths:" => Some(List::Paths),
                "components:" => Some(List::Components),
                _ => None,
            };
        } else if let (Some(section), Some(list), Some(item)) =
            (section, list, trimmed.strip_prefix('-'))
        {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            match (section, list) {
                (Section::Analyzed, List::Paths) => analyzed_paths.push(item.to_string()),
                (Section::Analyzed, List::Components) => {
                    analyzed_components.push(item.to_string());
                }
                (Section::Shallow, List::Paths) => shallow_paths.push(item.to_string()),
                (Section::Shallow, List::Components) => {}
            }
        }
    }

    if !saw_kind {
        return ReScopeParseView::Malformed("missing kind: line".to_string());
    }
    let claims_root = analyzed_paths.iter().any(|path| path == "./");
    if kind == "partial" {
        if analyzed_paths.is_empty() {
            return ReScopeParseView::Malformed(
                "kind: partial requires analyzed.paths entries".to_string(),
            );
        }
        if claims_root {
            return ReScopeParseView::Malformed(
                "repository-root coverage (./) requires kind: full".to_string(),
            );
        }
    } else if !claims_root {
        return ReScopeParseView::Malformed(
            "kind: full requires repository-root coverage (analyzed.paths must include ./)"
                .to_string(),
        );
    }

    ReScopeParseView::Parsed(ReScopeView::new(
        kind,
        intent,
        fingerprint,
        analyzed_paths,
        analyzed_components,
        shallow_paths,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Analyzed,
    Shallow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum List {
    Paths,
    Components,
}

/// 本文のどこにあってもよい fenced yaml の走査範囲ブロックを取り出す。
///
/// 見出しではなく **`scope_version:` の行**を鍵にするので、ブロックの周りの散文が編集されても
/// 読める。走査範囲ブロックでない yaml ブロックは閉じ fence の先から探し直す。
fn extract_scope_block(body: &str) -> Option<String> {
    let lines: Vec<&str> = body
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect();
    let mut index = 0;
    while index < lines.len() {
        let opens = lines
            .get(index)
            .is_some_and(|line| matches!(line.trim(), "```yaml" | "```yml"));
        if !opens {
            index += 1;
            continue;
        }
        let mut inner: Vec<&str> = Vec::new();
        let mut cursor = index + 1;
        while let Some(line) = lines.get(cursor) {
            if line.trim() == "```" {
                break;
            }
            inner.push(line);
            cursor += 1;
        }
        let block = inner.join("\n");
        if block.split('\n').any(|line| {
            line.trim_start()
                .strip_prefix("scope_version")
                .is_some_and(|rest| rest.trim_start().starts_with(':'))
        }) {
            return Some(block);
        }
        // 走査範囲ブロックではなかった — 閉じ fence の先から探し直す。
        index = cursor + 1;
    }
    None
}
