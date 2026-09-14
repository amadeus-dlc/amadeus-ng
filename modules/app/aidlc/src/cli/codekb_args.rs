//! `aidlc-utility` の codekb 系読取動詞が運ぶフラグ（upstream `parseArgs` — `aidlc-lib.ts:22279`）。

/// `codekb-path` / `codekb-scope-diff` が運ぶフラグ。
///
/// 値の妥当性（repo 名が単一パス片か、`--compare` の指す先が在るか）は判断しない — それは
/// 消費側の仕事である。ここが持つのは「どのフラグにどの生値が付いたか」だけである。
///
/// upstream の `parseArgs` は真偽フラグと値つきフラグを**綴りで区別しない** — 次のトークンが
/// `--` で始まらなければ値として食い、さもなくば `"true"` を入れる。したがって
/// `--json` / `--mint` は「値が `"true"`」として現れ、`--repo` に値を与えなければ同じく
/// `"true"` になる。その意味論をそのまま写す。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodekbArgs {
    repo: Option<String>,
    json: Option<String>,
    mint: Option<String>,
    paths: Option<String>,
    compare: Option<String>,
    staged: Option<String>,
    expect_store: Option<String>,
    expect_source: Option<String>,
}

impl CodekbArgs {
    /// 対象リポジトリ（`--repo`）。**空文字は「与えられていない」と同じ**に扱われる —
    /// upstream の `flags.repo && flags.repo.length > 0` の条件をここで表す。
    #[must_use]
    pub fn repo(&self) -> Option<&str> {
        self.repo.as_deref().filter(|value| !value.is_empty())
    }

    /// JSON 形で出すか（`--json`）。upstream は `flags.json === "true"` で判定する。
    #[must_use]
    pub fn is_json(&self) -> bool {
        self.json.as_deref() == Some("true")
    }

    /// 指紋の鋳造モードか（`--mint`）。upstream は `flags.mint === "true"` で判定する。
    #[must_use]
    pub fn is_mint(&self) -> bool {
        self.mint.as_deref() == Some("true")
    }

    /// 指紋を取る対象（`--paths`、コンマ区切りの生値）。
    #[must_use]
    pub fn paths(&self) -> Option<&str> {
        self.paths.as_deref()
    }

    /// 突合する取込側の走査記録（`--compare`）。**有無それ自体がモードの契約**なので
    /// `Option` で運ぶ（upstream も `flags.compare !== undefined` で分岐する）。
    #[must_use]
    pub fn compare(&self) -> Option<&str> {
        self.compare.as_deref()
    }

    /// `--paths` をコンマで割り、前後の空白を落として空片を捨てる（upstream の
    /// `.split(",").map(trim).filter(!== "")`）。
    ///
    /// **重複は落とさない** — 読取側 (`codekb-scope-diff --mint`) の upstream がそうだからで
    /// ある。書込側は [`CodekbArgs::unique_path_list`] を使う。
    #[must_use]
    pub fn path_list(&self) -> Vec<String> {
        self.paths
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .map(|path| path.trim().to_string())
            .filter(|path| !path.is_empty())
            .collect()
    }

    /// `--paths` を割ってから**重複を落とす**（upstream `codekbPaths` の `[...new Set(paths)]`）。
    ///
    /// 初出の順序を保つ。書込側 (`codekb-snapshot` / `codekb-publish`) は、この一覧をそのまま
    /// `SOURCE_PATHS` として出し、compare-and-swap の対象範囲にも使う。
    #[must_use]
    pub fn unique_path_list(&self) -> Vec<String> {
        let mut unique: Vec<String> = Vec::new();
        for path in self.path_list() {
            if !unique.contains(&path) {
                unique.push(path);
            }
        }
        unique
    }

    /// staged された公開候補のディレクトリ（`--staged`）。
    ///
    /// **空文字は「与えられていない」と同じ**に扱う — upstream の `if (!stagedFlag)` は空文字も
    /// 偽と見るためである。
    #[must_use]
    pub fn staged(&self) -> Option<&str> {
        self.staged.as_deref().filter(|value| !value.is_empty())
    }

    /// 公開が前提とするストアの世代（`--expect-store`）。空文字は未指定と同じ。
    #[must_use]
    pub fn expect_store(&self) -> Option<&str> {
        self.expect_store
            .as_deref()
            .filter(|value| !value.is_empty())
    }

    /// 公開が前提とする源の指紋（`--expect-source`）。空文字は未指定と同じ。
    #[must_use]
    pub fn expect_source(&self) -> Option<&str> {
        self.expect_source
            .as_deref()
            .filter(|value| !value.is_empty())
    }
}

/// codekb 系動詞のフラグを [`CodekbArgs`] へ畳む（upstream `parseArgs` の写し）。
///
/// `CodekbArgs` のフィールドはすべて private なので、フィールド単位で組み立てる本関数は
/// 同じファイル（同じモジュール）に置く（`coding-rules/field-visibility.md`）。
/// upstream の `parseArgs` は 3 形を受ける。
///
/// 1. `--key=value` — 最初の `=` で割る（値が空でも辞書に載る）
/// 2. `--key value` — 次のトークンが `--` 始まりでなければ値として食う
/// 3. `--key`       — 次が `--` 始まり、または引数列の末尾なら値は `"true"`
///
/// 真偽フラグと値つきフラグを**綴りで区別しない**のがこの関数の要である。`--json` が真になる
/// のは 3 形で `"true"` が入るからであって、`--json` を真偽フラグとして特別扱いするからでは
/// ない。したがって `--json false` と書けば `flags.json` は `"false"` になり、upstream の
/// `flags.json === "true"` は偽になる — その意味論をそのまま写す。
///
/// 位置引数は upstream も `positional` へ分けて読まないので捨てる。
pub(super) fn parse_codekb(args: &[String]) -> CodekbArgs {
    let mut flags = CodekbArgs::default();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        let Some(token) = arg.strip_prefix("--") else {
            index += 1;
            continue;
        };
        let (key, value) = match token.split_once('=') {
            Some((key, value)) => {
                index += 1;
                (key, value.to_string())
            }
            None => match args.get(index + 1) {
                Some(next) if !next.starts_with("--") => {
                    index += 2;
                    (token, next.clone())
                }
                _ => {
                    index += 1;
                    (token, "true".to_string())
                }
            },
        };
        match key {
            "repo" => flags.repo = Some(value),
            "json" => flags.json = Some(value),
            "mint" => flags.mint = Some(value),
            "paths" => flags.paths = Some(value),
            "compare" => flags.compare = Some(value),
            "staged" => flags.staged = Some(value),
            "expect-store" => flags.expect_store = Some(value),
            "expect-source" => flags.expect_source = Some(value),
            // upstream の `flags[key] = value` は未知のフラグも辞書に載せるだけで拒否しない —
            // 読まれないので黙って捨てる。
            _ => {}
        }
    }
    flags
}
