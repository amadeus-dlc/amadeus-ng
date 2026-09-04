//! `StateVersion` 分類器 — `ok / unparseable / past / future` の 4 値。**runtime と doctor が
//! 同一関数を使う**ことを、分類結果型のコンストラクタをモジュール内 private にして強制する
//! (W7 の E1 装置: この型の値は `StateVersionClassification::classify` 経由でしか生成できない)。
//! 行照合は `^- \*\*State Version\*\*:[ \t]*(\S+)[ \t]*$` (m) — 行末アンカーのため
//! `8 garbage` は unparseable (upstream `aidlc-lib.ts:10627`, 03 §5.5)。

use super::state_version_kind::StateVersionKind;

/// `Ok` と判定される唯一の版 (upstream `CURRENT_STATE_VERSION = "8"`)。これ未満は `Past`、
/// 超過は `Future`。
pub const CURRENT_STATE_VERSION: u32 = 8;

/// 分類結果 (private フィールドにより外部構築不能)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateVersionClassification {
    kind: StateVersionKind,
    version: Option<String>,
}

impl StateVersionClassification {
    /// 4 分類の値。runtime と doctor はこの同一の判定を見る (W7)。
    #[must_use]
    pub const fn kind(&self) -> StateVersionKind {
        self.kind
    }

    /// 読めた `State Version` の**生トークン** (`Past` / `Future` のときだけ在る)。
    ///
    /// upstream の拒否文言は読めた値をそのまま埋める (`State Version ${v} predates …`) ので、
    /// 正規化した数値ではなく行に書かれていた綴りを運ぶ (`aidlc-lib.ts:10640` の `v`)。
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// 唯一の分類器 (runtime / doctor の双方がこれを呼ぶ — 2026-08-29 是正で
    /// 自由関数 `classify_state_version` から本型の関連関数へ)。
    #[must_use]
    pub fn classify(state_content: &str) -> StateVersionClassification {
        StateVersionClassification::token_of(state_content).map_or_else(
            || StateVersionClassification {
                kind: StateVersionKind::Unparseable,
                version: None,
            },
            |token| StateVersionClassification::of(&token),
        )
    }

    /// 行から `State Version` の生トークンを取り出す。
    ///
    /// 行末アンカー: 値は空白を含まない 1 トークンのみ (`State Version: 8 garbage` は
    /// 取り出せず unparseable に落ちる)。
    fn token_of(content: &str) -> Option<String> {
        const PREFIX: &str = "- **State Version**:";
        let rest = content
            .lines()
            .find_map(|line| line.strip_prefix(PREFIX))?
            .trim_matches([' ', '\t']);
        (!rest.is_empty() && !rest.contains(' ') && !rest.contains('\t')).then(|| rest.to_string())
    }

    /// 生トークンを 4 分類へ写す。
    ///
    /// 一致判定は**綴りの一致**である (upstream `v === CURRENT_STATE_VERSION`) — 数値に
    /// 畳んでから比べると `008` のような非正準の綴りが `ok` になってしまい、upstream が
    /// `past` として拒む状態を通してしまう。数値比較は綴りが一致しなかった後で使う
    /// (`aidlc-lib.ts:10642-10643`)。
    fn of(token: &str) -> StateVersionClassification {
        let kind = if token == CURRENT_STATE_VERSION.to_string() {
            StateVersionKind::Ok
        } else {
            match token.parse::<u32>() {
                Err(_) => StateVersionKind::Unparseable,
                Ok(value) if value > CURRENT_STATE_VERSION => StateVersionKind::Future,
                Ok(_) => StateVersionKind::Past,
            }
        };
        StateVersionClassification {
            kind,
            version: matches!(kind, StateVersionKind::Past | StateVersionKind::Future)
                .then(|| token.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_non_integer_token_is_unparseable() {
        assert_eq!(
            StateVersionClassification::classify(&with_version("v8")).kind(),
            StateVersionKind::Unparseable
        );
    }

    fn with_version(v: &str) -> String {
        format!("# AI-DLC State Tracking\n\n## Project Information\n- **State Version**: {v}\n")
    }

    #[test]
    fn current_version_classifies_ok() {
        assert_eq!(
            StateVersionClassification::classify(&with_version("8")).kind(),
            StateVersionKind::Ok
        );
    }

    #[test]
    fn older_and_newer_versions_classify_past_and_future() {
        assert_eq!(
            StateVersionClassification::classify(&with_version("7")).kind(),
            StateVersionKind::Past
        );
        assert_eq!(
            StateVersionClassification::classify(&with_version("9")).kind(),
            StateVersionKind::Future
        );
    }

    #[test]
    fn a_non_canonical_spelling_of_the_current_version_is_not_ok() {
        // upstream は綴りで一致を見る (`v === "8"`) ので `008` は `past` である。数値に
        // 畳んでから比べると `ok` になり、拒むべき状態を通してしまう。
        let classified = StateVersionClassification::classify(&with_version("008"));
        assert_eq!(classified.kind(), StateVersionKind::Past);
        assert_eq!(classified.version(), Some("008"));
    }

    #[test]
    fn only_the_past_and_future_arms_carry_the_version_token() {
        assert_eq!(
            StateVersionClassification::classify(&with_version("8")).version(),
            None
        );
        assert_eq!(
            StateVersionClassification::classify(&with_version("7")).version(),
            Some("7")
        );
        assert_eq!(
            StateVersionClassification::classify(&with_version("9")).version(),
            Some("9")
        );
        assert_eq!(
            StateVersionClassification::classify("# no version row\n").version(),
            None
        );
    }

    #[test]
    fn trailing_garbage_and_missing_rows_are_unparseable() {
        assert_eq!(
            StateVersionClassification::classify(&with_version("8 garbage")).kind(),
            StateVersionKind::Unparseable
        );
        assert_eq!(
            StateVersionClassification::classify("# no version row\n").kind(),
            StateVersionKind::Unparseable
        );
    }
}
