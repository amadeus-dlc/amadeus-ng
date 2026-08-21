//! `StateVersion` 分類器 — `ok / unparseable / past / future` の 4 値。**runtime と doctor が
//! 同一関数を使う**ことを、分類結果型のコンストラクタをモジュール内 private にして強制する
//! (W7 の E1 装置: この型の値は `classify_state_version` 経由でしか生成できない)。
//! 行照合は `^- \*\*State Version\*\*:[ \t]*(\S+)[ \t]*$` (m) — 行末アンカーのため
//! `8 garbage` は unparseable (upstream `aidlc-lib.ts:10627`, 03 §5.5)。

pub const CURRENT_STATE_VERSION: u32 = 8;

/// 分類結果 (private フィールドにより外部構築不能)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateVersionClassification {
    kind: StateVersionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateVersionKind {
    Ok,
    Unparseable,
    Past,
    Future,
}

impl StateVersionClassification {
    pub fn kind(self) -> StateVersionKind {
        self.kind
    }
}

/// 唯一の分類器 (runtime / doctor の双方がこれを呼ぶ)。
pub fn classify_state_version(state_content: &str) -> StateVersionClassification {
    let kind = classify(state_content);
    StateVersionClassification { kind }
}

fn classify(content: &str) -> StateVersionKind {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn with_version(v: &str) -> String {
        format!("# AI-DLC State Tracking\n\n## Project Information\n- **State Version**: {v}\n")
    }

    #[test]
    fn current_version_classifies_ok() {
        assert_eq!(classify_state_version(&with_version("8")).kind(), StateVersionKind::Ok);
    }

    #[test]
    fn older_and_newer_versions_classify_past_and_future() {
        assert_eq!(classify_state_version(&with_version("7")).kind(), StateVersionKind::Past);
        assert_eq!(classify_state_version(&with_version("9")).kind(), StateVersionKind::Future);
    }

    #[test]
    fn trailing_garbage_and_missing_rows_are_unparseable() {
        assert_eq!(
            classify_state_version(&with_version("8 garbage")).kind(),
            StateVersionKind::Unparseable
        );
        assert_eq!(
            classify_state_version("# no version row\n").kind(),
            StateVersionKind::Unparseable
        );
    }
}
