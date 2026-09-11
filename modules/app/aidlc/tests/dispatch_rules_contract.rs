//! 規則受渡しフック (`aidlc hook deliver-stage-rules`) を公開プロセス面で検証する。
//!
//! 期待値の出所は、固定本家 2.7.1 `a277af21` の `hooks/aidlc-deliver-stage-rules.ts` を
//! **実際に走らせて採った出力**である（記録は
//! `aidlc/spaces/default/intents/260907-selfhost-stage1/construction/u2-workflow-authority/code-generation/stage-rules-logs/`）。
#![allow(clippy::unwrap_used, clippy::expect_used)]
use core_infrastructure::canon_json::{JsonValue, SerializationProfile, serialize};
use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

/// 本家 `DISPATCH_HOOK_OUTPUT_MAX_BYTES`。
const LIMIT: usize = 512 * 1024;

/// 合成 fixture の規則本文。`stage-rules-logs/fixture-memory/` と同じバイトである。
const ORG: &str = "# Org\n\nOrg rule body.\n";
const TEAM: &str = "# Team\n\n\
> This team's affirmed practices and corrections. Loaded after `org.md` as\n\
> strict-additive guidance; contradictions with broader policy are rejected.\n\
> Populated by the practices-discovery affirmation gate. Edit at the gate,\n\
> not directly.\n\n\
<!-- only comments and the shipped preamble: not substantive -->\n";
const PROJECT: &str =
    "# Project\n\nNon-ASCII と \"引用符\" と バックスラッシュ \\ と タブ\tと 制御文字の並び。\n";
const CONSTRUCTION: &str = "# Construction\n\nPhase rule body.\n";

/// 本家を実走行して採った、この fixture に対する束のダイジェスト。
const FIXTURE_DIGEST: &str = "3072de5dc9edccbd72b5e2110e0d07483b8c3376fe37d845710f6aa4422e5e64";

/// 本家を実走行して採った stdout の 1 行そのもの（`upstream-fixture-block.txt` の逐語。
/// 末尾改行を含めた sha256 は `13db371a0e2308c40722bdf003bbd1183732266bdc9a77a19b007874a72c5fa5`）。
/// 期待値を実装と同じ直列化で組み立てると同語反復になるので、採取バイトをそのまま置く。
const UPSTREAM_STDOUT_LINE: &str = r#"{"hookSpecificOutput":{"hookEventName":"PreToolUse","updatedInput":{"subagent_type":"aidlc-developer-agent","prompt":"BASE /stages/construction/code-generation.md\n\n<!-- AIDLC_DISPATCH_RULES_BEGIN sha256:3072de5dc9edccbd72b5e2110e0d07483b8c3376fe37d845710f6aa4422e5e64 stage:code-generation -->\n## Active AI-DLC Rule Bundle\nThese are the required rules for this stage. Apply the content verbatim; later prose summaries do not replace it.\n\n### aidlc/spaces/default/memory/org.md\n# Org\n\nOrg rule body.\n\n### aidlc/spaces/default/memory/project.md\n# Project\n\nNon-ASCII と \"引用符\" と バックスラッシュ \\ と タブ\tと 制御文字の並び。\n\n### aidlc/spaces/default/memory/phases/construction.md\n# Construction\n\nPhase rule body.\n\n<!-- AIDLC_DISPATCH_RULES_END sha256:3072de5dc9edccbd72b5e2110e0d07483b8c3376fe37d845710f6aa4422e5e64 -->"}}}"#;

/// 本家が付けたブロックそのもの（`upstream-fixture-block.txt` の逐語）。
fn upstream_block() -> String {
    format!(
        "\n\n<!-- AIDLC_DISPATCH_RULES_BEGIN sha256:{FIXTURE_DIGEST} stage:code-generation -->\n\
         ## Active AI-DLC Rule Bundle\n\
         These are the required rules for this stage. Apply the content verbatim; later prose summaries do not replace it.\n\
         \n### aidlc/spaces/default/memory/org.md\n{ORG}\
         \n### aidlc/spaces/default/memory/project.md\n{PROJECT}\
         \n### aidlc/spaces/default/memory/phases/construction.md\n{CONSTRUCTION}\
         \n<!-- AIDLC_DISPATCH_RULES_END sha256:{FIXTURE_DIGEST} -->"
    )
}

struct Workspace {
    temp: tempfile::TempDir,
}

impl Workspace {
    /// 規則層・ペルソナ・配布データだけを持つ最小ワークスペース（記録は作らない）。
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        let data = root.join(".claude/tools/data");
        fs::create_dir_all(&data).unwrap();
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        for name in ["stage-graph.json", "harness.json"] {
            fs::copy(
                repository
                    .join("tests/golden/upstream-a277af21/data")
                    .join(name),
                data.join(name),
            )
            .unwrap();
        }
        let agents = root.join(".claude/agents");
        fs::create_dir_all(&agents).unwrap();
        for name in ["aidlc-developer-agent", "aidlc-quality-agent"] {
            fs::write(agents.join(format!("{name}.md")), "persona").unwrap();
        }
        let memory = root.join("aidlc/spaces/default/memory");
        fs::create_dir_all(memory.join("phases")).unwrap();
        fs::write(memory.join("org.md"), ORG).unwrap();
        fs::write(memory.join("team.md"), TEAM).unwrap();
        fs::write(memory.join("project.md"), PROJECT).unwrap();
        fs::write(memory.join("phases/construction.md"), CONSTRUCTION).unwrap();
        // 他フェーズの規則も置く。無いと ideation / inception のステージを名指した brief が
        // 「規則が読めない」で止まり、stage 解決の検証にならない。
        for phase in ["ideation", "inception", "operation"] {
            fs::write(
                memory.join(format!("phases/{phase}.md")),
                format!("# {phase}\n\nPhase rule body.\n"),
            )
            .unwrap();
        }
        fs::create_dir_all(root.join("aidlc/spaces/default/intents")).unwrap();
        Self { temp }
    }

    fn root(&self) -> PathBuf {
        self.temp.path().join("workspace")
    }

    fn memory(&self) -> PathBuf {
        self.root().join("aidlc/spaces/default/memory")
    }

    /// `Current Stage` を名乗る記録を据える（フックは読むだけ）。
    fn with_current_stage(self, stage: &str) -> Self {
        self.with_status_line(&format!("- **Current Stage**: {stage}"), "rec-0001")
    }

    /// `## Current Status` 節に任意の 1 行を置いた記録 `rec-0001` を据え、カーソルを
    /// `cursor` の綴りで書く（状態欄の読取りとカーソル解決の境界を駆動するため）。
    fn with_status_line(self, line: &str, cursor: &str) -> Self {
        let record = self.root().join("aidlc/spaces/default/intents/rec-0001");
        fs::create_dir_all(&record).unwrap();
        fs::write(
            record.join("aidlc-state.md"),
            format!("# AI-DLC State\n\n## Current Status\n{line}\n"),
        )
        .unwrap();
        fs::write(
            self.root()
                .join("aidlc/spaces/default/intents/active-intent"),
            format!("{cursor}\n"),
        )
        .unwrap();
        self
    }

    fn hook(&self, stdin: &str, env: &[(&str, &str)]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_aidlc"));
        command
            .args(["hook", "deliver-stage-rules"])
            .current_dir(self.root())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.temp.path().join("home"))
            .env("PATH", "/usr/bin:/bin");
        for (key, value) in env {
            command.env(key, value);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }
}

fn dispatch(prompt: &str) -> String {
    let escaped = serialize(
        &JsonValue::String(prompt.to_string()),
        SerializationProfile::ContractCompact,
    );
    format!(
        r#"{{"tool_name":"Task","tool_input":{{"subagent_type":"aidlc-developer-agent","prompt":{escaped}}}}}"#
    )
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

fn stage_of(output: &Output) -> Option<String> {
    stdout(output)
        .split("AIDLC_DISPATCH_RULES_BEGIN sha256:")
        .nth(1)
        .and_then(|rest| rest.split(" stage:").nth(1))
        .and_then(|rest| rest.split(' ').next())
        .map(str::to_string)
}

/// 巨大な規則を置いて、出力を狙った大きさへ寄せる。
fn grow_org_to(workspace: &Workspace, output_bytes: usize) -> usize {
    let write = |body: usize| {
        fs::write(
            workspace.memory().join("org.md"),
            format!("# Org\n{}\n", "x".repeat(body)),
        )
        .unwrap();
        workspace
            .hook(&dispatch("B /stages/construction/code-generation.md"), &[])
            .stdout
            .len()
    };
    let low = write(1_000);
    let high = write(2_000);
    let slope = (high - low) / 1_000;
    let mut body = 1_000 + (output_bytes - low) / slope;
    // 傾きは 1 バイト/文字だが、丸めの分だけ 1 歩ずつ寄せる。
    loop {
        let measured = write(body);
        if measured == output_bytes {
            return body;
        }
        assert!(
            measured.abs_diff(output_bytes) < 64,
            "狙いに寄せられない: {measured} vs {output_bytes}"
        );
        if measured < output_bytes {
            body += 1;
        } else {
            body -= 1;
        }
    }
}

#[test]
fn a_dispatch_to_an_aidlc_agent_receives_the_bundle_upstream_produced() {
    let workspace = Workspace::new();
    let output = workspace.hook(
        &dispatch("BASE /stages/construction/code-generation.md"),
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        format!("{UPSTREAM_STDOUT_LINE}\n"),
        "本家の採取バイトと一致しない"
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn the_team_template_is_dropped_because_it_carries_no_rule() {
    let workspace = Workspace::new();
    let body = stdout(&workspace.hook(&dispatch("B /stages/construction/code-generation.md"), &[]));
    assert!(body.contains("memory/org.md"), "{body}");
    assert!(body.contains("memory/project.md"), "{body}");
    assert!(body.contains("memory/phases/construction.md"), "{body}");
    assert!(
        !body.contains("memory/team.md"),
        "中身の無い規則は束に載らない"
    );
}

#[test]
fn an_explicit_stage_path_outranks_the_current_stage() {
    let workspace = Workspace::new().with_current_stage("code-generation");
    let output = workspace.hook(&dispatch("see x/stages/ideation/intent-capture.md"), &[]);
    assert_eq!(stage_of(&output).as_deref(), Some("intent-capture"));
}

#[test]
fn the_current_stage_outranks_a_slug_merely_mentioned_in_the_brief() {
    let workspace = Workspace::new().with_current_stage("code-generation");
    let output = workspace.hook(&dispatch("after scope-definition completes, continue"), &[]);
    assert_eq!(stage_of(&output).as_deref(), Some("code-generation"));
}

#[test]
fn the_current_stage_is_read_with_a_tab_or_no_separator_after_the_colon() {
    // 本家 `getField` は `^- \*\*Field\*\*:[ \t]*(.*)$` で、コロンの後の区切りは空白か
    // タブが 0 個以上である。固定 2.7.1 の実走行 (stage-rules closeout case 76 `tabstage` /
    // 77 `nospace`) はどちらも `Current Stage` を読めて code-generation の束を返した。
    for line in [
        "- **Current Stage**:\tcode-generation",
        "- **Current Stage**:code-generation",
        "- **Current Stage**:  \t code-generation",
    ] {
        let workspace = Workspace::new().with_status_line(line, "rec-0001");
        let output = workspace.hook(&dispatch("work on intent-capture now"), &[]);
        assert_eq!(output.status.code(), Some(0), "{line:?}: {output:?}");
        assert_eq!(
            stage_of(&output).as_deref(),
            Some("code-generation"),
            "{line:?}: 現在のステージは brief に混ざった slug より優先される"
        );
    }
}

#[test]
fn a_cursor_naming_an_absent_record_falls_back_to_the_lone_record() {
    // 本家 `activeIntent` はカーソルが実在する記録を名指さないとき無視し、記録がちょうど
    // 1 つならそれを選ぶ (lone intent)。固定 2.7.1 の実走行 (stage-rules closeout case 79
    // `dangling`) は `gone` を指すカーソルの下で `rec-0001` の Current Stage を読んだ。
    let workspace =
        Workspace::new().with_status_line("- **Current Stage**: code-generation", "gone");
    let output = workspace.hook(&dispatch("work on intent-capture now"), &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(stage_of(&output).as_deref(), Some("code-generation"));
}

#[test]
fn a_cursor_naming_a_directory_without_a_state_file_falls_back_to_the_lone_record() {
    // 本家 `activeIntent` の実在判定は `existsSync(join(dir, raw, "aidlc-state.md"))` —
    // ディレクトリだけ在って状態ファイルが無い記録も case 79 と同じく無視される
    // (裁定 F-H1 = B)。
    let workspace =
        Workspace::new().with_status_line("- **Current Stage**: code-generation", "half-made");
    fs::create_dir_all(
        workspace
            .root()
            .join("aidlc/spaces/default/intents/half-made"),
    )
    .unwrap();
    let output = workspace.hook(&dispatch("work on intent-capture now"), &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(stage_of(&output).as_deref(), Some("code-generation"));
}

#[test]
fn a_path_the_graph_does_not_know_falls_back_to_the_current_stage() {
    let workspace = Workspace::new().with_current_stage("code-generation");
    for prompt in [
        "see x/stages/construction/not-a-real-stage.md",
        "see x/stages/Construction/Code-Generation.md",
        "see x/stages/unknown-phase/intent-capture.md",
        "see x/stages/ideation/intent-capture.mdx",
        "see x/stages/ideation/intent-capture.draft.md",
        "see stages/ideation/intent-capture.md",
    ] {
        let output = workspace.hook(&dispatch(prompt), &[]);
        assert_eq!(
            stage_of(&output).as_deref(),
            Some("code-generation"),
            "{prompt}"
        );
    }
}

#[test]
fn every_separator_shape_the_upstream_matcher_reads_resolves_the_same_stage() {
    let workspace = Workspace::new().with_current_stage("code-generation");
    for prompt in [
        "see /stages/ideation/intent-capture.md",
        "stages/ideation/intent-capture.md is the file",
        "see x\\stages\\ideation\\intent-capture.md",
        "see x/stages\\ideation/intent-capture.md",
        "see x/STAGES/IDEATION/intent-capture.md",
        "first x/stages/ideation/intent-capture.md then x/stages/inception/domain-design.md",
    ] {
        let output = workspace.hook(&dispatch(prompt), &[]);
        assert_eq!(
            stage_of(&output).as_deref(),
            Some("intent-capture"),
            "{prompt}"
        );
    }
}

#[test]
fn without_a_current_stage_a_single_slug_mention_binds_and_two_bind_nothing() {
    let workspace = Workspace::new();
    let bound = workspace.hook(&dispatch("work on intent-capture now"), &[]);
    assert_eq!(stage_of(&bound).as_deref(), Some("intent-capture"));
    assert_eq!(
        stage_of(&workspace.hook(&dispatch("work on _intent-capture_ now"), &[])).as_deref(),
        Some("intent-capture"),
        "アンダースコアは境界である"
    );
    assert_eq!(
        stage_of(&workspace.hook(&dispatch("work on Intent-Capture now"), &[])).as_deref(),
        Some("intent-capture"),
        "綴りの大小は問わない"
    );
    for prompt in [
        "work on intent-capture then domain-design",
        "just do the work",
        "work on xintent-capturex now",
        "work on pre-intent-capture now",
    ] {
        let output = workspace.hook(&dispatch(prompt), &[]);
        assert_eq!(output.status.code(), Some(0), "{prompt}");
        assert_eq!(stdout(&output), "", "{prompt}");
    }
}

#[test]
fn a_brief_that_already_carries_the_exact_bundle_is_left_alone() {
    let workspace = Workspace::new();
    let base = "BASE /stages/construction/code-generation.md";
    let first = workspace.hook(&dispatch(base), &[]);
    let block = upstream_block();
    for prompt in [
        format!("{base}{block}"),
        format!("{block}\ntail /stages/construction/code-generation.md"),
        format!("{base}{block}{block}"),
    ] {
        let output = workspace.hook(&dispatch(&prompt), &[]);
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(stdout(&output), "", "同じ束は 2 度配らない");
    }
    // 1 バイト違えば抑止されない。
    let near = format!("{base}{}", &block[..block.len() - 1]);
    let output = workspace.hook(&dispatch(&near), &[]);
    assert_eq!(
        stdout(&output)
            .matches("AIDLC_DISPATCH_RULES_BEGIN")
            .count(),
        2
    );
    assert!(!stdout(&first).is_empty());
}

#[test]
fn a_dispatch_that_is_not_a_delegated_aidlc_agent_passes_untouched() {
    let workspace = Workspace::new().with_current_stage("code-generation");
    for stdin in [
        r#"{"tool_name":"Write","tool_input":{"subagent_type":"aidlc-developer-agent","prompt":"work"}}"#,
        r#"{"tool_name":"Task","tool_input":{"subagent_type":"general-purpose","prompt":"work"}}"#,
        r#"{"tool_name":"Task","tool_input":{"subagent_type":"aidlc-composer-agent","prompt":"work"}}"#,
        r#"{"tool_name":"Task","tool_input":{"subagent_type":"aidlc-missing-agent","prompt":"work"}}"#,
        r#"{"tool_name":"Task","tool_input":{"subagent_type":"aidlc-developer-agent","prompt":""}}"#,
        r#"{"tool_name":"Task"}"#,
        "{not json",
        "",
        "null",
        "[1,2,3]",
    ] {
        let output = workspace.hook(stdin, &[]);
        assert_eq!(output.status.code(), Some(0), "{stdin}");
        assert_eq!(stdout(&output), "", "{stdin}");
        assert_eq!(stderr(&output), "", "{stdin}");
    }
}

#[test]
fn a_subagent_dispatch_augments_each_stage_entry_with_its_own_bundle() {
    let workspace = Workspace::new().with_current_stage("code-generation");
    let stdin = r#"{"tool_name":"subagent","tool_input":{"stages":[null,{"role":"general-purpose","prompt_template":"skip"},{"role":"aidlc-developer-agent","prompt_template":"x/stages/inception/domain-design.md"},{"role":"aidlc-quality-agent","prompt_template":"x/stages/ideation/intent-capture.md"}]}}"#;
    let output = workspace.hook(stdin, &[]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let body = stdout(&output);
    assert_eq!(body.matches("AIDLC_DISPATCH_RULES_BEGIN").count(), 2);
    assert!(body.contains("stage:domain-design"), "{body}");
    assert!(body.contains("stage:intent-capture"), "{body}");
    assert!(
        body.contains(r#""role":"general-purpose","prompt_template":"skip""#),
        "{body}"
    );
}

#[test]
fn a_rule_file_that_cannot_be_read_stops_the_dispatch_with_the_upstream_wording() {
    let workspace = Workspace::new();
    fs::remove_file(workspace.memory().join("org.md")).unwrap();
    let output = workspace.hook(
        &dispatch("BASE /stages/construction/code-generation.md"),
        &[],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "", "部分的には書かない");
    let message = stderr(&output);
    assert!(
        message.starts_with(
            "Cannot load required stage rule \"aidlc/spaces/default/memory/org.md\" ("
        ),
        "{message}"
    );
    assert!(
        message.trim_end().ends_with(
            "). The stage has not started. Restore the file or fix its permissions/UTF-8 encoding, then run `next` again."
        ),
        "{message}"
    );
}

#[test]
fn a_rule_file_that_is_not_utf8_stops_the_dispatch_too() {
    let workspace = Workspace::new();
    fs::write(workspace.memory().join("org.md"), [0x80_u8, 0x81]).unwrap();
    let output = workspace.hook(
        &dispatch("BASE /stages/construction/code-generation.md"),
        &[],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("Cannot load required stage rule"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_leading_utf8_bom_is_dropped_the_way_the_upstream_decoder_drops_it() {
    // 本家は `new TextDecoder("utf-8", { fatal: true }).decode(bytes)` で規則を読む。WHATWG の
    // 既定（`ignoreBOM: false`）は先頭の BOM を出力へ含めないので、BOM 付きの org.md でも束は
    // BOM 無しの採取（`upstream_block()`）とバイト一致する — closeout-differential 67 の実測。
    let workspace = Workspace::new();
    let mut bytes = vec![0xEF_u8, 0xBB, 0xBF];
    bytes.extend_from_slice(ORG.as_bytes());
    fs::write(workspace.memory().join("org.md"), bytes).unwrap();
    let output = workspace.hook(
        &dispatch("BASE /stages/construction/code-generation.md"),
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        format!("{UPSTREAM_STDOUT_LINE}\n"),
        "先頭の BOM は本文ではない"
    );

    // 落ちるのは先頭の 1 つだけである — 2 つ目は本文として残る（closeout-differential 90）。
    let mut twice = vec![0xEF_u8, 0xBB, 0xBF, 0xEF, 0xBB, 0xBF];
    twice.extend_from_slice(ORG.as_bytes());
    fs::write(workspace.memory().join("org.md"), twice).unwrap();
    let output = workspace.hook(
        &dispatch("BASE /stages/construction/code-generation.md"),
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("memory/org.md\\n\u{feff}# Org"),
        "2 つ目の BOM は本文である: {}",
        stdout(&output)
    );
    assert!(
        !stdout(&output).contains(FIXTURE_DIGEST),
        "本文が違えばダイジェストも違う"
    );
}

#[test]
fn the_output_limit_admits_the_exact_boundary_and_refuses_one_byte_past_it() {
    let workspace = Workspace::new();
    let at_limit = grow_org_to(&workspace, LIMIT);

    let write = |body: usize| {
        fs::write(
            workspace.memory().join("org.md"),
            format!("# Org\n{}\n", "x".repeat(body)),
        )
        .unwrap();
    };

    write(at_limit - 1);
    let under = workspace.hook(&dispatch("B /stages/construction/code-generation.md"), &[]);
    assert_eq!(under.status.code(), Some(0));
    assert_eq!(under.stdout.len(), LIMIT - 1);

    write(at_limit);
    let exact = workspace.hook(&dispatch("B /stages/construction/code-generation.md"), &[]);
    assert_eq!(exact.status.code(), Some(0), "ちょうど上限は通る");
    assert_eq!(exact.stdout.len(), LIMIT);

    write(at_limit + 1);
    let over = workspace.hook(&dispatch("B /stages/construction/code-generation.md"), &[]);
    assert_eq!(over.status.code(), Some(2));
    assert_eq!(over.stdout.len(), 0, "部分的には書かない");
    assert_eq!(
        stderr(&over).trim_end(),
        format!(
            "[aidlc] This stage's rule files add up to {} bytes, exceeding the safe {LIMIT}-byte \
             output limit for attaching them to a subagent brief. The subagent was not started, \
             and nothing partial was written. Shorten or split the rule files for the active \
             stage, then start the subagent again.",
            LIMIT + 1
        )
    );

    let advisory = workspace.hook(
        &dispatch("B /stages/construction/code-generation.md"),
        &[("AIDLC_DISPATCH_RULES_PRELOAD_FALLBACK", "1")],
    );
    assert_eq!(advisory.status.code(), Some(3), "先読みハーネスは止めない");
    assert_eq!(advisory.stdout.len(), 0);
    assert_eq!(
        stderr(&advisory).trim_end(),
        format!(
            "[aidlc] Advisory: this stage's rule files add up to {} bytes, which exceeds the safe \
             {LIMIT}-byte limit for attaching them to a subagent brief. Nothing partial was \
             written. This harness loads the same rule files itself, through its own \
             active-memory preload fallback, so the work continues without them attached.",
            LIMIT + 1
        )
    );

    write(at_limit);
    let still_fine = workspace.hook(
        &dispatch("B /stages/construction/code-generation.md"),
        &[("AIDLC_DISPATCH_RULES_PRELOAD_FALLBACK", "1")],
    );
    assert_eq!(still_fine.status.code(), Some(0), "上限内なら助言は出ない");
    assert_eq!(stderr(&still_fine), "");
}

#[test]
fn a_dispatch_never_writes_anything_into_the_workspace() {
    let workspace = Workspace::new().with_current_stage("code-generation");
    let before = tree(&workspace.root());
    for stdin in [
        dispatch("BASE /stages/construction/code-generation.md"),
        dispatch("just do the work"),
        r#"{"tool_name":"Task","tool_input":{"subagent_type":"aidlc-developer-agent","prompt":"work","run_in_background":true}}"#.to_string(),
        r#"{"tool_name":"Write","tool_input":{}}"#.to_string(),
    ] {
        workspace.hook(&stdin, &[]);
    }
    assert_eq!(before, tree(&workspace.root()), "規則配送は読むだけである");
}

/// ディレクトリの中身を、相対パスと長さの列にする（存在・大きさの変化を見る）。
fn tree(root: &Path) -> Vec<(String, u64)> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(meta) = fs::metadata(&path) {
                found.push((
                    path.strip_prefix(root).unwrap().display().to_string(),
                    meta.len(),
                ));
            }
        }
    }
    found.sort();
    found
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;
