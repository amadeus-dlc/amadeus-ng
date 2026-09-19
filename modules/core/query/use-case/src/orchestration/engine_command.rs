//! `EngineCommand` — `next` が人間・conductor へ名指しするエンジンコマンドの概念と綴り。
//!
//! 概念 (どの操作を指しているか) と綴りはどちらも読み手の閉じた出力語彙である。綴りは
//! 本家 2.8.2 を**コンパイル済み実行形で動かしたときの出力**と逐語で揃える。そこでは
//! `aidlcInvocation()` が `aidlc` を返し、`aidlcToolInvocation` も `bun ` で始まらないので
//! `aidlcDispatcherInvocation` へ畳まれる (`.claude/tools/aidlc-runtime-paths.ts:145-168`)。
//! したがって実行時の指示が綴るのは二段形 `aidlc engine <route> …` である。native バイナリは
//! そのコンパイル済み実行形を置き換えるものなので、指揮者は返った綴りを**書き換えずに**
//! 実行できなければならない。
//!
//! 綴りの組み立ては [`dispatcher_invocation`] 1 点に集約し、生成箇所ごとに文字列を手で
//! 綴らない。2.8.2 自身が二段形以外を綴る箇所 (`doctor` / `version` / stale ポインタの回復
//! 報告 / composer のディスパッチ / DocumentKB) だけが例外で、例外に足してよいのは 2.8.2 の
//! 実バイトで別の綴りが確認できた箇所に限る (綴りを推測しない)。綴りの導出は CPU とメモリ
//! だけの純計算なのでポートにしない。

use super::read_only_verb::ReadOnlyVerb;
use crate::orchestration::{ScopeSlugView, StageSlugView};

/// `next` が名指しするエンジンコマンドの閉じた語彙。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineCommand {
    /// 読み取り専用ユーティリティ (分岐 1)。
    ReadOnlyUtility(ReadOnlyVerb),
    /// 名詞トークン列の逐語通し (分岐 1b/1c/1d — 人間の語をそのまま運ぶ)。
    ///
    /// 先頭トークンが族を決め、綴りは族ごとに分かれる — workspace は route へ翻訳した
    /// 二段形 (upstream `:4220-4231`)、plugin は noun をそのまま置いた二段形 (`:4250`)、
    /// DocumentKB だけは 2.8.2 も専用ツールを名指す (`:4269`)。
    NounTokens(Vec<String>),
    /// park 解除 (分岐 2.6 — upstream `:4400` の `aidlcToolInvocation("state") unpark`)。
    Unpark,
    /// jump の実行の名指し (分岐 7 — upstream `:7157` の `aidlcToolInvocation("jump") execute`)。
    ///
    /// `next --stage` は自分で跳ばず、方向と scope を解決した `execute` を名指す。方向は
    /// 集約の答え (`read_next_jump.outcome` の綴り) をそのまま運ぶ。
    ExecuteJump {
        /// ジャンプ先ステージ。
        stage: StageSlugView,
        /// 解決済みの方向の綴り (`forward` / `backward` / `redo`)。
        direction: String,
        /// jump が属する scope。
        scope: ScopeSlugView,
    },
    /// intent の鋳造 (birth — `next` は自身で実行しない)。
    ///
    /// 引数面は upstream `createPrintDirective` (`aidlc-orchestrate.ts:1762-1791`) の完全形で
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
    /// scope 変更の名指し (分岐 5 — upstream `:4583-4590` の引数面)。
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
    /// depth / test-strategy / review の設定変更の名指し (分岐 5 — upstream `:4606-4622`)。
    ///
    /// 与えられた修飾は**1 本**の命令に載る。フィールドごとに命令を分けない。載り方は
    /// upstream の `route` / `extra` に従う — 先頭の 1 つが route のキーになり、2 つ目だけが
    /// フラグで続く。
    ChangeConfig {
        /// `--depth` の値 (人間が与えた逐語)。
        depth: Option<String>,
        /// `--test-strategy` の値 (人間が与えた逐語)。
        test_strategy: Option<String>,
        /// `--review` の値 (人間が与えた逐語)。
        review: Option<String>,
    },
    /// composer ディスパッチの名指し (分岐 4c)。
    ///
    /// 2.8.2 のこの分岐 (`:1831-1842`) はエージェントのファイルを名指すだけで engine の
    /// コマンドを綴らない。二段形の例外である。
    DispatchComposer,
    /// stale ポインタの回復報告 (分岐 10 手順 3 — SKIP なのにカーソルが残っている)。
    ///
    /// 2.8.2 (`:4953-4955`) もこの 1 本だけは生の配布入口形を綴る。二段形の例外である。
    /// `--reason` の値は upstream が定数で持つ逐語である (`:4951`)。
    ReportSkipped {
        /// 回復対象のステージ。
        stage: StageSlugView,
    },
}

/// stale ポインタ回復の理由文言 (upstream `:4951` の `reason` 定数)。
const SKIP_REASON: &str = "stage is SKIP in the approved workflow plan";

impl EngineCommand {
    /// コマンド概念を CLI 綴り (2.8.2 の実行時の指示と同じ形) に写す。
    #[must_use]
    pub fn cli_spelling(&self) -> String {
        match self {
            EngineCommand::ReadOnlyUtility(verb) => read_only_spelling(*verb),
            EngineCommand::NounTokens(tokens) => noun_tokens_spelling(tokens),
            EngineCommand::Unpark => dispatcher_invocation("state unpark"),
            EngineCommand::ExecuteJump {
                stage,
                direction,
                scope,
            } => format!(
                "{} --target {} --direction {direction} --scope {}",
                dispatcher_invocation("jump execute"),
                stage.as_str(),
                scope.as_str()
            ),
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
                let mut spelled = format!(
                    "{} --scope {}",
                    dispatcher_invocation("scope change"),
                    scope.as_str()
                );
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
            } => config_set_spelling(
                depth.as_deref(),
                test_strategy.as_deref(),
                review.as_deref(),
            ),
            EngineCommand::DispatchComposer => "aidlc-composer detect".to_string(),
            EngineCommand::ReportSkipped { stage } => format!(
                "bun .claude/tools/aidlc-orchestrate.ts report --stage {} --result skipped --reason {}",
                shell_arg(stage.as_str()),
                shell_arg(SKIP_REASON)
            ),
        }
    }
}

/// upstream `aidlcDispatcherInvocation` (`.claude/tools/aidlc-runtime-paths.ts:151`) の移植。
///
/// コンパイル済み実行形では `aidlcInvocation()` が `"aidlc"` を返す (`:145-149`) ため、
/// 二段形は `aidlc engine <route>` になる。`aidlcToolInvocation` (`:155-168`) も
/// `invoke` が `bun ` で始まらない限りこの関数へ畳まれるので、移植する規則はこの 1 本で足りる。
///
/// `EngineCommand` の値にならない案内文（pipeline link の拒否・やり直しの選択・SessionStart の
/// グラフ乖離通知）も同じ規則で綴らなければならないので、合成ルートへ公開する。
#[must_use]
pub fn dispatcher_invocation(route: &str) -> String {
    format!("aidlc engine {route}")
}

/// 読み取り専用ユーティリティの綴り (upstream `:4186-4191`)。
///
/// `--status` と `--help` は engine の下の route だが、`--doctor` / `--version` だけは engine を
/// 挟まず `${aidlcInvocation()} ${sub}` を綴る。2.8.2 の実バイトが示す例外なのでそのまま写す。
fn read_only_spelling(verb: ReadOnlyVerb) -> String {
    match verb {
        ReadOnlyVerb::Status => dispatcher_invocation("status"),
        ReadOnlyVerb::Help => dispatcher_invocation("orchestrate help"),
        ReadOnlyVerb::Doctor => "aidlc doctor".to_string(),
        ReadOnlyVerb::Version => "aidlc version".to_string(),
    }
}

/// DocumentKB の noun (upstream 分岐 1d の `aidlc-knowledge.ts`)。
const KNOWLEDGE_NOUN: &str = "knowledge";

/// 名詞トークン列の綴り (upstream 分岐 1b/1c/1d — `:4205-4275`)。
///
/// 分岐 1b (workspace) は先頭トークンを route へ翻訳してから残りを載せる (`:4220-4231`)。
///
/// 分岐 1c (plugin) は `plugin ` を前置し、動詞だけを route の後半へ置く — upstream は先に
/// `parsePluginCommand` (`.claude/tools/aidlc-lib.ts:1017-1042`) が `plugin <verb>` を一段形の
/// 動詞 (`plugin-list` / `plugin-sync` / `select-plugins` / `plugin-validate` / `plugin-build`)
/// へ翻訳し、`:4247` がその動詞から `plugin-` 接頭辞 (`select-plugins` は `select`) を外して
/// リテラル `plugin ` の後ろへ戻す (`:4250`)。native は利用者の綴り `plugin <verb>` を
/// そのまま持つので、到達しうる 5 動詞では素通しで同じ route になる。
///
/// 分岐 1d (DocumentKB) だけは 2.8.2 も engine を挟まず専用ツールを名指すので、その綴りを
/// 例外として写す (`:4269`)。
fn noun_tokens_spelling(tokens: &[String]) -> String {
    let Some((noun, tail)) = tokens.split_first() else {
        return dispatcher_invocation("");
    };
    if noun == KNOWLEDGE_NOUN {
        return format!(
            "bun .claude/tools/aidlc-knowledge.ts{}",
            trailing_arguments(tail)
        );
    }
    let (route, rest) = workspace_route(noun, tail);
    format!(
        "{}{}",
        dispatcher_invocation(&route),
        trailing_arguments(rest)
    )
}

/// workspace の先頭トークンを route と残りの引数へ割る (upstream `:4220-4229` 逐語)。
///
/// `intent` / `space` は次の位置トークン (フラグでないもの) を動詞として route へ吸い上げ、
/// 無ければ `list` を補う。`intent-create` / `space-create` は二語の route へ開く。
fn workspace_route<'a>(noun: &str, tail: &'a [String]) -> (String, &'a [String]) {
    match noun {
        "intent-create" => ("intent create".to_string(), tail),
        "space-create" => ("space create".to_string(), tail),
        "intent" | "space" => match tail.split_first() {
            Some((verb, rest)) if !verb.starts_with("--") => (format!("{noun} {verb}"), rest),
            _ => (format!("{noun} list"), tail),
        },
        _ => (noun.to_string(), tail),
    }
}

/// route の後ろに続く引数 (upstream `suffix` — `tail.map(shellArg).join(" ")`)。
fn trailing_arguments(args: &[String]) -> String {
    if args.is_empty() {
        return String::new();
    }
    format!(
        " {}",
        args.iter()
            .map(|arg| shell_arg(arg))
            .collect::<Vec<_>>()
            .join(" ")
    )
}

/// `config set` の綴り (upstream `:4606-4622` の `route` / `extra` 逐語)。
///
/// upstream は先頭の 1 つ (`depth` → `test-strategy` → `review` の順) を route のキーに据え、
/// 2 つ目だけをフラグで足す。3 つ与えられたときに `--review` が落ちるのも upstream の
/// 振る舞いであり、native の内部構造に合わせて綴りを変えない。修飾が 1 つも無い組は
/// `turn.rs` の分岐 5 が作らない (少なくとも 1 つあるときだけ `ChangeConfig` を組む)。
fn config_set_spelling(
    depth: Option<&str>,
    test_strategy: Option<&str>,
    review: Option<&str>,
) -> String {
    let route = match (depth, test_strategy, review) {
        (Some(depth), _, _) => format!("config set depth {depth}"),
        (None, Some(test_strategy), _) => format!("config set test-strategy {test_strategy}"),
        (None, None, Some(review)) => format!("config set review {review}"),
        (None, None, None) => "config set".to_string(),
    };
    let extra = match (depth, test_strategy, review) {
        (Some(_), Some(test_strategy), _) => format!(" --test-strategy {test_strategy}"),
        (depth, test_strategy, Some(review)) if depth.is_some() || test_strategy.is_some() => {
            format!(" --review {review}")
        }
        _ => String::new(),
    };
    format!("{}{extra}", dispatcher_invocation(&route))
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

/// `intent create` の綴り (upstream `createPrintDirective` `aidlc-orchestrate.ts:1762-1791`
/// の引数組み立て逐語 — `` Run `${aidlcDispatcherInvocation("intent create")} ${cmd.join(" ")}` ``)。
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
    let mut spelled = format!(
        "{} --scope {}",
        dispatcher_invocation("intent create"),
        scope.as_str()
    );
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

    /// 綴りは 2.8.2 をコンパイル済み実行形で動かしたときの出力と同じ二段形になる。
    ///
    /// 例外は 2.8.2 自身が別の綴りを出す 4 件 (`doctor` / `version` / stale ポインタの回復
    /// 報告 / composer のディスパッチ) と、DocumentKB の専用ツールだけである。
    #[test]
    fn every_command_concept_spells_in_the_two_stage_form() {
        assert_eq!(
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Status).cli_spelling(),
            "aidlc engine status"
        );
        assert_eq!(
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Help).cli_spelling(),
            "aidlc engine orchestrate help"
        );
        // `doctor` / `version` だけは engine を挟まない (upstream `:4191`)。
        assert_eq!(
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Doctor).cli_spelling(),
            "aidlc doctor"
        );
        assert_eq!(
            EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Version).cli_spelling(),
            "aidlc version"
        );
        assert_eq!(
            EngineCommand::NounTokens(vec!["intent".to_string(), "list".to_string()])
                .cli_spelling(),
            "aidlc engine intent list"
        );
        assert_eq!(
            EngineCommand::Unpark.cli_spelling(),
            "aidlc engine state unpark"
        );
        assert_eq!(
            EngineCommand::ExecuteJump {
                stage: StageSlugView::parse("domain-design").unwrap(),
                direction: "forward".to_string(),
                scope: ScopeSlugView::parse("classic").unwrap(),
            }
            .cli_spelling(),
            "aidlc engine jump execute --target domain-design --direction forward --scope classic"
        );
        assert_eq!(
            mint("bugfix", None).cli_spelling(),
            "aidlc engine intent create --scope bugfix"
        );
        assert_eq!(
            EngineCommand::ChangeScope {
                scope: ScopeSlugView::parse("mvp").expect("固定の scope 名"),
                depth: None,
                test_strategy: None,
                review: None,
            }
            .cli_spelling(),
            "aidlc engine scope change --scope mvp"
        );
        assert_eq!(
            config(Some("standard"), None, None).cli_spelling(),
            "aidlc engine config set depth standard"
        );
        assert_eq!(
            config(None, Some("minimal"), None).cli_spelling(),
            "aidlc engine config set test-strategy minimal"
        );
        assert_eq!(
            config(None, None, Some("advisory")).cli_spelling(),
            "aidlc engine config set review advisory"
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
            "bun .claude/tools/aidlc-orchestrate.ts report --stage domain-design --result skipped --reason 'stage is SKIP in the approved workflow plan'"
        );
    }

    /// workspace の名詞は upstream と同じ route へ開かれる (upstream `:4220-4231`)。
    ///
    /// 動詞を省いた `intent` / `space` は `list` が補われ、位置引数はそのまま route の
    /// 2 語目になり、フラグは route ではなく後続の引数として shell-quote される。
    #[test]
    fn a_workspace_noun_opens_into_the_route_upstream_spells() {
        for (tokens, expected) in [
            (vec!["intent"], "aidlc engine intent list"),
            (vec!["intent", "--json"], "aidlc engine intent list --json"),
            (vec!["intent", "list"], "aidlc engine intent list"),
            (
                vec!["intent", "switch", "auth-service"],
                "aidlc engine intent switch auth-service",
            ),
            (
                vec!["intent", "sample name"],
                "aidlc engine intent sample name",
            ),
            (
                vec!["intent-create", "--scope", "bugfix"],
                "aidlc engine intent create --scope bugfix",
            ),
            (vec!["space"], "aidlc engine space list"),
            (vec!["space", "teamB"], "aidlc engine space teamB"),
            (
                vec!["space-create", "team A"],
                "aidlc engine space create 'team A'",
            ),
        ] {
            let command = EngineCommand::NounTokens(
                tokens.iter().map(|token| (*token).to_string()).collect(),
            );
            assert_eq!(command.cli_spelling(), expected, "{tokens:?}");
        }
    }

    /// plugin の名詞は `plugin <verb>` の route へ開く (upstream `:4247-4250` — 翻訳した動詞から
    /// `plugin-` 接頭辞を外し、リテラル `plugin ` の後ろへ戻した形)。
    #[test]
    fn a_plugin_noun_rides_the_dispatcher_form() {
        assert_eq!(
            EngineCommand::NounTokens(vec!["plugin".to_string(), "list".to_string()])
                .cli_spelling(),
            "aidlc engine plugin list"
        );
    }

    /// DocumentKB だけは 2.8.2 も専用ツールを名指す (upstream `:4269`)。
    #[test]
    fn the_document_knowledge_noun_keeps_the_upstream_tool_form() {
        assert_eq!(
            EngineCommand::NounTokens(vec!["knowledge".to_string(), "list".to_string()])
                .cli_spelling(),
            "bun .claude/tools/aidlc-knowledge.ts list"
        );
    }

    /// 自由記述があると `--arguments` と `--label` が対で出る (upstream `:1773-1780`)。
    #[test]
    fn a_description_brings_the_arguments_and_label_pair() {
        assert_eq!(
            mint("bugfix", Some("fix the crash")).cli_spelling(),
            "aidlc engine intent create --scope bugfix --arguments='fix the crash' --label \"<2-3 word kebab essence>\""
        );
    }

    /// 空の自由記述は「記述なし」と同じ (upstream `description.length > 0`)。
    #[test]
    fn an_empty_description_brings_neither_arguments_nor_label() {
        assert_eq!(
            mint("bugfix", Some("")).cli_spelling(),
            "aidlc engine intent create --scope bugfix"
        );
    }

    /// 安全文字だけの記述は裸で載る (upstream `shellArg` の第 1 分岐)。
    #[test]
    fn a_shell_safe_description_is_not_quoted() {
        assert_eq!(
            mint("bugfix", Some("fix-the-crash")).cli_spelling(),
            "aidlc engine intent create --scope bugfix --arguments=fix-the-crash --label \"<2-3 word kebab essence>\""
        );
    }

    /// 単一引用符は `'\"'\"'` へ展開される (upstream `replaceAll`)。
    #[test]
    fn a_single_quote_in_the_description_is_expanded() {
        assert_eq!(
            mint("bugfix", Some("don't drop it")).cli_spelling(),
            "aidlc engine intent create --scope bugfix --arguments='don'\"'\"'t drop it' --label \"<2-3 word kebab essence>\""
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
            "aidlc engine intent create --scope classic --arguments='build the auth service' --label \"<2-3 word kebab essence>\" --depth standard --test-strategy minimal --review advisory"
        );
    }

    /// 設定変更は 1 本の命令に載る。載り方は upstream の `route` / `extra` に従う。
    ///
    /// 先頭の 1 つが route のキーになり、2 つ目だけがフラグで続く — 3 つ与えたときに
    /// `--review` が落ちるのは upstream `:4617-4621` の分岐そのものである。
    #[test]
    fn the_modifiers_ride_one_command_together() {
        assert_eq!(
            config(Some("standard"), Some("minimal"), Some("advisory")).cli_spelling(),
            "aidlc engine config set depth standard --test-strategy minimal"
        );
        assert_eq!(
            config(Some("standard"), None, Some("advisory")).cli_spelling(),
            "aidlc engine config set depth standard --review advisory"
        );
        assert_eq!(
            config(None, Some("minimal"), Some("advisory")).cli_spelling(),
            "aidlc engine config set test-strategy minimal --review advisory"
        );
        assert_eq!(
            EngineCommand::ChangeScope {
                scope: ScopeSlugView::parse("mvp").expect("固定の scope 名"),
                depth: Some("standard".to_string()),
                test_strategy: None,
                review: Some("none".to_string()),
            }
            .cli_spelling(),
            "aidlc engine scope change --scope mvp --depth standard --review none"
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
