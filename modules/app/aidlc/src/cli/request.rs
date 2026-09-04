//! 型付きの要求（Controller の入口）と、起動名・引数からの解決。
//!
//! # b29 で実装していない `next` の文法
//!
//! upstream `parseNextFlags`（`aidlc-orchestrate.ts:710`）は plugin / knowledge / workspace
//! コマンド、読み取り専用フラグ（`--status` 等）とその `--doctor` 専用引数、そして
//! **先頭位置引数の scope 剥がし**（`next bugfix "Fix duplicate todos"`）も持つ。ここでは
//! [`NextTurnInput`](core_query_use_case::orchestration::NextTurnInput) が表現できる分だけを
//! 写しており、位置引数はすべて freeform として渡る。scope 剥がしには「妥当な scope 名の
//! 一覧」= 定義が要り、それを読むのはラダーが I/O を遅延させている段より手前になるので、
//! 素直には差し込めない。**後続 Bolt の課題**として残す。

use core_query_use_case::orchestration::NextTurnInput;

use super::face::Face;
use super::intent_create_args::{IntentCreateArgs, parse_intent_create};
use super::promote_args::{PromoteArgs, parse_promote};
use super::report_args::{ReportArgs, parse_report};
use super::review_args::{ReviewArgs, parse_review};

/// 型付きの要求。
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
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
    /// `intent-create` — utility 面。
    IntentCreate(IntentCreateArgs),
    /// エンジン面の未知動詞 — **自己防衛拒否**（stderr + exit 1）。
    UnknownOrchestrateVerb {
        /// 与えられた動詞（無ければ `None`）。
        given: Option<String>,
    },
    /// ユーティリティ面の未知動詞 — 同上。
    UnknownUtilityVerb {
        /// 与えられた動詞（無ければ `None`）。
        given: Option<String>,
    },
    /// `aidlc-log review` — フラグ一式。
    LogReview(ReviewArgs),
    /// `aidlc-log <decision|answer|link>` — **この build に無い**（自己防衛拒否）。
    LogNotWired {
        /// 認識はしているが配線されていない動詞。
        verb: String,
    },
    /// 記録面の未知動詞 — 同上。
    UnknownLogVerb {
        /// 与えられた動詞（無ければ `None`）。
        given: Option<String>,
    },
    /// `aidlc-state practices-promote` — フラグ一式。
    StatePracticesPromote(PromoteArgs),
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
}

/// upstream の `aidlc-state` が受理するが**この build には無い**動詞
/// （ピン `3c3146cf` `aidlc-state.ts:530-627` の switch から `practices-promote` を除いた 24）。
///
/// `unit` は upstream の `Valid:` 一覧には現れないが switch は受理するので、こちらでも
/// 「認識はする」側に置く — 未知動詞の逐語に混ぜると「綴りが違う」と読まれる。
const RECOGNISED_STATE_VERBS: [&str; 24] = [
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
    "reuse-artifact",
    "lookup",
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
        (Face::Orchestrate, given) => Request::UnknownOrchestrateVerb {
            given: given.map(str::to_string),
        },
        (Face::Utility, Some("intent-create" | "init")) => {
            Request::IntentCreate(parse_intent_create(rest))
        }
        (Face::Utility, given) => Request::UnknownUtilityVerb {
            given: given.map(str::to_string),
        },
        (Face::Log, Some("review")) => Request::LogReview(parse_review(rest)),
        // 認識はする 3 動詞。upstream にはあるが本 build には無い（b46 の「not wired in this
        // build」層と同じ扱い — 未知動詞の逐語に混ぜると「綴りが違う」と読まれる）。
        (Face::Log, Some(verb @ ("decision" | "answer" | "link"))) => Request::LogNotWired {
            verb: verb.to_string(),
        },
        (Face::Log, given) => Request::UnknownLogVerb {
            given: given.map(str::to_string),
        },
        (Face::State, Some("practices-promote")) => {
            Request::StatePracticesPromote(parse_promote(rest))
        }
        (Face::State, Some(verb)) if RECOGNISED_STATE_VERBS.contains(&verb) => {
            Request::StateNotWired {
                verb: verb.to_string(),
            }
        }
        (Face::State, given) => Request::UnknownStateVerb {
            given: given.map(str::to_string),
        },
    }
}

/// `next` のフラグを [`NextTurnInput`] へ畳む。
fn parse_next(args: &[String]) -> NextTurnInput {
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
        // `--new-intent` は記述を自分の欄で運ぶ（分岐 4a が空白を拒否する）。
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

    /// 状態面は `practices-promote` だけを配線し、認識する 24 動詞は not-wired へ落とす。
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
}
