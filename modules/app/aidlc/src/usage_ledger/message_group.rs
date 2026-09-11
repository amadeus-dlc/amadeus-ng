//! 同じ `message.id` が連続する行の群と、その代表行。
//!
//! Claude は 1 回の llm 呼出しを内容ブロックごとの複数行に分けて書き、同じ `usage` を
//! 各行に刻む（旧形式）か、先頭行を 0 にして最後の行に実測値を置く（新形式）。
//! どちらも「総量が最大の行、同点なら後の行」を代表に採れば 1 回として数えられる
//! （本家 `foldFileIntoLedger` の群化と `representativeOfRun`）。
//! `message.id` の無い行は単独の群で、まとめない。離れて再出現した id は別の群になる
//! （本家の畳み込み経路は `seen` を持たない）。
use harness_claude::TranscriptUsageRow;

/// 1 群。代表行と、群の先頭行のバイト位置（保留時にここまで巻き戻す）。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MessageGroup {
    representative: TranscriptUsageRow,
    byte_start: u64,
}

impl MessageGroup {
    const fn new(representative: TranscriptUsageRow, byte_start: u64) -> Self {
        Self {
            representative,
            byte_start,
        }
    }

    /// ファイル順の行を、連続する同一 `message.id` ごとにまとめる。
    pub(crate) fn collapse(rows: Vec<(u64, TranscriptUsageRow)>) -> Vec<Self> {
        let mut groups = Vec::new();
        let mut open: Option<(String, Self)> = None;
        for (byte_start, row) in rows {
            let message_id = row.message_id().to_string();
            open = Some(match open.take() {
                Some((open_id, group)) if !open_id.is_empty() && open_id == message_id => {
                    let representative = if group.representative.outranked_by(&row) {
                        row
                    } else {
                        group.representative
                    };
                    (open_id, Self::new(representative, group.byte_start))
                }
                Some((_, group)) => {
                    groups.push(group);
                    (message_id, Self::new(row, byte_start))
                }
                None => (message_id, Self::new(row, byte_start)),
            });
        }
        if let Some((_, group)) = open {
            groups.push(group);
        }
        groups
    }

    /// 群を代表する行。
    pub(crate) const fn representative(&self) -> &TranscriptUsageRow {
        &self.representative
    }

    /// 群の先頭行のバイト位置。
    pub(crate) const fn byte_start(&self) -> u64 {
        self.byte_start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(uuid: &str, message_id: &str, input: u64) -> TranscriptUsageRow {
        TranscriptUsageRow::parse(
            &format!(
                r#"{{"uuid":"{uuid}","message":{{"id":"{message_id}","role":"assistant","usage":{{"input_tokens":{input}}}}}}}"#
            ),
            None,
        )
        .expect("課金対象の行")
    }

    #[test]
    fn contiguous_lines_of_one_message_collapse_to_the_last_maximum() {
        let groups = MessageGroup::collapse(vec![
            (0, row("s1", "msg_a", 0)),
            (300, row("s2", "msg_a", 1000)),
            (600, row("s3", "msg_b", 10)),
        ]);
        assert_eq!(
            groups,
            vec![
                MessageGroup::new(row("s2", "msg_a", 1000), 0),
                MessageGroup::new(row("s3", "msg_b", 10), 600),
            ]
        );
    }

    #[test]
    fn old_style_ties_choose_the_last_line() {
        let groups = MessageGroup::collapse(vec![
            (0, row("s1", "msg_a", 7)),
            (100, row("s2", "msg_a", 7)),
        ]);
        assert_eq!(groups, vec![MessageGroup::new(row("s2", "msg_a", 7), 0)]);
    }

    #[test]
    fn id_less_rows_are_never_merged() {
        let groups = MessageGroup::collapse(vec![(0, row("a", "", 1)), (50, row("b", "", 2))]);
        assert_eq!(
            groups,
            vec![
                MessageGroup::new(row("a", "", 1), 0),
                MessageGroup::new(row("b", "", 2), 50)
            ]
        );
    }

    #[test]
    fn a_reappearing_id_starts_a_new_group() {
        let groups = MessageGroup::collapse(vec![
            (0, row("a", "msg_a", 1)),
            (50, row("b", "msg_b", 1)),
            (100, row("c", "msg_a", 1)),
        ]);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups.last().map(MessageGroup::byte_start), Some(100));
    }
}
