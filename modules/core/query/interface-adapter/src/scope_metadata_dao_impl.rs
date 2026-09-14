//! `ScopeMetadataDao` の実 Gateway — `scopes/aidlc-*.md` の frontmatter を読んで写す。
//!
//! 媒体は Markdown frontmatter (compile コンテキストが読む入力の投影 = リードモデル) であり、
//! それを読む実装がクエリ側に在るのは規則どおりである (`coding-rules/cqrs-boundaries.md`
//! 規則 6/7)。読むのは `scope-table` が要る `name` / `depth` / `testStrategy` の 3 つだけである。

use std::path::{Path, PathBuf};

use core_query_use_case::orchestration::{ReadModelReadError, ScopeMetadataDao, ScopeMetadataView};

/// scope 定義メタを読む実装。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeMetadataDaoImpl {
    scopes_dir: PathBuf,
}

impl ScopeMetadataDaoImpl {
    /// scope 定義置き場 (`<harness>/scopes`) を受け取る (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn new(scopes_dir: &Path) -> ScopeMetadataDaoImpl {
        ScopeMetadataDaoImpl {
            scopes_dir: scopes_dir.to_path_buf(),
        }
    }
}

impl ScopeMetadataDao for ScopeMetadataDaoImpl {
    fn find_all(&self) -> Result<Vec<ScopeMetadataView>, ReadModelReadError> {
        let entries = match std::fs::read_dir(&self.scopes_dir) {
            Ok(entries) => entries,
            Err(error) => {
                return Err(ReadModelReadError::new(
                    error.kind(),
                    Some(self.scopes_dir.clone()),
                ));
            }
        };
        let mut rows = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| {
                ReadModelReadError::new(error.kind(), Some(self.scopes_dir.clone()))
            })?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            let raw = std::fs::read_to_string(&path)
                .map_err(|error| ReadModelReadError::new(error.kind(), Some(path.clone())))?;
            if let Some(view) = parse_metadata(&raw) {
                rows.push(view);
            }
        }
        // 行に順序の列は無いので **scope 名の綴り順**で並べる (upstream `validScopes` の `.sort()`)。
        rows.sort_by(|left, right| left.scope().cmp(right.scope()));
        Ok(rows)
    }
}

/// frontmatter (最初の `---` から次の `---` まで) の `name` / `depth` / `testStrategy` を読む。
///
/// `name` が無いファイルは scope 定義ではないので `None` を返す。値は前後の一致する引用符を
/// 剥がす (upstream `coerceScalar` の引用符処理と同じ趣旨)。
fn parse_metadata(raw: &str) -> Option<ScopeMetadataView> {
    let mut lines = raw.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut name: Option<String> = None;
    let mut depth: Option<String> = None;
    let mut test_strategy: Option<String> = None;
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        // リスト項目 (`  - fix`) やインデント行はスカラーではないので飛ばす。
        if line.starts_with(char::is_whitespace) {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = unquote(value.trim());
        match key.trim() {
            "name" if !value.is_empty() => name = Some(value),
            "depth" if !value.is_empty() => depth = Some(value),
            "testStrategy" if !value.is_empty() => test_strategy = Some(value),
            _ => {}
        }
    }
    Some(ScopeMetadataView::new(
        name?,
        depth.unwrap_or_default(),
        test_strategy,
    ))
}

/// 前後の一致する `"` / `'` を剥がす（両端が同じ引用符のときだけ）。
fn unquote(value: &str) -> String {
    for quote in ['"', '\''] {
        if let Some(stripped) = value
            .strip_prefix(quote)
            .and_then(|inner| inner.strip_suffix(quote))
        {
            return stripped.to_string();
        }
    }
    value.to_string()
}
