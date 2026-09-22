//! 実行時に出す指示（directive の本文・拒否文言・フックの案内）が綴るコマンドの契約。
//!
//! # 何を固定するか
//!
//! 本家 aidlc 2.8.2 をコンパイル済み実行形で動かすと、実行時の指示は
//! `aidlcInvocation()` → `"aidlc"` を経由するため **`aidlc engine <route> …`** を綴る
//! （出典 `.claude/tools/aidlc-runtime-paths.ts` の `aidlcInvocation` /
//! `aidlcDispatcherInvocation` / `aidlcToolInvocation`）。native はこのコンパイル済み実行形を
//! 置き換えるものなので、同じ綴りで指示を出さなければならない。
//!
//! 2.8.2 自身が別の綴りを出す箇所は例外として扱うが、**例外に足してよいのは 2.8.2 の実バイトで
//! 別の綴りが確認できた箇所だけ**である（綴りを推測しない）。この試験の例外表は 1 行ごとに
//! 出典の文言を持ち、その文言が実在することまで確かめる。
//!
//! # 出典の所在
//!
//! 実行時の指示の正本は、リポジトリに commit 済みの 2.8.2 配布物（`.claude/**`）の実バイト
//! そのものである。配布本文の入口と違って複製は作らない — 配布物が更新されれば指紋照合が
//! 落ち、実行時の指示を測り直す合図になるためである。対象は綴り規則の定義
//! （`.claude/tools/aidlc-runtime-paths.ts`）、エンジン側の呼び出し箇所
//! （`.claude/tools/aidlc-orchestrate.ts`）、Stop フックの案内文
//! （`.claude/hooks/aidlc-continue-workflow.ts`）である。受領証の記録を求める文言だけは
//! 状態面（`.claude/tools/aidlc-state.ts`）が綴るので、その 1 件の照合にだけ同じ配布物の
//! このファイルを読む。
//!
//! 採取元・版・sha256 の記録と実バイトの照合は
//! `tests/golden/selfhost-stage1/required-surface-sources/provenance.json` と
//! `required_surface_contract.rs` の `the_frozen_sources_are_the_2_8_2_distribution_bytes` が
//! 所有する（対象は `required-surface.json` が実行時の指示の出所として挙げる 4 本であり、
//! 状態面はそこに入らない）。この試験が見るのは、その実バイトに出典の文言が実在することだけ
//! である。
// 契約テストは固定の添字参照と panic を検証の合図として使う（既存の契約テストと同じ許容）。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use aidlc::cli::{EngineRoute, Face, parse};
use core_query_use_case::orchestration::{
    EngineCommand, ReadOnlyVerb, ScopeSlugView, StageSlugView,
};

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;

#[path = "../../../../tests/support/shell_split.rs"]
mod shell_split;
use shell_split::shell_split;

// ---------------------------------------------------------------------------
// 資料の所在
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// 綴り規則の定義（`aidlcInvocation` / `aidlcDispatcherInvocation` / `aidlcToolInvocation`）。
const UPSTREAM_RUNTIME_PATHS: &str = ".claude/tools/aidlc-runtime-paths.ts";
/// エンジン側 directive の呼び出し箇所。
const UPSTREAM_ORCHESTRATE: &str = ".claude/tools/aidlc-orchestrate.ts";
/// Stop フック（`continue-workflow`）の案内文。
const UPSTREAM_CONTINUE_HOOK: &str = ".claude/hooks/aidlc-continue-workflow.ts";
/// 状態面の拒否文言（レビュー受領証の記録を求める案内）。
const UPSTREAM_STATE: &str = ".claude/tools/aidlc-state.ts";

/// 2.8.2 配布物のファイル本文（リポジトリ直下の実バイト）。無ければ、その旨で落とす。
fn upstream(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "実行時の指示の出典が読めない（{relative}: {error}）— \
             2.8.2 配布物の実バイトが {} に無い",
            path.display()
        )
    })
}

/// 出典に upstream の文言が実在することを確かめる。
fn assert_upstream_spells(relative: &str, marker: &str, label: &str) {
    assert!(
        upstream(relative).contains(marker),
        "{label}: 2.8.2 の {relative} に出典の文言が無い — {marker}"
    );
}

// ---------------------------------------------------------------------------
// 2.8.2 が別の綴りを出す箇所（§9 Scenario 2 の `But` 節）
// ---------------------------------------------------------------------------

/// 例外 1 件。`spelling` はその綴りの接頭辞、残り 2 つは 2.8.2 の実バイトでの出典である。
struct Exception {
    spelling: &'static str,
    upstream_file: &'static str,
    upstream_marker: &'static str,
}

/// 例外表 — 2.8.2 が `aidlc engine …` 以外を綴ると実バイトで確認できた箇所だけ。
const EXCEPTIONS: &[Exception] = &[
    // 読み取り専用のうち `doctor` / `version` は engine を挟まず `aidlc <sub>` を綴る。
    Exception {
        spelling: "aidlc doctor",
        upstream_file: UPSTREAM_ORCHESTRATE,
        upstream_marker: r"`${aidlcInvocation()} ${sub}`",
    },
    Exception {
        spelling: "aidlc version",
        upstream_file: UPSTREAM_ORCHESTRATE,
        upstream_marker: r"`${aidlcInvocation()} ${sub}`",
    },
    // stale ポインタの回復報告は 2.8.2 でも生の配布入口形のままである。
    Exception {
        spelling: "bun .claude/tools/aidlc-orchestrate.ts report ",
        upstream_file: UPSTREAM_ORCHESTRATE,
        upstream_marker: r"Run \`bun ${harnessDir()}/tools/aidlc-orchestrate.ts report ",
    },
    // composer のディスパッチは 2.8.2 ではエージェントのファイルを名指すだけで、
    // `aidlc engine …` のコマンドを綴らない。
    Exception {
        spelling: "aidlc-composer detect",
        upstream_file: UPSTREAM_ORCHESTRATE,
        upstream_marker: "as a subagent to propose the workflow plan for:",
    },
    // DocumentKB の動詞だけは 2.8.2 も engine を挟まず専用ツールを名指す (分岐 1d)。
    Exception {
        spelling: "bun .claude/tools/aidlc-knowledge.ts ",
        upstream_file: UPSTREAM_ORCHESTRATE,
        upstream_marker: r"Run \`bun ${harnessDir()}/tools/aidlc-knowledge.ts ${verb}${suffix}\`",
    },
];

fn exception_for(spelling: &str) -> Option<&'static Exception> {
    EXCEPTIONS
        .iter()
        .find(|exception| spelling.starts_with(exception.spelling))
}

/// 例外はすべて、2.8.2 の実バイトが裏づける。
#[test]
fn every_exception_is_backed_by_the_frozen_2_8_2_bytes() {
    for exception in EXCEPTIONS {
        assert_upstream_spells(
            exception.upstream_file,
            exception.upstream_marker,
            exception.spelling,
        );
    }
}

/// 綴り規則の正本（`aidlcInvocation` が `aidlc` を返し、`aidlcToolInvocation` が
/// ディスパッチャ形へ畳まれる）が出典に実在する。
#[test]
fn the_frozen_sources_carry_the_2_8_2_spelling_rule() {
    let raw = upstream(UPSTREAM_RUNTIME_PATHS);
    for marker in [
        r#"const PROJECTED_INVOKE = "aidlc";"#,
        "export function aidlcDispatcherInvocation(route: string): string {",
        r"return `${aidlcInvocation()} engine ${route}`;",
        r#"if (!invoke.startsWith("bun ")) return aidlcDispatcherInvocation(route);"#,
    ] {
        assert!(
            raw.contains(marker),
            "{UPSTREAM_RUNTIME_PATHS}: 綴り規則の正本が 2.8.2 の実バイトに無い — {marker}"
        );
    }
}

// ---------------------------------------------------------------------------
// `EngineCommand` の全ヴァリアントの綴り
// ---------------------------------------------------------------------------

/// ヴァリアント名。**新しいヴァリアントを足すとこの `match` がコンパイルエラーになる**ので、
/// 綴りの検査から漏れたまま追加できない。
const fn variant(command: &EngineCommand) -> &'static str {
    match command {
        EngineCommand::ReadOnlyUtility(_) => "ReadOnlyUtility",
        EngineCommand::NounTokens(_) => "NounTokens",
        EngineCommand::Unpark => "Unpark",
        EngineCommand::ExecuteJump { .. } => "ExecuteJump",
        EngineCommand::MintIntent { .. } => "MintIntent",
        EngineCommand::ChangeScope { .. } => "ChangeScope",
        EngineCommand::ChangeConfig { .. } => "ChangeConfig",
        EngineCommand::DispatchComposer => "DispatchComposer",
        EngineCommand::ReportSkipped { .. } => "ReportSkipped",
    }
}

/// `variant` が名乗りうる全ヴァリアント。代表値の取りこぼしはこの集合との突合で落ちる。
const VARIANTS: [&str; 9] = [
    "ReadOnlyUtility",
    "NounTokens",
    "Unpark",
    "ExecuteJump",
    "MintIntent",
    "ChangeScope",
    "ChangeConfig",
    "DispatchComposer",
    "ReportSkipped",
];

fn scope(value: &str) -> ScopeSlugView {
    ScopeSlugView::parse(value).expect("固定の scope 名")
}

fn stage(value: &str) -> StageSlugView {
    StageSlugView::parse(value).expect("固定の stage slug")
}

/// 全ヴァリアントの代表値。
fn samples() -> Vec<EngineCommand> {
    vec![
        EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Status),
        EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Help),
        EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Doctor),
        EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Version),
        EngineCommand::NounTokens(vec!["intent".to_string(), "list".to_string()]),
        // DocumentKB の動詞は 2.8.2 の例外を踏む枝である (分岐 1d)。
        EngineCommand::NounTokens(vec!["knowledge".to_string(), "list".to_string()]),
        EngineCommand::Unpark,
        EngineCommand::ExecuteJump {
            stage: stage("reverse-engineering"),
            direction: "forward".to_string(),
            scope: scope("bugfix"),
        },
        EngineCommand::MintIntent {
            scope: scope("bugfix"),
            description: Some("fix the crash".to_string()),
            depth: None,
            test_strategy: None,
            review: None,
        },
        EngineCommand::ChangeScope {
            scope: scope("bugfix"),
            depth: None,
            test_strategy: None,
            review: None,
        },
        EngineCommand::ChangeConfig {
            depth: Some("standard".to_string()),
            test_strategy: None,
            review: None,
        },
        EngineCommand::DispatchComposer,
        EngineCommand::ReportSkipped {
            stage: stage("domain-design"),
        },
    ]
}

/// 代表値は全ヴァリアントを覆う。
#[test]
fn the_samples_cover_every_engine_command_variant() {
    let covered: BTreeSet<&str> = samples().iter().map(variant).collect();
    let expected: BTreeSet<&str> = VARIANTS.into_iter().collect();
    assert_eq!(covered, expected, "代表値が覆っていないヴァリアントがある");
}

/// 実行時の指示が綴るコマンドは、2.8.2 と同じ二段形である。
///
/// 例外は 2.8.2 の実バイトが裏づける `EXCEPTIONS` の 5 件（`doctor` / `version` / stale ポインタの
/// 回復報告 / composer のディスパッチ / DocumentKB の専用ツール）だけで、それ以外の綴りに
/// 2.7.1 の配布入口形（`bun .claude/tools/…`）は 1 件も残らない。
#[test]
fn every_engine_command_spells_the_two_stage_form() {
    for command in samples() {
        let spelled = command.cli_spelling();
        if let Some(exception) = exception_for(&spelled) {
            assert_upstream_spells(exception.upstream_file, exception.upstream_marker, &spelled);
            continue;
        }
        assert!(
            spelled.starts_with("aidlc engine "),
            "{}: 実行時の指示が二段形で綴られていない — {spelled}",
            variant(&command)
        );
        assert!(
            !spelled.contains("bun .claude/tools/"),
            "{}: 2.7.1 の配布入口形が残っている — {spelled}",
            variant(&command)
        );
    }
}

/// intent 作成の指示は、2.8.2 と同じ引数面のまま二段形で綴られる。
///
/// bugfix 1 周の開始でこの 1 本だけが出る（`order.md` §1 の失敗点）。
#[test]
fn the_mint_intent_directive_spells_the_two_stage_form_with_the_upstream_argument_face() {
    let spelled = EngineCommand::MintIntent {
        scope: scope("bugfix"),
        description: Some("fix the crash".to_string()),
        depth: None,
        test_strategy: None,
        review: None,
    }
    .cli_spelling();
    assert_eq!(
        spelled,
        "aidlc engine intent create --scope bugfix --arguments='fix the crash' \
         --label \"<2-3 word kebab essence>\""
    );
    assert_upstream_spells(
        UPSTREAM_ORCHESTRATE,
        r#"aidlcDispatcherInvocation("intent create")"#,
        "intent create",
    );
}

/// 二段形で綴るコマンドは、本家 ROUTES が知る入口を指す（`Unknown` に落ちない）。
///
/// 配線の有無（`Mapped` か `NotWired` か）とは別の軸である — `Unknown` は
/// 「本家にも無い入口を native が名指した」ことを意味し、綴りそのものの誤りである。
#[test]
fn every_two_stage_spelling_names_a_route_upstream_knows() {
    for command in samples() {
        let spelled = command.cli_spelling();
        if exception_for(&spelled).is_some() {
            continue;
        }
        let argv = route_argv(&spelled);
        let resolved = EngineRoute::resolve(&argv);
        assert!(
            !matches!(resolved, Some(EngineRoute::Unknown { .. })),
            "{}: 本家 ROUTES に無い入口を名指している — {spelled} → {resolved:?}",
            variant(&command)
        );
    }
}

/// bugfix 1 周で出る指示は、綴りをそのまま渡してこの build が受理する。
///
/// 受理の判定は doctor の `Native engine entry points`（`runtime/doctor.rs` の `missing_from`）と
/// 同じ — 二段形を `EngineRoute::resolve` に通し、写せた面と動詞を `parse` して未知・未配線に
/// 落ちないことを見る。
#[test]
fn the_directive_the_bugfix_loop_starts_with_is_accepted_by_this_build() {
    let spelled = EngineCommand::MintIntent {
        scope: scope("bugfix"),
        description: None,
        depth: None,
        test_strategy: None,
        review: None,
    }
    .cli_spelling();
    let argv = route_argv(&spelled);
    let Some(EngineRoute::Mapped { face, argv: target }) = EngineRoute::resolve(&argv) else {
        panic!(
            "intent 作成の指示が二段形で写せない — {spelled} → {:?}",
            EngineRoute::resolve(&argv)
        );
    };
    assert!(
        parse(face, &target).is_wired(),
        "intent 作成の写像先がこの build に拒否された — {spelled}"
    );
}

/// 既に受理している入口は、写像表の書き換えで落ちない。
#[test]
fn the_already_wired_entry_points_keep_resolving() {
    for (noun, verb) in [
        ("orchestrate", "next"),
        ("orchestrate", "continue"),
        ("orchestrate", "report"),
        ("orchestrate", "park"),
        ("log", "link"),
        ("intent", "list"),
        // オーナー裁定 2026-09-22 D20 で加わった組。一段形は元から実装済みだった。
        ("jump", "execute"),
    ] {
        let argv = ["engine", noun, verb].map(str::to_string).to_vec();
        assert!(
            matches!(
                EngineRoute::resolve(&argv),
                Some(EngineRoute::Mapped { .. })
            ),
            "engine {noun} {verb}: 既存の受理が壊れている"
        );
    }
}

/// 綴りの先頭 `aidlc` を落とし、`EngineRoute::resolve` が読む残りを作る。
fn route_argv(spelled: &str) -> Vec<String> {
    spelled
        .split_whitespace()
        .skip(1)
        .map(str::to_string)
        .collect()
}

// ---------------------------------------------------------------------------
// 本番コードに残る一段形の配布入口
// ---------------------------------------------------------------------------

/// 綴りを生成箇所ごとに手で書いてよい箇所（2.8.2 も同じ生の形を綴ると確認できた箇所だけ）。
struct AllowedLiteral {
    file: &'static str,
    fragment: &'static str,
    upstream_file: &'static str,
    upstream_marker: &'static str,
}

/// 一段形の配布入口を本番コードに残してよい箇所（[`spells_one_stage_entry_form`] の両形）。
const ALLOWED_LITERALS: &[AllowedLiteral] = &[
    // Stop フックの案内文 3 綴り — 2.8.2 の同じ場面も生の配布入口形を綴る。
    AllowedLiteral {
        file: "modules/app/aidlc/src/runtime/continuation.rs",
        fragment: "bun .claude/tools/aidlc-orchestrate.ts continue",
        upstream_file: UPSTREAM_CONTINUE_HOOK,
        upstream_marker: r#"`bun ${harnessDir()}/tools/aidlc-orchestrate.ts continue "${continueToken}"\` `"#,
    },
    AllowedLiteral {
        file: "modules/app/aidlc/src/runtime/continuation.rs",
        fragment: "bun .claude/tools/aidlc-orchestrate.ts next",
        upstream_file: UPSTREAM_CONTINUE_HOOK,
        upstream_marker: r"`bun ${harnessDir()}/tools/aidlc-orchestrate.ts next\`, do what the step it prints ",
    },
    AllowedLiteral {
        file: "modules/app/aidlc/src/runtime/continuation.rs",
        fragment: "bun .claude/tools/aidlc-orchestrate.ts park",
        upstream_file: UPSTREAM_CONTINUE_HOOK,
        upstream_marker: r"session, run \`bun ${harnessDir()}/tools/aidlc-orchestrate.ts park\` to stop ",
    },
    // stale ポインタの回復報告 — 2.8.2 も生の配布入口形を綴る。
    AllowedLiteral {
        file: "modules/core/query/use-case/src/orchestration/engine_command.rs",
        fragment: "bun .claude/tools/aidlc-orchestrate.ts report",
        upstream_file: UPSTREAM_ORCHESTRATE,
        upstream_marker: r"Run \`bun ${harnessDir()}/tools/aidlc-orchestrate.ts report ",
    },
    // DocumentKB の終端案内 — 2.8.2 も engine を挟まず専用ツールを名指す (分岐 1d)。
    AllowedLiteral {
        file: "modules/core/query/use-case/src/orchestration/engine_command.rs",
        fragment: "bun .claude/tools/aidlc-knowledge.ts",
        upstream_file: UPSTREAM_ORCHESTRATE,
        upstream_marker: r"Run \`bun ${harnessDir()}/tools/aidlc-knowledge.ts ${verb}${suffix}\`",
    },
    // レビュー受領証の記録を求める案内 — 2.8.2 の状態面も一段形の `aidlc-log.ts review` を綴る。
    AllowedLiteral {
        file: "modules/app/aidlc/src/wording.rs",
        fragment: "aidlc-log.ts review",
        upstream_file: UPSTREAM_STATE,
        upstream_marker: r"\`aidlc-log.ts review --stage ",
    },
    // Stop フックの報告の案内 — 2.8.2 の同じ場面も一段形の `aidlc-orchestrate report` を綴る。
    AllowedLiteral {
        file: "modules/app/aidlc/src/runtime/continuation.rs",
        fragment: "`aidlc-orchestrate report",
        upstream_file: UPSTREAM_CONTINUE_HOOK,
        upstream_marker: "asks, then run `aidlc-orchestrate report --stage <stage> --result <outcome>` to record ",
    },
];

/// 2.7.1 の配布入口形 — `bun .claude/tools/aidlc-<face>.ts <verb>`。
const DISTRIBUTION_ENTRY_FORM: &str = "bun .claude/tools/aidlc-";
/// 逐語文言がバッククォートで括る一段形 — `` `aidlc-<face> <verb>` ``（`bun ` を伴わない）。
const BACKTICKED_ONE_STAGE_FORM: &str = "`aidlc-";

/// この行が一段形の配布入口を**指示として**綴っているか。
///
/// 一段形は 2 つの形で出る。`bun .claude/tools/aidlc-<face>.ts` の生の配布入口形と、
/// 逐語文言がバッククォートで括る `` `aidlc-<face> <verb>` `` である。後者は `bun ` の字面を
/// 持たないので、配布入口形だけを見ていると綴りの取り残しが観測できない。
///
/// upstream の出典を `ファイル:行` で引く doc コメントは綴りの指示ではないので、後者からは
/// 外す（`// ` で始まる行は `///` と `//!` も含めてコメントである）。前者は 2.8.2 の
/// 実バイトでの出典を `ALLOWED_LITERALS` が持つ形なのでコメントも区別せず見る。
fn spells_one_stage_entry_form(line: &str) -> bool {
    line.contains(DISTRIBUTION_ENTRY_FORM)
        || (!line.trim_start().starts_with("//") && line.contains(BACKTICKED_ONE_STAGE_FORM))
}

/// 実行時の指示を出す本番コードに、一段形の配布入口が残っていない。
///
/// `EngineCommand` の網羅検査は**値として取り出せる綴り**しか見られない。指示の本文へ直に
/// 綴りを埋める箇所（拒否文言・フックの案内文）は値にならないので、本番ソースの文字列
/// リテラルを走査して同じ不変条件を見る。許可するのは 2.8.2 も同じ一段形を綴ると
/// 実バイトで確認できた箇所だけである。
#[test]
fn no_production_site_spells_the_2_7_1_distribution_entry_form() {
    for allowed in ALLOWED_LITERALS {
        assert_upstream_spells(
            allowed.upstream_file,
            allowed.upstream_marker,
            allowed.fragment,
        );
    }
    let mut offenders: Vec<String> = Vec::new();
    for relative in production_sources() {
        let raw = fs::read_to_string(repo_root().join(&relative))
            .unwrap_or_else(|error| panic!("本番ソースが読めない ({relative}): {error}"));
        if !raw.contains(DISTRIBUTION_ENTRY_FORM) && !raw.contains(BACKTICKED_ONE_STAGE_FORM) {
            continue;
        }
        // `#[cfg(test)]` から先はテスト側である。印が 2 つ以上ある構成では前半だけを
        // 本番と見なす判定が成り立たないので、その場で落として分類をやり直させる。
        assert!(
            raw.matches("#[cfg(test)]").count() <= 1,
            "{relative}: `#[cfg(test)]` が複数あり、本番/テストの切り分けができない"
        );
        for (index, line) in production_lines(&raw) {
            if !spells_one_stage_entry_form(line) {
                continue;
            }
            let allowed = ALLOWED_LITERALS
                .iter()
                .any(|entry| entry.file == relative && line.contains(entry.fragment));
            if !allowed {
                offenders.push(format!("{relative}:{}: {}", index + 1, line.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "実行時の指示に一段形の配布入口が残っている（2.8.2 は `aidlc engine …` を綴る）:\n{}",
        offenders.join("\n")
    );
}

/// `#[cfg(test)]` より前の行だけを（0 起点の行番号つきで）返す。
fn production_lines(raw: &str) -> Vec<(usize, &str)> {
    raw.lines()
        .enumerate()
        .take_while(|(_, line)| line.trim() != "#[cfg(test)]")
        .collect()
}

/// 実行時の指示を組み立てるクレートの本番ソース（リポジトリ相対）。
fn production_sources() -> Vec<String> {
    let mut found = Vec::new();
    for crate_src in [
        "modules/core/query/use-case/src",
        "modules/app/aidlc/src",
        "modules/harness/claude/src",
    ] {
        collect_rust_sources(&repo_root().join(crate_src), crate_src, &mut found);
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "本番ソースが 1 件も見つからない — 走査の起点が誤っている"
    );
    found
}

fn collect_rust_sources(dir: &Path, prefix: &str, found: &mut Vec<String>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
    for entry in entries {
        let entry = entry.expect("ディレクトリ要素");
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = format!("{prefix}/{name}");
        if entry.file_type().expect("種別").is_dir() {
            collect_rust_sources(&entry.path(), &relative, found);
        } else if name.ends_with(".rs") {
            found.push(relative);
        }
    }
}

// ---------------------------------------------------------------------------
// 開始の通し検査 — 指示をそのまま実行して 1 周が始まる
// ---------------------------------------------------------------------------

/// bugfix スコープの合成グラフだけを置いた、記録の無い一時ワークスペース。
struct Workspace {
    temp: tempfile::TempDir,
}

impl Workspace {
    fn new() -> Workspace {
        let temp = tempfile::tempdir().expect("一時ディレクトリ");
        let root = temp.path().join("workspace");
        fs::create_dir(&root).expect("workspace");
        let data = root.join(".claude/tools/data");
        fs::create_dir_all(&data).expect("data");
        let repository = repo_root();
        for name in ["stage-graph.json", "scope-grid.json", "harness.json"] {
            fs::copy(
                repository
                    .join("tests/golden/upstream-a277af21/data")
                    .join(name),
                data.join(name),
            )
            .expect("合成グラフ");
        }
        fs::create_dir_all(root.join(".claude/scopes")).expect("scopes");
        fs::copy(
            repository.join(".claude/scopes/aidlc-bugfix.md"),
            root.join(".claude/scopes/aidlc-bugfix.md"),
        )
        .expect("scope 定義");
        // 面は `argv[0]` の葉名で決まる。指示が綴る `aidlc` の名前で置く。
        tool_link::link_tool(&temp.path().join("aidlc")).expect("aidlc");
        fs::create_dir_all(root.join("aidlc/spaces/default/intents")).expect("intents");
        Workspace { temp }
    }

    fn root(&self) -> PathBuf {
        self.temp.path().join("workspace")
    }

    /// 指定した名前で 1 回起動する。`PATH` には bun を置かない。
    fn launch(&self, argv0: &str, args: &[String]) -> Output {
        Command::new(self.temp.path().join(argv0))
            .args(args)
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path().join("home"))
            .env("PATH", NO_BUN_PATH)
            .output()
            .expect("起動")
    }

    fn record(&self) -> Option<PathBuf> {
        let intents = self.root().join("aidlc/spaces/default/intents");
        fs::read_to_string(intents.join("active-intent"))
            .ok()
            .map(|name| intents.join(name.trim()))
    }
}

/// 配布版 TypeScript の実行系を 1 つも持たない `PATH`。
const NO_BUN_PATH: &str = "/usr/bin:/bin";

/// 1 行 directive の指定キーを読む。
fn directive(output: &Output) -> serde_json::Value {
    let stdout = String::from_utf8(output.stdout.clone()).expect("UTF-8");
    serde_json::from_str(stdout.trim()).unwrap_or_else(|error| {
        panic!(
            "directive が 1 行 JSON でない ({error}): stdout={stdout} stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn field(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("{key} が無い: {value}"))
        .to_string()
}

/// print が綴るコマンドをバッククォートから切り出す。
fn quoted_command(message: &str) -> String {
    message
        .split('`')
        .nth(1)
        .unwrap_or_else(|| panic!("print はコマンドをバッククォートで括る: {message}"))
        .to_string()
}

/// 試験環境に bun が無いこと自体を先に確かめる（`CT-NO-BUN` の前提）。
#[test]
fn the_contract_path_carries_no_bun_runtime() {
    for dir in NO_BUN_PATH.split(':') {
        assert!(
            !Path::new(dir).join("bun").exists(),
            "{dir}: この PATH に bun が居ると「配布版を起動しない」ことを示せない"
        );
    }
}

/// 指示をそのまま実行するだけで bugfix 1 周が始まる。
///
/// 1. 記録の無い作業領域で `aidlc engine orchestrate next --scope bugfix "<説明>"` を実行する。
/// 2. 返った print の綴りを**書き換えずに**（ラベルのプレースホルダだけを畳んで）実行する。
/// 3. native のストアに intent が 1 件入り、続く `next` が bugfix 最初のステージを名指す。
///
/// 全行程で `PATH` に bun が無い — 配布版 TypeScript は一度も起動できない。
#[tokio::test]
async fn the_named_command_starts_the_bugfix_loop_without_the_distributed_runtime() {
    use core_command_domain::workspace::{SpaceName, StorePath};
    use core_command_use_case::orchestration::IntentRepository as _;

    let workspace = Workspace::new();
    let named = workspace.launch(
        "aidlc",
        &[
            "engine",
            "orchestrate",
            "next",
            "--scope",
            "bugfix",
            "fix the crash",
        ]
        .map(str::to_string),
    );
    assert!(named.status.success(), "{named:?}");
    let print = directive(&named);
    assert_eq!(field(&print, "kind"), "print", "{print}");
    let message = field(&print, "message");
    assert!(
        !message.contains("bun .claude/tools/"),
        "誕生の print に 2.7.1 の呼び方が残っている — {message}"
    );
    let command = quoted_command(&message);
    assert!(
        command.starts_with("aidlc engine intent create --scope bugfix"),
        "誕生の print が二段形を綴っていない — {command}"
    );

    // 唯一の置換は upstream が明示的に conductor へ委ねたラベルのプレースホルダである。
    let argv = shell_split(&command.replace("<2-3 word kebab essence>", "crash fix"));
    let (runner, rest) = argv.split_first().expect("argv0 がある");
    assert_eq!(
        runner, "aidlc",
        "指示が配布版の実行系を名指している — {command}"
    );
    let created = workspace.launch(runner, rest);
    assert!(created.status.success(), "{created:?}");

    // native のストアに intent が 1 件入っている（配布版が作る記録ではない）。
    let record = workspace.record().expect("記録が作られている");
    let cursor = aidlc::execution_cursor::ExecutionCursor::read(&record)
        .expect("実行カーソルが読める")
        .expect("実行カーソルが据わっている");
    let store = StorePath::for_space(&workspace.root().join("aidlc"), &SpaceName::default());
    let repository =
        core_command_interface_adapter::orchestration::IntentRepositoryImpl::open(&store)
            .expect("ストアが開ける");
    repository
        .find_by_id(cursor.intent_id())
        .await
        .expect("native のイベントストアに intent が入っている");
    drop(repository);

    // 続く `next` は bugfix の最初のステージを名指す（`No workflow state found` で止まらない）。
    //
    // この作業領域は記録も原本も無い greenfield なので、brownfield 条件つきの
    // `reverse-engineering` は計画から落ち、1 周は `requirements-analysis` から始まる。
    let resumed = workspace.launch(
        "aidlc",
        &["engine", "orchestrate", "next"].map(str::to_string),
    );
    assert!(resumed.status.success(), "{resumed:?}");
    let next = directive(&resumed);
    assert_eq!(
        field(&next, "kind"),
        "run-stage",
        "1 周が続かず別の指示に落ちた — {next}"
    );
    assert_eq!(
        field(&next, "stage"),
        "requirements-analysis",
        "1 周の最初のステージへ着地していない — {next}"
    );
}

/// 二段形 `intent create` は、一段形 `intent-create` と同じ要求へ写る。
///
/// 写像だけで受理する（新しいユースケースもイベントも作らない）ことを、要求の同値で見る。
#[test]
fn the_two_stage_intent_create_resolves_to_the_one_stage_request() {
    let tail = [
        "--scope",
        "bugfix",
        "--arguments=fix the crash",
        "--label",
        "crash fix",
    ]
    .map(str::to_string);

    let mut two_stage = vec![
        "engine".to_string(),
        "intent".to_string(),
        "create".to_string(),
    ];
    two_stage.extend(tail.iter().cloned());
    let Some(EngineRoute::Mapped { face, argv }) = EngineRoute::resolve(&two_stage) else {
        panic!(
            "二段形 `intent create` が写らない — {:?}",
            EngineRoute::resolve(&two_stage)
        );
    };

    let mut one_stage = vec!["intent-create".to_string()];
    one_stage.extend(tail.iter().cloned());

    assert_eq!(face, Face::of("aidlc-utility"), "写像先の面が違う");
    assert_eq!(
        parse(face, &argv),
        parse(Face::Utility, &one_stage),
        "二段形と一段形が別の要求になっている"
    );
}
