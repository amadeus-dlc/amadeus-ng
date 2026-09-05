//! `aidlc-bolt set-autonomy` が運ぶ引数（upstream `parseFlags` — `aidlc-bolt.ts:162-178`）。

/// `aidlc-bolt set-autonomy` が運ぶ引数。
///
/// 値の妥当性（`--mode` が 2 値の閉集合か）は判断しない — それは合成ルートの構文段の仕事で
/// ある。ここが持つのは「どのフラグにどの生値が付いたか」と、**フラグ文法そのものの違反**
/// （値が必要なフラグに値が無い）だけである。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetAutonomyArgs {
    mode: Option<String>,
    parse_error: Option<String>,
}

impl SetAutonomyArgs {
    /// 切替先の生値（`--mode`）。**有無それ自体が契約**（無ければ usage 拒否）なので `Option`。
    #[must_use]
    pub fn mode(&self) -> Option<&str> {
        self.mode.as_deref()
    }

    /// フラグ文法そのものの違反（値が必要なフラグに値が無い）。
    #[must_use]
    pub fn parse_error(&self) -> Option<&str> {
        self.parse_error.as_deref()
    }
}

/// `set-autonomy` のフラグを [`SetAutonomyArgs`] へ畳む（upstream `parseFlags` の写し）。
///
/// `aidlc-bolt` の `parseFlags` に**真偽フラグは無い** — `--` で始まるトークンはすべて次の
/// トークンを値に取り、値が無い（引数列が尽きた／次も `--` 始まり）ときは即座に `error()` で
/// 止まる。こちらは最初の違反を `parse_error` に載せて運ぶ（出すのは合成ルート）。
/// 逐語は `aidlc-log` / `aidlc-orchestrate` の `parseFlags` と同じ 2 形なので
/// [`crate::wording::flag_expects_a_value`] / [`crate::wording::flag_expects_a_value_got_flag`]
/// を再利用する。
///
/// `SetAutonomyArgs` のフィールドはすべて private なので、フィールド単位で組み立てる本関数は
/// 同じファイル（同じモジュール）に置く（`coding-rules/field-visibility.md`）。
pub(super) fn parse_set_autonomy(args: &[String]) -> SetAutonomyArgs {
    let mut flags = SetAutonomyArgs::default();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        index += 1;
        // 位置引数は upstream も `continue` で読み飛ばす。
        if !arg.starts_with("--") {
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
        // upstream の `flags[a.slice(2)] = val` は未知のフラグも辞書に載せるだけで拒否
        // しない — `set-autonomy` は読まないので黙って捨てる。
        if arg.as_str() == "--mode" {
            flags.mode = Some(value.clone());
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

    fn parse(values: &[&str]) -> SetAutonomyArgs {
        parse_set_autonomy(&args(values))
    }

    #[test]
    fn the_mode_flag_takes_the_next_token_as_its_value() {
        let flags = parse(&["--mode", "autonomous"]);
        assert_eq!(flags.mode(), Some("autonomous"));
        assert_eq!(flags.parse_error(), None);
    }

    /// 閉集合の検査は**ここではしない**（合成ルートが逐語で断る）。
    #[test]
    fn an_unknown_mode_value_is_carried_verbatim() {
        assert_eq!(parse(&["--mode", "turbo"]).mode(), Some("turbo"));
    }

    /// 値が必要なフラグに値が無い 2 形（upstream `:168` / `:172` 逐語）。
    #[test]
    fn a_value_flag_without_a_value_carries_the_upstream_refusal() {
        assert_eq!(
            parse(&["--mode"]).parse_error(),
            Some("--mode expects a value, got end of arguments.")
        );
        assert_eq!(
            parse(&["--mode", "--batch"]).parse_error(),
            Some(
                "--mode expects a value, got another flag: \"--batch\". Did you forget the value?"
            )
        );
    }

    /// 真偽フラグは無い — 未知のフラグも次のトークンを値として食う（upstream どおり）。
    #[test]
    fn an_unknown_flag_still_swallows_its_value() {
        let flags = parse(&["--frobnicate", "x", "--mode", "gated"]);
        assert_eq!(flags.mode(), Some("gated"));
        assert_eq!(flags.parse_error(), None);
    }

    /// 位置引数は読み飛ばす。
    #[test]
    fn positional_tokens_are_ignored() {
        assert_eq!(
            parse(&["set-autonomy", "--mode", "gated"]).mode(),
            Some("gated")
        );
    }

    #[test]
    fn an_empty_argument_list_yields_the_default() {
        assert_eq!(parse(&[]), SetAutonomyArgs::default());
        assert_eq!(parse(&[]).mode(), None);
    }
}
