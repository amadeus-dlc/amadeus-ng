//! argv → 型付きの要求（Controller の入口）。
//!
//! ここは**検証も業務判断も持たない**（10 §4 Controllers）。生の引数を型付きの値へ写し、
//! どの動詞へ行くかを決めるだけである。値の妥当性は消費側の値オブジェクトが決める。
//!
//! # 面の解決（マルチコール）
//!
//! 配布物は 1 つのバイナリで、**`argv[0]` がどのツールとして振る舞うかを決める**
//! （busybox 式。ADR 0002 決定 3 — 素の `aidlc-<tool>` 綴りが Markdown 資産・フック設定・
//! 文言に焼き込まれているため）。
//!
//! | 起動名 | 面 | 動詞 |
//! | --- | --- | --- |
//! | `aidlc-orchestrate` | エンジン | `next` / `continue` / `report` / `park` |
//! | `aidlc-utility` | ユーティリティ | `intent-create`（b29 の範囲） |
//! | `aidlc` | トップ | 上の 4 動詞をそのまま通す（top-passthrough） |
//!
//! **ディスパッチャの noun 形（`aidlc <noun> <verb>` の 30 経路）は実装していない。**
//! 逐語の写しが手元に無く、推測で綴りを作ると逸脱台帳 #1 の写像表と食い違うためである。
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

/// 起動名が指すツール面。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// `aidlc-orchestrate` / 素の `aidlc`。
    Orchestrate,
    /// `aidlc-utility`。
    Utility,
}

impl Face {
    /// `argv[0]` の basename から面を決める。
    ///
    /// 未知の名前は `Orchestrate` に倒す — 配布物の既定の顔がエンジンだからである。
    #[must_use]
    pub fn of(argv0: &str) -> Face {
        let name = argv0.rsplit(['/', '\\']).next().unwrap_or(argv0);
        // Windows の `.exe` 接尾辞を落としてから比べる。
        let name = name.strip_suffix(".exe").unwrap_or(name);
        match name {
            "aidlc-utility" => Face::Utility,
            _ => Face::Orchestrate,
        }
    }
}

/// `report` が運ぶ引数（upstream `parseReportFlags` — `aidlc-orchestrate.ts:4825`）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportArgs {
    result: Option<String>,
    stage: Option<String>,
    user_input: Option<String>,
    reason: Option<String>,
    skeleton_stance: Option<String>,
    single: bool,
}

impl ReportArgs {
    /// 報告された結末の生値（`--result`）。
    #[must_use]
    pub fn result(&self) -> Option<&str> {
        self.result.as_deref()
    }
    /// 明示されたステージ（`--stage`）。**有無それ自体が契約**なので `Option` で運ぶ。
    #[must_use]
    pub fn stage(&self) -> Option<&str> {
        self.stage.as_deref()
    }
    /// 承認時の人間入力（`--user-input`）。
    #[must_use]
    pub fn user_input(&self) -> Option<&str> {
        self.user_input.as_deref()
    }
    /// 読み飛ばし理由（`--reason`）。
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    /// skeleton stance の報告（`--skeleton-stance`）。
    #[must_use]
    pub fn skeleton_stance(&self) -> Option<&str> {
        self.skeleton_stance.as_deref()
    }
    /// 単独ステージ実行の報告（`--single`）。
    #[must_use]
    pub const fn is_single(&self) -> bool {
        self.single
    }
}

/// `intent-create` が運ぶ引数（upstream `aidlc-utility.ts:5989` の usage）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntentCreateArgs {
    scope: Option<String>,
    arguments: Option<String>,
    label: Option<String>,
    depth: Option<String>,
    test_strategy: Option<String>,
    review: Option<String>,
}

impl IntentCreateArgs {
    /// 鋳造する scope（`--scope`）。
    #[must_use]
    pub fn scope(&self) -> Option<&str> {
        self.scope.as_deref()
    }
    /// 自由記述（`--arguments`）。
    #[must_use]
    pub fn arguments(&self) -> Option<&str> {
        self.arguments.as_deref()
    }
    /// 記録ディレクトリ名の短いラベル（`--label`）。
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
    /// 深さの上書き（`--depth`）。
    #[must_use]
    pub fn depth(&self) -> Option<&str> {
        self.depth.as_deref()
    }
    /// テスト戦略の上書き（`--test-strategy`）。
    #[must_use]
    pub fn test_strategy(&self) -> Option<&str> {
        self.test_strategy.as_deref()
    }
    /// レビュー上限の上書き（`--review`）。
    #[must_use]
    pub fn review(&self) -> Option<&str> {
        self.review.as_deref()
    }
}

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
}

/// 全域フラグを剥がした残り（upstream の `--project-dir` 抽出と同じ前処理）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    project_dir: Option<String>,
    rest: Vec<String>,
}

impl Invocation {
    /// `--project-dir <path>` を剥がす。
    ///
    /// upstream は `--` 以降を literal として扱うので、そこから先の `--project-dir` は
    /// 剥がさない（`aidlc-orchestrate.ts:6111`）。
    #[must_use]
    pub fn strip_global_flags(args: &[String]) -> Invocation {
        let mut project_dir = None;
        let mut rest = Vec::new();
        let mut literal = false;
        let mut index = 0;
        while let Some(arg) = args.get(index) {
            if arg == "--" {
                literal = true;
                rest.push(arg.clone());
            } else if !literal && arg == "--project-dir" && index + 1 < args.len() {
                project_dir = args.get(index + 1).cloned();
                index += 1;
            } else {
                rest.push(arg.clone());
            }
            index += 1;
        }
        Invocation { project_dir, rest }
    }

    /// `--project-dir` の値。
    #[must_use]
    pub fn project_dir(&self) -> Option<&str> {
        self.project_dir.as_deref()
    }

    /// 残りの引数。
    #[must_use]
    pub fn rest(&self) -> &[String] {
        &self.rest
    }
}

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

fn parse_report(args: &[String]) -> ReportArgs {
    let mut flags = ReportArgs::default();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        let value = args.get(index + 1).cloned();
        let mut took_value = true;
        match arg.as_str() {
            "--result" => flags.result = value,
            "--user-input" => flags.user_input = value,
            "--reason" => flags.reason = value,
            "--skeleton-stance" => flags.skeleton_stance = value,
            "--stage" => flags.stage = value,
            "--single" => {
                flags.single = true;
                took_value = false;
            }
            _ => took_value = false,
        }
        if took_value {
            index += 1;
        }
        index += 1;
    }
    flags
}

fn parse_intent_create(args: &[String]) -> IntentCreateArgs {
    let mut flags = IntentCreateArgs::default();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        // upstream は `--arguments=<value>` の等号形も出す (`createPrintDirective`)。
        let (name, inline) = arg
            .split_once('=')
            .map_or((arg.as_str(), None), |(name, value)| {
                (name, Some(value.to_string()))
            });
        let value = inline.clone().or_else(|| args.get(index + 1).cloned());
        let mut took_value = inline.is_none();
        match name {
            "--scope" => flags.scope = value,
            "--arguments" => flags.arguments = value,
            "--label" => flags.label = value,
            "--depth" => flags.depth = value,
            "--test-strategy" => flags.test_strategy = value,
            "--review" => flags.review = value,
            _ => took_value = false,
        }
        if took_value {
            index += 1;
        }
        index += 1;
    }
    flags
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
    fn the_launch_name_selects_the_tool_face() {
        assert_eq!(Face::of("aidlc-utility"), Face::Utility);
        assert_eq!(Face::of("/usr/local/bin/aidlc-utility"), Face::Utility);
        assert_eq!(Face::of("aidlc-utility.exe"), Face::Utility);
        assert_eq!(Face::of("aidlc-orchestrate"), Face::Orchestrate);
        assert_eq!(Face::of("aidlc"), Face::Orchestrate);
        // 未知の名前はエンジンに倒す（配布物の既定の顔）。
        assert_eq!(Face::of("something-else"), Face::Orchestrate);
    }

    #[test]
    fn the_project_dir_flag_is_stripped_before_the_subcommand_is_read() {
        let invocation = Invocation::strip_global_flags(&argv(&[
            "--project-dir",
            "/tmp/ws",
            "next",
            "--resume",
        ]));
        assert_eq!(invocation.project_dir(), Some("/tmp/ws"));
        assert_eq!(invocation.rest(), argv(&["next", "--resume"]).as_slice());
    }

    /// `--` 以降は literal なので、そこの `--project-dir` は剥がさない。
    #[test]
    fn a_project_dir_after_the_literal_marker_stays_in_the_arguments() {
        let invocation =
            Invocation::strip_global_flags(&argv(&["next", "--", "--project-dir", "/tmp/ws"]));
        assert_eq!(invocation.project_dir(), None);
        assert_eq!(
            invocation.rest(),
            argv(&["next", "--", "--project-dir", "/tmp/ws"]).as_slice()
        );
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
