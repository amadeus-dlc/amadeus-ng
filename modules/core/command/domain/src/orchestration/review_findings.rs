//! `ReviewFindings` — レビュー本文の `### Findings` 表（所見の列）。
use super::{ReviewEvidenceError, ReviewFinding};
use core_infrastructure::collections::FirstClassCollection;
use core_infrastructure::ecmascript::trim;

/// 表が宣言しなければならない列の名前（upstream `aidlc-lib.ts:10736`）。
const COLUMNS: [&str; 6] = [
    "ID",
    "Severity",
    "Location",
    "Finding",
    "Required action",
    "Status",
];

/// レビュー本文から読んだ所見の列（表の行順）。
///
/// 読み方の単一実装である。判定の受理（[`ReviewAppendix`] の検証）と、ゲートへ描く
/// 文脈・レビュー記録の `findings` 欄は、どれもこの型を通して同じ表を同じ規則で読む —
/// 受理は通ったのに記録が読めない、という食い違いを作らないためである。
///
/// [`ReviewAppendix`]: super::review_appendix::ReviewAppendix
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewFindings {
    items: Vec<ReviewFinding>,
}

impl ReviewFindings {
    const fn of_items(items: Vec<ReviewFinding>) -> Self {
        Self { items }
    }

    /// レビュー本文の所見表を読む（upstream 2.8.2 `parseReviewSection` の所見部分）。
    ///
    /// `### Findings` 見出しが無い、表が見出し行だけ、または宣言すべき列が欠けた表は
    /// 「所見なし」と読む（upstream と同じ）。行の形が宣言と食い違ったら黙って読み飛ばさずに
    /// 止める。`artifact` は拒否の文言で成果物を名指すための綴りである。
    ///
    /// # Errors
    ///
    /// 行のセル数が見出しと合わない、ID が `R-<数字>` でない、または状態が語彙の外の場合、
    /// upstream 逐語の理由を持つ [`ReviewEvidenceError::InvalidAppendix`]。
    pub fn parse(review: &str, artifact: &str) -> Result<ReviewFindings, ReviewEvidenceError> {
        let normalized = review.replace("\r\n", "\n");
        let lines: Vec<&str> = normalized.split('\n').collect();
        let Some(heading) = lines.iter().position(|line| {
            line.strip_prefix("### Findings")
                .is_some_and(|rest| trim(rest).is_empty())
        }) else {
            return Ok(ReviewFindings::of_items(Vec::new()));
        };
        let end = lines
            .iter()
            .enumerate()
            .skip(heading.saturating_add(1))
            .find(|(_, line)| line.starts_with("### "))
            .map_or(lines.len(), |(index, _)| index);
        let table: Vec<&str> = lines
            .get(heading.saturating_add(1)..end)
            .unwrap_or_default()
            .iter()
            .copied()
            .filter(|line| trim(line).starts_with('|'))
            .collect();
        let Some((header, body)) = table.split_first() else {
            return Ok(ReviewFindings::of_items(Vec::new()));
        };
        let headers = split_row(header);
        if COLUMNS
            .iter()
            .any(|name| !headers.iter().any(|found| found == name))
        {
            return Ok(ReviewFindings::of_items(Vec::new()));
        }
        let mut items = Vec::new();
        for line in body.iter().skip(1) {
            items.push(ReviewFinding::parse_row(
                &split_row(line),
                &headers,
                artifact,
            )?);
        }
        Ok(ReviewFindings::of_items(items))
    }

    /// 所見の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// 所見が 1 件も無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 表の行順の添字参照。範囲外は `None`（panic しない）。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&ReviewFinding> {
        self.items.get(index)
    }

    /// 表の行順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<'a, A>(
        &'a self,
        initial: A,
        fold: impl FnMut(A, &'a ReviewFinding) -> A,
    ) -> A {
        self.items.iter().fold(initial, fold)
    }

    /// 条件に一致する所見を行順のまま残す。結果は空になり得る。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&ReviewFinding) -> bool) -> ReviewFindings {
        ReviewFindings::of_items(
            self.items
                .iter()
                .filter(|finding| predicate(finding))
                .cloned()
                .collect(),
        )
    }
}

impl FirstClassCollection for ReviewFindings {
    type Item<'a> = &'a ReviewFinding;
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<Self::Item<'_>> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, Self::Item<'a>) -> A) -> A {
        Self::fold_left(self, initial, fold)
    }
    fn filter(&self, predicate: impl FnMut(Self::Item<'_>) -> bool) -> Self {
        Self::filter(self, predicate)
    }
}

/// Markdown 表の 1 行をセルへ割る（upstream `splitMarkdownRow`）。
fn split_row(line: &str) -> Vec<String> {
    let trimmed = trim(line);
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let body = inner.strip_suffix('|').unwrap_or(inner);
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut escaped = false;
    for char in body.chars() {
        if escaped {
            cell.push(char);
            escaped = false;
        } else if char == '\\' {
            escaped = true;
        } else if char == '|' {
            cells.push(trim(&cell).to_string());
            cell.clear();
        } else {
            cell.push(char);
        }
    }
    if escaped {
        cell.push('\\');
    }
    cells.push(trim(&cell).to_string());
    cells
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    fn owned(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|cell| (*cell).to_string()).collect()
    }

    const HEADER: &str = "### Findings\n| ID | Severity | Location | Finding | Required action | Status |\n|---|---|---|---|---|---|\n";

    /// 所見表を行順に読む。宣言に無い列がある表・見出しの無い本文は所見なしと読む。
    #[test]
    fn the_findings_table_is_read_in_row_order() {
        let section = "\n**Verdict:** NOT-READY\n\n### Findings\n\n\
             | ID | Severity | Location | Finding | Required action | Status |\n\
             |---|---|---|---|---|---|\n\
             | R-01 | Major | a.md | 欠落 | 足す | New |\n\
             | R-02 | Minor | b.md | 誤り | 直す | Resolved |\n";
        let findings = ReviewFindings::parse(section, "a.md").expect("読める");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings.at(0).map(ReviewFinding::id), Some("R-01"));
        assert_eq!(findings.at(1).map(ReviewFinding::status), Some("Resolved"));
        assert_eq!(
            findings.at(0).map(ReviewFinding::required_action),
            Some("足す")
        );
        // 宣言に無い列がある表は所見として読まない。
        let none = ReviewFindings::parse(
            "### Findings\n| ID | Status |\n|---|---|\n| R-01 | New |\n",
            "a.md",
        )
        .expect("読める");
        assert!(none.is_empty(), "列が足りない表から所見を作らない");
        // `### Findings` が無ければ所見は無い。
        assert!(
            ReviewFindings::parse("**Verdict:** READY\n", "a.md")
                .expect("読める")
                .is_empty()
        );
        // CRLF の本文も同じに読む。
        let crlf = section.replace('\n', "\r\n");
        assert_eq!(
            ReviewFindings::parse(&crlf, "a.md").expect("読める"),
            findings
        );
    }

    /// セル数・ID・状態のいずれかが宣言と食い違えば、その行を名指して止める。
    #[test]
    fn a_malformed_row_names_the_artifact_and_the_row() {
        let reason = |text: &str| {
            let error = ReviewFindings::parse(text, "a.md").expect_err("行が壊れている");
            assert!(
                matches!(error, ReviewEvidenceError::InvalidAppendix(_)),
                "{error:?}"
            );
            error.to_string()
        };
        let short = reason(&format!("{HEADER}| R-01 | Major | a.md | 欠落 | New |\n"));
        assert!(
            short.contains("a.md#R-01") && short.contains("header declares 6"),
            "{short}"
        );
        let extra = reason(&format!(
            "{HEADER}| R-01 | Major | a.md | 欠落 | 足す | New | 余り |\n"
        ));
        assert!(extra.contains("unexpected extra cell"), "{extra}");
        let bad_id = reason(&format!(
            "{HEADER}| 1 | Major | a.md | 欠落 | 足す | New |\n"
        ));
        assert_eq!(bad_id, "a.md: invalid finding ID \"1\"");
        let bad_status = reason(&format!(
            "{HEADER}| R-01 | Major | a.md | 欠落 | 足す | Maybe |\n"
        ));
        assert_eq!(bad_status, "a.md#R-01: invalid finding status \"Maybe\"");
    }

    /// 次の `###` 見出しで表は終わる。その後ろの壊れた表は読まない。
    #[test]
    fn the_table_ends_at_the_next_sub_heading() {
        let text =
            format!("{HEADER}| R-01 | Major | a.md | 欠落 | 足す | New |\n### Notes\n| x | y |\n");
        assert_eq!(
            ReviewFindings::parse(&text, "a.md").expect("読める").len(),
            1
        );
    }

    /// セルは区切りの逃がしを解く。
    #[test]
    fn a_row_is_split_on_unescaped_separators() {
        assert_eq!(split_row("| a | b\\|c | |"), owned(&["a", "b|c", ""]));
        assert_eq!(split_row("a | b"), owned(&["a", "b"]));
    }

    /// 一級コレクションの操作は行順を保つ。
    #[test]
    fn the_collection_operations_keep_the_row_order() {
        let text = format!(
            "{HEADER}| R-01 | Major | a.md | 欠落 | 足す | New |\n| R-02 | Minor | b.md | 誤り | 直す | Resolved |\n"
        );
        let findings = ReviewFindings::parse(&text, "a.md").expect("読める");
        let ids = findings.fold_left(Vec::new(), |mut ids, finding| {
            ids.push(finding.id().to_string());
            ids
        });
        assert_eq!(ids, owned(&["R-01", "R-02"]));
        let open = findings.filter(|finding| finding.status() == "New");
        assert_eq!(open.len(), 1);
        assert!(findings.at(2).is_none());
    }
}
