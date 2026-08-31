//! `CheckboxState` — Stage Progress 行の 6 状態マーカー。
//!
//! リードモデル `aidlc-state.md` の行文法
//! `/^- \[([ xSR?-])\] (\S+)\s*—\s*(.*)$/gm` (upstream `aidlc-lib.ts:6678`, 03 §5.4) が
//! 運ぶ語彙で、投影 (RMU) が書き、クエリ側が読む。**分類の判断は本型が所有する**
//! (Tell, Don't Ask) — 呼出側で変種集合を再列挙しないこと。

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
        // `cargo lint` の CHECKBOX_OWNER 定数はコマンド側の語彙ファイル 1 本を指している。
        // B26 段階 1 の増築で語彙の所有者が一時的に 2 本になったための抑制であり、段階 2 で
        // コマンド側を撤去したらリンター側の定数を本ファイルへ付け替えてこの抑制を外す。
        // amadeus-lint: allow(checkbox-vocabulary) — 語彙の所有者そのもの (マーカーの正本写像)
        match self {
            CheckboxState::Pending => ' ',
            CheckboxState::InProgress => '-',
            CheckboxState::AwaitingApproval => '?',
            CheckboxState::Revising => 'R',
            CheckboxState::Completed => 'x',
            CheckboxState::Skipped => 'S',
        }
    }

    /// マーカー 1 文字から状態へ。閉集合 `[ xSR?-]` の外は `None` — 呼出側はその行を
    /// checkbox 行と見なさない。
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
    #[must_use]
    pub const fn is_in_flight(self) -> bool {
        !self.is_finished()
    }

    /// 終了済み (`completed` / `skipped`) — 前進走査 (`nextInScopeStage`) が読み飛ばす集合。
    #[must_use]
    pub const fn is_finished(self) -> bool {
        // 抑制の理由は `marker` と同じ — 段階 2 でリンターの CHECKBOX_OWNER を付け替える。
        // amadeus-lint: allow(checkbox-vocabulary) — 分類述語の定義そのもの (所有者)
        matches!(self, CheckboxState::Completed | CheckboxState::Skipped)
    }

    /// 着手済み (in-flight のうち `pending` を除く: `in-progress / awaiting-approval /
    /// revising`)。
    #[must_use]
    pub const fn is_active(self) -> bool {
        self.is_in_flight() && !matches!(self, CheckboxState::Pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [CheckboxState; 6] = [
        CheckboxState::Pending,
        CheckboxState::InProgress,
        CheckboxState::AwaitingApproval,
        CheckboxState::Revising,
        CheckboxState::Completed,
        CheckboxState::Skipped,
    ];

    #[test]
    fn the_marker_round_trips_for_all_six() {
        for state in ALL {
            assert_eq!(CheckboxState::from_marker(state.marker()), Some(state));
        }
        assert_eq!(CheckboxState::from_marker('z'), None);
    }

    #[test]
    fn the_classification_predicates_partition_the_six() {
        assert!(CheckboxState::Completed.is_finished());
        assert!(CheckboxState::Skipped.is_finished());
        for state in [
            CheckboxState::Pending,
            CheckboxState::InProgress,
            CheckboxState::AwaitingApproval,
            CheckboxState::Revising,
        ] {
            assert!(state.is_in_flight());
            assert!(!state.is_finished());
        }
        assert!(!CheckboxState::Pending.is_active());
        assert!(CheckboxState::InProgress.is_active());
        assert!(CheckboxState::AwaitingApproval.is_active());
        assert!(CheckboxState::Revising.is_active());
        assert!(!CheckboxState::Completed.is_active());
        assert!(!CheckboxState::Skipped.is_active());
    }
}
