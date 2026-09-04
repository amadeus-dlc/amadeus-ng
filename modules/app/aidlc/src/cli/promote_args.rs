//! `aidlc-state practices-promote` が運ぶ引数（upstream の写し — `aidlc-state.ts:3512-3519`）。

/// `aidlc-state practices-promote` が運ぶ引数。
///
/// 値の妥当性（ドラフトが在るか・正本が在るか・contributions が揃っているか）は判断しない —
/// それは合成ルートの構文段の仕事である。ここが持つのは「どのフラグにどの生値が付いたか」
/// だけである。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PromoteArgs {
    team_practices: Option<String>,
    discovered_rules: Option<String>,
    affirming_user: Option<String>,
    target_dir: Option<String>,
}

impl PromoteArgs {
    /// `team-practices.md` ドラフトのパス（`--team-practices`）。
    #[must_use]
    pub fn team_practices(&self) -> Option<&str> {
        self.team_practices.as_deref()
    }
    /// `discovered-rules.md` ドラフトのパス（`--discovered-rules`）。
    #[must_use]
    pub fn discovered_rules(&self) -> Option<&str> {
        self.discovered_rules.as_deref()
    }
    /// 昇格を打った人（`--affirming-user`）。既定は upstream と同じ `unknown`。
    #[must_use]
    pub fn affirming_user(&self) -> &str {
        self.affirming_user.as_deref().unwrap_or(UNKNOWN_USER)
    }
    /// 書込先の差替（`--target-dir`）。本 build では未配線の拒否材料である。
    #[must_use]
    pub fn target_dir(&self) -> Option<&str> {
        self.target_dir.as_deref()
    }
}

/// `--affirming-user` を省いたときの値（upstream `:3734` の `?? "unknown"`）。
const UNKNOWN_USER: &str = "unknown";

/// `practices-promote` のフラグを [`PromoteArgs`] へ畳む（upstream `:3512-3519` の写し）。
///
/// upstream は「`--` で始まるトークンの**次のトークン**を値に取る」だけである — 真偽フラグは
/// 無く、値が無いフラグ（引数列の末尾に来た `--x`）は**黙って捨てる**（`i + 1 < args.length`
/// が偽なら辞書に載らない）。次のトークンが `--` 始まりでも値として取る点も upstream どおり。
///
/// `PromoteArgs` のフィールドはすべて private なので、フィールド単位で組み立てる本関数は
/// 同じファイル（同じモジュール）に置く（`coding-rules/field-visibility.md`）。
pub(super) fn parse_promote(args: &[String]) -> PromoteArgs {
    let mut flags = PromoteArgs::default();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        index += 1;
        if !arg.starts_with("--") {
            continue;
        }
        // 末尾の孤立した `--x` は捨てる（upstream の `i + 1 < args.length`）。
        let Some(value) = args.get(index) else { break };
        index += 1;
        match arg.as_str() {
            "--team-practices" => flags.team_practices = Some(value.clone()),
            "--discovered-rules" => flags.discovered_rules = Some(value.clone()),
            "--affirming-user" => flags.affirming_user = Some(value.clone()),
            "--target-dir" => flags.target_dir = Some(value.clone()),
            // upstream の `flags[a.slice(2)] = args[i + 1]` は未知のフラグも辞書に載せる
            // だけで拒否しない — 読まれないので黙って捨てる。
            _ => {}
        }
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn parse(values: &[&str]) -> PromoteArgs {
        parse_promote(&args(values))
    }

    #[test]
    fn the_four_flags_take_the_next_token_as_their_value() {
        let flags = parse(&[
            "--team-practices",
            "a/team-practices.md",
            "--discovered-rules",
            "a/discovered-rules.md",
            "--affirming-user",
            "owner",
            "--target-dir",
            "/tmp/fixture",
        ]);
        assert_eq!(flags.team_practices(), Some("a/team-practices.md"));
        assert_eq!(flags.discovered_rules(), Some("a/discovered-rules.md"));
        assert_eq!(flags.affirming_user(), "owner");
        assert_eq!(flags.target_dir(), Some("/tmp/fixture"));
    }

    /// `--affirming-user` を省くと upstream と同じ既定値になる。
    #[test]
    fn an_omitted_affirming_user_reads_as_unknown() {
        assert_eq!(parse(&[]).affirming_user(), "unknown");
    }

    /// 真偽フラグは無い — 次のトークンが `--` 始まりでも値として食う（upstream どおり）。
    #[test]
    fn a_flag_swallows_the_next_token_even_when_it_looks_like_a_flag() {
        let flags = parse(&["--team-practices", "--discovered-rules", "b.md"]);
        assert_eq!(flags.team_practices(), Some("--discovered-rules"));
        assert_eq!(flags.discovered_rules(), None);
    }

    /// 末尾の孤立したフラグは捨てる（値が無いので辞書に載らない）。
    #[test]
    fn a_trailing_flag_without_a_value_is_dropped() {
        let flags = parse(&["--discovered-rules", "b.md", "--team-practices"]);
        assert_eq!(flags.discovered_rules(), Some("b.md"));
        assert_eq!(flags.team_practices(), None);
    }

    /// 位置引数と未知のフラグは黙って捨てる。
    #[test]
    fn positional_tokens_and_unknown_flags_are_ignored() {
        let flags = parse(&["promote", "--frobnicate", "x", "--team-practices", "a.md"]);
        assert_eq!(flags.team_practices(), Some("a.md"));
    }

    #[test]
    fn an_empty_argument_list_yields_the_default() {
        assert_eq!(parse(&[]), PromoteArgs::default());
    }
}
