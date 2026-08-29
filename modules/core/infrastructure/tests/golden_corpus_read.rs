//! cli / hooks ゴールデンコーパスの読取と正規化 (FR7.2 / BR2.1 / BR2.2 / BR2.4)。
//!
//! 本 Unit (U1) が固定するのは「コーパスが読めて、正規化規則が適用できて、範囲を満たして
//! いる」ところまでである。実装出力との突合せは U6 (next / continue) と U7 (CLI・フック)
//! が同じ比較器を使って行う。
//!
//! 不一致が出たときに直すのは実装であってゴールデンではない (BR2.3 / BR2.5)。

// テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
// allow)。panic! は想定外ケースの即時失敗という検証用途で使っており、テスト失敗の
// シグナルとして妥当なため同様に許容する。
#![allow(clippy::indexing_slicing, clippy::panic)]

mod support;

use support::{Channel, Normalization, RuntimeValues, cases, corpus_root, line_diff, read_json};

/// BR2.4 が最小集合として要求する CLI の遷移。`<verb>/<case>` のうち verb だけを見る。
const REQUIRED_CLI_VERBS: &[&str] = &[
    "next",
    "continue",
    "report",
    "skip",
    "jump",
    "park",
    "unpark",
    "recompose",
    "set-autonomy",
];

/// BR2.4 が要求する report の結果値。`report/<case>` の case 側を見る。
const REQUIRED_REPORT_CASES: &[&str] = &["awaiting-approval", "approved", "rejected", "revised"];

/// C2 のフック 4 本。
const REQUIRED_HOOKS: &[&str] = &[
    "stop-forwarding-loop",
    "record-human-turn",
    "state-transition-guard",
    "write-audit-log",
];

/// cli 族の 1 ケースが必ず持つファイル (`stdout.json` / `stdout.txt` は排他なので別扱い)。
const CLI_REQUIRED_FILES: &[&str] = &[
    "argv",
    "stdin",
    "exit",
    "stderr",
    "state.diff",
    "audit.md",
    "case.json",
];

/// hooks 族の 1 ケースが必ず持つファイル。
const HOOK_REQUIRED_FILES: &[&str] = &[
    "stdin.json",
    "exit",
    "stderr",
    "stdout",
    "audit.md",
    "case.json",
];

/// 正規化規則が使ってよいプレースホルダ (NFR1.3 — この 4 種のみ)。
const ALLOWED_PLACEHOLDERS: &[&str] = &["<TS>", "<CLONE>", "<ROOT>", "<SESSION>"];

#[test]
fn every_cli_case_is_readable() {
    let rows = cases("cli");
    assert!(!rows.is_empty(), "cli 族のケースが 1 件も読めない");

    for case in &rows {
        for name in CLI_REQUIRED_FILES {
            assert!(
                case.read(name).is_some(),
                "{}: {name} が無い ({})",
                case.id(),
                case.dir().display()
            );
        }
        let stdout = case.read("stdout.json").or_else(|| case.read("stdout.txt"));
        assert!(
            stdout.is_some(),
            "{}: stdout.json も stdout.txt も無い",
            case.id()
        );

        let argv = case.read("argv").unwrap_or_default();
        let parsed: serde_json::Value = serde_json::from_str(&argv)
            .unwrap_or_else(|e| panic!("{}: argv が JSON でない: {e}", case.id()));
        let argv_items = parsed
            .as_array()
            .unwrap_or_else(|| panic!("{}: argv が配列でない", case.id()));
        assert!(argv_items.len() >= 3, "{}: argv が短すぎる", case.id());
        assert_eq!(
            argv_items.first().and_then(serde_json::Value::as_str),
            Some("bun"),
            "{}: argv の先頭が bun でない",
            case.id()
        );

        let exit = case.read("exit").unwrap_or_default();
        exit.trim()
            .parse::<i32>()
            .unwrap_or_else(|e| panic!("{}: exit が整数でない ({exit:?}): {e}", case.id()));
    }
}

#[test]
fn every_hook_case_is_readable() {
    let rows = cases("hooks");
    assert!(!rows.is_empty(), "hooks 族のケースが 1 件も読めない");

    for case in &rows {
        for name in HOOK_REQUIRED_FILES {
            assert!(case.read(name).is_some(), "{}: {name} が無い", case.id());
        }
        let stdin = case.read("stdin.json").unwrap_or_default();
        serde_json::from_str::<serde_json::Value>(&stdin)
            .unwrap_or_else(|e| panic!("{}: stdin.json が JSON でない: {e}", case.id()));

        let exit = case.read("exit").unwrap_or_default();
        let code = exit
            .trim()
            .parse::<i32>()
            .unwrap_or_else(|e| panic!("{}: exit が整数でない: {e}", case.id()));
        assert!(
            code == 0 || code == 2,
            "{}: フックの終了コードは 0 (許可) か 2 (拒否) のはずが {code}",
            case.id()
        );
        if code == 2 {
            assert!(
                !case.read("stderr").unwrap_or_default().trim().is_empty(),
                "{}: 拒否 (exit 2) なのに stderr に理由が無い",
                case.id()
            );
        }
    }
}

#[test]
fn both_families_carry_their_provenance() {
    for family in ["cli", "hooks"] {
        let provenance = read_json(&format!("{family}/provenance.json"));
        assert_eq!(
            provenance["upstream_commit"].as_str(),
            Some("3c3146cfd7cef33020d48e8d48d4e80d0f8c2820"),
            "{family}: 来歴の upstream commit が違う"
        );
        for field in [
            "fetch_method",
            "tree_manifest_sha256",
            "captured_at",
            "command",
            "bun_version",
        ] {
            assert!(
                provenance[field].as_str().is_some_and(|v| !v.is_empty()),
                "BR2.1: {family}/provenance.{field} が無い"
            );
        }
        assert!(
            provenance["non_interactive_env"].is_object(),
            "BR2.1: {family}: 非対話化に使った env が記録されていない"
        );
        assert_eq!(
            provenance["case_count"].as_u64(),
            Some(cases(family).len() as u64),
            "{family}: 来歴のケース数とディスク上のケース数が食い違う"
        );

        for case in cases(family) {
            let meta = read_json(&format!("{family}/{}/case.json", case.id()));
            assert_eq!(
                meta["provenance"]["commit"].as_str(),
                Some("3c3146cfd7cef33020d48e8d48d4e80d0f8c2820"),
                "{}: ケース単位の provenance が無い",
                case.id()
            );
            assert_eq!(
                meta["id"].as_str(),
                Some(format!("{family}/{}", case.id()).as_str()),
                "{}: case.json の id がディレクトリと食い違う",
                case.id()
            );
        }
    }
}

#[test]
fn the_br2_4_range_is_covered() {
    let cli: Vec<String> = cases("cli").iter().map(|c| c.id().to_string()).collect();
    for verb in REQUIRED_CLI_VERBS {
        assert!(
            cli.iter().any(|id| id.starts_with(&format!("{verb}/"))),
            "BR2.4 の遷移 {verb} が cli 族に無い"
        );
    }
    for result in REQUIRED_REPORT_CASES {
        assert!(
            cli.iter().any(|id| id == &format!("report/{result}")),
            "BR2.4 の report --result {result} が無い"
        );
    }

    let hooks: Vec<String> = cases("hooks").iter().map(|c| c.id().to_string()).collect();
    for hook in REQUIRED_HOOKS {
        let count = hooks
            .iter()
            .filter(|id| id.starts_with(&format!("{hook}/")))
            .count();
        assert!(
            count >= 2,
            "BR2.4: フック {hook} の代表ケースが {count} 件しかない (2〜3 件必要)"
        );
    }
}

#[test]
fn the_normalization_rules_load_from_the_corpus() {
    let norm = Normalization::load();
    assert!(
        norm.rule_count() >= 4,
        "正規化規則が {} 本しか読めない",
        norm.rule_count()
    );
    for placeholder in norm.placeholders() {
        assert!(
            ALLOWED_PLACEHOLDERS.contains(&placeholder.as_str()),
            "NFR1.3: 許されないプレースホルダ {placeholder}"
        );
    }
}

#[test]
fn normalization_replaces_every_environment_specific_value() {
    let norm = Normalization::load();
    let runtime = RuntimeValues::new(
        vec!["/tmp/aidlc-golden-cli-abc123".to_string()],
        vec!["build-host-1a2b3c4d5e6f".to_string()],
    );
    let raw = concat!(
        "- **Start Date**: 2026-08-22T13:43:00Z\n",
        "- **Project Root**: /tmp/aidlc-golden-cli-abc123\n",
        "shard build-host-1a2b3c4d5e6f\n",
        "path aidlc/spaces/default/intents/260822-golden/inception\n",
        "bare 260822-golden\n",
        "session 11111111-2222-4333-8444-555555555555\n",
        "token AAAAAAAAAABBBBBBBBBBCCCCCCCCCCDDDDDDDDDDEEEEEEEEEEFFFFFFFFFFGGGGGGGGGGHHHHHHHHHHIIIIIIIIIIJJJJJJJJJJKKKKKKKKKKLLLLLLLLLLMMMMMMMMMMNNNNNNNNNNOOOOOOOOOOPPPPPPPPPPQQQQQQQQQQRRRRRRRRRRSSSSSSSSSSTTTTTTTTTT\n",
    );
    let out = norm.normalize(raw, "cli", Channel::StateDiff, &runtime);

    assert!(
        out.contains("- **Start Date**: <TS>"),
        "タイムスタンプが残っている:\n{out}"
    );
    assert!(
        out.contains("- **Project Root**: <ROOT>"),
        "絶対パスが残っている:\n{out}"
    );
    assert!(
        out.contains("shard <CLONE>"),
        "シャード名が残っている:\n{out}"
    );
    assert!(
        out.contains("intents/<TS>-golden"),
        "記録ディレクトリの日付が残っている:\n{out}"
    );
    assert!(
        out.contains("bare <TS>-golden"),
        "裸の記録ディレクトリ名が残っている:\n{out}"
    );
    assert!(
        out.contains("session <SESSION>"),
        "セッション ID が残っている:\n{out}"
    );
    assert!(
        out.contains("token <SESSION>"),
        "継続トークンが残っている:\n{out}"
    );
}

#[test]
fn normalization_is_a_fixpoint_over_the_captured_corpus() {
    let norm = Normalization::load();
    // 採取時に正規化済みなので、比較器を通しても 1 バイトも動かないのが正しい。
    // 動いたら「採取側と比較側で規則の解釈がずれている」ということ (NFR1.3)。
    let runtime = RuntimeValues::new(
        vec![corpus_root().to_string_lossy().to_string()],
        Vec::new(),
    );

    let mut failures: Vec<String> = Vec::new();
    for (family, channels) in [
        (
            "cli",
            &[
                ("stdout.json", Channel::Stdout),
                ("stdout.txt", Channel::Stdout),
                ("stderr", Channel::Stderr),
                ("state.diff", Channel::StateDiff),
                ("audit.md", Channel::Audit),
            ][..],
        ),
        (
            "hooks",
            &[
                ("stdout", Channel::Stdout),
                ("stderr", Channel::Stderr),
                ("audit.md", Channel::Audit),
            ][..],
        ),
    ] {
        for case in cases(family) {
            for (name, channel) in channels {
                let Some(text) = case.read(name) else {
                    continue;
                };
                let normalized = norm.normalize(&text, family, *channel, &runtime);
                if normalized != text {
                    let diff = line_diff(&text, &normalized);
                    failures.push(format!(
                        "  [{family}/{}] {name}\n{}",
                        case.id(),
                        diff.join("\n")
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "正規化が固定点になっていない ({} 件):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn missing_cases_are_recorded_with_a_reason() {
    for family in ["cli", "hooks"] {
        let doc = read_json(&format!("{family}/cases-missing.json"));
        let entries = doc["missing"]
            .as_array()
            .unwrap_or_else(|| panic!("{family}: cases-missing.json の missing が配列でない"));
        assert!(
            !entries.is_empty(),
            "{family}: 欠落ケースが 1 件も記録されていない"
        );
        for entry in entries {
            for field in ["id", "reason", "evidence", "follow_up"] {
                assert!(
                    entry[field].as_str().is_some_and(|v| !v.trim().is_empty()),
                    "{family}: 欠落ケース {} に {field} が無い (W4: 理由なき欠落は認めない)",
                    entry["id"]
                );
            }
        }
    }
}

#[test]
fn line_diff_points_at_the_lines_that_differ() {
    let diff = line_diff("a\nb\nc\n", "a\nB\nc\n");
    assert_eq!(
        diff.len(),
        2,
        "1 行の差なので - と + の 2 行になるはず: {diff:?}"
    );
    assert_eq!(diff[0], "2 - b");
    assert_eq!(diff[1], "2 + B");

    assert!(line_diff("same\n", "same\n").is_empty(), "同一なら差分は空");

    let longer = line_diff("a\n", "a\nb\n");
    assert_eq!(
        longer,
        vec!["2 + b".to_string()],
        "行数が増えた分も出す: {longer:?}"
    );
}
