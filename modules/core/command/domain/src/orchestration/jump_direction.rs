//! `JumpDirection` — forward / backward / redo。resolveは位置から導き、executeは指定値を保存する。

/// jumpが指示する3つの適用方式。executeでは到達点の位置関係と一致するとは限らない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JumpDirection {
    /// `target > cursor` — まだ通過していないステージへ跳ぶ。介在する in-flight ステージと、
    /// pending でない現ステージを `skipped` にする (skip 1 件につき `STAGE_SKIPPED` 1 行)。
    Forward,
    /// `target < cursor` — 通過済みのステージへ戻る。ターゲットより下流の EXECUTE ステージを
    /// `pending` に戻し、承認履歴を無効化する (I3 の後段)。
    Backward,
    /// `target == cursor` — 現ステージのやり直し。ターゲットを開き直し、その承認履歴を落とす。
    Redo,
}

impl JumpDirection {
    /// resolveが現在位置から推奨方向を導く。executeの指定値を置き換える用途には使わない。
    #[must_use]
    pub fn of(cursor: usize, target: usize) -> JumpDirection {
        use std::cmp::Ordering::*;
        match target.cmp(&cursor) {
            Greater => JumpDirection::Forward,
            Less => JumpDirection::Backward,
            Equal => JumpDirection::Redo,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_is_derived_from_index_comparison() {
        assert_eq!(JumpDirection::of(2, 4), JumpDirection::Forward);
        assert_eq!(JumpDirection::of(4, 2), JumpDirection::Backward);
        assert_eq!(JumpDirection::of(3, 3), JumpDirection::Redo);
    }
}
