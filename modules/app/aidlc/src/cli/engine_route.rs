//! 二段形 `aidlc engine <noun> <verb>` の解決 — 境界での正規化（腐敗防止層）。
//!
//! 2.8.2 の配布物は入口を二段形で綴り、noun ごとに既存の `aidlc-<face>.ts` へ委譲する。
//! ここはその写像表の**写し**であり、所有者は本家（`.claude/tools/aidlc.ts` の ROUTES 表と
//! `handleWorkspace` / `handleConfig` / `handleGen` の翻訳結果）である。こちらの都合で
//! 綴りを変えない（`coding-rules/upstream-contracts.md` §「同形でローカル定義」）。
//!
//! この層は `rest[0] == "engine"` のときだけ働く。それ以外では [`resolve`] が `None` を返し、
//! `argv[0]` の basename による multi-call 面（9 面）の経路を一文字も変えない。

use super::Face;

/// 二段形を解決した結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineRoute {
    /// 既存の面・動詞へ写せた。`argv` は写像先へ渡す引数列である。
    Mapped {
        /// 写像先の面。
        face: Face,
        /// 写像先の動詞と、それに続く引数。
        argv: Vec<String>,
    },
    /// 本家の ROUTES 表にはあるが、この build が配線していない入口。
    NotWired {
        /// 本家の noun。
        noun: String,
        /// 本家の verb（動詞を取らない top ルートは `None`）。
        verb: Option<String>,
    },
    /// 本家の ROUTES 表に無い noun / verb。
    Unknown {
        /// 与えられた noun。
        noun: String,
        /// 与えられた verb。
        verb: Option<String>,
    },
}

/// 二段形の第 1 引数。
const ENGINE: &str = "engine";

/// この build が配線した写像（noun, verb） → (面, 写像先の argv)。
///
/// 写像先は ROUTES の `tool:` と `targets:`、および `custom` noun の翻訳結果から引いた。
/// 推測で足さない — 出典は `scripts/aidlc-selfhost/required-surface.json` の `mapping` 列である。
///
/// argv を**列**で持つのは、本家が 2 通りの運び方を使い分けるからである。ほとんどの組は
/// 二段形の verb が一段形の動詞そのものになる（`noun-map`）が、`intent` のように noun が
/// 一段形の動詞になり、二段形の verb は**最初の位置引数**として渡る組もある
/// （`noun-passthrough` — 本家 `handleIntent` は `positional[1]` を読む）。動詞 1 語だけを
/// 持つと後者で verb が落ちるので、運ぶ形をそのまま書く。
const WIRED: [(&str, &str, Face, &[&str]); 27] = [
    ("orchestrate", "next", Face::Orchestrate, &["next"]),
    ("orchestrate", "continue", Face::Orchestrate, &["continue"]),
    ("orchestrate", "report", Face::Orchestrate, &["report"]),
    ("orchestrate", "park", Face::Orchestrate, &["park"]),
    ("log", "decision", Face::Log, &["decision"]),
    ("log", "answer", Face::Log, &["answer"]),
    ("log", "review", Face::Log, &["review"]),
    ("log", "link", Face::Log, &["link"]),
    ("learnings", "surface", Face::Learnings, &["surface"]),
    ("learnings", "persist", Face::Learnings, &["persist"]),
    ("state", "lookup", Face::State, &["lookup"]),
    ("state", "reuse-artifact", Face::State, &["reuse-artifact"]),
    ("workspace", "codekb", Face::Utility, &["codekb-path"]),
    (
        "workspace",
        "codekb-scope-diff",
        Face::Utility,
        &["codekb-scope-diff"],
    ),
    (
        "workspace",
        "codekb-snapshot",
        Face::Utility,
        &["codekb-snapshot"],
    ),
    (
        "workspace",
        "codekb-publish",
        Face::Utility,
        &["codekb-publish"],
    ),
    (
        "workspace",
        "project-description",
        Face::Utility,
        &["project-description"],
    ),
    (
        "workspace",
        "document-input",
        Face::Utility,
        &["document-input"],
    ),
    (
        "testing-posture",
        "render",
        Face::TestingPosture,
        &["render"],
    ),
    (
        "testing-posture",
        "fingerprint",
        Face::TestingPosture,
        &["fingerprint"],
    ),
    ("testing-posture", "brief", Face::TestingPosture, &["brief"]),
    ("review-brief", "review", Face::ReviewBrief, &["review"]),
    ("review-brief", "context", Face::ReviewBrief, &["context"]),
    ("review-brief", "summary", Face::ReviewBrief, &["summary"]),
    ("gen", "scope-table", Face::Utility, &["scope-table"]),
    ("gen", "stage-table", Face::Utility, &["stage-table"]),
    // noun-passthrough — 一段形の動詞は noun 側で決まり、verb は位置引数として渡る。
    ("intent", "list", Face::Utility, &["intent", "list"]),
];

/// 本家の ROUTES 表のうち、`aidlc engine <noun> <verb>` で解決する noun とその verb。
///
/// 写したのは凍結出典 `tests/golden/selfhost-stage1/required-surface-sources/.claude/tools/aidlc.ts`
/// の ROUTES 行のうち、**実効 `namespace` が `engine`** で、かつ **verb を取る**もの
/// （`group` が `top` でない行）である。同じ `group` に複数の行がある noun
/// （`state` / `audit` / `orchestrate`）はその和で綴る。verb を取らない行は [`VERBLESS`]、
/// `hook` は [`EngineRoute::resolve`] の特例、`namespace` が `public` / `system` の行
/// （`unit` / `versions` / `config global` / `plugin` の作者向け 2 動詞 / `lifecycle` /
/// `completions` / `workspace-sync`）は `aidlc engine` から解決しないので写さない。
///
/// `custom` 群（`config` / `gen` / `intent` / `plugin` / `space`）が綴る `<name>` のような
/// 位置引数の目印は verb ではないので写さない。
///
/// この表に**ある**が [`WIRED`] に無い組は「本家にはあるがこの build が未配線」であり、
/// その旨を名指して拒否する。この表に**無い** noun / verb は本家にも無いものとして、
/// 本家 `nounError` の逐語で拒否する（オーナー裁定 D13 の `WT-4`）。
const ROUTES: [(&str, &[&str]); 23] = [
    (
        "orchestrate",
        &["next", "continue", "report", "park", "help"],
    ),
    ("log", &["decision", "answer", "review", "link"]),
    ("learnings", &["surface", "persist"]),
    (
        "state",
        &[
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
            "practices-promote",
            "set-unit-ownership",
            "set-unit-gate-rhythm",
            "fork",
            "merge",
            "park",
            "unpark",
            "unit",
            // `state-utility` 行（同じ noun の 2 つ目の行）が綴る 2 動詞。
            "set-status",
            "init",
        ],
    ),
    (
        "workspace",
        &[
            "detect",
            "codekb",
            "codekb-scope-diff",
            "codekb-snapshot",
            "codekb-publish",
            "project-description",
            "document-input",
        ],
    ),
    (
        "testing-posture",
        &[
            "resolve",
            "render",
            "fingerprint",
            "verify",
            "begin",
            "brief",
        ],
    ),
    (
        "gen",
        &[
            "runners",
            "runner-list",
            "runner-scopes",
            "stage-table",
            "scope-table",
        ],
    ),
    ("intent", &["list", "switch", "create"]),
    ("config", &["set", "get", "list"]),
    (
        "graph",
        &[
            "artifacts",
            "producers",
            "consumers",
            "topo",
            "cycles",
            "scope",
            "validate-scope",
            "validate-grid",
            "ars",
            "compile",
            "resolve",
            "export",
        ],
    ),
    (
        "worktree",
        &["create", "merge", "discard", "list", "verify", "info"],
    ),
    ("review-brief", &["review", "context", "summary"]),
    // 以下は `WIRED` に 1 組も無い noun である。写さないと、本家が `aidlc engine` の下で
    // 解決する入口を「本家にも無い」として `nounError` の逐語で拒否してしまう。
    (
        "audit",
        &["append", "append-batch", "append-raw", "fork", "merge"],
    ),
    (
        "bolt",
        &[
            "start",
            "complete",
            "fail",
            "abort",
            "set-autonomy",
            "dispatch-event",
            "hold-merge",
            "release-merge",
        ],
    ),
    ("jump", &["resolve", "execute"]),
    (
        "knowledge",
        &[
            "onboard",
            "sync",
            "list",
            "show",
            "associate",
            "dissociate",
            "rebind",
            "summarize",
        ],
    ),
    ("plugin", &["select", "sync", "list", "validate", "build"]),
    (
        "runtime",
        &[
            "compile",
            "read",
            "summary",
            "fragment-fork",
            "fragment-merge",
        ],
    ),
    ("scope", &["change", "detect", "resolve-env"]),
    ("sensor", &["list", "describe", "fire"]),
    ("space", &["list", "switch", "create"]),
    ("swarm", &["prepare", "check", "finalize"]),
    ("validate", &["outputs"]),
];

/// 動詞を取らない本家の engine ルート（`aidlc engine <noun> [flags]`）。
///
/// 2 群ある。`group: "top"` の engine 行が綴る top トークン（`statusline` / `recompose` /
/// `status` / `adapter` — 本家 `resolveEngine` は noun 解決の後にこの 4 つを引く）と、
/// `SENSOR_WORKERS` から生える 6 つの `sensor-<id>` 行（`kind: "routing-only"` なので
/// 次のトークンを動詞として読まない）である。
const VERBLESS: [&str; 10] = [
    "statusline",
    "recompose",
    "status",
    "adapter",
    "sensor-claim-sources",
    "sensor-linter",
    "sensor-required-sections",
    "sensor-traceability",
    "sensor-type-check",
    "sensor-upstream-coverage",
];

/// [`VERBLESS`] のうち、この build が配線したもの → (面, 写像先の argv)。
///
/// 本家ではこれらは面（`aidlc-<face>.ts`）を持たず、engine 自身が受ける `routeOnly` ルート
/// である（`.claude/tools/aidlc.ts` の `handleRouteOnly`）。この build でも engine の顔
/// （`Face::Orchestrate`）が受ける — `hook` と同じ扱いである。
const WIRED_VERBLESS: [(&str, Face, &[&str]); 1] =
    [("statusline", Face::Orchestrate, &["statusline"])];

impl EngineRoute {
    /// 全域フラグを剥がした残りから二段形を解決する。
    ///
    /// `rest[0]` が `engine` でなければ `None` を返す — multi-call 面の経路は変わらない。
    #[must_use]
    pub fn resolve(rest: &[String]) -> Option<EngineRoute> {
        if rest.first().map(String::as_str) != Some(ENGINE) {
            return None;
        }
        let Some(noun) = rest.get(1) else {
            return Some(EngineRoute::Unknown {
                noun: String::new(),
                verb: None,
            });
        };
        // フックは名前によらずこの build のフック面へ届ける。未知の名前の拒否は
        // フック面が既存の逐語で担う（オーナー裁定 D13 の `WT-1`）。
        if noun == "hook" {
            return Some(Self::mapped(Face::Orchestrate, &["hook"], rest.get(2..)));
        }
        // 動詞を取らない top ルートは、次のトークンを動詞として読まない。
        if VERBLESS.contains(&noun.as_str()) {
            if let Some((_, face, target)) = WIRED_VERBLESS
                .iter()
                .find(|(route_noun, _, _)| route_noun == noun)
            {
                return Some(Self::mapped(*face, target, rest.get(2..)));
            }
            return Some(EngineRoute::NotWired {
                noun: noun.clone(),
                verb: None,
            });
        }
        let verb = rest.get(2).filter(|token| !token.starts_with('-'));
        let Some(verb) = verb else {
            return Some(Self::unmapped(noun, None));
        };
        if let Some((_, _, face, target)) = WIRED
            .iter()
            .find(|(route_noun, route_verb, _, _)| route_noun == noun && route_verb == verb)
        {
            return Some(Self::mapped(*face, target, rest.get(3..)));
        }
        Some(Self::unmapped(noun, Some(verb.clone())))
    }

    /// 写像先の argv を組む — 前置き（写像表が持つ列）の後ろへ、残りの引数をそのまま運ぶ。
    fn mapped(face: Face, target: &[&str], carried: Option<&[String]>) -> EngineRoute {
        let mut argv: Vec<String> = target.iter().map(|part| (*part).to_string()).collect();
        argv.extend(carried.unwrap_or_default().iter().cloned());
        EngineRoute::Mapped { face, argv }
    }

    /// 写像の無い組を、本家の ROUTES 表にあるかどうかで振り分ける。
    fn unmapped(noun: &str, verb: Option<String>) -> EngineRoute {
        let known = ROUTES.iter().find(|(route_noun, _)| *route_noun == noun);
        let listed = match (known, verb.as_deref()) {
            (Some((_, verbs)), Some(verb)) => verbs.contains(&verb),
            (Some(_), None) => false,
            (None, _) => false,
        };
        if listed {
            EngineRoute::NotWired {
                noun: noun.to_string(),
                verb,
            }
        } else {
            EngineRoute::Unknown {
                noun: noun.to_string(),
                verb,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_string()).collect()
    }

    /// 一段形は一文字も経路が変わらない。
    #[test]
    fn a_launch_that_does_not_start_with_engine_is_left_alone() {
        assert_eq!(EngineRoute::resolve(&argv(&["next", "--resume"])), None);
        assert_eq!(EngineRoute::resolve(&[]), None);
    }

    /// noun-passthrough の組は、一段形の動詞の後ろに verb を位置引数として残す。
    #[test]
    fn a_noun_passthrough_pair_keeps_the_verb_as_a_positional_argument() {
        assert_eq!(
            EngineRoute::resolve(&argv(&["engine", "intent", "list", "--json"])),
            Some(EngineRoute::Mapped {
                face: Face::Utility,
                argv: argv(&["intent", "list", "--json"]),
            })
        );
    }

    /// 動詞を取らない top ルートでも、配線したものは engine の顔へ写る。
    ///
    /// 次のトークンは動詞として読まれず、引数としてそのまま運ばれる。
    #[test]
    fn a_wired_verbless_route_maps_to_the_engine_face() {
        assert_eq!(
            EngineRoute::resolve(&argv(&["engine", "statusline"])),
            Some(EngineRoute::Mapped {
                face: Face::Orchestrate,
                argv: argv(&["statusline"]),
            })
        );
        assert_eq!(
            EngineRoute::resolve(&argv(&["engine", "statusline", "--project-dir", "/tmp"])),
            Some(EngineRoute::Mapped {
                face: Face::Orchestrate,
                argv: argv(&["statusline", "--project-dir", "/tmp"]),
            })
        );
    }

    /// 写像のある組は、写像先の面と動詞へ写り、後続の引数をそのまま運ぶ。
    #[test]
    fn a_wired_pair_maps_to_the_existing_face_and_verb() {
        assert_eq!(
            EngineRoute::resolve(&argv(&["engine", "workspace", "codekb", "--repo", "app"])),
            Some(EngineRoute::Mapped {
                face: Face::Utility,
                argv: argv(&["codekb-path", "--repo", "app"]),
            })
        );
    }

    /// `review-brief` の 3 動詞は、本家と同じくレビュー面へ委譲される。
    ///
    /// 本家 ROUTES はこの noun を `TOOLS.reviewBrief` へ委ねるので、engine 自身が受ける
    /// `routeOnly` ルート（`statusline`）とは写り先が違う。
    #[test]
    fn the_three_review_brief_verbs_reach_the_review_face() {
        for verb in ["review", "context", "summary"] {
            assert_eq!(
                EngineRoute::resolve(&argv(&[
                    "engine",
                    "review-brief",
                    verb,
                    "--stage",
                    "requirements-analysis",
                ])),
                Some(EngineRoute::Mapped {
                    face: Face::ReviewBrief,
                    argv: argv(&[verb, "--stage", "requirements-analysis"]),
                })
            );
        }
    }

    /// 動詞の次の位置トークンは引数として素通しされ、動詞の一部にならない。
    #[test]
    fn a_token_after_the_verb_is_carried_as_an_argument() {
        assert_eq!(
            EngineRoute::resolve(&argv(&[
                "engine",
                "state",
                "reuse-artifact",
                "reverse-engineering",
                "--decision",
                "keep",
            ])),
            Some(EngineRoute::Mapped {
                face: Face::State,
                argv: argv(&[
                    "reuse-artifact",
                    "reverse-engineering",
                    "--decision",
                    "keep"
                ]),
            })
        );
    }

    /// フックは名前によらずこの build のフック面へ届く。
    #[test]
    fn a_hook_reaches_the_hook_face_whatever_the_name_is() {
        for name in ["write-audit-log", "plan-approval-guard", "not-a-hook"] {
            assert_eq!(
                EngineRoute::resolve(&argv(&["engine", "hook", name])),
                Some(EngineRoute::Mapped {
                    face: Face::Orchestrate,
                    argv: argv(&["hook", name]),
                })
            );
        }
    }

    /// 本家にあってこの build が配線していない入口は、その旨で拒否する。
    #[test]
    fn a_route_upstream_has_but_this_build_does_not_wire_is_reported_as_not_wired() {
        assert_eq!(
            EngineRoute::resolve(&argv(&["engine", "state", "practices-event"])),
            Some(EngineRoute::NotWired {
                noun: "state".to_string(),
                verb: Some("practices-event".to_string()),
            })
        );
        assert_eq!(
            EngineRoute::resolve(&argv(&["engine", "recompose", "--skip", "x"])),
            Some(EngineRoute::NotWired {
                noun: "recompose".to_string(),
                verb: None,
            })
        );
    }

    /// 本家にも無い noun / verb は、未知として拒否する。
    #[test]
    fn a_route_upstream_does_not_have_is_reported_as_unknown() {
        assert_eq!(
            EngineRoute::resolve(&argv(&["engine", "frobnicate", "run"])),
            Some(EngineRoute::Unknown {
                noun: "frobnicate".to_string(),
                verb: Some("run".to_string()),
            })
        );
        assert_eq!(
            EngineRoute::resolve(&argv(&["engine", "state", "frobnicate"])),
            Some(EngineRoute::Unknown {
                noun: "state".to_string(),
                verb: Some("frobnicate".to_string()),
            })
        );
    }

    /// 本家が `aidlc engine` の下で解決する noun は、未配線でも「本家にも無い」に落ちない。
    ///
    /// 一段形をこの build が配線しているかどうかは関係しない — `bolt set-autonomy` と
    /// `jump execute` はどちらも一段形の面を持つが、二段形の写像が無いので未配線である。
    #[test]
    fn every_engine_noun_upstream_resolves_is_reported_as_not_wired_rather_than_unknown() {
        for (noun, verb) in [
            ("audit", "append"),
            ("bolt", "start"),
            ("bolt", "set-autonomy"),
            ("jump", "execute"),
            ("knowledge", "onboard"),
            ("plugin", "select"),
            ("runtime", "summary"),
            ("scope", "change"),
            ("sensor", "fire"),
            ("space", "create"),
            ("swarm", "prepare"),
            ("validate", "outputs"),
            ("state", "set-status"),
            ("state", "init"),
        ] {
            assert_eq!(
                EngineRoute::resolve(&argv(&["engine", noun, verb])),
                Some(EngineRoute::NotWired {
                    noun: noun.to_string(),
                    verb: Some(verb.to_string()),
                }),
                "{noun} {verb}"
            );
            // 同じ noun でも、本家に無い動詞は本家の逐語で拒否する。
            assert_eq!(
                EngineRoute::resolve(&argv(&["engine", noun, "frobnicate"])),
                Some(EngineRoute::Unknown {
                    noun: noun.to_string(),
                    verb: Some("frobnicate".to_string()),
                }),
                "{noun} frobnicate"
            );
        }
    }

    /// 動詞を取らない engine ルートは、未配線でも動詞を綴らない。
    #[test]
    fn a_verbless_engine_route_is_reported_without_a_verb() {
        for noun in [
            "recompose",
            "status",
            "adapter",
            "sensor-claim-sources",
            "sensor-linter",
            "sensor-required-sections",
            "sensor-traceability",
            "sensor-type-check",
            "sensor-upstream-coverage",
        ] {
            assert_eq!(
                EngineRoute::resolve(&argv(&["engine", noun, "--path", "x"])),
                Some(EngineRoute::NotWired {
                    noun: noun.to_string(),
                    verb: None,
                }),
                "{noun}"
            );
        }
        // 本家に無い sensor 名は、動詞を取らない扱いにもならない。
        assert_eq!(
            EngineRoute::resolve(&argv(&["engine", "sensor-frobnicate", "--path", "x"])),
            Some(EngineRoute::Unknown {
                noun: "sensor-frobnicate".to_string(),
                verb: None,
            })
        );
    }
}
