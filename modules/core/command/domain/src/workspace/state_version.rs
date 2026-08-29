//! `StateVersion` 分類器 — `ok / unparseable / past / future` の 4 値。**runtime と doctor が
//! 同一関数を使う**ことを、分類結果型のコンストラクタをモジュール内 private にして強制する
//! (W7 の E1 装置: この型の値は `StateVersionClassification::classify` 経由でしか生成できない)。
//! 行照合は `^- \*\*State Version\*\*:[ \t]*(\S+)[ \t]*$` (m) — 行末アンカーのため
//! `8 garbage` は unparseable (upstream `aidlc-lib.ts:10627`, 03 §5.5)。

/// `Ok` と判定される唯一の版 (upstream `CURRENT_STATE_VERSION = "8"`)。これ未満は `Past`、
/// 超過は `Future`。
pub const CURRENT_STATE_VERSION: u32 = 8;

/// 分類結果 (private フィールドにより外部構築不能)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateVersionClassification {
    kind: StateVersionKind,
}

/// 4 分類 (upstream `{kind:"ok"|"unparseable"|"past"|"future"}` と 1:1)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateVersionKind {
    /// `CURRENT_STATE_VERSION` と一致 — そのまま読み書きしてよい。
    Ok,
    /// State Version 行が無い、または値が行末アンカーに収まらない / 整数でない。
    /// upstream はこの分類でアーカイブ (`mv aidlc aidlc.archive`) と作り直しを指示する。
    Unparseable,
    /// `CURRENT_STATE_VERSION` 未満 — 旧版が書いた state ファイル。
    Past,
    /// `CURRENT_STATE_VERSION` 超過 — 新しい版の state ファイルを古い実装が読んでいる。
    Future,
}

impl StateVersionClassification {
    /// 4 分類の値。runtime と doctor はこの同一の判定を見る (W7)。
    #[must_use]
    pub const fn kind(self) -> StateVersionKind {
        self.kind
    }

    /// 唯一の分類器 (runtime / doctor の双方がこれを呼ぶ — 2026-08-29 是正で
    /// 自由関数 `classify_state_version` から本型の関連関数へ)。
    #[must_use]
    pub fn classify(state_content: &str) -> StateVersionClassification {
        StateVersionClassification {
            kind: StateVersionClassification::kind_of(state_content),
        }
    }

    fn kind_of(content: &str) -> StateVersionKind {
        const PREFIX: &str = "- **State Version**:";
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix(PREFIX) {
                let trimmed = rest.trim_matches([' ', '\t']);
                // 行末アンカー: 値は空白を含まない 1 トークンのみ
                if trimmed.is_empty() || trimmed.contains(' ') || trimmed.contains('\t') {
                    return StateVersionKind::Unparseable;
                }
                // TODO(golden: stage-0): 数値比較の受理集合 (非整数トークンの扱い) を upstream 実測で確定
                return match trimmed.parse::<u32>() {
                    Err(_) => StateVersionKind::Unparseable,
                    Ok(v) if v == CURRENT_STATE_VERSION => StateVersionKind::Ok,
                    Ok(v) if v < CURRENT_STATE_VERSION => StateVersionKind::Past,
                    Ok(_) => StateVersionKind::Future,
                };
            }
        }
        StateVersionKind::Unparseable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
