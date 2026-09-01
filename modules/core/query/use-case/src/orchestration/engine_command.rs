//! `EngineCommand` — `next` が人間・conductor へ名指しするエンジンコマンドの概念と綴り。
//!
//! 概念 (どの操作を指しているか) と綴り (upstream 3 形のうち self-host 正準の
//! **素のマルチコール形** — 例 `aidlc-utility status`、`07-hooks.md:260` に実在し ADR 0002
//! 決定 3) はどちらも読み手の閉じた出力語彙である。綴りの導出は CPU とメモリだけの純計算
//! なのでポートにしない。ディスパッチャ語彙の完全 ROUTES 写し (30 経路 +
//! SLASH_FLAG_ALIASES) は U7 / A1 で表として実体化し、差し替えは
//! [`EngineCommand::cli_spelling`] 1 点で行う (逸脱台帳 #1)。

use super::read_only_verb::ReadOnlyVerb;
use crate::orchestration::{ScopeSlugView, StageSlugView};

/// `next` が名指しするエンジンコマンドの閉じた語彙。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineCommand {
    /// 読み取り専用ユーティリティ (分岐 1)。
    ReadOnlyUtility(ReadOnlyVerb),
    /// 名詞トークン列の逐語通し (分岐 1b/1c/1d — 人間の語をそのまま運ぶ)。
    NounTokens(Vec<String>),
    /// park 解除 (分岐 2.6)。
    Unpark,
    /// jump の純読み取り解決 (分岐 7)。
    ResolveJump {
        /// ジャンプ先ステージ。
        stage: StageSlugView,
    },
    /// intent の鋳造 (birth — `next` は自身で実行しない)。
    ///
    /// 引数面は upstream `createPrintDirective` (`aidlc-orchestrate.ts:878-894`) の完全形で
    /// ある — 自由記述は `--arguments=` に shell-quote して載せ、conductor が畳む `--label`
    /// プレースホルダを続け、`--depth` / `--test-strategy` / `--review` は与えられた分だけ
    /// この順で並べる。
    MintIntent {
        /// 鋳造する intent の scope。
        scope: ScopeSlugView,
        /// 新規作業の自由記述 (`--arguments=`)。空・不在なら `--arguments` も `--label` も
        /// 出さない (upstream `if (description && description.length > 0)`)。
        description: Option<String>,
        /// `--depth` の値 (人間が与えた逐語)。
        depth: Option<String>,
        /// `--test-strategy` の値 (人間が与えた逐語)。
        test_strategy: Option<String>,
        /// `--review` の値 (人間が与えた逐語)。
        review: Option<String>,
    },
    /// scope 変更の名指し (分岐 5 — upstream `:3051-3054` の引数面)。
    ///
    /// 併記された設定修飾は同じ 1 本の命令へ載る (upstream は 1 回の実行で両方を適用する)。
    ChangeScope {
        /// 変更先 scope。
        scope: ScopeSlugView,
        /// `--depth` の値 (人間が与えた逐語)。
        depth: Option<String>,
        /// `--test-strategy` の値 (人間が与えた逐語)。
        test_strategy: Option<String>,
        /// `--review` の値 (人間が与えた逐語)。
        review: Option<String>,
    },
    /// depth / test-strategy / review の設定変更の名指し (分岐 5 — upstream `:3067-3070`)。
    ///
    /// 与えられた修飾を**まとめて 1 本**に載せる。フィールドごとに命令を分けない。
    ChangeConfig {
        /// `--depth` の値 (人間が与えた逐語)。
        depth: Option<String>,
        /// `--test-strategy` の値 (人間が与えた逐語)。
        test_strategy: Option<String>,
        /// `--review` の値 (人間が与えた逐語)。
        review: Option<String>,
    },
    /// composer ディスパッチの名指し (分岐 4c)。
    DispatchComposer,
    /// stale ポインタの回復報告 (分岐 10 手順 3 — SKIP なのにカーソルが残っている)。
    ///
    /// `--reason` の値は upstream が定数で持つ逐語である (`aidlc-orchestrate.ts:3292`)。
    ReportSkipped {
        /// 回復対象のステージ。
        stage: StageSlugView,
    },
}

/// stale ポインタ回復の理由文言 (upstream `:3292` の `reason` 定数)。
const SKIP_REASON: &str = "stage is SKIP in the approved workflow plan";

impl EngineCommand {
    /// コマンド概念を CLI 綴り (マルチコール正準形) に写す。
    #[must_use]
    pub fn cli_spelling(&self) -> String {
        match self {
            EngineCommand::ReadOnlyUtility(verb) => {
                format!("aidlc-utility {}", read_only_subcommand(*verb))
            }
            EngineCommand::NounTokens(tokens) => {
                format!("aidlc-utility {}", tokens.join(" "))
            }
            EngineCommand::Unpark => "aidlc-state unpark".to_string(),
            EngineCommand::ResolveJump { stage } => {
                format!("aidlc-jump resolve --stage {}", stage.as_str())
            }
            // ラベルは conductor が置換するプレースホルダ付き。
            EngineCommand::MintIntent {
                scope,
                description,
                depth,
                test_strategy,
                review,
            } => mint_intent_spelling(
                scope,
                description.as_deref(),
                depth.as_deref(),
                test_strategy.as_deref(),
                review.as_deref(),
            ),
            EngineCommand::ChangeScope {
                scope,
                depth,
                test_strategy,
                review,
            } => {
                let mut spelled = format!("aidlc-utility scope-change --scope {}", scope.as_str());
                push_modifiers(
                    &mut spelled,
                    depth.as_deref(),
                    test_strategy.as_deref(),
                    review.as_deref(),
                );
                spelled
            }
            EngineCommand::ChangeConfig {
                depth,
                test_strategy,
                review,
            } => {
                let mut spelled = "aidlc-utility config-change".to_string();
                push_modifiers(
                    &mut spelled,
                    depth.as_deref(),
                    test_strategy.as_deref(),
                    review.as_deref(),
                );
                spelled
            }
            EngineCommand::DispatchComposer => "aidlc-composer detect".to_string(),
            EngineCommand::ReportSkipped { stage } => format!(
                "aidlc-orchestrate report --stage {} --result skipped --reason {}",
                shell_arg(stage.as_str()),
                shell_arg(SKIP_REASON)
            ),
        }
    }
}

/// 設定修飾を upstream の push 順 (`--depth` → `--test-strategy` → `--review`) で足す。
fn push_modifiers(
    spelled: &mut String,
    depth: Option<&str>,
    test_strategy: Option<&str>,
    review: Option<&str>,
) {
    if let Some(depth) = depth {
        spelled.push_str(&format!(" --depth {depth}"));
    }
    if let Some(test_strategy) = test_strategy {
        spelled.push_str(&format!(" --test-strategy {test_strategy}"));
    }
    if let Some(review) = review {
        spelled.push_str(&format!(" --review {review}"));
    }
}

/// `intent-create` の綴り (upstream `createPrintDirective` `:879-894` の引数組み立て逐語)。
///
/// `--arguments` と `--label` は自由記述があるときだけ対で出る — ラベルは記述を畳んだ
/// 短い名前なので、畳む元が無ければ求める意味が無い。任意フラグは upstream の push 順
/// (`--depth` → `--test-strategy` → `--review`) を保つ。
fn mint_intent_spelling(
    scope: &ScopeSlugView,
    description: Option<&str>,
    depth: Option<&str>,
    test_strategy: Option<&str>,
    review: Option<&str>,
) -> String {
    let mut spelled = format!("aidlc-utility intent-create --scope {}", scope.as_str());
    if let Some(description) = description.filter(|text| !text.is_empty()) {
        spelled.push_str(&format!(" --arguments={}", shell_arg(description)));
        spelled.push_str(" --label \"<2-3 word kebab essence>\"");
    }
    push_modifiers(&mut spelled, depth, test_strategy, review);
    spelled
}

/// upstream `shellArg` (`aidlc-orchestrate.ts:644-647`) の逐語移植。
///
/// 安全文字だけで出来た非空文字列は裸のまま、それ以外は単一引用符で括り、内側の単一引用符は
/// `'"'"'` へ展開する。空文字列は安全集合の `+`(1 文字以上) に合致しないので `''` になる。
fn shell_arg(value: &str) -> String {
    if !value.is_empty() && value.chars().all(is_shell_safe) {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

/// upstream の安全文字クラス `[A-Za-z0-9_./:@%+=,-]` (ASCII のみ)。
const fn is_shell_safe(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(
            character,
            '_' | '.' | '/' | ':' | '@' | '%' | '+' | '=' | ',' | '-'
        )
}

/// 読み取り専用ユーティリティのサブコマンド綴り。
const fn read_only_subcommand(verb: ReadOnlyVerb) -> &'static str {
    match verb {
        ReadOnlyVerb::Status => "status",
        ReadOnlyVerb::Help => "help",
        ReadOnlyVerb::Doctor => "doctor",
        ReadOnlyVerb::Version => "version",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `MintIntent` の素形 (自由記述だけを振る)。
    fn mint(scope: &str, description: Option<&str>) -> EngineCommand {
        EngineCommand::MintIntent {
            scope: ScopeSlugView::parse(scope).expect("固定の scope 名"),
            description: description.map(str::to_string),
            depth: None,
            test_strategy: None,
            review: None,
        }
    }

    /// 設定変更コマンド (与えた修飾だけを載せる)。
    fn config(
        depth: Option<&str>,
        test_strategy: Option<&str>,
        review: Option<&str>,
    ) -> EngineCommand {
        EngineCommand::ChangeConfig {
            depth: depth.map(str::to_string),
            test_strategy: test_strategy.map(str::to_string),
            review: review.map(str::to_string),
        }
    }

    #[test]
    fn commands_compare_by_value() {
        assert_eq!(
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Status),
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Status)
        );
        assert_ne!(EngineCommand::Unpark, EngineCommand::DispatchComposer);
        assert_eq!(
            config(Some("standard"), None, None),
            config(Some("standard"), None, None)
        );
        assert_ne!(
            config(Some("standard"), None, None),
            config(None, None, Some("advisory"))
        );
    }

    #[test]
    fn every_command_concept_spells_in_multicall_form() {
        assert_eq!(
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Status).cli_spelling(),
            "aidlc-utility status"
        );
        assert_eq!(
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Help).cli_spelling(),
            "aidlc-utility help"
        );
        assert_eq!(
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Doctor).cli_spelling(),
            "aidlc-utility doctor"
        );
        assert_eq!(
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Version).cli_spelling(),
            "aidlc-utility version"
        );
        assert_eq!(
            EngineCommand::NounTokens(vec!["intent".to_string(), "list".to_string()])
                .cli_spelling(),
            "aidlc-utility intent list"
        );
        assert_eq!(EngineCommand::Unpark.cli_spelling(), "aidlc-state unpark");
        assert_eq!(
            EngineCommand::ResolveJump {
                stage: StageSlugView::parse("domain-design").unwrap(),
            }
            .cli_spelling(),
            "aidlc-jump resolve --stage domain-design"
        );
        assert_eq!(
            mint("bugfix", None).cli_spelling(),
            "aidlc-utility intent-create --scope bugfix"
        );
        assert_eq!(
            EngineCommand::ChangeScope {
                scope: ScopeSlugView::parse("mvp").expect("固定の scope 名"),
                depth: None,
                test_strategy: None,
                review: None,
            }
            .cli_spelling(),
            "aidlc-utility scope-change --scope mvp"
        );
        assert_eq!(
            config(Some("standard"), None, None).cli_spelling(),
            "aidlc-utility config-change --depth standard"
        );
        assert_eq!(
            config(None, Some("minimal"), None).cli_spelling(),
            "aidlc-utility config-change --test-strategy minimal"
        );
        assert_eq!(
            config(None, None, Some("advisory")).cli_spelling(),
            "aidlc-utility config-change --review advisory"
        );
        assert_eq!(
            EngineCommand::DispatchComposer.cli_spelling(),
            "aidlc-composer detect"
        );
        assert_eq!(
            EngineCommand::ReportSkipped {
                stage: StageSlugView::parse("domain-design").expect("固定の slug"),
            }
            .cli_spelling(),
            "aidlc-orchestrate report --stage domain-design --result skipped --reason 'stage is SKIP in the approved workflow plan'"
        );
    }

    /// 自由記述があると `--arguments` と `--label` が対で出る (upstream `:881-888`)。
    #[test]
    fn a_description_brings_the_arguments_and_label_pair() {
        assert_eq!(
            mint("bugfix", Some("fix the crash")).cli_spelling(),
            "aidlc-utility intent-create --scope bugfix --arguments='fix the crash' --label \"<2-3 word kebab essence>\""
        );
    }

    /// 空の自由記述は「記述なし」と同じ (upstream `description.length > 0`)。
    #[test]
    fn an_empty_description_brings_neither_arguments_nor_label() {
        assert_eq!(
            mint("bugfix", Some("")).cli_spelling(),
            "aidlc-utility intent-create --scope bugfix"
        );
    }

    /// 安全文字だけの記述は裸で載る (upstream `shellArg` の第 1 分岐)。
    #[test]
    fn a_shell_safe_description_is_not_quoted() {
        assert_eq!(
            mint("bugfix", Some("fix-the-crash")).cli_spelling(),
            "aidlc-utility intent-create --scope bugfix --arguments=fix-the-crash --label \"<2-3 word kebab essence>\""
        );
    }

    /// 単一引用符は `'\"'\"'` へ展開される (upstream `replaceAll`)。
    #[test]
    fn a_single_quote_in_the_description_is_expanded() {
        assert_eq!(
            mint("bugfix", Some("don't drop it")).cli_spelling(),
            "aidlc-utility intent-create --scope bugfix --arguments='don'\"'\"'t drop it' --label \"<2-3 word kebab essence>\""
        );
    }

    /// 任意フラグは upstream の push 順で並ぶ。
    #[test]
    fn the_optional_flags_keep_the_upstream_order() {
        let command = EngineCommand::MintIntent {
            scope: ScopeSlugView::parse("classic").expect("固定の scope 名"),
            description: Some("build the auth service".to_string()),
            depth: Some("standard".to_string()),
            test_strategy: Some("minimal".to_string()),
            review: Some("advisory".to_string()),
        };
        assert_eq!(
            command.cli_spelling(),
            "aidlc-utility intent-create --scope classic --arguments='build the auth service' --label \"<2-3 word kebab essence>\" --depth standard --test-strategy minimal --review advisory"
        );
    }

    /// 修飾は 1 本の命令へまとめて載る (upstream は 1 回の実行で全部を適用する)。
    #[test]
    fn the_modifiers_ride_one_command_together() {
        assert_eq!(
            config(Some("standard"), Some("minimal"), Some("advisory")).cli_spelling(),
            "aidlc-utility config-change --depth standard --test-strategy minimal --review advisory"
        );
        assert_eq!(
            EngineCommand::ChangeScope {
                scope: ScopeSlugView::parse("mvp").expect("固定の scope 名"),
                depth: Some("standard".to_string()),
                test_strategy: None,
                review: Some("none".to_string()),
            }
            .cli_spelling(),
            "aidlc-utility scope-change --scope mvp --depth standard --review none"
        );
    }

    /// 安全文字クラスの境界 — upstream `[A-Za-z0-9_./:@%+=,-]` の全記号が裸で通る。
    #[test]
    fn the_safe_character_class_matches_upstream() {
        assert_eq!(shell_arg("aA0_./:@%+=,-"), "aA0_./:@%+=,-");
        assert_eq!(shell_arg(""), "''");
        assert_eq!(shell_arg("a b"), "'a b'");
        assert_eq!(shell_arg("a!b"), "'a!b'");
        // ASCII 以外は安全集合の外 (upstream の正規表現も ASCII クラス)。
        assert_eq!(shell_arg("日本語"), "'日本語'");
    }
}
