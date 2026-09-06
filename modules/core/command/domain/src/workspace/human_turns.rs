//! `HumanTurns` — 監査台帳から読み取った「人が居た」証拠（B9 の材料、I11）。

use chrono::{DateTime, Utc};

use super::audit_events::{EventCategory, EventType};
use super::ordered_audit_events::OrderedAuditEvents;

/// 監査台帳から読み取った「人が居た」証拠（値オブジェクト）。
///
/// `HUMAN_TURN` 行はハーネスのフック（`aidlc-record-human-turn.ts`）が**シャードへ直接**
/// 追記する一次の事実であり、我々のドメインイベントの投影ではない — 台帳が唯一の記録である。
/// したがってこれは集約が自分の歴史から導ける状態ではなく、合成ルートが読んで**引数で渡す
/// 外部の入力**である（`coding-rules/aggregate-references.md`「判断に要るデータは
/// メソッド引数で渡す」）。判断（昇格の可否）そのものは集約のクエリ
/// [`human_acted_since_gate`] が持つ。
///
/// # 構築経路は [`HumanTurns::find_in`] だけ
///
/// フィールドは private で、値を直に組む口は無い。合成ルートは必ず連結バッファを渡して
/// 組む — 「読取規則を通っていない証拠」を証拠として扱う経路を作らないためである
/// （同型の先例は [`OrderedAuditEvents::find_in`]）。[`Default`] は「台帳が無い」
/// （追跡が有効でなく、人間の turn も 1 つも無い）を表し、モデル駆動のテストのためにある。
///
/// [`human_acted_since_gate`]: crate::orchestration::IntentExecution::human_acted_since_gate
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HumanTurns {
    latest: Option<DateTime<Utc>>,
    tracked: bool,
}

impl HumanTurns {
    /// 連結済みの台帳バッファから証拠を読み取る（唯一の構築子 — upstream
    /// `humanActedSinceGate` の走査部（`aidlc-lib.ts:3801-3818`）の写し）。
    ///
    /// - **追跡が有効か**（`tracked`）は「DocumentKB の来歴 3 行以外のイベントが 1 つでも
    ///   在るか」で決まる（upstream `sawPresenceTrackingEvent`）。DocumentKB の行だけの台帳は
    ///   presence 追跡を有効にしないので、そこに `HUMAN_TURN` が無くても「人が居ない」とは
    ///   言えない。
    /// - **最新の人間の turn**（`latest`）は `HUMAN_TURN` 行のうち最大のタイムスタンプである。
    ///   秒精度 ISO として読めない行は無視する — 読めない値を最小値として比較に混ぜると、
    ///   壊れた 1 行が判定を動かしてしまう。
    #[must_use]
    pub fn find_in(buffer: &str) -> HumanTurns {
        let events = OrderedAuditEvents::find_in(buffer);
        let (latest, tracked) = events.fold_left((None, false), |(latest, tracked), record| {
            let latest = if record.event() == EventType::HumanTurn {
                latest.max(record.instant())
            } else {
                latest
            };
            (
                latest,
                tracked || record.event().category() != EventCategory::Documents,
            )
        });
        HumanTurns { latest, tracked }
    }

    /// 最新の `HUMAN_TURN` の発生時刻（1 つも読めなければ `None`）。
    #[must_use]
    pub const fn latest(&self) -> Option<DateTime<Utc>> {
        self.latest
    }

    /// この台帳で human presence の追跡が有効か（DocumentKB の来歴行だけなら偽）。
    #[must_use]
    pub const fn is_tracked(&self) -> bool {
        self.tracked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(timestamp: &str, event: &str) -> String {
        format!("\n## H\n**Timestamp**: {timestamp}\n**Event**: {event}\n")
    }

    fn ledger(blocks: &[String]) -> String {
        format!("{}\n---\n", blocks.join("\n---\n"))
    }

    fn instant(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .expect("固定の ISO")
            .with_timezone(&Utc)
    }

    /// 台帳が無い（既定）は「追跡なし・turn なし」である。
    #[test]
    fn an_absent_ledger_is_untracked_and_carries_no_turn() {
        let turns = HumanTurns::default();
        assert!(!turns.is_tracked());
        assert_eq!(turns.latest(), None);
        assert_eq!(HumanTurns::find_in(""), turns);
    }

    /// `HUMAN_TURN` が無くても、他のイベントが在れば追跡は有効である。
    #[test]
    fn any_non_document_event_activates_presence_tracking() {
        let turns = HumanTurns::find_in(&ledger(&[block("2026-08-21T09:00:00Z", "GATE_APPROVED")]));
        assert!(turns.is_tracked());
        assert_eq!(turns.latest(), None);
    }

    /// DocumentKB の来歴 3 行だけの台帳は追跡を有効にしない（upstream の carve-out）。
    #[test]
    fn a_ledger_of_document_provenance_alone_stays_untracked() {
        let turns = HumanTurns::find_in(&ledger(&[
            block("2026-08-21T09:00:00Z", "DOCUMENT_INDEXED"),
            block("2026-08-21T09:00:01Z", "DOCUMENT_UPDATED"),
            block("2026-08-21T09:00:02Z", "DOCUMENT_REMOVED"),
        ]));
        assert!(!turns.is_tracked());
        assert_eq!(turns.latest(), None);
    }

    /// 最新の `HUMAN_TURN` を採る（バッファの並びではなく時刻の最大）。
    #[test]
    fn the_latest_human_turn_wins_regardless_of_buffer_order() {
        let turns = HumanTurns::find_in(&ledger(&[
            block("2026-08-21T09:00:05Z", "HUMAN_TURN"),
            block("2026-08-21T09:00:01Z", "HUMAN_TURN"),
            block("2026-08-21T09:00:03Z", "GATE_APPROVED"),
        ]));
        assert!(turns.is_tracked());
        assert_eq!(turns.latest(), Some(instant("2026-08-21T09:00:05Z")));
    }

    /// 秒精度 ISO として読めない行は無視する（追跡の有効化には効く）。
    #[test]
    fn an_unreadable_timestamp_is_ignored_rather_than_compared() {
        let turns = HumanTurns::find_in(&ledger(&[
            block("2026-08-21 09:00:09", "HUMAN_TURN"),
            block("2026-08-21T09:00:02Z", "HUMAN_TURN"),
        ]));
        assert!(turns.is_tracked());
        assert_eq!(turns.latest(), Some(instant("2026-08-21T09:00:02Z")));
    }

    /// 読めない行しか無ければ turn は無い（読めない値を最小値として混ぜない）。
    #[test]
    fn a_ledger_of_unreadable_turns_carries_no_turn() {
        let turns = HumanTurns::find_in(&ledger(&[block("never", "HUMAN_TURN")]));
        assert!(turns.is_tracked());
        assert_eq!(turns.latest(), None);
    }
}
