//! `JumpDirection` — forward / backward / redo。`Current Stage` とのインデックス比較から
//! **導出**される (upstream `aidlc-jump.ts:175-181`、02 §8)。

/// jump の 3 方向。`target` と `cursor` の大小関係そのもの (閉集合・全域)。
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
    /// 方向は宣言ではなく導出 (E1 — 矛盾した方向指定は表現不能)。
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
