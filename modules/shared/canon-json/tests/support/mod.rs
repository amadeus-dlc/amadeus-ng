//! ゴールデン比較器 (テスト支援、BR2.2 / W5)。
//!
//! `tests/golden/upstream-3c3146cf/` のコーパスを読み、`normalization.json` の規則で
//! 非決定値をプレースホルダへ潰し、行ごとの差分を出す。**期待値と実測値の双方に同じ
//! 規則を適用してから**比較するのが BR2.2 の要件であり、この 3 つ (読取・正規化・差分)
//! が比較器の全機能である。
//!
//! 置き場はテスト支援であってプロダクトコードではない (`nfr-design/logical-components.md`
//! §4)。canon-json のライブラリ本体には 1 バイトも入らない。
//!
//! cli / hooks 族について本 Unit (U1) が固定するのは「読めて正規化できる」ところまでで、
//! 実装出力との突合せは U6 (next / continue) と U7 (CLI・フック) が同じ比較器で行う。

use std::fs;
use std::path::{Path, PathBuf};

use regex::{NoExpand, Regex};

/// 正規化を適用する観測チャネル。`normalization.json` の `applies_to` に対応する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Channel {
    Stdout,
    Stderr,
    StateDiff,
    Audit,
}

impl Channel {
    /// `normalization.json` の `applies_to` に現れる綴り。
    const fn as_str(self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
            Self::StateDiff => "state-diff",
            Self::Audit => "audit",
        }
    }
}

/// 規則の適用方式。`normalization.json` の `kind` に対応する。
#[derive(Debug)]
enum Matcher {
    /// 形から拾うパターン。
    Pattern(Regex),
    /// 実行時に与えられた作業ツリーの絶対パスを literal 置換する。
    RuntimePath,
    /// 実行時に与えられた監査シャード名・ホスト名を literal 置換する。
    RuntimeClone,
}

/// 正規化規則 1 本。
#[derive(Debug)]
struct Rule {
    placeholder: String,
    matcher: Matcher,
    replacement: String,
    channels: Vec<String>,
}

/// 実行時にしか決まらない値。`runtime-*` 方式の規則へ与える。
#[derive(Debug, Default)]
pub(crate) struct RuntimeValues {
    roots: Vec<String>,
    clones: Vec<String>,
}

impl RuntimeValues {
    /// 作業ツリーの絶対パス群 (realpath 解決の前後の両方を入れてよい) と、
    /// 監査シャード名・ホスト名から作る。
    pub(crate) const fn new(roots: Vec<String>, clones: Vec<String>) -> Self {
        Self { roots, clones }
    }

    fn roots(&self) -> &[String] {
        &self.roots
    }

    fn clones(&self) -> &[String] {
        &self.clones
    }
}

/// `normalization.json` から読んだ規則一式。コーパスの一部であり、比較器はこのファイル
/// だけを規則の正本として読む。
#[derive(Debug)]
pub(crate) struct Normalization {
    rules: Vec<Rule>,
    families: std::collections::HashMap<String, Vec<String>>,
}

impl Normalization {
    /// コーパスの `normalization.json` を読む。壊れていたら比較器としては続行不能なので
    /// その場で panic する (テスト支援であり、失敗はテスト失敗として現れるのが正しい)。
    pub(crate) fn load() -> Self {
        let path = corpus_root().join("normalization.json");
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("正規化規則を読めない ({}): {e}", path.display()));
        let doc: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("正規化規則が JSON でない: {e}"));

        let raw_rules = doc["rules"]
            .as_array()
            .unwrap_or_else(|| panic!("normalization.json の rules が配列でない"));
        let mut rules = Vec::with_capacity(raw_rules.len());
        for (index, raw) in raw_rules.iter().enumerate() {
            let placeholder = raw["placeholder"]
                .as_str()
                .unwrap_or_else(|| panic!("規則 {index}: placeholder が無い"))
                .to_string();
            let kind = raw["kind"]
                .as_str()
                .unwrap_or_else(|| panic!("規則 {index}: kind が無い"));
            let matcher = match kind {
                "regex" => {
                    let pattern = raw["pattern"]
                        .as_str()
                        .unwrap_or_else(|| panic!("規則 {index}: pattern が無い"));
                    Matcher::Pattern(Regex::new(pattern).unwrap_or_else(|e| {
                        panic!("規則 {index}: pattern が正規表現でない ({pattern}): {e}")
                    }))
                }
                "runtime-path" => Matcher::RuntimePath,
                "runtime-clone" => Matcher::RuntimeClone,
                other => panic!("規則 {index}: 未知の kind {other}"),
            };
            let replacement = raw["replacement"]
                .as_str()
                .unwrap_or(placeholder.as_str())
                .to_string();
            let channels = raw["applies_to"]
                .as_array()
                .unwrap_or_else(|| panic!("規則 {index}: applies_to が配列でない"))
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect();
            rules.push(Rule {
                placeholder,
                matcher,
                replacement,
                channels,
            });
        }

        let mut families = std::collections::HashMap::new();
        if let Some(map) = doc["families"].as_object() {
            for (name, spec) in map {
                let applies = spec["applies"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                families.insert(name.clone(), applies);
            }
        }

        Self { rules, families }
    }

    /// 読み込んだ規則の本数。
    pub(crate) const fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// 規則が使っているプレースホルダの一覧 (重複あり)。NFR1.3 の 4 種検査に使う。
    pub(crate) fn placeholders(&self) -> Vec<String> {
        self.rules.iter().map(|r| r.placeholder.clone()).collect()
    }

    /// 1 チャネル分のテキストを正規化する。
    ///
    /// 規則は `normalization.json` の配列順に適用する (順序は意味を持つ)。族が許した
    /// プレースホルダの規則だけを、そのチャネルに `applies_to` が届くときだけ当てる。
    /// 置換文字列は literal 扱いで、`$1` のような後方参照は展開しない。
    pub(crate) fn normalize(
        &self,
        text: &str,
        family: &str,
        channel: Channel,
        runtime: &RuntimeValues,
    ) -> String {
        let allowed = self.families.get(family);
        let mut out = text.to_string();
        for rule in &self.rules {
            if allowed.is_some_and(|list| !list.contains(&rule.placeholder)) {
                continue;
            }
            if !rule.channels.iter().any(|c| c == channel.as_str()) {
                continue;
            }
            match &rule.matcher {
                Matcher::Pattern(re) => {
                    out = re
                        .replace_all(&out, NoExpand(rule.replacement.as_str()))
                        .into_owned();
                }
                Matcher::RuntimePath => {
                    out = replace_longest_first(&out, runtime.roots(), &rule.replacement);
                }
                Matcher::RuntimeClone => {
                    out = replace_longest_first(&out, runtime.clones(), &rule.replacement);
                }
            }
        }
        out
    }
}

/// 実行時 literal 置換。短い値が長い値の一部を先に食わないよう、長い順に当てる。
fn replace_longest_first(text: &str, needles: &[String], replacement: &str) -> String {
    let mut ordered: Vec<&String> = needles.iter().filter(|n| !n.is_empty()).collect();
    ordered.sort_by_key(|n| std::cmp::Reverse(n.len()));
    let mut out = text.to_string();
    for needle in ordered {
        out = out.replace(needle.as_str(), replacement);
    }
    out
}

/// コーパスのルート。テストの実行位置に依存しないようクレートの位置から解決する。
pub(crate) fn corpus_root() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tests/golden/upstream-3c3146cf"
    ))
}

/// 1 ケース分のディレクトリ (`<family>/<group>/<case>`)。
#[derive(Debug)]
pub(crate) struct GoldenCase {
    dir: PathBuf,
    id: String,
}

impl GoldenCase {
    /// 族の中でのケース ID (`<group>/<case>`)。
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    /// ケースディレクトリの絶対パス。
    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    /// ケースディレクトリ直下のファイルを読む。無ければ `None`。
    pub(crate) fn read(&self, name: &str) -> Option<String> {
        fs::read_to_string(self.dir.join(name)).ok()
    }
}

/// 族 (`cli` / `hooks`) の全ケースを ID 昇順で返す。
///
/// レイアウトは C7 の `<family>/<group>/<case>/` の 2 階層。族直下の JSON
/// (`provenance.json` / `cases-missing.json`) はケースではないので拾わない。
pub(crate) fn cases(family: &str) -> Vec<GoldenCase> {
    let root = corpus_root().join(family);
    let mut found = Vec::new();
    let Ok(groups) = fs::read_dir(&root) else {
        return found;
    };
    for group in groups.flatten() {
        if !group.path().is_dir() {
            continue;
        }
        let group_name = group.file_name().to_string_lossy().to_string();
        let Ok(entries) = fs::read_dir(group.path()) else {
            continue;
        };
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let case_name = entry.file_name().to_string_lossy().to_string();
            found.push(GoldenCase {
                dir: entry.path(),
                id: format!("{group_name}/{case_name}"),
            });
        }
    }
    found.sort_by(|a, b| a.id.cmp(&b.id));
    found
}

/// コーパス相対パスの JSON を読む。読めない/壊れていればテスト失敗として panic する。
pub(crate) fn read_json(relative: &str) -> serde_json::Value {
    let path = corpus_root().join(relative);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("ゴールデンを読めない ({}): {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("ゴールデンが JSON でない ({}): {e}", path.display()))
}

/// 行ごとの差分。同じ位置の行だけを突き合わせ、食い違った行を
/// `<行番号> - <期待値>` / `<行番号> + <実測値>` の 2 行で返す (1 始まり)。
///
/// 失敗表示のためのものなので、挿入・削除をずらして合わせにいく LCS は使わない —
/// ゴールデン比較で見たいのは「何行目のどのバイトが違うのか」だからである。
pub(crate) fn line_diff(expected: &str, actual: &str) -> Vec<String> {
    let want: Vec<&str> = expected.lines().collect();
    let got: Vec<&str> = actual.lines().collect();
    let mut out = Vec::new();
    for index in 0..want.len().max(got.len()) {
        let line = index + 1;
        match (want.get(index), got.get(index)) {
            (Some(w), Some(g)) if w == g => {}
            (Some(w), Some(g)) => {
                out.push(format!("{line} - {w}"));
                out.push(format!("{line} + {g}"));
            }
            (Some(w), None) => out.push(format!("{line} - {w}")),
            (None, Some(g)) => out.push(format!("{line} + {g}")),
            (None, None) => {}
        }
    }
    out
}
