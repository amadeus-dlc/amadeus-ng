//! `Checkboxes` — Stage Progress のチェックボックス行群 (一級コレクション)。
//!
//! パース文法は `/^- \[([ xSR?-])\] (\S+)\s*—\s*(.*)$/gm` (upstream `aidlc-lib.ts:6678`,
//! 03 §5.4)。marker writer と suffix writer は同一行の互いに素なフィールドを編集する別 API
//! (recompose と jump が合成できる根拠)。suffix writer の正確な直列化は
//! TODO(golden: stage-0) — 本スライスは marker 側のみ実装する。

use crate::workflow_definition::PlanAction;

use super::checkbox_entry::CheckboxEntry;
use super::checkbox_state::CheckboxState;
use super::checkbox_update_error::CheckboxUpdateError;

/// 行末に書かれる計画側トークン 2 種 (upstream 逐語)。
const PLAN_SUFFIXES: [&str; 2] = ["EXECUTE", "SKIP"];

/// 計画を行末トークンへ写す。
const fn plan_suffix(action: PlanAction) -> &'static str {
    match action {
        PlanAction::Execute => "EXECUTE",
        PlanAction::Skip => "SKIP",
    }
}

/// Stage Progress のチェックボックス行群 — **一級コレクション**。パース・集計・行編集の
/// 全操作を本型が所有する（2026-08-29 是正: 自由関数 `parse_checkboxes` /
/// `count_completed` / `with_checkbox_marker` / `with_checkbox_suffix` を関連メソッドへ収容。
/// 対象（この型）を決めれば `::` で全タスクが見える — OOUI 的プログラミング、
/// `coding-rules/domain-services.md`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkboxes(Vec<CheckboxEntry>);

impl Checkboxes {
    /// Stage Progress 行のパース。文法に一致しない行は黙って無視する
    /// (upstream 同等の寛容パース)。filter/mapはパース済みの列から新しい列を作る。
    #[must_use]
    pub fn parse(content: &str) -> Checkboxes {
        let mut out = Vec::new();
        for line in content.lines() {
            if let Some(entry) = CheckboxEntry::parse_line(line) {
                out.push(entry);
            }
        }
        Checkboxes(out)
    }

    /// 読み取れた行を先頭から辿る。
    pub fn iter(&self) -> impl Iterator<Item = &CheckboxEntry> {
        self.0.iter()
    }

    /// 条件に一致する行を、元の順序で保持する。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&CheckboxEntry) -> bool) -> Self {
        Self(
            self.0
                .iter()
                .filter(|entry| predicate(entry))
                .cloned()
                .collect(),
        )
    }

    /// 各行を変換し、同じ順序のチェックボックス列を返す。
    #[must_use]
    pub fn map(&self, transform: impl FnMut(&CheckboxEntry) -> CheckboxEntry) -> Self {
        Self(self.0.iter().map(transform).collect())
    }

    /// 行順に左から畳み込む。空なら初期値を返す。
    pub fn fold_left<'a, A>(
        &'a self,
        initial: A,
        fold: impl FnMut(A, &'a CheckboxEntry) -> A,
    ) -> A {
        self.0.iter().fold(initial, fold)
    }

    /// slugに一致する最初の行。重複行の順序を変更しない。
    #[must_use]
    pub fn find(&self, slug: &str) -> Option<&CheckboxEntry> {
        self.0.iter().find(|entry| entry.slug() == slug)
    }

    /// 同じslugの行のうち、完了済みの行があるか。
    #[must_use]
    pub fn has_completed(&self, slug: &str) -> bool {
        self.0
            .iter()
            .any(|entry| entry.slug() == slug && entry.state() == CheckboxState::Completed)
    }

    /// 行数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// 1 行も無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 位置指定の参照（範囲外は `None`）。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&CheckboxEntry> {
        self.0.get(index)
    }

    /// `Completed` フィールド同期のための集計 (upstream `countCheckboxes`)。
    #[must_use]
    pub fn count_completed(&self) -> usize {
        self.fold_left(0, |count, entry| {
            count + usize::from(entry.state() == CheckboxState::Completed)
        })
    }

    /// marker のみを書き換える (rest と suffix には触れない — 互いに素なフィールド編集)。
    ///
    /// テキスト → テキストの verbatim 編集なので `&self` ではなく内容文字列を取る
    /// (元の行の空白配置を 1 文字も動かさないため、パース済み表現からの再構成はしない)。
    ///
    /// # Errors
    ///
    /// 対象 slug の行が存在しなければ `MissingStage`。
    pub fn with_marker(
        content: &str,
        slug: &str,
        state: CheckboxState,
    ) -> Result<String, CheckboxUpdateError> {
        let mut found = false;
        let mut out: Vec<String> = Vec::new();
        for line in content.lines() {
            match CheckboxEntry::parse_line(line) {
                Some(entry) if entry.slug() == slug => {
                    found = true;
                    // 元の行の marker 1 文字だけを置換 (前後は verbatim 保存)
                    let prefix_len = "- [".len();
                    let mut rebuilt = String::with_capacity(line.len());
                    rebuilt.push_str(&line[..prefix_len]);
                    rebuilt.push(state.marker());
                    let after_marker = line[prefix_len..]
                        .chars()
                        .next()
                        .map(char::len_utf8)
                        .unwrap_or(1);
                    rebuilt.push_str(&line[prefix_len + after_marker..]);
                    out.push(rebuilt);
                }
                _ => out.push(line.to_string()),
            }
        }
        if !found {
            return Err(CheckboxUpdateError::MissingStage(slug.to_string()));
        }
        let mut result = out.join("\n");
        if content.ends_with('\n') {
            result.push('\n');
        }
        Ok(result)
    }

    /// 計画側の EXECUTE/SKIP サフィックスのみを書き換える (marker と rest の前半には触れない)。
    ///
    /// marker writer とは**同一行の互いに素なフィールドを編集する別 API** である
    /// (11-workspace §2.2 `CheckboxLine`)。recompose がサフィックスを、jump がマーカーを動かす —
    /// 別 API だからこそ 2 つの編集が合成できる。
    ///
    /// # Errors
    ///
    /// 対象 slug の行が存在しなければ `MissingStage`。行末が EXECUTE / SKIP のどちらでもなければ
    /// `MissingSuffix` (書き換え先が無いのに無言で no-op しない)。
    pub fn with_suffix(
        content: &str,
        slug: &str,
        action: PlanAction,
    ) -> Result<String, CheckboxUpdateError> {
        let mut found = false;
        let mut out: Vec<String> = Vec::new();
        for line in content.lines() {
            match CheckboxEntry::parse_line(line) {
                Some(entry) if entry.slug() == slug => {
                    found = true;
                    let trimmed = line.trim_end();
                    let Some(head) = PLAN_SUFFIXES
                        .iter()
                        .find_map(|suffix| trimmed.strip_suffix(suffix))
                    else {
                        return Err(CheckboxUpdateError::MissingSuffix(slug.to_string()));
                    };
                    out.push(format!("{head}{}", plan_suffix(action)));
                }
                _ => out.push(line.to_string()),
            }
        }
        if !found {
            return Err(CheckboxUpdateError::MissingStage(slug.to_string()));
        }
        let mut result = out.join("\n");
        if content.ends_with('\n') {
            result.push('\n');
        }
        Ok(result)
    }
}

impl core_infrastructure::collections::FirstClassCollection for Checkboxes {
    type Item<'a> = &'a CheckboxEntry;
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<&CheckboxEntry> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a CheckboxEntry) -> A) -> A {
        Self::fold_left(self, initial, fold)
    }
    fn filter(&self, predicate: impl FnMut(&CheckboxEntry) -> bool) -> Self {
        Self::filter(self, predicate)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing)]
    #[test]
    fn completed_lookup_considers_all_duplicate_rows_but_find_uses_the_first() {
        let entries = Checkboxes::parse("- [ ] a — SKIP\n- [x] a — EXECUTE\n");
        assert!(entries.has_completed("a"));
        assert!(!entries.has_completed("missing"));
        assert_eq!(entries.find("a").unwrap().state(), CheckboxState::Pending);
    }
    #[test]
    fn collection_operations_keep_order_and_return_a_collection() {
        use super::*;
        let entries = Checkboxes::parse("- [x] a — EXECUTE\n- [ ] b — SKIP\n- [x] c — EXECUTE\n");
        let completed = entries.filter(|entry| entry.state() == CheckboxState::Completed);
        assert_eq!(completed.len(), 2);
        assert_eq!(completed.at(1).unwrap().slug(), "c");
        assert_eq!(
            completed.fold_left(String::new(), |acc, entry| acc + entry.slug()),
            "ac"
        );
        let renamed = completed.map(|entry| {
            CheckboxEntry::new(
                entry.state(),
                format!("{}-copy", entry.slug()),
                entry.rest(),
            )
        });
        assert_eq!(renamed.at(0).unwrap().slug(), "a-copy");
        assert_eq!(entries.at(0).unwrap().slug(), "a");
        assert!(entries.at(usize::MAX).is_none());
        assert!(entries.filter(|_| false).at(0).is_none());
        assert_eq!(entries.find("b").unwrap().state(), CheckboxState::Pending);
        assert!(entries.find("absent").is_none());
        let empty = entries.filter(|_| false);
        let count = |acc, _: &CheckboxEntry| acc + 1;
        assert_eq!(empty.fold_left(7, count), 7);
        assert_eq!(entries.fold_left(7, count), 10);
        let mut calls = 0;
        let mut transform = |entry: &CheckboxEntry| {
            calls += 1;
            entry.clone()
        };
        assert!(empty.map(&mut transform).is_empty());
        assert_eq!(entries.map(&mut transform), entries);
        assert_eq!(calls, 3);
    }

    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
    // allow)。
    use super::*;

    /// checkbox 行の文法を外れた行は checkbox として読まない (材料が揃わない行を拾わない)。
    #[test]
    fn a_line_that_misses_any_part_of_the_grammar_is_not_a_checkbox() {
        for line in [
            "- [",                    // マーカーが無い
            "- [x] slug",             // 全角ダッシュが無い
            "- [x]slug — tail",       // `] ` の空白が無い
            "- [x]  — tail",          // slug が空
            "- [x] two words — tail", // slug に空白が混ざる
        ] {
            assert!(
                Checkboxes::parse(line).is_empty(),
                "checkbox として読んではいけない: {line:?}"
            );
        }
    }

    /// 末尾に改行が無い本文の書き換えは、改行を足さずに返す (逐語維持)。
    #[test]
    fn rewriting_a_body_without_a_trailing_newline_keeps_it_that_way() {
        let content = "## Stage Progress\n- [ ] state-init — Not started";
        let updated = Checkboxes::with_marker(content, "state-init", CheckboxState::Completed)
            .expect("stage はある");
        assert_eq!(updated, "## Stage Progress\n- [x] state-init — Not started");
        assert!(!updated.ends_with('\n'));
    }

    const SAMPLE: &str = "\
## Stage Progress

### IDEATION PHASE
- [x] intent-capture — Intent Capture — EXECUTE
- [-] requirements-analysis — Requirements Analysis — EXECUTE
- [ ] domain-modeling — Domain Modeling — SKIP
- [?] gated-stage — Gated — EXECUTE
- [R] revising-stage — Revising — EXECUTE
- [S] skipped-stage — Skipped — EXECUTE
not a checkbox line
";

    #[test]
    fn parses_all_six_marker_states_and_ignores_non_matching_lines() {
        let entries = Checkboxes::parse(SAMPLE);
        assert_eq!(entries.len(), 6);
        assert_eq!(entries.at(0).unwrap().state(), CheckboxState::Completed);
        assert_eq!(entries.at(1).unwrap().state(), CheckboxState::InProgress);
        assert_eq!(entries.at(2).unwrap().state(), CheckboxState::Pending);
        assert_eq!(
            entries.at(3).unwrap().state(),
            CheckboxState::AwaitingApproval
        );
        assert_eq!(entries.at(4).unwrap().state(), CheckboxState::Revising);
        assert_eq!(entries.at(5).unwrap().state(), CheckboxState::Skipped);
        assert_eq!(entries.at(0).unwrap().slug(), "intent-capture");
        assert_eq!(entries.at(2).unwrap().rest(), "Domain Modeling — SKIP");
    }

    #[test]
    fn iteration_walks_entries_in_document_order() {
        let entries = Checkboxes::parse("- [x] a — A EXECUTE\n- [ ] b — B SKIP\n");
        let slugs: Vec<&str> = entries.iter().map(CheckboxEntry::slug).collect();
        assert_eq!(slugs, ["a", "b"]);
        assert_eq!(entries.len(), 2);
        assert!(!entries.is_empty());
        assert!(entries.at(9).is_none(), "範囲外は None");
    }

    #[test]
    fn a_line_with_an_unknown_marker_is_ignored() {
        // 閉集合 [ xSR?-] の外 (`z`) は checkbox 行と見なさない (寛容パース)。
        assert!(Checkboxes::parse("- [z] a — A EXECUTE\n").is_empty());
    }

    #[test]
    fn with_checkbox_marker_edits_only_the_marker_and_preserves_the_rest_verbatim() {
        let updated =
            Checkboxes::with_marker(SAMPLE, "requirements-analysis", CheckboxState::Completed)
                .unwrap();
        assert!(updated.contains("- [x] requirements-analysis — Requirements Analysis — EXECUTE"));
        // 他の行は不変
        assert!(updated.contains("- [ ] domain-modeling — Domain Modeling — SKIP"));
        assert_eq!(Checkboxes::parse(&updated).count_completed(), 2);
    }

    #[test]
    fn with_checkbox_marker_refuses_missing_stages() {
        assert_eq!(
            Checkboxes::with_marker(SAMPLE, "no-such-stage", CheckboxState::Completed),
            Err(CheckboxUpdateError::MissingStage("no-such-stage".into()))
        );
    }

    #[test]
    fn the_suffix_writer_replaces_only_the_plan_token() {
        // recompose がサフィックスを動かす。マーカーと表題は verbatim のまま残る
        // (marker writer とは互いに素なフィールド編集)。
        let before = "- [ ] incident-response — EXECUTE\n- [x] practices-discovery — EXECUTE\n";
        let after = Checkboxes::with_suffix(before, "incident-response", PlanAction::Skip).unwrap();
        assert_eq!(
            after,
            "- [ ] incident-response — SKIP\n- [x] practices-discovery — EXECUTE\n"
        );
    }

    #[test]
    fn the_suffix_writer_leaves_a_title_between_the_slug_and_the_token() {
        let before = "- [-] requirements-analysis — Requirements Analysis — EXECUTE\n";
        let after =
            Checkboxes::with_suffix(before, "requirements-analysis", PlanAction::Skip).unwrap();
        assert_eq!(
            after,
            "- [-] requirements-analysis — Requirements Analysis — SKIP\n"
        );
    }

    #[test]
    fn the_two_writers_compose_on_the_same_line() {
        // jump がマーカーを、recompose がサフィックスを動かしても互いを潰さない。
        let before = "- [ ] incident-response — EXECUTE\n";
        let marked =
            Checkboxes::with_marker(before, "incident-response", CheckboxState::Skipped).unwrap();
        let both = Checkboxes::with_suffix(&marked, "incident-response", PlanAction::Skip).unwrap();
        assert_eq!(both, "- [S] incident-response — SKIP\n");
    }

    #[test]
    fn the_suffix_writer_refuses_a_missing_stage_and_a_missing_token() {
        assert_eq!(
            Checkboxes::with_suffix("- [ ] other — EXECUTE\n", "absent", PlanAction::Skip),
            Err(CheckboxUpdateError::MissingStage("absent".to_string()))
        );
        // 無言 no-op は検出不能なドリフトなので、書き換え先が無ければ止める。
        assert_eq!(
            Checkboxes::with_suffix("- [ ] here — Something Else\n", "here", PlanAction::Skip),
            Err(CheckboxUpdateError::MissingSuffix("here".to_string()))
        );
    }
}
