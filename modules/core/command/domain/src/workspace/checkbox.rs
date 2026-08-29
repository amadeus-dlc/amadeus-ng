//! `CheckboxState` / Stage Progress 行 — 6 状態マーカー＋ em dash 区切り。パース文法は
//! `/^- \[([ xSR?-])\] (\S+)\s*—\s*(.*)$/gm` (upstream `aidlc-lib.ts:6678`, 03 §5.4)。
//! marker writer と suffix writer は同一行の互いに素なフィールドを編集する別 API
//! (recompose と jump が合成できる根拠)。suffix writer の正確な直列化は
//! TODO(golden: stage-0) — 本スライスは marker 側のみ実装する。

use serde::{Deserialize, Serialize};

use crate::workflow_definition::PlanAction;

/// 6 状態 (01 §3.3 — E1)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CheckboxState {
    /// `[ ]` = upstream `pending` — 未着手。
    Pending,
    /// `[-]` = upstream `in-progress` — 実行中 (ゲートはまだ開いていない)。
    InProgress,
    /// `[?]` = upstream `awaiting-approval` — 承認ゲート開放済み (`[-]` → `[?]`)。
    AwaitingApproval,
    /// `[R]` = upstream `revising` — 差戻し後の改訂中。ゲートに再入できる唯一の状態。
    Revising,
    /// `[x]` = upstream `completed` — 完了。`Completed` フィールド同期の集計対象。
    Completed,
    /// `[S]` = upstream `skipped` — 経路上の帰結としての読み飛ばし (完了ではない)。
    Skipped,
}

impl CheckboxState {
    /// 行に書かれる 1 文字マーカー。`from_marker` の逆写像であり 6 状態と 1:1
    /// (往復忠実: `from_marker(s.marker()) == Some(s)`)。
    #[must_use]
    pub const fn marker(self) -> char {
        match self {
            CheckboxState::Pending => ' ',
            CheckboxState::InProgress => '-',
            CheckboxState::AwaitingApproval => '?',
            CheckboxState::Revising => 'R',
            CheckboxState::Completed => 'x',
            CheckboxState::Skipped => 'S',
        }
    }

    /// マーカー 1 文字から状態へ。閉集合 `[ xSR?-]` の外は `None` — 呼出側 (`CheckboxEntry::parse_line`) は
    /// その行を checkbox 行と見なさない。
    #[must_use]
    pub const fn from_marker(c: char) -> Option<CheckboxState> {
        Some(match c {
            ' ' => CheckboxState::Pending,
            '-' => CheckboxState::InProgress,
            '?' => CheckboxState::AwaitingApproval,
            'R' => CheckboxState::Revising,
            'x' => CheckboxState::Completed,
            'S' => CheckboxState::Skipped,
            _ => return None,
        })
    }

    // ---- 分類述語 (マーカー語彙の分類は本型が所有する — Tell, Don't Ask)。
    //      呼出側で変種集合を再列挙しないこと。 ----

    /// 未終了 (in-flight)。upstream `next` の in-flight 判定 (02 §5.1 手順 10-2) の集合:
    /// `pending / in-progress / awaiting-approval / revising`。
    /// 「checkbox 行の欠落も in-flight」の扱いは行の存否を知る呼出側の責務。
    #[must_use]
    pub const fn is_in_flight(self) -> bool {
        !self.is_finished()
    }

    /// 終了済み (`completed` / `skipped`) — 前進走査 (`nextInScopeStage`) が読み飛ばす集合。
    #[must_use]
    pub const fn is_finished(self) -> bool {
        matches!(self, CheckboxState::Completed | CheckboxState::Skipped)
    }

    /// 着手済み (in-flight のうち `pending` を除く: `in-progress / awaiting-approval /
    /// revising`)。jump forward が skipped 化する「現ステージ」の条件 (09 §3)。
    #[must_use]
    pub const fn is_active(self) -> bool {
        self.is_in_flight() && !matches!(self, CheckboxState::Pending)
    }
}

/// パース済みの Stage Progress 行 — マーカー / stage slug / em dash 以降のテキストの 3 分割。
/// 元の行の空白配置は保持しない (書き戻しは `Checkboxes::with_marker` が元の行を verbatim に扱う)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckboxEntry {
    state: CheckboxState,
    slug: String,
    /// em dash 以降のテキスト (verbatim 保存 — title や EXECUTE/SKIP サフィックスを含む)。
    rest: String,
}

impl CheckboxEntry {
    /// 3 成分から組む。行文法の検査は行わない (検査済みの値を運ぶ入れ物であり、行の正本は
    /// `Checkboxes::parse`)。
    #[must_use]
    pub fn new(
        state: CheckboxState,
        slug: impl Into<String>,
        rest: impl Into<String>,
    ) -> CheckboxEntry {
        CheckboxEntry {
            state,
            slug: slug.into(),
            rest: rest.into(),
        }
    }

    /// マーカーが表す run-state (計画側の EXECUTE/SKIP サフィックスとは別フィールド)。
    #[must_use]
    pub const fn state(&self) -> CheckboxState {
        self.state
    }

    /// stage slug — 行の識別子。空白を含まない 1 トークンで、`Checkboxes::with_marker` の照合キー。
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// em dash 以降のテキスト (verbatim 保存 — title や EXECUTE/SKIP サフィックスを含む)。
    #[must_use]
    pub fn rest(&self) -> &str {
        &self.rest
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
    /// (upstream 同等の寛容パース — 唯一の構築子)。
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
    pub fn get(&self, index: usize) -> Option<&CheckboxEntry> {
        self.0.get(index)
    }

    /// `Completed` フィールド同期のための集計 (upstream `countCheckboxes`)。
    #[must_use]
    pub fn count_completed(&self) -> usize {
        self.0
            .iter()
            .filter(|e| e.state() == CheckboxState::Completed)
            .count()
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

impl CheckboxEntry {
    /// 1 行のパース (行文法の正本)。文法に一致しない行は `None`。
    fn parse_line(line: &str) -> Option<CheckboxEntry> {
        // `- [<m>] <slug>\s*—\s*<rest>`
        let rest = line.strip_prefix("- [")?;
        let mut chars = rest.chars();
        let marker = chars.next()?;
        let state = CheckboxState::from_marker(marker)?;
        let rest = chars.as_str().strip_prefix("] ")?;
        let dash = rest.find('—')?;
        let (slug_part, tail) = rest.split_at(dash);
        let slug = slug_part.trim_end_matches([' ', '\t']);
        if slug.is_empty() || slug.contains(char::is_whitespace) {
            return None;
        }
        let tail = tail.strip_prefix('—').unwrap_or(tail);
        let tail = tail.trim_start_matches([' ', '\t']);
        Some(CheckboxEntry::new(state, slug, tail))
    }
}

/// marker writer (`Checkboxes::with_marker`) の拒否理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckboxUpdateError {
    /// 対象 slug の行が存在しない。
    MissingStage(String),
    /// 対象行の末尾が EXECUTE / SKIP のどちらでもない (書き換え先が無い)。
    MissingSuffix(String),
}

/// 行末に書かれる計画側トークン 2 種 (upstream 逐語)。
const PLAN_SUFFIXES: [&str; 2] = ["EXECUTE", "SKIP"];

/// 計画を行末トークンへ写す。
const fn plan_suffix(action: PlanAction) -> &'static str {
    match action {
        PlanAction::Execute => "EXECUTE",
        PlanAction::Skip => "SKIP",
    }
}

#[cfg(test)]
mod tests {

    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
    // allow)。
    #![allow(clippy::indexing_slicing)]

    use super::*;

    /// マーカー 1 文字は 6 状態の閉集合と 1:1 に対応する (往復忠実)。
    ///
    /// この対応は state ファイルの逐語形そのものであり、ワイヤ形式のラウンドトリップ PBT が
    /// 副次的に踏んでいた経路でもある (Bolt B6 でワイヤ形式を退役させたので、ここで直接
    /// 固定し直す)。
    #[test]
    fn every_marker_round_trips_through_the_closed_set() {
        for (state, marker) in [
            (CheckboxState::Pending, ' '),
            (CheckboxState::InProgress, '-'),
            (CheckboxState::AwaitingApproval, '?'),
            (CheckboxState::Revising, 'R'),
            (CheckboxState::Completed, 'x'),
            (CheckboxState::Skipped, 'S'),
        ] {
            assert_eq!(state.marker(), marker, "{state:?}");
            assert_eq!(
                CheckboxState::from_marker(marker),
                Some(state),
                "{marker:?}"
            );
        }
    }

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

    #[test]
    fn a_marker_outside_the_closed_set_is_not_a_checkbox() {
        for marker in ['X', 'r', '*', '\u{3000}'] {
            assert_eq!(CheckboxState::from_marker(marker), None, "{marker:?}");
        }
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
        assert_eq!(entries.get(0).unwrap().state(), CheckboxState::Completed);
        assert_eq!(entries.get(1).unwrap().state(), CheckboxState::InProgress);
        assert_eq!(entries.get(2).unwrap().state(), CheckboxState::Pending);
        assert_eq!(
            entries.get(3).unwrap().state(),
            CheckboxState::AwaitingApproval
        );
        assert_eq!(entries.get(4).unwrap().state(), CheckboxState::Revising);
        assert_eq!(entries.get(5).unwrap().state(), CheckboxState::Skipped);
        assert_eq!(entries.get(0).unwrap().slug(), "intent-capture");
        assert_eq!(entries.get(2).unwrap().rest(), "Domain Modeling — SKIP");
    }

    #[test]
    fn iteration_walks_entries_in_document_order() {
        let entries = Checkboxes::parse("- [x] a — A EXECUTE\n- [ ] b — B SKIP\n");
        let slugs: Vec<&str> = entries.iter().map(CheckboxEntry::slug).collect();
        assert_eq!(slugs, ["a", "b"]);
        assert_eq!(entries.len(), 2);
        assert!(!entries.is_empty());
        assert!(entries.get(9).is_none(), "範囲外は None");
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
    fn classification_predicates_partition_the_six_markers() {
        use CheckboxState::{AwaitingApproval, Completed, InProgress, Pending, Revising, Skipped};
        let all = [
            Pending,
            InProgress,
            AwaitingApproval,
            Revising,
            Completed,
            Skipped,
        ];
        for cb in all {
            // in-flight と finished は補集合
            assert_ne!(cb.is_in_flight(), cb.is_finished(), "{cb:?}");
            // active = in-flight ∧ ¬pending
            assert_eq!(cb.is_active(), cb.is_in_flight() && cb != Pending, "{cb:?}");
        }
        assert!(Pending.is_in_flight() && !Pending.is_active());
        assert!(Revising.is_active());
        assert!(Completed.is_finished() && Skipped.is_finished());
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
