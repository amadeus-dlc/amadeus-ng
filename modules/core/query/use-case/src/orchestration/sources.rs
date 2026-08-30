//! ユースケースが受け取る**読み終えた**読取素材の分類。
//!
//! クエリ側のユースケースはポートを 1 本も持たない (`coding-rules/cqrs-boundaries.md`
//! 規則 6 / use-case-rules §4 の 2026-08-30 夕・再々裁定 — 「読取専用ポートの注入」という
//! 型保証の手法は、読むだけの動詞がクエリ側へ移ったことで対象を失って失効した)。読取そのものは
//! Controller (U7) が行い、その**結果**を値で渡す。
//!
//! 読取が 3 通り (在る / 無い / 読めない) に分かれるのは観測可能な契約であり、`Option` で
//! 潰すと「無い」と「読めない」が同じ扱いになってしまう — 前者は誕生分岐へ、後者は逐語で
//! 止める、と行き先が違う。ルール束だけは「無い」が正常 (列に現れないだけ) なので 2 値である。

use super::memory_rules::MemoryRules;
use crate::execution_view::ExecutionStateView;
use crate::workflow_view::DefinitionView;

/// 実行状態リードモデル (`aidlc-state.md`) の読取結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStateSource<'a> {
    /// active-intent カーソルが無い — 稼働中ワークフローが存在しない (誕生分岐の群へ)。
    Missing,
    /// 読み終えたリードモデル。
    Loaded(&'a ExecutionStateView),
    /// 読取・復号に失敗した。逐語文言を運び、カーソルを使う前に止める
    /// (旧 state バージョンガードの相当)。
    Unreadable(&'a str),
}

/// ワークフロー定義リードモデル (3 入力) の読取結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionSource<'a> {
    /// 読み終えた定義ビュー。
    Loaded(&'a DefinitionView),
    /// どの定義を読むべきかが決まらない (state も harness の指定も無い)。
    Unidentified,
    /// 定義リードモデルが読めない。逐語文言を運ぶ。
    Unreadable(&'a str),
}

/// memory 層ルール束 (決定論的 steering) の読取結果。
///
/// ファイルが**無い**のは正常なので、ここには来ない — 無いファイルは単に
/// [`MemoryRules`] の列に現れない (02 §10 / b24 の読み順)。分類が捉えるのは
/// 「在るのに読めない」だけであり、それは blocking で `error` directive になる。
///
/// 分割不能セクションはここに現れない — 読取時ではなく
/// [`MemoryRules::plan_for`] のパック時に判明するので、ユースケースがその `Err` を
/// `error` directive へ写す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SteeringSource<'a> {
    /// 読み終えたルール束 (空も正常 — ルール未整備は空計画 = bare run-stage)。
    Loaded(&'a MemoryRules),
    /// 必須ルールファイルが在るのに読めない (権限・UTF-8 破損)。材料のみを運び、
    /// 文言はユースケースの `wording` が組む。
    Unreadable {
        /// 読もうとしたパス。
        path: &'a str,
        /// 失敗の理由 (OS 由来)。
        cause: &'a str,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::test_fixtures::{definition, genesis_state};

    #[test]
    fn the_three_state_outcomes_are_distinct() {
        let held = genesis_state(2);
        assert_ne!(
            ExecutionStateSource::Missing,
            ExecutionStateSource::Loaded(&held)
        );
        assert_ne!(
            ExecutionStateSource::Missing,
            ExecutionStateSource::Unreadable("boom")
        );
    }

    #[test]
    fn a_readable_bundle_and_an_unreadable_one_are_distinct() {
        let held = MemoryRules::default();
        assert_ne!(
            SteeringSource::Loaded(&held),
            SteeringSource::Unreadable {
                path: "memory/org.md",
                cause: "permission denied",
            }
        );
    }

    #[test]
    fn the_three_definition_outcomes_are_distinct() {
        let held = definition(2);
        assert_ne!(
            DefinitionSource::Unidentified,
            DefinitionSource::Loaded(&held)
        );
        assert_ne!(
            DefinitionSource::Unidentified,
            DefinitionSource::Unreadable("boom")
        );
    }
}
