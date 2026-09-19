//! 型付きの要求（Controller の入口）と、起動名・引数からの解決。
//!
//! # `next` の入力の所有
//!
//! 先頭のworkspace/plugin/knowledge名詞が、その後のフラグを含むargv全体を所有する。
//! 文中の同じ語や `--` より後の語は自由記述である。本家2.7.1の`parseNextFlags`に対応する。
//! 名詞の構文から終端コマンドへの変換は `turn` が担い、状態を読んで判断しない。
//! 通常位置引数は `freeform` として渡し、定義を要するscope照合は後段のルーティングで行う。
//! `aidlc --doctor` (Orchestrate 面の先頭引数) は自己診断の入口 (契約 C7) であり、`next --doctor`
//! (本家どおり TS 委譲の print) とは別である。

use core_query_use_case::orchestration::{NextTurnInput, NounFamily, NounToken, ReadOnlyVerb};

use super::codekb_args::{CodekbArgs, parse_codekb};
use super::face::Face;
use super::intent_args::{IntentArgs, parse_intent};
use super::intent_create_args::{IntentCreateArgs, parse_intent_create};
use super::interaction_args::{InteractionArgs, parse_interaction};
use super::learnings_args::{LearningsArgs, parse_learnings};
use super::link_args::{LinkArgs, parse_link};
use super::promote_args::{PromoteArgs, parse_promote};
use super::report_args::{ReportArgs, parse_report};
use super::reuse_artifact_args::{ReuseArtifactArgs, parse_reuse_artifact};
use super::review_args::{ReviewArgs, parse_review};
use super::set_autonomy_args::{SetAutonomyArgs, parse_set_autonomy};

/// 型付きの要求。
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// 本家jump面の引数列。
    Jump(Vec<String>),
    /// テスト契約の公開入口。
    TestingPosture {
        /// 本家の動詞/フラグ列。
        args: Vec<String>,
    },
    /// レビュー判断の文脈の公開入口（読取専用）。
    ReviewBrief {
        /// 本家の動詞/フラグ列。
        args: Vec<String>,
    },
    /// Claudeフックの接続入口。
    Hook {
        /// 接続するフック名。
        name: String,
    },
    /// `next` — 観測を畳んだ入力を運ぶ。
    Next(Box<NextTurnInput>),
    /// `continue <token>` — 位置引数 1 つ。
    Continue {
        /// 受け取ったトークン（検証は消費側）。
        token: String,
    },
    /// `report` — フラグ一式。
    Report(ReportArgs),
    /// `park` — 引数を取らない。
    Park,
    /// `--doctor` — 自己診断 (契約 C7)。公開入力は `--doctor` だけで、追加引数は拒む。
    Doctor {
        /// `--doctor` の後ろに与えられた引数 (空でなければ拒否)。
        extra: Vec<String>,
    },
    /// `intent-create` — utility 面。
    IntentCreate(IntentCreateArgs),
    /// `statusline` — 端末の状態表示へ 1 行を描く（読取専用・引数なし）。
    ///
    /// upstream の ROUTES 表では面を持たない engine 自身の入口（`routeOnly`）であり、
    /// フックと同じくこの build のエンジン面が受ける。
    Statusline,
    /// エンジン面の未知動詞 — **自己防衛拒否**（stderr + exit 1）。
    UnknownOrchestrateVerb {
        /// 与えられた動詞（無ければ `None`）。
        given: Option<String>,
    },
    /// `aidlc-utility scope-table` — scope グリッドの Markdown 表を plain 出力（群 A）。
    UtilityScopeTable,
    /// `aidlc-utility stage-table` — stage グラフの Markdown 表を plain 出力（群 A）。
    UtilityStageTable,
    /// `aidlc-utility project-description` — 依頼原文の正本を JSON 1 行で出す（群 B）。
    ///
    /// 引数を取らない（upstream `handleProjectDescription(projectDir)` も同様）。
    UtilityProjectDescription,
    /// `aidlc-utility intent [list] [--json]` — 空間の依頼一覧（読取専用・副作用なし）。
    ///
    /// 位置動詞をそのまま運ぶ。`list` と動詞なしが一覧で、それ以外（切替）はこの build が
    /// 配線していないので消費側が名指して拒否する。
    UtilityIntent(IntentArgs),
    /// `aidlc-utility document-input` — 活動記録が名指す 1 ファイルを直接入力として出す（群 B）。
    ///
    /// 引数を取らない — 読む対象は活動記録直下の転送ファイルが名指す（upstream
    /// `handleDocumentInput(projectDir)` も同様で、顧客由来の綴りを argv に載せない）。
    UtilityDocumentInput,
    /// `aidlc-utility codekb-path` — codekb の保管先を出す（群 C・読取専用・副作用なし）。
    UtilityCodekbPath(CodekbArgs),
    /// `aidlc-utility codekb-scope-diff` — 走査範囲の突合（群 C・読取専用）。
    UtilityCodekbScopeDiff(CodekbArgs),
    /// `aidlc-utility codekb-snapshot` — 走査の直前に 2 つの世代の写しを取る（群 D・書込）。
    UtilityCodekbSnapshot(CodekbArgs),
    /// `aidlc-utility codekb-publish` — compare-and-swap を確かめて公開する（群 D・書込）。
    UtilityCodekbPublish(CodekbArgs),
    /// ユーティリティ面の未知動詞 — 同上。
    UnknownUtilityVerb {
        /// 与えられた動詞（無ければ `None`）。
        given: Option<String>,
    },
    /// `aidlc-log review` — フラグ一式。
    LogReview(ReviewArgs),
    /// 通常の質問提示。
    LogDecision(InteractionArgs),
    /// 通常質問への回答。
    LogAnswer(InteractionArgs),
    /// 宣言されたpipelineの引継ぎ完了。
    LogLink(LinkArgs),
    /// 記録面の未知動詞 — 同上。
    UnknownLogVerb {
        /// 与えられた動詞（無ければ `None`）。
        given: Option<String>,
    },
    /// `aidlc-state lookup <sub> [args...]` — コンパイル済みグラフの読取解決（群 A）。
    ///
    /// サブ動詞（`phase-of` / `agent-for` / `validate-stage` / `next-stage`）と以降の位置引数を
    /// そのまま運ぶ。どのサブを引くかの構文的ルーティングは消費側（読取クエリ）が担う。
    StateLookup {
        /// サブ動詞（無ければ `None`）。
        sub: Option<String>,
        /// サブ動詞以降の位置引数。
        args: Vec<String>,
    },
    /// `aidlc-state practices-promote` — フラグ一式。
    StatePracticesPromote(PromoteArgs),
    /// `aidlc-state reuse-artifact` — 既存成果物の再利用の受領（記録専用）。
    StateReuseArtifact(ReuseArtifactArgs),
    /// `aidlc-state <他の動詞>` — **この build に無い**（自己防衛拒否）。
    StateNotWired {
        /// 認識はしているが配線されていない動詞。
        verb: String,
    },
    /// 状態面の未知動詞 — 同上。
    UnknownStateVerb {
        /// 与えられた動詞（無ければ `None`）。
        given: Option<String>,
    },
    /// `aidlc-bolt set-autonomy` — フラグ一式。
    BoltSetAutonomy(SetAutonomyArgs),
    /// `aidlc-bolt <他の動詞>` — **この build に無い**（自己防衛拒否）。
    BoltNotWired {
        /// 認識はしているが配線されていない動詞。
        verb: String,
    },
    /// Bolt 面の未知動詞 — 同上。
    UnknownBoltVerb {
        /// 与えられた動詞（無ければ `None`）。
        given: Option<String>,
    },
    /// `aidlc-learnings surface` — 学びの候補を並べる読取面。
    LearningsSurface(LearningsArgs),
    /// `aidlc-learnings persist` — 確定した学びを正本へ書く更新面。
    LearningsPersist(LearningsArgs),
    /// `aidlc-learnings --help`。
    LearningsHelp,
    /// 学びの面の未知動詞 — 同上。
    UnknownLearningsVerb {
        /// 与えられた動詞（無ければ `None`）。
        given: Option<String>,
    },
}

impl Request {
    /// 未知動詞・未配線動詞へ落ちない要求か。
    ///
    /// 受理／拒否の分類語彙は要求の型が所有する。呼出側（`runtime::doctor` の自己診断、
    /// 契約テスト）が拒否ヴァリアントの集合を各々列挙すると、ヴァリアントを足したときに
    /// 一部だけが追随して判定が食い違う（Tell, Don't Ask）。
    #[must_use]
    pub const fn is_wired(&self) -> bool {
        !matches!(
            self,
            Request::UnknownOrchestrateVerb { .. }
                | Request::UnknownUtilityVerb { .. }
                | Request::UnknownLogVerb { .. }
                | Request::StateNotWired { .. }
                | Request::UnknownStateVerb { .. }
                | Request::BoltNotWired { .. }
                | Request::UnknownBoltVerb { .. }
                | Request::UnknownLearningsVerb { .. }
        )
    }
}

/// upstream の `aidlc-bolt` が受理するが**この build には無い**動詞
/// （ピン `3c3146cf` `aidlc-bolt.ts:881-908` の switch から `set-autonomy` を除いた 7）。
const RECOGNISED_BOLT_VERBS: [&str; 7] = [
    "start",
    "complete",
    "fail",
    "abort",
    "dispatch-event",
    "hold-merge",
    "release-merge",
];

/// upstream の `aidlc-state` が受理するが**この build には無い**動詞
/// （ピン `3c3146cf` `aidlc-state.ts:530-627` の switch から `practices-promote` を除いた 24）。
///
/// `unit` は upstream の `Valid:` 一覧には現れないが switch は受理するので、こちらでも
/// 「認識はする」側に置く — 未知動詞の逐語に混ぜると「綴りが違う」と読まれる。
///
/// `lookup` と `reuse-artifact` は**この build に配線した**ので、この未配線集合からは
/// 外れている（それぞれの配線アームが先に受ける）。`reuse-artifact` はオーナー裁定 D12 の
/// 受領証イベントへ配線した。
const RECOGNISED_STATE_VERBS: [&str; 22] = [
    "get",
    "set",
    "set-skeleton-stance",
    "set-construction-iteration",
    "checkbox",
    "count",
    "advance",
    "finalize",
    "complete-workflow",
    "gate-start",
    "approve",
    "reject",
    "revise",
    "skip",
    "resume",
    "acknowledge-compaction",
    "practices-event",
    "fork",
    "merge",
    "unit",
    "park",
    "unpark",
];

/// 起動名と引数から要求を組む。
#[must_use]
pub fn parse(face: Face, args: &[String]) -> Request {
    let verb = args.first().map(String::as_str);
    let rest = args.get(1..).unwrap_or_default();
    match (face, verb) {
        (Face::Jump, _) => Request::Jump(args.to_vec()),
        (Face::TestingPosture, _) => Request::TestingPosture {
            args: args.to_vec(),
        },
        (Face::ReviewBrief, _) => Request::ReviewBrief {
            args: args.to_vec(),
        },
        (Face::Orchestrate, Some("hook")) => Request::Hook {
            name: rest.first().cloned().unwrap_or_default(),
        },
        (Face::Orchestrate, Some("next")) => Request::Next(Box::new(parse_next(rest))),
        (Face::Orchestrate, Some("continue")) => Request::Continue {
            // 引数の個数違いもトークン不正と同じ fail-closed に落とすので、ここでは
            // 数を判定しない（1 つ目をそのまま運び、2 つ以上あれば空へ倒す）。
            token: if rest.len() == 1 {
                rest.first().cloned().unwrap_or_default()
            } else {
                String::new()
            },
        },
        (Face::Orchestrate, Some("report")) => Request::Report(parse_report(rest)),
        (Face::Orchestrate, Some("park")) => Request::Park,
        (Face::Orchestrate, Some("--doctor")) => Request::Doctor {
            extra: rest.to_vec(),
        },
        (Face::Orchestrate, Some("statusline")) => Request::Statusline,
        (Face::Orchestrate, given) => Request::UnknownOrchestrateVerb {
            given: given.map(str::to_string),
        },
        (Face::Utility, Some("intent-create" | "init")) => {
            Request::IntentCreate(parse_intent_create(rest))
        }
        (Face::Utility, Some("scope-table")) => Request::UtilityScopeTable,
        (Face::Utility, Some("stage-table")) => Request::UtilityStageTable,
        (Face::Utility, Some("project-description")) => Request::UtilityProjectDescription,
        (Face::Utility, Some("document-input")) => Request::UtilityDocumentInput,
        (Face::Utility, Some("intent")) => Request::UtilityIntent(parse_intent(rest)),
        (Face::Utility, Some("codekb-path")) => Request::UtilityCodekbPath(parse_codekb(rest)),
        (Face::Utility, Some("codekb-scope-diff")) => {
            Request::UtilityCodekbScopeDiff(parse_codekb(rest))
        }
        (Face::Utility, Some("codekb-snapshot")) => {
            Request::UtilityCodekbSnapshot(parse_codekb(rest))
        }
        (Face::Utility, Some("codekb-publish")) => {
            Request::UtilityCodekbPublish(parse_codekb(rest))
        }
        (Face::Utility, given) => Request::UnknownUtilityVerb {
            given: given.map(str::to_string),
        },
        (Face::Log, Some("answer")) => Request::LogAnswer(parse_interaction(rest)),
        (Face::Log, Some("decision")) => Request::LogDecision(parse_interaction(rest)),
        (Face::Log, Some("review")) => Request::LogReview(parse_review(rest)),
        (Face::Log, Some("link")) => Request::LogLink(parse_link(rest)),
        (Face::Log, given) => Request::UnknownLogVerb {
            given: given.map(str::to_string),
        },
        (Face::State, Some("lookup")) => Request::StateLookup {
            sub: rest.first().cloned(),
            args: rest.get(1..).unwrap_or_default().to_vec(),
        },
        (Face::State, Some("practices-promote")) => {
            Request::StatePracticesPromote(parse_promote(rest))
        }
        (Face::State, Some("reuse-artifact")) => {
            Request::StateReuseArtifact(parse_reuse_artifact(rest))
        }
        (Face::State, Some(verb)) if RECOGNISED_STATE_VERBS.contains(&verb) => {
            Request::StateNotWired {
                verb: verb.to_string(),
            }
        }
        (Face::State, given) => Request::UnknownStateVerb {
            given: given.map(str::to_string),
        },
        (Face::Bolt, Some("set-autonomy")) => Request::BoltSetAutonomy(parse_set_autonomy(rest)),
        (Face::Bolt, Some(verb)) if RECOGNISED_BOLT_VERBS.contains(&verb) => {
            Request::BoltNotWired {
                verb: verb.to_string(),
            }
        }
        (Face::Bolt, given) => Request::UnknownBoltVerb {
            given: given.map(str::to_string),
        },
        (Face::Learnings, Some("surface")) => Request::LearningsSurface(parse_learnings(rest)),
        (Face::Learnings, Some("persist")) => Request::LearningsPersist(parse_learnings(rest)),
        (Face::Learnings, Some("--help" | "-h")) => Request::LearningsHelp,
        (Face::Learnings, given) => Request::UnknownLearningsVerb {
            given: given.map(str::to_string),
        },
    }
}

/// `next` のフラグを [`NextTurnInput`] へ畳む。
fn parse_next(args: &[String]) -> NextTurnInput {
    if args.len() == 1 && args.first().is_some_and(|arg| arg == "help" || arg == "-h") {
        return NextTurnInput::new().with_read_only(ReadOnlyVerb::Help);
    }
    let family = match args.first().map(String::as_str) {
        Some("space" | "space-create" | "intent") => Some(NounFamily::Workspace),
        Some("plugin" | "plugin-create") => Some(NounFamily::Plugin),
        Some("knowledge") => Some(NounFamily::Knowledge),
        _ => None,
    };
    if let Some(family) = family {
        return NextTurnInput::new().with_noun_token(NounToken::new(family, args.to_vec()));
    }
    let mut input = NextTurnInput::new();
    let mut freeform: Vec<String> = Vec::new();
    let mut literal = false;
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        let value = args.get(index + 1);
        if literal {
            freeform.push(arg.clone());
            index += 1;
            continue;
        }
        match arg.as_str() {
            "--" => literal = true,
            "--status" => input = input.with_read_only(ReadOnlyVerb::Status),
            "--help" => input = input.with_read_only(ReadOnlyVerb::Help),
            "--doctor" => input = input.with_read_only(ReadOnlyVerb::Doctor),
            "--version" => input = input.with_read_only(ReadOnlyVerb::Version),
            // 先頭の `compose` だけが動詞。文中の "compose" は自由記述のままにする。
            "compose" if index == 0 => input = input.with_compose(),
            "--resume" => input = input.with_resume(),
            "--single" => input = input.with_single(),
            "--new-scope" | "--report" => input = input.with_compose(),
            "--new-intent" => {
                // 記述は後続の自由記述から拾う（upstream も `flags.intent` に乗せる）。
                input = input.with_new_intent(String::new());
            }
            "--scope" => {
                if let Some(value) = value {
                    input = input.with_scope(value);
                    index += 1;
                }
            }
            "--stage" => {
                if let Some(value) = value {
                    input = input.with_stage(value);
                    index += 1;
                }
            }
            "--phase" => {
                if let Some(value) = value {
                    input = input.with_phase(value);
                    index += 1;
                }
            }
            "--depth" => {
                if let Some(value) = value {
                    input = input.with_depth(value);
                    index += 1;
                }
            }
            "--test-strategy" => {
                if let Some(value) = value {
                    input = input.with_test_strategy(value);
                    index += 1;
                }
            }
            "--review" => match value {
                Some(value) if !value.starts_with("--") => {
                    input = input.with_review(value);
                    index += 1;
                }
                _ => {
                    input =
                        input.with_parse_error("--review requires <adversarial|advisory|none>.");
                }
            },
            // 認識できないフラグ様のトークンも自由記述である（upstream の逐語コメント:
            // 「Unknown flag-looking tokens are task text, not disposable noise」）。
            other => freeform.push(other.to_string()),
        }
        index += 1;
    }
    if !freeform.is_empty() {
        let text = freeform.join(" ");
        input = if input.new_intent().is_some() {
            input.with_new_intent(text)
        } else {
            input.with_freeform(text)
        };
    }
    input
}

#[cfg(test)]
mod tests {
    // panic! は想定外バリアントの即時失敗という検証用途で使う。
    #![allow(clippy::panic)]

    use super::*;

    /// 変種の取り出し — `let ... else { panic! }` を各テストに散らすと、到達しない腕が
    /// テストの数だけ増える。取り出しはここ 1 か所に閉じる（テスト衛生）。
    fn expect_next(request: Request) -> Box<NextTurnInput> {
        match request {
            Request::Next(input) => input,
            other => panic!("next へ行く: {other:?}"),
        }
    }

    fn expect_report(request: Request) -> ReportArgs {
        match request {
            Request::Report(flags) => flags,
            other => panic!("report へ行く: {other:?}"),
        }
    }

    fn expect_intent_create(request: Request) -> IntentCreateArgs {
        match request {
            Request::IntentCreate(flags) => flags,
            other => panic!("intent-create へ行く: {other:?}"),
        }
    }

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|a| (*a).to_string()).collect()
    }

    #[test]
    fn the_four_engine_verbs_route_to_their_requests() {
        assert!(matches!(
            parse(Face::Orchestrate, &argv(&["next"])),
            Request::Next(_)
        ));
        assert!(matches!(
            parse(Face::Orchestrate, &argv(&["report"])),
            Request::Report(_)
        ));
        assert_eq!(parse(Face::Orchestrate, &argv(&["park"])), Request::Park);
    }

    /// `continue` はトークンを**位置引数**で受ける。
    #[test]
    fn continue_takes_its_token_as_a_positional_argument() {
        assert_eq!(
            parse(Face::Orchestrate, &argv(&["continue", "abc123"])),
            Request::Continue {
                token: "abc123".to_string()
            }
        );
    }

    /// 引数の個数違いは fail-closed へ倒す（空トークンは検証に必ず落ちる）。
    #[test]
    fn continue_with_the_wrong_argument_count_yields_an_empty_token() {
        for args in [vec!["continue"], vec!["continue", "a", "b"]] {
            assert_eq!(
                parse(Face::Orchestrate, &argv(&args)),
                Request::Continue {
                    token: String::new()
                }
            );
        }
    }

    /// `--doctor` は Orchestrate 面の先頭引数として受け、後続の引数はそのまま運ぶ (拒否は消費側)。
    #[test]
    fn doctor_is_a_leading_engine_flag_that_carries_its_extra_arguments() {
        assert_eq!(
            parse(Face::Orchestrate, &argv(&["--doctor"])),
            Request::Doctor { extra: Vec::new() }
        );
        assert_eq!(
            parse(Face::Orchestrate, &argv(&["--doctor", "--export"])),
            Request::Doctor {
                extra: vec!["--export".to_string()]
            }
        );
        assert!(matches!(
            parse(Face::Orchestrate, &argv(&["next", "--doctor"])),
            Request::Next(_)
        ));
        assert_eq!(
            parse(Face::Utility, &argv(&["--doctor"])),
            Request::UnknownUtilityVerb {
                given: Some("--doctor".to_string())
            }
        );
    }

    #[test]
    fn an_unknown_engine_verb_is_reported_with_the_given_word() {
        assert_eq!(
            parse(Face::Orchestrate, &argv(&["frobnicate"])),
            Request::UnknownOrchestrateVerb {
                given: Some("frobnicate".to_string())
            }
        );
        assert_eq!(
            parse(Face::Orchestrate, &[]),
            Request::UnknownOrchestrateVerb { given: None }
        );
    }

    #[test]
    fn report_collects_every_flag_it_understands() {
        let flags = expect_report(parse(
            Face::Orchestrate,
            &argv(&[
                "report",
                "--result",
                "approved",
                "--stage",
                "domain-design",
                "--user-input",
                "looks good",
                "--reason",
                "not applicable",
                "--skeleton-stance",
                "off",
                "--single",
            ]),
        ));
        assert_eq!(flags.result(), Some("approved"));
        assert_eq!(flags.stage(), Some("domain-design"));
        assert_eq!(flags.user_input(), Some("looks good"));
        assert_eq!(flags.reason(), Some("not applicable"));
        assert_eq!(flags.skeleton_stance(), Some("off"));
        assert!(flags.is_single());
    }

    /// `--stage` の**有無それ自体が契約**なので、省略は `None` のまま運ぶ。
    #[test]
    fn report_without_an_explicit_stage_carries_none() {
        let flags = expect_report(parse(
            Face::Orchestrate,
            &argv(&["report", "--result", "approved"]),
        ));
        assert_eq!(flags.stage(), None);
        assert!(!flags.is_single());
    }

    #[test]
    fn intent_create_is_reached_through_the_utility_face() {
        let flags = expect_intent_create(parse(
            Face::Utility,
            &argv(&[
                "intent-create",
                "--scope",
                "bugfix",
                "--label",
                "fix crash",
                "--depth",
                "standard",
                "--test-strategy",
                "minimal",
                "--review",
                "advisory",
            ]),
        ));
        assert_eq!(flags.scope(), Some("bugfix"));
        assert_eq!(flags.label(), Some("fix crash"));
        assert_eq!(flags.depth(), Some("standard"));
        assert_eq!(flags.test_strategy(), Some("minimal"));
        assert_eq!(flags.review(), Some("advisory"));
    }

    /// upstream の誕生 print は `--arguments=<shell-quoted>` の等号形を出す。
    #[test]
    fn intent_create_accepts_the_equals_form_upstream_emits() {
        let flags = expect_intent_create(parse(
            Face::Utility,
            &argv(&[
                "intent-create",
                "--scope",
                "bugfix",
                "--arguments=fix the crash",
            ]),
        ));
        assert_eq!(flags.arguments(), Some("fix the crash"));
        assert_eq!(flags.scope(), Some("bugfix"));
    }

    #[test]
    fn intent_create_is_not_reachable_from_the_engine_face() {
        assert_eq!(
            parse(
                Face::Orchestrate,
                &argv(&["intent-create", "--scope", "bugfix"])
            ),
            Request::UnknownOrchestrateVerb {
                given: Some("intent-create".to_string())
            }
        );
    }

    #[test]
    fn next_collects_the_flags_the_ladder_understands() {
        let input = expect_next(parse(
            Face::Orchestrate,
            &argv(&[
                "next",
                "--scope",
                "bugfix",
                "--stage",
                "domain-design",
                "--phase",
                "inception",
                "--depth",
                "standard",
                "--test-strategy",
                "minimal",
                "--review",
                "advisory",
                "--resume",
                "--single",
            ]),
        ));
        assert_eq!(input.scope(), Some("bugfix"));
        assert_eq!(input.stage(), Some("domain-design"));
        assert_eq!(input.phase(), Some("inception"));
        assert_eq!(input.depth(), Some("standard"));
        assert_eq!(input.test_strategy(), Some("minimal"));
        assert_eq!(input.review(), Some("advisory"));
        assert!(input.is_resume());
        assert!(input.is_single());
    }

    /// 値を伴わない `--review` は**パース失敗**として運ぶ（ラダーが逐語で拒否する）。
    #[test]
    fn review_without_a_value_is_a_parse_error() {
        let input = expect_next(parse(Face::Orchestrate, &argv(&["next", "--review"])));
        assert_eq!(
            input.parse_error(),
            Some("--review requires <adversarial|advisory|none>.")
        );

        let input = expect_next(parse(
            Face::Orchestrate,
            &argv(&["next", "--review", "--resume"]),
        ));
        assert_eq!(
            input.parse_error(),
            Some("--review requires <adversarial|advisory|none>.")
        );
    }

    #[test]
    fn free_text_becomes_the_freeform_description() {
        let input = expect_next(parse(
            Face::Orchestrate,
            &argv(&["next", "build", "the", "auth", "service"]),
        ));
        assert_eq!(input.freeform(), Some("build the auth service"));
    }

    /// `--new-intent` は後続の自由記述を**自分の欄**で運ぶ。
    #[test]
    fn new_intent_carries_the_description_in_its_own_slot() {
        let input = expect_next(parse(
            Face::Orchestrate,
            &argv(&["next", "--new-intent", "--scope", "bugfix", "fix the crash"]),
        ));
        assert_eq!(input.new_intent(), Some("fix the crash"));
        assert_eq!(input.scope(), Some("bugfix"));
        assert_eq!(input.freeform(), None);
    }

    /// 先頭の `compose` だけが動詞。文中の "compose" は自由記述のままである。
    #[test]
    fn only_a_leading_compose_token_is_the_verb() {
        let input = expect_next(parse(Face::Orchestrate, &argv(&["next", "compose"])));
        assert!(input.is_compose());

        let input = expect_next(parse(
            Face::Orchestrate,
            &argv(&["next", "help", "me", "compose", "a", "song"]),
        ));
        assert!(!input.is_compose());
        assert_eq!(input.freeform(), Some("help me compose a song"));
    }

    /// `--` 以降は逐語の自由記述（フラグと同じ綴りでも解釈しない）。
    #[test]
    fn the_literal_marker_stops_flag_interpretation() {
        let input = expect_next(parse(
            Face::Orchestrate,
            &argv(&["next", "--", "--scope", "bugfix"]),
        ));
        assert_eq!(input.scope(), None);
        assert_eq!(input.freeform(), Some("--scope bugfix"));
    }

    /// 知らないフラグは**値を食わない** — 次のトークンは次のフラグとして読まれる。
    #[test]
    fn an_unknown_report_flag_does_not_swallow_the_next_token() {
        let flags = expect_report(parse(
            Face::Orchestrate,
            &argv(&["report", "--wat", "--result", "approved"]),
        ));
        assert_eq!(flags.result(), Some("approved"));
    }

    #[test]
    fn an_unknown_intent_create_flag_does_not_swallow_the_next_token() {
        let flags = expect_intent_create(parse(
            Face::Utility,
            &argv(&["intent-create", "--wat", "--scope", "bugfix"]),
        ));
        assert_eq!(flags.scope(), Some("bugfix"));
    }

    fn expect_promote(request: Request) -> PromoteArgs {
        match request {
            Request::StatePracticesPromote(flags) => flags,
            other => panic!("practices-promote へ行く: {other:?}"),
        }
    }

    /// 状態面は配線済みの動詞だけを受け、認識する残りは not-wired へ落とす。
    #[test]
    fn the_state_face_routes_the_wired_verb_and_recognises_the_rest() {
        let flags = expect_promote(parse(
            Face::State,
            &argv(&[
                "practices-promote",
                "--team-practices",
                "a/team-practices.md",
                "--discovered-rules",
                "a/discovered-rules.md",
            ]),
        ));
        assert_eq!(flags.team_practices(), Some("a/team-practices.md"));
        assert_eq!(flags.discovered_rules(), Some("a/discovered-rules.md"));

        for verb in RECOGNISED_STATE_VERBS {
            assert_eq!(
                parse(Face::State, &argv(&[verb])),
                Request::StateNotWired {
                    verb: verb.to_string()
                }
            );
        }
        assert_eq!(
            parse(Face::State, &argv(&["frobnicate"])),
            Request::UnknownStateVerb {
                given: Some("frobnicate".to_string())
            }
        );
        assert_eq!(
            parse(Face::State, &[]),
            Request::UnknownStateVerb { given: None }
        );
    }

    /// `lookup` は配線済みの読取面 — サブ動詞と以降の引数をそのまま運ぶ（群 A）。
    #[test]
    fn the_state_lookup_verb_carries_its_subcommand_and_arguments() {
        assert_eq!(
            parse(Face::State, &argv(&["lookup", "phase-of", "state-init"])),
            Request::StateLookup {
                sub: Some("phase-of".to_string()),
                args: vec!["state-init".to_string()],
            }
        );
        assert_eq!(
            parse(
                Face::State,
                &argv(&["lookup", "next-stage", "state-init", "bugfix"])
            ),
            Request::StateLookup {
                sub: Some("next-stage".to_string()),
                args: vec!["state-init".to_string(), "bugfix".to_string()],
            }
        );
        // サブ動詞が無くても未配線には落とさない（配線アームに入り、使い方の拒否は消費側）。
        assert_eq!(
            parse(Face::State, &argv(&["lookup"])),
            Request::StateLookup {
                sub: None,
                args: Vec::new(),
            }
        );
    }

    /// `lookup` は `RECOGNISED_STATE_VERBS`（未配線 24 動詞）から外れ、配線アームへ入る。
    #[test]
    fn lookup_is_no_longer_in_the_not_wired_state_verb_set() {
        assert!(!RECOGNISED_STATE_VERBS.contains(&"lookup"));
        assert!(matches!(
            parse(Face::State, &argv(&["lookup", "phase-of", "x"])),
            Request::StateLookup { .. }
        ));
    }

    /// `scope-table` / `stage-table` は utility 面から配線済みの plain 出力面へ入る（群 A）。
    #[test]
    fn scope_table_and_stage_table_route_from_the_utility_face() {
        assert_eq!(
            parse(Face::Utility, &argv(&["scope-table"])),
            Request::UtilityScopeTable
        );
        assert_eq!(
            parse(Face::Utility, &argv(&["stage-table"])),
            Request::UtilityStageTable
        );
    }

    /// 群 B/C の 3 動詞は utility 面から配線済みの読取面へ入る。
    #[test]
    fn the_description_and_codekb_verbs_route_from_the_utility_face() {
        assert_eq!(
            parse(Face::Utility, &argv(&["project-description"])),
            Request::UtilityProjectDescription
        );
        assert!(matches!(
            parse(Face::Utility, &argv(&["codekb-path"])),
            Request::UtilityCodekbPath(_)
        ));
        assert!(matches!(
            parse(Face::Utility, &argv(&["codekb-scope-diff"])),
            Request::UtilityCodekbScopeDiff(_)
        ));
    }

    /// 群 B/C の動詞はエンジン面からは届かない（面が違えば未知動詞である）。
    #[test]
    fn the_codekb_verbs_are_not_reachable_from_the_engine_face() {
        for verb in [
            "project-description",
            "codekb-path",
            "codekb-scope-diff",
            "codekb-snapshot",
            "codekb-publish",
        ] {
            assert_eq!(
                parse(Face::Orchestrate, &argv(&[verb])),
                Request::UnknownOrchestrateVerb {
                    given: Some(verb.to_string())
                }
            );
        }
    }

    /// 群 D の 2 動詞（`codekb-snapshot` / `codekb-publish`）は utility 面から書込面へ入る。
    #[test]
    fn the_codekb_write_verbs_route_from_the_utility_face() {
        assert!(matches!(
            parse(Face::Utility, &argv(&["codekb-snapshot"])),
            Request::UtilityCodekbSnapshot(_)
        ));
        assert!(matches!(
            parse(Face::Utility, &argv(&["codekb-publish"])),
            Request::UtilityCodekbPublish(_)
        ));
    }

    /// 群 D の compare-and-swap のフラグを運ぶ。upstream は `!flags["expect-store"]` で判定
    /// するので、**空文字は「与えられていない」**と同じである。
    #[test]
    fn the_write_verbs_carry_their_compare_and_swap_flags() {
        let flags = expect_codekb(parse(
            Face::Utility,
            &argv(&[
                "codekb-publish",
                "--repo",
                "svc-a",
                "--staged",
                "record/.aidlc-codekb-stage-svc-a/",
                "--expect-store",
                "sha256:abc",
                "--expect-source",
                "git:def",
                "--paths",
                "src/",
                "--json",
            ]),
        ));
        assert_eq!(flags.repo(), Some("svc-a"));
        assert_eq!(flags.staged(), Some("record/.aidlc-codekb-stage-svc-a/"));
        assert_eq!(flags.expect_store(), Some("sha256:abc"));
        assert_eq!(flags.expect_source(), Some("git:def"));
        assert_eq!(flags.path_list(), vec!["src/".to_string()]);
        assert!(flags.is_json());

        let blank = expect_codekb(parse(
            Face::Utility,
            &argv(&[
                "codekb-publish",
                "--staged",
                "",
                "--expect-store",
                "",
                "--expect-source",
                "",
            ]),
        ));
        assert_eq!(blank.staged(), None, "空文字は未指定と同じ");
        assert_eq!(blank.expect_store(), None, "空文字は未指定と同じ");
        assert_eq!(blank.expect_source(), None, "空文字は未指定と同じ");
    }

    /// `--paths` は重複を落とす（upstream `codekbPaths` の `[...new Set(paths)]`）。
    /// 読取側の [`CodekbArgs::path_list`] は重複を落とさないので、別の口で表す。
    #[test]
    fn the_write_verbs_deduplicate_the_paths_flag() {
        let flags = expect_codekb(parse(
            Face::Utility,
            &argv(&["codekb-snapshot", "--paths", "src/, docs/ ,src/,"]),
        ));
        assert_eq!(
            flags.unique_path_list(),
            vec!["src/".to_string(), "docs/".to_string()],
            "初出の順序を保って重複だけを落とす"
        );
        assert_eq!(
            flags.path_list(),
            vec!["src/".to_string(), "docs/".to_string(), "src/".to_string()],
            "読取側の口は従来どおり重複を残す"
        );
    }

    fn expect_codekb(request: Request) -> CodekbArgs {
        match request {
            Request::UtilityCodekbPath(flags)
            | Request::UtilityCodekbScopeDiff(flags)
            | Request::UtilityCodekbSnapshot(flags)
            | Request::UtilityCodekbPublish(flags) => flags,
            other => panic!("codekb へ行く: {other:?}"),
        }
    }

    /// upstream `parseArgs` の写し — 次のトークンが `--` 始まりでなければ値、さもなくば
    /// `"true"`。真偽フラグと値つきフラグを綴りで区別しない。
    #[test]
    fn codekb_flags_follow_the_upstream_parse_args_semantics() {
        let flags = expect_codekb(parse(
            Face::Utility,
            &argv(&[
                "codekb-scope-diff",
                "--repo",
                "svc-a",
                "--mint",
                "--paths",
                "src/",
                "--json",
            ]),
        ));
        assert_eq!(flags.repo(), Some("svc-a"));
        assert!(flags.is_mint());
        assert_eq!(flags.paths(), Some("src/"));
        assert!(flags.is_json());
        assert_eq!(flags.compare(), None);
    }

    /// `--compare` は**有無それ自体が契約**で、値が無ければ upstream と同じく `"true"` を運ぶ
    /// （その後 `existsSync("true")` が偽なので拒否される — 判断は消費側）。
    #[test]
    fn a_valueless_compare_carries_the_literal_true_like_upstream() {
        let flags = expect_codekb(parse(
            Face::Utility,
            &argv(&["codekb-scope-diff", "--compare"]),
        ));
        assert_eq!(flags.compare(), Some("true"));

        let flags = expect_codekb(parse(
            Face::Utility,
            &argv(&["codekb-scope-diff", "--compare", "incoming.md", "--json"]),
        ));
        assert_eq!(flags.compare(), Some("incoming.md"));
        assert!(flags.is_json());
    }

    /// `--repo=<value>` の等号形も upstream の `parseArgs` は受ける。空値は「与えられていない」。
    #[test]
    fn the_equals_form_is_accepted_and_a_blank_repo_reads_as_absent() {
        let flags = expect_codekb(parse(
            Face::Utility,
            &argv(&["codekb-path", "--repo=svc-a"]),
        ));
        assert_eq!(flags.repo(), Some("svc-a"));

        let flags = expect_codekb(parse(Face::Utility, &argv(&["codekb-path", "--repo", ""])));
        assert_eq!(flags.repo(), None, "空文字は fallback へ倒す");
    }

    /// `--paths` はコンマで割り、前後の空白を落として空片を捨てる。
    #[test]
    fn the_paths_flag_is_split_trimmed_and_compacted() {
        let flags = expect_codekb(parse(
            Face::Utility,
            &argv(&["codekb-scope-diff", "--mint", "--paths", " src/ , "]),
        ));
        assert_eq!(flags.path_list(), vec!["src/".to_string()]);

        let flags = expect_codekb(parse(
            Face::Utility,
            &argv(&["codekb-scope-diff", "--mint", "--paths", "src/,,docs/"]),
        ));
        assert_eq!(
            flags.path_list(),
            vec!["src/".to_string(), "docs/".to_string()]
        );

        let bare = expect_codekb(parse(
            Face::Utility,
            &argv(&["codekb-scope-diff", "--mint"]),
        ));
        assert!(bare.path_list().is_empty(), "`--paths` 無しは空の一覧");
    }

    /// 位置引数は落とす（upstream も `positional` へ分けて読まない）。
    #[test]
    fn positional_tokens_are_ignored_by_the_codekb_flag_parser() {
        let flags = expect_codekb(parse(
            Face::Utility,
            &argv(&["codekb-path", "stray", "--repo", "svc-a"]),
        ));
        assert_eq!(flags.repo(), Some("svc-a"));
    }

    /// 状態面の動詞はエンジン面からは届かない（面が違えば未知動詞である）。
    #[test]
    fn practices_promote_is_not_reachable_from_the_engine_face() {
        assert_eq!(
            parse(Face::Orchestrate, &argv(&["practices-promote"])),
            Request::UnknownOrchestrateVerb {
                given: Some("practices-promote".to_string())
            }
        );
    }

    fn expect_set_autonomy(request: Request) -> SetAutonomyArgs {
        match request {
            Request::BoltSetAutonomy(flags) => flags,
            other => panic!("set-autonomy へ行く: {other:?}"),
        }
    }

    /// Bolt 面は `set-autonomy` だけを配線し、認識する 7 動詞は not-wired へ落とす。
    #[test]
    fn the_bolt_face_routes_the_wired_verb_and_recognises_the_rest() {
        let flags = expect_set_autonomy(parse(
            Face::Bolt,
            &argv(&["set-autonomy", "--mode", "autonomous"]),
        ));
        assert_eq!(flags.mode(), Some("autonomous"));

        for verb in RECOGNISED_BOLT_VERBS {
            assert_eq!(
                parse(Face::Bolt, &argv(&[verb])),
                Request::BoltNotWired {
                    verb: verb.to_string()
                }
            );
        }
        assert_eq!(
            parse(Face::Bolt, &argv(&["frobnicate"])),
            Request::UnknownBoltVerb {
                given: Some("frobnicate".to_string())
            }
        );
        assert_eq!(
            parse(Face::Bolt, &[]),
            Request::UnknownBoltVerb { given: None }
        );
    }

    /// Bolt 面の動詞はエンジン面からは届かない（面が違えば未知動詞である）。
    #[test]
    fn set_autonomy_is_not_reachable_from_the_engine_face() {
        assert_eq!(
            parse(Face::Orchestrate, &argv(&["set-autonomy"])),
            Request::UnknownOrchestrateVerb {
                given: Some("set-autonomy".to_string())
            }
        );
    }

    /// ユーティリティ面の未知動詞は動詞名を運ぶ（診断に出す材料）。
    #[test]
    fn an_unknown_utility_verb_carries_the_given_word() {
        assert_eq!(
            parse(Face::Utility, &argv(&["teleport"])),
            Request::UnknownUtilityVerb {
                given: Some("teleport".to_string())
            }
        );
        assert_eq!(
            parse(Face::Utility, &argv(&[])),
            Request::UnknownUtilityVerb { given: None }
        );
    }
    #[test]
    fn next_preserves_leading_workspace_noun_and_following_tokens() {
        let input = expect_next(parse(
            Face::Orchestrate,
            &argv(&["next", "intent", "list", "--status"]),
        ));
        let noun = input.noun_token().expect("名詞");
        assert_eq!(noun.family(), NounFamily::Workspace);
        assert_eq!(
            noun.tokens(),
            &[
                "intent".to_string(),
                "list".to_string(),
                "--status".to_string()
            ]
        );
        assert!(
            input.read_only().is_none(),
            "後続フラグも名詞側の引数である"
        );
    }
}
