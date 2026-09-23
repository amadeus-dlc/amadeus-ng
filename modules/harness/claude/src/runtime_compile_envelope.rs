//! PostToolUse (Bash) の封筒から、runtime-graph の compile を発火するかを決める。
//!
//! 固定本家 2.7.1 `a277af21` の `aidlc-lib.ts:1499-1537`
//! (`classifyRuntimeCompileCommand`) と `hooks/aidlc-rebuild-stage-graph.ts:110-150`
//! に対応する。2.8.2 は指揮役の綴りを二段形 `aidlc engine <noun> …` へ揃えたので、
//! 判定の前に `canonicalEngineCommand` (2.8.2 `aidlc-lib.ts:1414-1425`) と同じ正規化で
//! 一段の `aidlc <noun> …` へ畳む。判定は**語彙的**で、遷移を書く公開面だけを通す。再帰ガードは
//! `aidlc-runtime` 自身を先に落とすことで成立する — 許可だけの一覧では
//! `bun aidlc-runtime.ts compile && bun aidlc-state.ts approve` のような合成が通り、
//! compile が自分を呼び続けるからである。
use harness_infrastructure::split_shell_segments;

/// 本家の probe 順に並んだハーネス根 (`KNOWN_HARNESS_DIRS`)。
const HARNESS_DIRS: [&str; 5] = [".claude", ".kiro", ".codex", ".aidlc", ".cursor"];
/// 旧来のツールファイル面のうち、遷移を書くもの。
const TRANSITION_TOOLS: [&str; 5] = ["state", "jump", "bolt", "unit", "utility"];
/// 二段形 `aidlc engine <noun>` のうち、一段の `aidlc <noun>` へ畳む名詞
/// (2.8.2 `canonicalEngineCommand`)。
const ENGINE_NOUNS: [&str; 9] = [
    "orchestrate",
    "state",
    "jump",
    "bolt",
    "swarm",
    "scope",
    "config",
    "status",
    "recompose",
];

/// Bash の PostToolUse 封筒が運ぶコマンドと、compile を発火するかの判定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCompileEnvelope {
    command: String,
}

impl RuntimeCompileEnvelope {
    const fn new(command: String) -> Self {
        Self { command }
    }

    /// 不正 JSON・オブジェクトでない入力を無視する。`tool_input.command` の不在は空文字で読む
    /// — 本家も `?? ""` で読み、コマンドフィルタで落とすからである。
    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_str(input).ok()?;
        let object = value.as_object()?;
        let command = object
            .get("tool_input")
            .and_then(serde_json::Value::as_object)
            .and_then(|input| input.get("command"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        Some(Self::new(command.to_string()))
    }

    /// 封筒が運んだコマンド。
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    /// このコマンドで compile を発火するか。
    ///
    /// `aidlc-runtime` 自身 (compile の呼出し) は**先に**落とす。合成コマンドの中に
    /// 1 つでも現れたら発火しない。
    #[must_use]
    pub fn compiles(&self) -> bool {
        !self.invokes_runtime()
            && Self::new(canonical_engine_command(&self.command)).invokes_transition_surface()
    }

    fn invokes_runtime(&self) -> bool {
        if self.tool_file_invocation("runtime") {
            return true;
        }
        let segments = split_shell_segments(&self.command);
        segments.fold_left(false, |found, segment| {
            found
                || starts_with_word_sequence(segment, "aidlc", &["runtime"])
                || starts_with_word_sequence(segment, "aidlc", &["engine", "runtime", "compile"])
        })
    }

    fn invokes_transition_surface(&self) -> bool {
        TRANSITION_TOOLS
            .iter()
            .any(|tool| self.tool_file_invocation(tool))
            || self.orchestrate_report()
            || self.dispatcher_transition()
    }

    /// `bun … <harness>/tools/aidlc-<tool>.ts` — 行をまたがない (本家の `.` は改行に当たらない)。
    fn tool_file_invocation(&self, tool: &str) -> bool {
        let needles: Vec<String> = HARNESS_DIRS
            .iter()
            .map(|dir| format!("{dir}/tools/aidlc-{tool}.ts"))
            .collect();
        self.command.lines().any(|line| {
            word_index(line, "bun").is_some_and(|start| {
                needles
                    .iter()
                    .any(|needle| literal_after(line, needle, start).is_some())
            })
        })
    }

    /// `bun … <harness>/tools/aidlc-orchestrate.ts … report`。
    fn orchestrate_report(&self) -> bool {
        let needles: Vec<String> = HARNESS_DIRS
            .iter()
            .map(|dir| format!("{dir}/tools/aidlc-orchestrate.ts"))
            .collect();
        self.command.lines().any(|line| {
            word_index(line, "bun").is_some_and(|start| {
                needles.iter().any(|needle| {
                    literal_after(line, needle, start)
                        .is_some_and(|end| word_index(&line[end..], "report").is_some())
                })
            })
        })
    }

    /// 新しい `aidlc <名詞>` 文法のうち、遷移を書く面。
    ///
    /// `workspace` / `gen` / `sensor` / `intent` / `space` の名詞は**意図的に外す** —
    /// 本家が D2 の同等性を公開ワンショットにだけ持たせているからである。
    fn dispatcher_transition(&self) -> bool {
        const PAIRS: [[&str; 2]; 11] = [
            ["aidlc", "state"],
            // 2.8.2 で加わった面 (`aidlc-lib.ts:1676`)。
            ["aidlc", "recompose"],
            ["aidlc", "jump"],
            ["aidlc", "bolt"],
            ["aidlc", "unit"],
            ["aidlc", "status"],
            ["aidlc", "doctor"],
            ["aidlc", "version"],
            ["aidlc", "help"],
            ["aidlc", "report"],
            ["aidlc", "next"],
        ];
        const TRIPLES: [[&str; 3]; 3] = [
            ["aidlc", "scope", "change"],
            ["aidlc", "config", "set"],
            ["aidlc", "orchestrate", "report"],
        ];
        for pair in PAIRS {
            if pair[1] == "next" {
                // `aidlc next … report` だけが発火する (`next` 単体は遷移を書かない)。
                if self.command.lines().any(|line| {
                    word_sequence_end(line, &pair)
                        .is_some_and(|end| word_index(&line[end..], "report").is_some())
                }) {
                    return true;
                }
                continue;
            }
            if word_sequence_end(&self.command, &pair).is_some() {
                return true;
            }
        }
        TRIPLES
            .iter()
            .any(|triple| word_sequence_end(&self.command, triple).is_some())
    }
}

/// `aidlc engine orchestrate help` を `aidlc help` へ、`aidlc engine <noun>` を
/// `aidlc <noun>` へ畳む (2.8.2 `canonicalEngineCommand` と同じ置換、左から重ならずに)。
fn canonical_engine_command(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    while let Some(offset) = word_index(text.get(cursor..).unwrap_or_default(), "aidlc") {
        let start = cursor + offset;
        let after = start + "aidlc".len();
        let folded = sequence_from(text, after, &["engine", "orchestrate", "help"])
            .map(|end| (end, "aidlc help".to_string()))
            .or_else(|| {
                ENGINE_NOUNS.iter().find_map(|noun| {
                    sequence_from(text, after, &["engine", noun])
                        .map(|end| (end, format!("aidlc {noun}")))
                })
            });
        out.push_str(text.get(cursor..start).unwrap_or_default());
        match folded {
            Some((end, replacement)) => {
                out.push_str(&replacement);
                cursor = end;
            }
            None => {
                out.push_str("aidlc");
                cursor = after;
            }
        }
    }
    out.push_str(text.get(cursor..).unwrap_or_default());
    out
}

/// ASCII の語構成文字 (本家 `\b` と同じ集合)。
const fn word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// その位置が語の内側か (範囲外は語の外 = `\b` が立つ側)。
fn word_byte_at(text: &str, index: usize) -> bool {
    text.as_bytes().get(index).copied().is_some_and(word_byte)
}

/// `\b<word>\b` の最初の出現位置。
fn word_index(text: &str, word: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(offset) = text.get(from..)?.find(word) {
        let start = from + offset;
        let end = start + word.len();
        let before = start == 0 || !word_byte_at(text, start.saturating_sub(1));
        if before && !word_byte_at(text, end) {
            return Some(start);
        }
        from = start + 1;
    }
    None
}

/// `from` より後に現れる `needle` の直後位置 (`\b` 終端付き)。
fn literal_after(text: &str, needle: &str, from: usize) -> Option<usize> {
    let mut cursor = from;
    while let Some(offset) = text.get(cursor..)?.find(needle) {
        let start = cursor + offset;
        let end = start + needle.len();
        if !word_byte_at(text, end) {
            return Some(end);
        }
        cursor = start + 1;
    }
    None
}

/// `\bw1\s+w2\s+…\b` の直後位置。
fn word_sequence_end(text: &str, words: &[&str]) -> Option<usize> {
    let first = words.first()?;
    let rest = words.get(1..)?;
    let mut from = 0;
    while let Some(offset) = word_index(text.get(from..)?, first) {
        let start = from + offset;
        if let Some(end) = sequence_from(text, start + first.len(), rest) {
            return Some(end);
        }
        from = start + 1;
    }
    None
}

fn sequence_from(text: &str, mut cursor: usize, words: &[&str]) -> Option<usize> {
    for word in words {
        let mut spaced = false;
        while text
            .as_bytes()
            .get(cursor)
            .is_some_and(u8::is_ascii_whitespace)
        {
            cursor += 1;
            spaced = true;
        }
        if !spaced || !text.get(cursor..)?.starts_with(word) {
            return None;
        }
        cursor += word.len();
        if word_byte_at(text, cursor) {
            return None;
        }
    }
    Some(cursor)
}

/// 区間の先頭が `first rest…` か (先頭の空白は読み飛ばす)。
///
/// 先頭語を別引数で受けるので、語列が空という形は型で作れない。
fn starts_with_word_sequence(segment: &str, first: &str, rest: &[&str]) -> bool {
    let trimmed = segment.trim_start();
    if !trimmed.starts_with(first) {
        return false;
    }
    if word_byte_at(trimmed, first.len()) {
        return false;
    }
    sequence_from(trimmed, first.len(), rest).is_some()
}

#[cfg(test)]
mod tests {
    use super::RuntimeCompileEnvelope;

    fn envelope(command: &str) -> RuntimeCompileEnvelope {
        let quoted = core_infrastructure::canon_json::serialize(
            &core_infrastructure::canon_json::JsonValue::String(command.to_string()),
            core_infrastructure::canon_json::SerializationProfile::ContractCompact,
        );
        RuntimeCompileEnvelope::parse(&format!(
            r#"{{"tool_name":"Bash","tool_input":{{"command":{quoted}}}}}"#
        ))
        .expect("封筒")
    }

    #[test]
    fn a_malformed_envelope_is_refused() {
        assert!(RuntimeCompileEnvelope::parse("not json").is_none());
        assert!(RuntimeCompileEnvelope::parse("[]").is_none());
        assert!(RuntimeCompileEnvelope::parse("\"text\"").is_none());
    }

    #[test]
    fn an_envelope_without_a_command_carries_the_empty_command() {
        let parsed = RuntimeCompileEnvelope::parse(r#"{"tool_name":"Bash"}"#).expect("封筒");
        assert_eq!(parsed.command(), "");
        assert!(!parsed.compiles());
    }

    #[test]
    fn the_transition_tool_files_fire_the_compile() {
        for tool in ["state", "jump", "bolt", "unit", "utility"] {
            assert!(
                envelope(&format!("bun .claude/tools/aidlc-{tool}.ts approve")).compiles(),
                "{tool}"
            );
        }
        assert!(
            envelope("bun run .kiro/tools/aidlc-state.ts approve").compiles(),
            "既知のハーネス根はどれも同じ面である"
        );
    }

    #[test]
    fn the_orchestrate_report_surface_fires_the_compile() {
        assert!(
            envelope("bun .claude/tools/aidlc-orchestrate.ts report --result completed").compiles()
        );
        assert!(
            !envelope("bun .claude/tools/aidlc-orchestrate.ts next").compiles(),
            "next は遷移を書かない"
        );
    }

    #[test]
    fn the_new_grammar_fires_only_on_the_public_transition_surface() {
        for command in [
            "aidlc state approve",
            "aidlc jump execute --stage domain-design",
            "aidlc bolt start",
            "aidlc unit complete",
            "aidlc status",
            "aidlc doctor",
            "aidlc version",
            "aidlc help",
            "aidlc scope change --scope feature",
            "aidlc config set autonomy on",
            "aidlc report --result completed",
            "aidlc orchestrate report --result completed",
            "aidlc next --resume report",
        ] {
            assert!(envelope(command).compiles(), "{command}");
        }
        for command in [
            "aidlc workspace scan",
            "aidlc gen stage",
            "aidlc sensor run",
            "aidlc intent create --label demo",
            "aidlc space create demo",
            "aidlc next",
            "ls -la aidlc/spaces",
        ] {
            assert!(!envelope(command).compiles(), "{command}");
        }
    }

    /// 2.8.2 の指揮役は二段形で綴る。一段の `aidlc <noun>` へ畳んでから同じ判定を当てる。
    #[test]
    fn the_two_part_engine_grammar_folds_to_the_one_part_surface() {
        for command in [
            "aidlc engine orchestrate report --stage requirements-analysis --result approved",
            "aidlc engine state set-status --status running",
            "aidlc engine jump execute --stage code-generation",
            "aidlc engine bolt start",
            "aidlc engine scope change --scope feature",
            "aidlc engine config set change-control strict",
            "aidlc engine orchestrate help",
            "aidlc engine recompose --skip market-research",
        ] {
            assert!(envelope(command).compiles(), "{command}");
        }
        for command in [
            "aidlc engine orchestrate next",
            "aidlc engine log review --stage x --reviewer r --iteration 1",
            "aidlc engine learnings surface --slug x",
            "aidlc engine intent create --scope bugfix",
            "aidlc engineering state",
        ] {
            assert!(!envelope(command).compiles(), "{command}");
        }
    }

    #[test]
    fn the_two_part_runtime_compile_never_retriggers_itself() {
        assert!(!envelope("aidlc engine runtime compile").compiles());
        assert!(
            !envelope(
                "aidlc engine runtime compile && aidlc engine orchestrate report --result approved"
            )
            .compiles()
        );
    }

    #[test]
    fn word_boundaries_are_honoured_and_the_search_moves_past_a_partial_match() {
        // `bunx` の中の `bun` は語ではない。後ろの本物の `bun` まで探索を進める。
        assert!(!envelope("bunx .claude/tools/aidlc-state.ts approve").compiles());
        assert!(envelope("bunx tsc && bun .claude/tools/aidlc-state.ts approve").compiles());
        // `.ts` の直後に語構成文字が続く綴りは面ではない。後ろの本物まで探索を進める。
        assert!(!envelope("bun .claude/tools/aidlc-state.tsx approve").compiles());
        assert!(
            envelope("bun .claude/tools/aidlc-state.tsx .claude/tools/aidlc-state.ts approve")
                .compiles()
        );
        // `aidlc statement` は `aidlc state` ではない。後ろの本物まで探索を進める。
        assert!(!envelope("aidlc statement").compiles());
        assert!(envelope("aidlc statement; aidlc state approve").compiles());
        assert!(!envelope("aidlc  stateful").compiles());
    }

    #[test]
    fn the_runtime_guard_requires_the_whole_word_runtime() {
        assert!(!envelope("aidlc runtime compile && aidlc state approve").compiles());
        assert!(!envelope("  aidlc runtime compile; aidlc state approve").compiles());
        assert!(envelope("aidlc runtimes && aidlc state approve").compiles());
        assert!(envelope("aidlcruntime && aidlc state approve").compiles());
        assert!(
            !envelope("bun .claude/tools/aidlc-runtime.ts compile && aidlc state approve")
                .compiles()
        );
    }

    /// 遷移面の照合は**語彙的**である — 引用の内側でも発火する。本家は発火側だけを
    /// 素の正規表現で見て、区間解析は再帰ガードにしか使わない
    /// (`aidlc-lib.ts:1499-1522` の注記)。余分な compile は冪等なので害が無く、
    /// 取り逃しは runtime-graph を古いままにするからである。
    #[test]
    fn the_transition_match_stays_lexical_even_inside_quotes() {
        assert!(envelope("git commit -m \"aidlc state approve\"").compiles());
    }

    #[test]
    fn the_compile_never_retriggers_itself() {
        for command in [
            "bun .claude/tools/aidlc-runtime.ts compile",
            "bun .claude/tools/aidlc-runtime.ts compile && bun .claude/tools/aidlc-state.ts approve",
            "aidlc runtime compile",
            "cd /tmp && aidlc runtime compile",
        ] {
            assert!(!envelope(command).compiles(), "{command}");
        }
    }

    #[test]
    fn a_quoted_runtime_mention_is_not_an_invocation() {
        assert!(
            envelope("bun .claude/tools/aidlc-state.ts approve --note 'aidlc runtime compile'")
                .compiles(),
            "引用の内側は実行位置ではない"
        );
    }

    #[test]
    fn the_bun_word_and_the_script_must_share_one_line() {
        assert!(
            !envelope("bun --version\necho .claude/tools/aidlc-state.ts").compiles(),
            "本家の `.` は改行に当たらない"
        );
    }
}
