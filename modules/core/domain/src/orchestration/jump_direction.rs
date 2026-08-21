//! `JumpDirection` — forward / backward / redo。`Current Stage` とのインデックス比較から
//! **導出**される (upstream `aidlc-jump.ts:175-181`、02 §8)。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JumpDirection {
    Forward,
    Backward,
    Redo,
}

impl JumpDirection {
    /// 方向は宣言ではなく導出 (E1 — 矛盾した方向指定は表現不能)。
    #[must_use]
    pub fn derive(cursor: usize, target: usize) -> JumpDirection {
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
        assert_eq!(JumpDirection::derive(2, 4), JumpDirection::Forward);
        assert_eq!(JumpDirection::derive(4, 2), JumpDirection::Backward);
        assert_eq!(JumpDirection::derive(3, 3), JumpDirection::Redo);
    }
}
