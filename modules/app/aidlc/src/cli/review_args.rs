//! `aidlc-log review` が運ぶ引数（upstream `parseFlags` — `aidlc-log.ts:92-119`）。

/// `aidlc-log review` が運ぶ引数。
///
/// 値の妥当性（`--iteration` が正の整数か、`--verdict` が閉集合か）は判断しない — それは
/// 合成ルートの構文段の仕事である。ここが持つのは「どのフラグにどの生値が付いたか」と、
/// **フラグ文法そのものの違反**（値が必要なフラグに値が無い）だけである。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReviewArgs {
    stage: Option<String>,
    reviewer: Option<String>,
    iteration: Option<String>,
    verdict: Option<String>,
    unit: Option<String>,
    intent: Option<String>,
    space: Option<String>,
    single: bool,
    retry_pending: bool,
    parse_error: Option<String>,
}

impl ReviewArgs {
    /// レビュー対象のステージ（`--stage`）。
    #[must_use]
    pub fn stage(&self) -> Option<&str> {
        self.stage.as_deref()
    }
    /// 名指されたレビュアー（`--reviewer`）。
    #[must_use]
    pub fn reviewer(&self) -> Option<&str> {
        self.reviewer.as_deref()
    }
    /// 通し番号の生値（`--iteration`）。正整数の検査は合成ルートが行う。
    #[must_use]
    pub fn iteration(&self) -> Option<&str> {
        self.iteration.as_deref()
    }
    /// 判定の生値（`--verdict`）。**有無それ自体が契約**（有れば判定形）なので `Option`。
    #[must_use]
    pub fn verdict(&self) -> Option<&str> {
        self.verdict.as_deref()
    }
    /// per-unit の対象（`--unit`）。本 build では未配線の拒否材料である。
    #[must_use]
    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }
    /// intent セレクタ（`--intent`）。review では拒否される。
    #[must_use]
    pub fn intent(&self) -> Option<&str> {
        self.intent.as_deref()
    }
    /// space セレクタ（`--space`）。review では拒否される。
    #[must_use]
    pub fn space(&self) -> Option<&str> {
        self.space.as_deref()
    }
    /// 隔離実行の受領証（`--single`）。本 build では未配線の拒否材料である。
    #[must_use]
    pub const fn is_single(&self) -> bool {
        self.single
    }
    /// 判定待ちの依頼の呼び直し（`--retry-pending`）。
    #[must_use]
    pub const fn is_retry_pending(&self) -> bool {
        self.retry_pending
    }
    /// フラグ文法そのものの違反（値が必要なフラグに値が無い）。
    #[must_use]
    pub fn parse_error(&self) -> Option<&str> {
        self.parse_error.as_deref()
    }
}

/// 値を取らない真偽フラグ（upstream `:101-104`）。
const BOOLEAN_FLAGS: [&str; 2] = ["--single", "--retry-pending"];

/// `review` のフラグを [`ReviewArgs`] へ畳む（upstream `parseFlags` の写し）。
///
/// upstream は `--` で始まるトークンをすべてフラグとして扱い、真偽 2 つ以外は**値を必須**に
/// する。値が無い（引数列が尽きた／次も `--` 始まり）ときは即座に `error()` で止まるので、
/// こちらは最初の違反を `parse_error` に載せて運ぶ（出すのは合成ルート）。
///
/// `ReviewArgs` のフィールドはすべて private なので、フィールド単位で組み立てる本関数は
/// 同じファイル（同じモジュール）に置く（`coding-rules/field-visibility.md`）。
pub(super) fn parse_review(args: &[String]) -> ReviewArgs {
    let mut flags = ReviewArgs::default();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        index += 1;
        // 位置引数は upstream も `positional` へ落として無視する。
        if !arg.starts_with("--") {
            continue;
        }
        if BOOLEAN_FLAGS.contains(&arg.as_str()) {
            match arg.as_str() {
                "--single" => flags.single = true,
                _ => flags.retry_pending = true,
            }
            continue;
        }
        let Some(value) = args.get(index) else {
            // upstream は最初の違反で `error()` して止まる — 残りは読まない。
            flags.parse_error = Some(crate::wording::flag_expects_a_value(arg));
            break;
        };
        if value.starts_with("--") {
            flags.parse_error = Some(crate::wording::flag_expects_a_value_got_flag(arg, value));
            break;
        }
        index += 1;
        match arg.as_str() {
            "--stage" => flags.stage = Some(value.clone()),
            "--reviewer" => flags.reviewer = Some(value.clone()),
            "--iteration" => flags.iteration = Some(value.clone()),
            "--verdict" => flags.verdict = Some(value.clone()),
            "--unit" => flags.unit = Some(value.clone()),
            "--intent" => flags.intent = Some(value.clone()),
            "--space" => flags.space = Some(value.clone()),
            // upstream の `flags[a.slice(2)] = val` は未知のフラグも辞書に載せるだけで
            // 拒否しない — 読まれないので黙って捨てる。
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

    fn parse(values: &[&str]) -> ReviewArgs {
        parse_review(&args(values))
    }

    /// 値つきのフラグは 1 つずつ値を取り、真偽 2 つは取らない（upstream `:101-104`）。
    #[test]
    fn the_value_flags_take_their_value_and_the_two_boolean_flags_do_not() {
        let flags = parse(&[
            "--stage",
            "domain-design",
            "--reviewer",
            "aidlc-architecture-reviewer-agent",
            "--iteration",
            "2",
            "--verdict",
            "NOT-READY",
            "--unit",
            "b48",
            "--intent",
            "260822",
            "--space",
            "default",
            "--single",
            "--retry-pending",
        ]);
        assert_eq!(flags.stage(), Some("domain-design"));
        assert_eq!(flags.reviewer(), Some("aidlc-architecture-reviewer-agent"));
        assert_eq!(flags.iteration(), Some("2"));
        assert_eq!(flags.verdict(), Some("NOT-READY"));
        assert_eq!(flags.unit(), Some("b48"));
        assert_eq!(flags.intent(), Some("260822"));
        assert_eq!(flags.space(), Some("default"));
        assert!(flags.is_single());
        assert!(flags.is_retry_pending());
        assert_eq!(flags.parse_error(), None);
    }

    /// 真偽フラグは**次のトークンを食べない**（食べると後続のフラグが消える）。
    #[test]
    fn a_boolean_flag_does_not_swallow_the_next_token() {
        let flags = parse(&["--retry-pending", "--stage", "domain-design"]);
        assert!(flags.is_retry_pending());
        assert_eq!(flags.stage(), Some("domain-design"));
        assert_eq!(flags.parse_error(), None);
    }

    /// 値が必要なフラグに値が無い 2 形（upstream `:106` / `:110` 逐語）。
    #[test]
    fn a_value_flag_without_a_value_carries_the_upstream_refusal() {
        assert_eq!(
            parse(&["--stage"]).parse_error(),
            Some("--stage expects a value, got end of arguments.")
        );
        assert_eq!(
            parse(&["--stage", "--reviewer"]).parse_error(),
            Some(
                "--stage expects a value, got another flag: \"--reviewer\". Did you forget the value?"
            )
        );
    }

    /// 位置引数と未知のフラグは黙って捨てる（upstream も辞書に載せるだけで読まない）。
    #[test]
    fn positional_tokens_and_unknown_flags_are_ignored() {
        let flags = parse(&["review", "--frobnicate", "x", "--stage", "domain-design"]);
        assert_eq!(flags.stage(), Some("domain-design"));
        assert_eq!(flags.parse_error(), None);
    }

    /// 空の引数列は既定値（どのフラグも無い）である。
    #[test]
    fn an_empty_argument_list_yields_the_default() {
        assert_eq!(parse(&[]), ReviewArgs::default());
    }
}
