//! bugfix スコープ 1 周が踏む配布側の入口の列挙 (`scripts/aidlc-selfhost/required-surface.json`) と、
//! この build の配線状態の突合。
//!
//! # 2.8.2 基準
//!
//! 配布物が aidlc 2.8.2 へ上がり、起動形が二段形 `aidlc engine <noun> <verb>` になった。
//! 入口として数えるのは**この起動形だけ**である。2.8.2 の本文にはまだ `aidlc-<face>.ts` の
//! 綴りが散文のラベルとして残っているが、それは起動形ではないので 1 件も数えない。
//!
//! 列挙は配布資産の**実バイト**から導く。出典は
//! `tests/golden/selfhost-stage1/required-surface-sources/` に凍結した 2.8.2 の配布物であり、
//! `.claude/settings.json` と `.claude/tools/aidlc.ts` をファイル全体で含む。
//!
//! # 列挙の範囲 — 配布本文と実行時の指示
//!
//! 入口は 2 つの出所から来る。
//!
//! 1. **配布本文** — 配布資産（スキル・プロトコル・ステージ定義・`settings.json` ほか）が
//!    散文として綴る起動形。`sources[]` の実バイトから `aidlc engine …` を拾って数える。
//! 2. **実行時の指示** — bugfix 1 周の実行中にエンジンとフックが出す directive・拒否文言・
//!    案内文が綴るコマンド。2.8.2 ではこれらが `aidlcDispatcherInvocation` /
//!    `aidlcToolInvocation` を通るので、綴りは同じ二段形になる。出所は
//!    `runtime_directive_sources[]` が指し、各行が `upstream_site` でその位置を指す。
//!
//! 両方に現れる入口（`orchestrate next` など）は 1 行のまま、`sources_kind` に両方を持つ。
//!
//! # 出典の解決根 — 行の種別で決まる
//!
//! 出典は 2 通りの採り方をしていて、**どちらの群に属するかで解決根が決まる**。
//!
//! - 配布本文の出典（`sources[]` / `provenance.json` の `files[]`）は、配布物をこのリポジトリの
//!   `tests/golden/selfhost-stage1/required-surface-sources/` へ写した複製である。
//! - 実行時の指示の出典（`runtime_directive_sources[]` / `provenance.json` の
//!   `runtime_directive_files[]`）は複製を作らず、リポジトリ直下に commit 済みの同じ 2.8.2
//!   配布物（`.claude/**`）をその場で指す。`upstream_site` の行番号と文言はこの実バイトから
//!   測っており、測定元を直接指すほうが記録が正確である。配布物が更新されれば sha256 照合が
//!   落ち、実行時の指示を測り直す合図になる。
//!
//! 解けない行・指紋の合わない行を、もう一方の根へ読み替えたり読み飛ばしたりはしない。
//!
//! # `required-surface.json` に要る形
//!
//! ```text
//! {
//!   "format": 2,
//!   "scope": "bugfix",
//!   "upstream_version": "2.8.2",
//!   "sources": ["tests/golden/selfhost-stage1/required-surface-sources/.claude/..."],
//!   "runtime_directive_sources": [".claude/tools/aidlc-orchestrate.ts"],
//!   "entries": [
//!     {
//!       "noun": "orchestrate",
//!       "verb": "report",            // 動詞を取らない top ルートは null
//!       "sources_kind": ["distribution", "runtime-directive"],
//!       "cites": ["<sources[] のパス>:<行>"],           // distribution のときだけ要る
//!       "upstream_site": ["<runtime_directive_sources[] のパス>:<行> <文言>"],  // runtime-directive のときだけ要る
//!       "bugfix_required": true,
//!       "condition": "必須/非必須と判定した根拠",
//!       "mapping": {"face": "aidlc-orchestrate", "verb": "report"},  // 写像先が無ければ null
//!       "acceptance": "accepted",    // 写像先の一段形をこの build が受理するか
//!       "classification": "mapped"   // mapped | needs-implementation
//!     }
//!   ],
//!   "hooks": [{"name": "fold-usage", "status": "native-available"}]
//! }
//! ```
//!
//! `acceptance` と `classification` は自己申告ではない — 下のテストが
//! `aidlc::cli::parse` の実挙動と突き合わせる。
// 契約テストは固定の添字参照と panic を検証の合図として使う (既存の契約テストと同じ許容)。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use aidlc::cli::{Face, parse};

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

// ---------------------------------------------------------------------------
// 起動形の抽出器 — 入口の定義そのもの
// ---------------------------------------------------------------------------

/// 出典から拾った起動形 1 件。動詞を取らない top ルートは `verb` が `None`。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EngineEntry {
    noun: String,
    verb: Option<String>,
}

impl EngineEntry {
    fn label(&self) -> String {
        match &self.verb {
            Some(verb) => format!("{} {verb}", self.noun),
            None => self.noun.clone(),
        }
    }
}

/// noun・verb として成立する綴り (小文字・数字・ハイフン、先頭は小文字)。
fn is_identifier(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// noun の位置が診断文の接頭辞か (`aidlc engine statusline: …`)。
///
/// 起動形の noun の直後にコロンは来ない。閉じ引用符を挟んだ `` `…`: `` は文の区切りなので
/// 起動形のままである。
fn is_diagnostic_prefix(raw_noun: &str) -> bool {
    raw_noun.ends_with(':')
        && raw_noun
            .chars()
            .rev()
            .nth(1)
            .is_some_and(|c| c.is_ascii_alphanumeric())
}

/// 語を囲む引用符・句読点を落とす (設定の `"…"`、Markdown の `` `…` ``、文末の `.` など)。
fn unwrap_token(token: &str) -> &str {
    token.trim_matches(|c: char| {
        matches!(
            c,
            '"' | '\'' | '`' | ',' | '.' | ':' | ';' | ')' | '(' | '[' | ']' | '\\'
        )
    })
}

/// 出典の本文から `aidlc engine <noun> [<verb>]` の**起動形だけ**を拾う。
///
/// 数えるもの — 行の中に現れた起動形。動詞の後ろに続く下位トークン・フラグ・引数は
/// 入口の一部ではないので読み飛ばす。行末の `\` で折り返した 2 行目以降は、それ自身が
/// 起動形を綴らない限り新しい入口にならない。
///
/// 数えないもの — 同じ字面の非起動形の変種。使用法の雛形 (`<noun>` `<verb>`)、権限 glob
/// (`Bash(aidlc engine *)`)、名前空間ヘルプ (`aidlc engine --help`)、テンプレート展開を
/// 含むエラー文言 (`aidlc engine hook ${…}`)、行末で途切れたコード註釈、そして接頭辞を
/// 省いた継続形 (`... unit complete …`)。散文中の旧綴り `aidlc-<face>.ts <verb>` は、
/// そもそもこの字面に当たらない。
fn engine_invocations(raw: &str) -> Vec<EngineEntry> {
    const MARKER: &str = "aidlc engine";
    let mut found = Vec::new();
    for line in raw.lines() {
        let mut offset = 0usize;
        while let Some(hit) = line.get(offset..).and_then(|rest| rest.find(MARKER)) {
            let start = offset + hit;
            // `my-aidlc engine` のような別語の末尾を起動形と取り違えない。
            let boundary = line[..start]
                .chars()
                .next_back()
                .is_none_or(|c| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/')));
            offset = start + MARKER.len();
            if !boundary {
                continue;
            }
            let mut tokens = line[offset..].split_whitespace();
            let Some(raw_noun) = tokens.next() else {
                // 行末で途切れている — 起動形ではない。
                continue;
            };
            if is_diagnostic_prefix(raw_noun) {
                // `aidlc engine statusline: not available in this install` のような
                // 診断文の接頭辞。noun の直後にコロンが来る起動形は存在しない。
                continue;
            }
            let noun = unwrap_token(raw_noun);
            if !is_identifier(noun) {
                // 雛形の `<noun>`、権限 glob の `*`、名前空間ヘルプの `--help`。
                continue;
            }
            match tokens.next().map(unwrap_token) {
                None => found.push(EngineEntry {
                    noun: noun.to_string(),
                    verb: None,
                }),
                Some(next) if next.starts_with('-') => found.push(EngineEntry {
                    noun: noun.to_string(),
                    verb: None,
                }),
                Some(next) if is_identifier(next) => found.push(EngineEntry {
                    noun: noun.to_string(),
                    verb: Some(next.to_string()),
                }),
                // 動詞の位置がテンプレート展開・プレースホルダである行は起動形ではない。
                Some(_) => {}
            }
        }
    }
    found
}

// ---------------------------------------------------------------------------
// 抽出器の正例・負例
// ---------------------------------------------------------------------------

fn labels(raw: &str) -> BTreeSet<String> {
    engine_invocations(raw)
        .iter()
        .map(EngineEntry::label)
        .collect()
}

/// 起動形は入口として列挙される。行末 `\` の折り返しは新しい入口を作らない。
#[test]
fn a_launch_form_is_listed_as_an_entry() {
    let raw = "1. `aidlc engine orchestrate report --stage <slug> --result approved`\n\
               aidlc engine log decision --stage code-generation \\\n\
               \x20 --decision \"<summary>\" --options \"<csv>\"\n";
    assert_eq!(
        labels(raw),
        ["orchestrate report", "log decision"]
            .into_iter()
            .map(str::to_string)
            .collect::<BTreeSet<_>>(),
        "折り返しの 2 行目を新しい入口として数えている"
    );
}

/// 動詞の直後の下位トークンは 2 つ目の動詞にならない。
#[test]
fn a_token_after_the_verb_is_not_a_second_verb() {
    let raw = "route it through `aidlc engine orchestrate next compose \"<their words>\"`\n";
    assert_eq!(
        labels(raw),
        ["orchestrate next"]
            .into_iter()
            .map(str::to_string)
            .collect::<BTreeSet<_>>(),
        "下位トークンを動詞として登録している"
    );
}

/// 同じ字面の非起動形の変種は列挙されない。
#[test]
fn a_non_launch_variant_of_the_same_spelling_is_not_an_entry() {
    let raw = "  usage: \"aidlc engine <noun> <verb> [args]\",\n\
                     \"Bash(aidlc engine *)\"\n\
               \x20   text(2, `aidlc engine hook ${action.name}: not available in this install\\n`);\n\
               \x20   message: `aidlc: ${detail} for engine noun '${noun}'; try 'aidlc engine --help'\\n`,\n\
               // remains \"unknown command 'plugin'\" (that engine surface is `aidlc engine\n\
               \x20   text(2, \"aidlc engine statusline: not available in this install\\n\");\n\
               \x20   text(2, \"aidlc engine statusline: hook does not export run(input)\\n\");\n\
               after the Unit's artifacts are written: `... unit complete --stage <slug> --unit <name>`\n";
    assert!(
        engine_invocations(raw).is_empty(),
        "非起動形を入口として数えている: {:?}",
        engine_invocations(raw)
    );
}

/// 散文中の旧綴り `aidlc-<face>.ts <verb>` は 1 件も入口にならない。
#[test]
fn an_old_spelling_in_prose_is_not_an_entry() {
    let raw = "Approval choices go only through `aidlc-orchestrate.ts report`.\n\
               the decision log (`aidlc-audit.ts append`) and the state writer `aidlc-state.ts`\n";
    assert!(
        engine_invocations(raw).is_empty(),
        "散文中の旧綴りを入口として数えている: {:?}",
        engine_invocations(raw)
    );
}

/// 動詞を取らない top ルートで、後続のフラグを動詞にしない。
#[test]
fn a_flag_after_a_verbless_noun_is_not_a_verb() {
    let raw = "on approve run `aidlc engine recompose --skip <slugs> --add <slugs>` directly\n\
               \"command\": \"aidlc engine statusline\"\n\
               the status line runs `aidlc engine statusline`:\n";
    assert_eq!(
        engine_invocations(raw),
        vec![
            EngineEntry {
                noun: "recompose".to_string(),
                verb: None,
            },
            EngineEntry {
                noun: "statusline".to_string(),
                verb: None,
            },
            EngineEntry {
                noun: "statusline".to_string(),
                verb: None,
            },
        ],
        "フラグを動詞として登録している、top ルートを落としている、\
         または閉じ引用符の後ろの句読点を診断文と取り違えている"
    );
}

// ---------------------------------------------------------------------------
// 資料の読み出し
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn surface_path() -> PathBuf {
    repo_root().join("scripts/aidlc-selfhost/required-surface.json")
}

fn surface_text() -> String {
    fs::read_to_string(surface_path()).unwrap_or_else(|error| panic!("必要集合が読めない: {error}"))
}

fn surface() -> serde_json::Value {
    serde_json::from_str(&surface_text()).expect("必要集合は JSON")
}

/// 2.8.2 基準の列挙。旧形 (`verbs` 配列) はここで止める — 取り違えを黙って通さない。
fn entries(surface: &serde_json::Value) -> Vec<&serde_json::Value> {
    assert!(
        surface.get("verbs").is_none(),
        "required-surface.json が 2.7.1 基準の `verbs` 配列のままである — \
         2.8.2 の二段形を `entries` として採り直すこと"
    );
    surface["entries"]
        .as_array()
        .expect("entries が無い — 2.8.2 基準の列挙が作られていない")
        .iter()
        .collect()
}

fn text(entry: &serde_json::Value, key: &str) -> String {
    entry[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} が無い: {entry}"))
        .to_string()
}

fn optional_text(entry: &serde_json::Value, key: &str) -> Option<String> {
    match entry.get(key) {
        None => panic!("{key} が無い: {entry}"),
        Some(serde_json::Value::Null) => None,
        Some(value) => Some(
            value
                .as_str()
                .unwrap_or_else(|| panic!("{key} は文字列か null: {entry}"))
                .to_string(),
        ),
    }
}

fn entry_label(entry: &serde_json::Value) -> String {
    EngineEntry {
        noun: text(entry, "noun"),
        verb: optional_text(entry, "verb"),
    }
    .label()
}

fn sources(surface: &serde_json::Value) -> Vec<String> {
    surface["sources"]
        .as_array()
        .expect("sources")
        .iter()
        .map(|value| value.as_str().expect("sources は文字列の配列").to_string())
        .collect()
}

/// 実行時の指示の出所（2.8.2 の生成箇所を凍結したファイル）。
fn runtime_directive_sources(surface: &serde_json::Value) -> Vec<String> {
    surface
        .get("runtime_directive_sources")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| {
            panic!(
                "runtime_directive_sources が無い — 実行時の指示の出所が列挙の範囲に入っていない"
            )
        })
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("runtime_directive_sources は文字列の配列")
                .to_string()
        })
        .collect()
}

/// 出典の種別（`distribution` / `runtime-directive`）。
fn source_kinds(entry: &serde_json::Value) -> BTreeSet<String> {
    let label = entry_label(entry);
    let listed = entry
        .get("sources_kind")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("{label}: sources_kind が無い"));
    assert!(!listed.is_empty(), "{label}: sources_kind が空である");
    listed
        .iter()
        .map(|value| {
            let kind = value
                .as_str()
                .unwrap_or_else(|| panic!("{label}: sources_kind は文字列の配列"));
            assert!(
                ["distribution", "runtime-directive"].contains(&kind),
                "{label}: 出典の種別が未知の値である ({kind})"
            );
            kind.to_string()
        })
        .collect()
}

fn string_list(entry: &serde_json::Value, key: &str) -> Vec<String> {
    let label = entry_label(entry);
    entry
        .get(key)
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("{label}: {key} が無い"))
        .iter()
        .map(|value| {
            value
                .as_str()
                .unwrap_or_else(|| panic!("{label}: {key} は文字列の配列"))
                .to_string()
        })
        .collect()
}

fn frozen_root() -> PathBuf {
    repo_root().join("tests/golden/selfhost-stage1/required-surface-sources")
}

fn provenance() -> serde_json::Value {
    let path = frozen_root().join("provenance.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("出典の採取記録が読めない ({}): {error}", path.display()));
    serde_json::from_str(&raw).expect("採取記録は JSON")
}

/// 出典 `path:line` の行本文。
fn cited_line(cite: &str) -> String {
    let (path, number) = cite.rsplit_once(':').expect("出典は path:line");
    let line: usize = number.parse().expect("行番号");
    let raw = fs::read_to_string(repo_root().join(path))
        .unwrap_or_else(|error| panic!("出典が読めない ({path}): {error}"));
    raw.lines()
        .nth(line - 1)
        .unwrap_or_else(|| panic!("{cite}: その行が無い"))
        .to_string()
}

// ---------------------------------------------------------------------------
// 出典と実バイトの照合 (2.8.2)
// ---------------------------------------------------------------------------

/// 採取記録の 1 群を、その群だけの解決根で読み直して指紋を照合する。
///
/// 解決根はこの引数で 1 つに決まる。読めない行・指紋の合わない行をもう一方の根へ読み替える
/// 回復経路は持たない（持てば、実体の無い記録が黙って通る）。
fn recorded_bytes_match(provenance: &serde_json::Value, key: &str, root: &Path, required: &[&str]) {
    let files = provenance[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key}: 採取記録が配列として無い"));
    assert_eq!(
        provenance["source"][key].as_u64(),
        Some(files.len() as u64),
        "{key}: 採取記録のファイル数と実体の数が食い違っている"
    );
    let listed: BTreeSet<String> = files.iter().map(|file| text(file, "path")).collect();
    for entry in required {
        assert!(
            listed.contains(*entry),
            "{entry}: {key} にファイル全体で記録されていない"
        );
    }
    for file in files {
        let relative = text(file, "path");
        let path = root.join(&relative);
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("{key} の出典が読めない ({}): {error}", path.display()));
        assert_eq!(
            core_infrastructure::hash::sha256_hex(&bytes),
            text(file, "sha256"),
            "{relative}: 記録した指紋と実バイトが一致しない ({})",
            path.display()
        );
    }
}

/// 出典は 2.8.2 配布物の実バイトであり、指紋つきで採取元が記録されている。
///
/// 群ごとに解決根が違う（モジュール doc の「出典の解決根」）。配布本文の複製は
/// `frozen_root()`、実行時の指示の正本はリポジトリ直下の `.claude/**` をそのまま指す。
#[test]
fn the_frozen_sources_are_the_2_8_2_distribution_bytes() {
    let provenance = provenance();
    assert_eq!(
        provenance["source"]["version"].as_str(),
        Some("2.8.2"),
        "出典が 2.8.2 の配布物を指していない"
    );
    for key in [
        "captured_at",
        "captured_from",
        "runtime_directive_captured_at",
        "runtime_directive_captured_from",
    ] {
        assert!(
            provenance["source"][key]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
            "{key} が記録されていない — 採取元と採取日の無い出典は照合の根拠にならない"
        );
    }
    recorded_bytes_match(
        &provenance,
        "files",
        &frozen_root(),
        &[".claude/settings.json", ".claude/tools/aidlc.ts"],
    );
    // 実行時の指示の正本 — 綴り規則の定義、エンジン側の呼び出し箇所、Stop フックと
    // SessionStart フックの案内文。複製ではなくリポジトリ直下の実バイトを指す。
    recorded_bytes_match(
        &provenance,
        "runtime_directive_files",
        &repo_root(),
        &[
            ".claude/tools/aidlc-runtime-paths.ts",
            ".claude/tools/aidlc-orchestrate.ts",
            ".claude/hooks/aidlc-continue-workflow.ts",
            ".claude/hooks/aidlc-session-start.ts",
        ],
    );
}

/// `upstream_site` が指せるファイルは、すべて指紋つきで採取記録に載っている。
///
/// 位置の照合（`a_runtime_directive_entry_cites_its_place_in_the_2_8_2_sources`）は、指した行に
/// 文言が実在することしか見ない。出所が採取記録に無いままなら、配布物が更新されても
/// 指紋照合が落ちず、乖離が黙って通る。
#[test]
fn every_runtime_directive_source_is_fingerprinted_in_the_provenance() {
    let recorded: BTreeSet<String> = provenance()["runtime_directive_files"]
        .as_array()
        .expect("runtime_directive_files は配列")
        .iter()
        .map(|file| text(file, "path"))
        .collect();
    for path in runtime_directive_sources(&surface()) {
        assert!(
            recorded.contains(&path),
            "{path}: 実行時の指示の出所として挙げながら sha256 が記録されていない"
        );
    }
}

/// 2.7.1-j5ik2o.1 由来の旧出典は正本から外れている。
#[test]
fn no_source_points_at_the_2_7_1_fork() {
    let provenance_text =
        fs::read_to_string(frozen_root().join("provenance.json")).expect("出典の採取記録");
    for (label, body) in [
        ("provenance.json", provenance_text),
        ("required-surface.json", surface_text()),
    ] {
        for stale in ["2.7.1", "688bd6e846441bed481afaaf4d90546c8751f66a"] {
            assert!(
                !body.contains(stale),
                "{label}: 2.7.1 基準の出典がまだ正本に残っている ({stale})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 列挙の形 — 5 項目と出典
// ---------------------------------------------------------------------------

/// 列挙の各行は、出典行・bugfix 必須と条件・写像・受理状況・分類の 5 項目を欠かさない。
#[test]
fn every_entry_carries_the_five_measured_fields() {
    let surface = surface();
    assert_eq!(
        surface["upstream_version"].as_str(),
        Some("2.8.2"),
        "必要集合が 2.8.2 基準と名乗っていない"
    );
    let listed_sources: BTreeSet<String> = sources(&surface).into_iter().collect();
    let entries = entries(&surface);
    assert!(!entries.is_empty(), "列挙が空である");
    for entry in entries {
        let label = entry_label(entry);
        // 1. 出典 file:line — その行が実際にその起動形を綴っている。
        //    配布本文から来た行だけが持つ（実行時の指示だけの行は `upstream_site` が担う）。
        let cites = string_list(entry, "cites");
        if source_kinds(entry).contains("distribution") {
            assert!(!cites.is_empty(), "{label}: 配布本文の出典が無い");
        }
        for cite in cites {
            let path = cite.rsplit_once(':').expect("出典は path:line").0;
            assert!(
                listed_sources.contains(path),
                "{cite}: 列挙の対象外のファイル"
            );
            let found = labels(&cited_line(&cite));
            assert!(
                found.contains(&label),
                "{cite}: その行に起動形 `aidlc engine {label}` が無い — 拾えたのは {found:?}"
            );
        }
        // 2. bugfix 必須かどうかと、その条件。
        assert!(
            entry["bugfix_required"].is_boolean(),
            "{label}: bugfix_required が真偽値でない"
        );
        assert!(
            !text(entry, "condition").trim().is_empty(),
            "{label}: 必須/非必須と判定した条件の記述が無い"
        );
        // 3. noun → 既存の面・動詞の写像。
        match entry.get("mapping") {
            None => panic!("{label}: mapping が無い"),
            Some(serde_json::Value::Null) => {}
            Some(mapping) => {
                assert!(
                    !text(mapping, "face").is_empty() && !text(mapping, "verb").is_empty(),
                    "{label}: 写像先の面と動詞が空である"
                );
            }
        }
        // 4. 現行 build の受理状況。5. §5.1 の分類。
        let acceptance = text(entry, "acceptance");
        assert!(
            ["accepted", "refused"].contains(&acceptance.as_str()),
            "{label}: 受理状況が未知の値である ({acceptance})"
        );
        let classification = text(entry, "classification");
        assert!(
            ["mapped", "needs-implementation"].contains(&classification.as_str()),
            "{label}: 分類が未知の値である ({classification})"
        );
    }
}

/// 列挙は、凍結した出典が綴る起動形を取りこぼしても増やしてもいない。
///
/// 突き合わせるのは**配布本文由来の行**である。実行時の指示だけから来た行は、配布資産の
/// 散文にその起動形が現れないので、この集合には入らない（`upstream_site` の検査が担う）。
#[test]
fn the_enumeration_matches_the_launch_forms_in_the_frozen_sources() {
    let surface = surface();
    let listed: BTreeSet<String> = entries(&surface)
        .iter()
        .filter(|entry| source_kinds(entry).contains("distribution"))
        .map(|entry| entry_label(entry))
        .collect();
    let mut found: BTreeSet<String> = BTreeSet::new();
    for path in sources(&surface) {
        let raw = fs::read_to_string(repo_root().join(&path))
            .unwrap_or_else(|error| panic!("出典が読めない ({path}): {error}"));
        found.extend(labels(&raw));
    }
    // フックは `hooks` として別に数える (`hook <name>` は 1 動詞ではなく 16 の名前である)。
    let listed: BTreeSet<String> = listed
        .into_iter()
        .filter(|label| !label.starts_with("hook "))
        .collect();
    let found: BTreeSet<String> = found
        .into_iter()
        .filter(|label| !label.starts_with("hook "))
        .collect();
    assert_eq!(
        listed, found,
        "列挙と凍結バイトの起動形がずれている (左: 列挙、右: 実バイト)"
    );
}

/// 実行時の指示から来た行は、2.8.2 同梱ソースでの位置を持つ。
///
/// 位置は `<runtime_directive_sources[] のパス>:<行> <文言>` の形で、指した行にその文言が
/// 実在することまで確かめる（配布本文の `cites` と同じ作法）。
#[test]
fn a_runtime_directive_entry_cites_its_place_in_the_2_8_2_sources() {
    let surface = surface();
    let listed: BTreeSet<String> = runtime_directive_sources(&surface).into_iter().collect();
    assert!(
        !listed.is_empty(),
        "実行時の指示の出所が 1 件も凍結されていない"
    );
    let mut from_runtime = 0;
    for entry in entries(&surface) {
        let label = entry_label(entry);
        let sites = string_list(entry, "upstream_site");
        if !source_kinds(entry).contains("runtime-directive") {
            assert!(
                sites.is_empty(),
                "{label}: 実行時の指示から来ていないのに upstream_site を持つ"
            );
            continue;
        }
        from_runtime += 1;
        assert!(!sites.is_empty(), "{label}: 2.8.2 同梱ソースでの位置が無い");
        for site in sites {
            let (cite, marker) = site.split_once(' ').unwrap_or_else(|| {
                panic!("{label}: 位置は `path:line <文言>` の形である ({site})")
            });
            let path = cite.rsplit_once(':').expect("位置は path:line").0;
            assert!(
                listed.contains(path),
                "{label}: 実行時の指示の出所として凍結されていないファイル ({path})"
            );
            assert!(
                cited_line(cite).contains(marker),
                "{label}: {cite} にその文言が無い — {marker}"
            );
        }
    }
    assert!(
        from_runtime > 0,
        "実行時の指示から来た入口が 1 件も無い — 列挙の範囲が配布本文のままである"
    );
}

/// 配布本文と実行時の指示の両方に現れる入口は、行を増やさず種別を 2 つ持つ。
#[test]
fn an_entry_both_sources_spell_stays_one_row_with_both_kinds() {
    let surface = surface();
    let mut shared = 0;
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for entry in entries(&surface) {
        let label = entry_label(entry);
        assert!(seen.insert(label.clone()), "{label}: 同じ入口が 2 行ある");
        if source_kinds(entry).len() == 2 {
            shared += 1;
        }
    }
    assert!(
        shared > 0,
        "両方の出所に現れる入口が 1 件も無い — `orchestrate next` / `log link` は \
         配布本文と実行時の指示の双方が綴る"
    );
}

/// `intent create` は bugfix 必須の入口として受理済みになっている。
///
/// bugfix 1 周の開始でエンジンがこの指示を出す（`order.md` §1 の失敗点）。写像先の一段形は
/// 実装済みなので、分類は「写像だけで受理可能」である。
#[test]
fn the_intent_create_entry_is_required_and_accepted() {
    let surface = surface();
    let entry = entries(&surface)
        .into_iter()
        .find(|entry| entry_label(entry) == "intent create")
        .unwrap_or_else(|| {
            panic!("`intent create` が列挙に無い — 実行時の指示の入口が数えられていない")
        });
    assert_eq!(
        entry["bugfix_required"].as_bool(),
        Some(true),
        "`intent create`: bugfix 1 周の開始で出る指示が必須になっていない"
    );
    assert!(
        source_kinds(entry).contains("runtime-directive"),
        "`intent create`: 実行時の指示から来た出典を持たない"
    );
    assert_eq!(
        text(entry, "acceptance"),
        "accepted",
        "`intent create`: 受理状況が accepted でない"
    );
    assert_eq!(
        text(entry, "classification"),
        "mapped",
        "`intent create`: 写像だけで受理可能と分類されていない"
    );
    let mapping = entry
        .get("mapping")
        .filter(|value| !value.is_null())
        .unwrap_or_else(|| panic!("`intent create`: 写像先が無い"));
    assert_eq!(text(mapping, "face"), "aidlc-utility");
    assert_eq!(text(mapping, "verb"), "intent-create");
}

/// bugfix 1 周で踏まない入口も、列挙から落とさず根拠つきで残す。
#[test]
fn an_entry_the_bugfix_loop_does_not_walk_is_still_listed_with_its_reason() {
    let surface = surface();
    let mut optional = 0;
    for entry in entries(&surface) {
        if entry["bugfix_required"].as_bool().unwrap_or(true) {
            continue;
        }
        optional += 1;
        assert!(
            !text(entry, "condition").trim().is_empty(),
            "{}: 踏まないと書きながら根拠が無い",
            entry_label(entry)
        );
    }
    assert!(
        optional > 0,
        "bugfix 1 周で踏まない入口が 1 件も無い — 必要集合を狭める方向に倒していないか"
    );
}

// ---------------------------------------------------------------------------
// 受理と拒否 — 実挙動との突合
// ---------------------------------------------------------------------------

/// 写像先の一段形をこの build が拒否するか。
///
/// 面名と動詞名から一段形 argv を組むのがここの責務で、受理／拒否の判定そのものは
/// 要求の型が所有する（`Request::is_wired`）。
fn one_stage_refuses(face: &str, verb: &str) -> bool {
    let argv = vec![verb.to_string()];
    !parse(Face::of(face), &argv).is_wired()
}

/// argv の写像だけでは受理/拒否が決まらない面 (引数をそのまま運ぶ面)。
fn decided_after_parse(face: &str) -> bool {
    matches!(
        Face::of(face),
        Face::Jump | Face::TestingPosture | Face::ReviewBrief
    )
}

/// `mapped` と書いた入口は、写像先の一段形をこの build が実際に受理する。
///
/// 分類は二段形を直接 `parse` へ通して測らない — 写像先の一段形で測る。
#[test]
fn an_entry_classified_as_mapped_resolves_to_an_accepted_one_stage_form() {
    let surface = surface();
    let mut mapped = 0;
    for entry in entries(&surface) {
        let label = entry_label(entry);
        let classification = text(entry, "classification");
        let mapping = entry.get("mapping").filter(|value| !value.is_null());
        if classification != "mapped" {
            continue;
        }
        mapped += 1;
        let mapping =
            mapping.unwrap_or_else(|| panic!("{label}: 写像先が無いのに mapped と分類されている"));
        let face = text(mapping, "face");
        let verb = text(mapping, "verb");
        assert_eq!(
            text(entry, "acceptance"),
            "accepted",
            "{label}: mapped と分類しながら受理状況が accepted でない"
        );
        if decided_after_parse(&face) {
            // 引数をそのまま運ぶ面は、実行してはじめて受理/拒否が決まる。
            let stderr = one_stage_stderr(&face, &[&verb]);
            assert!(
                !stderr.contains("Unknown subcommand") && !stderr.contains("is not connected"),
                "{label}: 写像先 {face} {verb} が受理されていない — {stderr}"
            );
            continue;
        }
        assert!(
            !one_stage_refuses(&face, &verb),
            "{label}: 写像先 {face} {verb} がこの build に拒否された"
        );
    }
    assert!(mapped > 0, "写像だけで受理できる入口が 1 件も無い");
}

/// 写像先が無い入口は、写像だけで受理可能とは分類されない。
#[test]
fn an_entry_without_a_mapping_target_is_classified_as_needing_implementation() {
    let surface = surface();
    for entry in entries(&surface) {
        if entry.get("mapping").is_some_and(|value| !value.is_null()) {
            continue;
        }
        assert_eq!(
            text(entry, "classification"),
            "needs-implementation",
            "{}: 写像先が無いのに新規実装が不要と分類されている",
            entry_label(entry)
        );
    }
}

/// bugfix 必須の入口は、二段形の起動でこの build に受理される。
#[test]
fn an_entry_the_bugfix_loop_requires_is_accepted_in_the_two_stage_form() {
    let surface = surface();
    let mut required = 0;
    for entry in entries(&surface) {
        if !entry["bugfix_required"].as_bool().unwrap_or(false) {
            continue;
        }
        let label = entry_label(entry);
        if label.starts_with("hook ") {
            continue;
        }
        required += 1;
        let mut argv = vec!["engine".to_string(), text(entry, "noun")];
        if let Some(verb) = optional_text(entry, "verb") {
            argv.push(verb);
        }
        let run = run_engine(&argv);
        assert!(
            !is_refusal_of_an_unwired_entry(&run.stderr),
            "{label}: 二段形の起動が未配線を理由に拒否された — {}",
            run.stderr.trim()
        );
    }
    assert!(required > 0, "bugfix 必須の入口が 1 件も無い");
}

/// bugfix 必須集合の外の入口は、未実装であることを示す明示的な拒否を返す。
///
/// 文言そのものは実装側の選択なので逐語では固定しない。固定するのは、成功に見えないこと
/// (終了コードが 0 でなく、directive を出さない)、入口を名指していること、そして
/// 一段形の総称フォールバック (`Unknown subcommand: engine.`) に落ちていないことである。
#[test]
fn an_entry_outside_the_bugfix_required_set_is_refused_explicitly() {
    let surface = surface();
    let mut refused = 0;
    for entry in entries(&surface) {
        if entry["bugfix_required"].as_bool().unwrap_or(true) {
            continue;
        }
        if text(entry, "classification") != "needs-implementation" {
            continue;
        }
        let label = entry_label(entry);
        let noun = text(entry, "noun");
        let verb = optional_text(entry, "verb");
        let mut argv = vec!["engine".to_string(), noun.clone()];
        if let Some(verb) = verb.clone() {
            argv.push(verb);
        }
        let run = run_engine(&argv);
        refused += 1;
        assert_ne!(run.code, Some(0), "{label}: 未実装の入口が成功で終わった");
        assert!(
            run.stdout.trim().is_empty(),
            "{label}: 未実装の入口が directive を出した — {}",
            run.stdout.trim()
        );
        assert!(
            !run.stderr.contains("Unknown subcommand: engine."),
            "{label}: 二段形が一段形の総称フォールバックに落ちている — {}",
            run.stderr.trim()
        );
        assert!(
            run.stderr.contains(&noun),
            "{label}: 拒否が入口の noun を名指していない — {}",
            run.stderr.trim()
        );
        if let Some(verb) = verb {
            assert!(
                run.stderr.contains(&verb),
                "{label}: 拒否が入口の verb を名指していない — {}",
                run.stderr.trim()
            );
        }
    }
    assert!(refused > 0, "bugfix 必須集合の外の入口が 1 件も無い");
}

/// フックの列挙は、接続定義 (`hook-binding.json`) と 1 対 1 で一致する。
#[test]
fn the_hook_enumeration_agrees_with_the_binding_definition() {
    let surface = surface();
    let mut listed: BTreeMap<String, String> = BTreeMap::new();
    for hook in surface["hooks"].as_array().expect("hooks") {
        listed.insert(text(hook, "name"), text(hook, "status"));
    }
    let raw =
        fs::read_to_string(repo_root().join("scripts/aidlc-selfhost/hook-binding.json")).unwrap();
    let binding: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let mut expected: BTreeMap<String, String> = BTreeMap::new();
    for name in binding["native_hooks"].as_array().expect("native_hooks") {
        expected.insert(
            name.as_str()
                .expect("native_hooks は文字列の配列")
                .to_string(),
            "native-available".to_string(),
        );
    }
    for entry in binding["distributed_hooks"]
        .as_array()
        .expect("distributed_hooks")
    {
        expected.insert(text(entry, "name"), "distributed-only".to_string());
    }
    assert_eq!(listed, expected, "フックの分類が 2 つの資料でずれている");
}

// ---------------------------------------------------------------------------
// この build の起動
// ---------------------------------------------------------------------------

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// 未配線・未知を理由とする拒否か (§5.2 の「受理されない」)。
///
/// 引数不足や記録の不在など、解決後の実行時エラーはここに入れない。
fn is_refusal_of_an_unwired_entry(stderr: &str) -> bool {
    stderr.contains("Unknown subcommand")
        || stderr.contains("is not wired in this build")
        || stderr.contains("is not connected")
        || stderr.contains("unknown verb")
        || stderr.contains("missing verb")
        || stderr.contains("Unknown hook")
}

/// この build のバイナリを `target` の名前で置く — **シンボリックリンク**で。
///
/// 実体のリンク数を動かさないので、同じ実体を並列に実行している別のテストを巻き込まない。
fn place_binary(target: &std::path::Path) {
    let source = env!("CARGO_BIN_EXE_aidlc");
    #[cfg(unix)]
    {
        if std::os::unix::fs::symlink(source, target).is_ok() {
            return;
        }
    }
    fs::copy(source, target).map(|_| ()).unwrap();
}

/// `argv[0]` を `aidlc` にして 1 回だけ起動する (空のワークスペース)。
fn run_engine(args: &[String]) -> Run {
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    let (code, stdout, stderr) = spawn("aidlc", &borrowed);
    Run {
        code,
        stdout,
        stderr,
    }
}

/// 一段形を 1 回起動して標準エラーを取る。
fn one_stage_stderr(face: &str, args: &[&str]) -> String {
    spawn(face, args).2
}

/// 出力は**ファイルへ**落とし、待つのは直接の子だけにする。この build はセッション補助の
/// 子を起こすことがあり (`aidlc/.aidlc-sessions/pids/`)、パイプの終端を待つ形 (`output()`) は
/// その孫に引きずられうるためである。
fn spawn(argv0: &str, args: &[&str]) -> (Option<i32>, String, String) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("workspace");
    fs::create_dir_all(root.join("aidlc/spaces/default/memory")).unwrap();
    fs::create_dir_all(root.join(".claude")).unwrap();
    let binary = temp.path().join(argv0);
    place_binary(&binary);
    let out = temp.path().join("stdout.txt");
    let err = temp.path().join("stderr.txt");
    let mut child = Command::new(binary)
        .args(args)
        .current_dir(&root)
        .env_clear()
        .envs(coverage_profile_env())
        .env("HOME", temp.path())
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .env("TZ", "UTC")
        .stdin(Stdio::null())
        .stdout(Stdio::from(fs::File::create(&out).unwrap()))
        .stderr(Stdio::from(fs::File::create(&err).unwrap()))
        .spawn()
        .unwrap();
    let status = child.wait().unwrap();
    (
        status.code(),
        fs::read_to_string(&out).unwrap_or_default(),
        fs::read_to_string(&err).unwrap_or_default(),
    )
}
