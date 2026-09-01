//! `CheckboxState` — Stage Progress 行の 6 状態マーカー (01 §3.3 — E1)。
//!
//! マーカー語彙の分類 (in-flight / finished / active) は本型が所有する — 呼出側で変種集合を
//! 再列挙しないこと (Tell, Don't Ask。`cargo lint` の checkbox-vocabulary が機械強制)。
//! 行の文法と行編集は `Checkboxes` 側にある。

/// 6 状態 (01 §3.3 — E1)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckboxState {
    /// `[ ]` = upstream `pending` — 未着手。
    Pending,
    /// `[-]` = upstream `in-progress` — 実行中 (ゲートはまだ開いていない)。
    InProgress,
    /// `[?]` = upstream `awaiting-approval` — 承認ゲート開放済み (`[-]` → `[?]`)。
    AwaitingApproval,
    /// `[R]` = upstream `revising` — 差戻し後の改訂中。ゲートに再入できる唯一の状態。
    Revising,
    /// `[x]` = upstream `completed` — 完了。`Completed` フィールド同期の集計対象。
    Completed,
    /// `[S]` = upstream `skipped` — 経路上の帰結としての読み飛ばし (完了ではない)。
    Skipped,
}

impl CheckboxState {
    /// 行に書かれる 1 文字マーカー。`from_marker` の逆写像であり 6 状態と 1:1
    /// (往復忠実: `from_marker(s.marker()) == Some(s)`)。
    #[must_use]
    pub const fn marker(self) -> char {
        match self {
            CheckboxState::Pending => ' ',
            CheckboxState::InProgress => '-',
            CheckboxState::AwaitingApproval => '?',
            CheckboxState::Revising => 'R',
            CheckboxState::Completed => 'x',
            CheckboxState::Skipped => 'S',
        }
    }

    /// マーカー 1 文字から状態へ。閉集合 `[ xSR?-]` の外は `None` — 呼出側 (`CheckboxEntry::parse_line`) は
    /// その行を checkbox 行と見なさない。
    #[must_use]
    pub const fn from_marker(c: char) -> Option<CheckboxState> {
        Some(match c {
            ' ' => CheckboxState::Pending,
            '-' => CheckboxState::InProgress,
            '?' => CheckboxState::AwaitingApproval,
            'R' => CheckboxState::Revising,
            'x' => CheckboxState::Completed,
            'S' => CheckboxState::Skipped,
            _ => return None,
        })
    }

    // ---- 分類述語 (マーカー語彙の分類は本型が所有する — Tell, Don't Ask)。
    //      呼出側で変種集合を再列挙しないこと。 ----

    /// 未終了 (in-flight)。upstream `next` の in-flight 判定 (02 §5.1 手順 10-2) の集合:
    /// `pending / in-progress / awaiting-approval / revising`。
    /// 「checkbox 行の欠落も in-flight」の扱いは行の存否を知る呼出側の責務。
    #[must_use]
    pub const fn is_in_flight(self) -> bool {
        !self.is_finished()
    }

    /// 終了済み (`completed` / `skipped`) — 前進走査 (`nextInScopeStage`) が読み飛ばす集合。
    #[must_use]
    pub const fn is_finished(self) -> bool {
        matches!(self, CheckboxState::Completed | CheckboxState::Skipped)
    }

    /// 着手済み (in-flight のうち `pending` を除く: `in-progress / awaiting-approval /
    /// revising`)。jump forward が skipped 化する「現ステージ」の条件 (09 §3)。
    #[must_use]
    pub const fn is_active(self) -> bool {
        self.is_in_flight() && !matches!(self, CheckboxState::Pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// マーカー 1 文字は 6 状態の閉集合と 1:1 に対応する (往復忠実)。
    ///
    /// この対応は state ファイルの逐語形そのものであり、ワイヤ形式のラウンドトリップ PBT が
    /// 副次的に踏んでいた経路でもある (Bolt B6 でワイヤ形式を退役させたので、ここで直接
    /// 固定し直す)。
    #[test]
    fn every_marker_round_trips_through_the_closed_set() {
        for (state, marker) in [
            (CheckboxState::Pending, ' '),
            (CheckboxState::InProgress, '-'),
            (CheckboxState::AwaitingApproval, '?'),
            (CheckboxState::Revising, 'R'),
            (CheckboxState::Completed, 'x'),
            (CheckboxState::Skipped, 'S'),
        ] {
            assert_eq!(state.marker(), marker, "{state:?}");
            assert_eq!(
                CheckboxState::from_marker(marker),
                Some(state),
                "{marker:?}"
            );
        }
    }

    #[test]
    fn a_marker_outside_the_closed_set_is_not_a_checkbox() {
        for marker in ['X', 'r', '*', '\u{3000}'] {
            assert_eq!(CheckboxState::from_marker(marker), None, "{marker:?}");
        }
    }

    #[test]
    fn classification_predicates_partition_the_six_markers() {
        use CheckboxState::{AwaitingApproval, Completed, InProgress, Pending, Revising, Skipped};
        let all = [
            Pending,
            InProgress,
            AwaitingApproval,
            Revising,
            Completed,
            Skipped,
        ];
        for cb in all {
            // in-flight と finished は補集合
            assert_ne!(cb.is_in_flight(), cb.is_finished(), "{cb:?}");
            // active = in-flight ∧ ¬pending
            assert_eq!(cb.is_active(), cb.is_in_flight() && cb != Pending, "{cb:?}");
        }
        assert!(Pending.is_in_flight() && !Pending.is_active());
        assert!(Revising.is_active());
        assert!(Completed.is_finished() && Skipped.is_finished());
    }
}
