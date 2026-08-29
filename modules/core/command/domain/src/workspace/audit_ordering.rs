//! シャード横断の**位置付き読取** — `OrderedAuditEvents::find_in`（upstream `findAllEvents`、11-workspace §2.3 / FR1.1）。
//!
//! 監査台帳にシーケンス番号は無い。行が持つ序数的なものは**秒精度の ISO タイムスタンプ**だけで
//! あり、それだけでは同一秒の順序が決まらない。upstream は連結バッファ上の**位置**でタイを
//! 破る（`aidlc-lib.ts:7799-7801`）。素朴に「最後の行が最新」と読むと、辞書順で後ろのシャード
//! から**より古い**イベントを拾ってしまうためである。
//!
//! # ここはドメインに残る（描画は投影へ移った）
//!
//! 11-workspace §2.3 は、描画（`render_audit_block` / `state_writers`）を投影へ移す一方で
//! `findAllEvents`（実装は `OrderedAuditEvents::find_in`）を本コンテキストに**残す**と定めている。順序付けは「集約に置けない横断の
//! 判断」であって描画ではない。シャードの列挙とファイル読取（I/O）は投影側が担い、ここへは
//! 連結済みのバッファが渡ってくる。
//!
//! # 通常読取は決して fail-closed しない
//!
//! 同一秒のタイを「順序不明」として拒否するのは authority 比較（`humanActedSinceGate` —
//! orchestration の述語、B9）の側の話である。本サービスは順序を**必ず 1 つ返す**。

use core::fmt;

use super::audit_events::EventType;

/// ブロックの区切り（upstream の読み手はここで割る）。
const BLOCK_SEPARATOR: &str = "\n---\n";
/// タイムスタンプ行の接頭辞。
const TIMESTAMP_PREFIX: &str = "**Timestamp**: ";
/// イベント行の接頭辞。
const EVENT_PREFIX: &str = "**Event**: ";

/// 台帳から読み取ったイベント 1 件（位置つき）。
///
/// フィールドは private。順序の材料（タイムスタンプと位置）を外から書き換えられると、
/// 並びの意味が壊れるためである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEventRecord {
    timestamp: String,
    event: EventType,
    position: usize,
}

impl AuditEventRecord {
    /// 行が名乗っていた秒精度 ISO タイムスタンプ（逐語 — 解釈しない）。
    #[must_use]
    pub fn timestamp(&self) -> &str {
        &self.timestamp
    }

    /// イベント型（閉集合の 86 語）。
    #[must_use]
    pub const fn event(&self) -> EventType {
        self.event
    }

    /// 連結バッファ内でのブロックの位置（0 始まり）。タイを破る材料である。
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }
}

impl fmt::Display for AuditEventRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} @{}",
            self.timestamp,
            self.event.as_str(),
            self.position
        )
    }
}

/// 順序付きのイベント列（W15 の E1 装置）。
///
/// **外から構築できず、並べ替えもできない**。[`OrderedAuditEvents::find_in`] を通ったものだけがこの型に
/// なるので、「順序規則を通っていない列」を順序付きとして扱う経路が存在しない。読み手が
/// 手元で `sort` を掛け直して規則を上書きすることもできない（`Vec` を返さない理由）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedAuditEvents(Vec<AuditEventRecord>);

impl OrderedAuditEvents {
    /// 古い順に走査する。
    pub fn iter(&self) -> impl Iterator<Item = &AuditEventRecord> {
        self.0.iter()
    }

    /// 最新のイベント（空なら `None`）。
    ///
    /// 「連結バッファの末尾」ではない — 辞書順で後ろのシャードにある**より古い**イベントを
    /// 拾わないために、順序規則を通したうえでの最後である。
    #[must_use]
    pub fn latest(&self) -> Option<&AuditEventRecord> {
        self.0.last()
    }

    /// 読み取れたイベントの件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// 1 件も無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 連結済みの台帳バッファから、順序規則を適用してイベントを読み取る
    /// （唯一の構築子 — upstream の `findAllEvents`（03 §6.4）に対応。2026-08-29 是正で
    /// 独立ドメインサービスから本型の関連関数へ — 型が自分の構築規則を所有する。
    /// 同型の先例は `ResolvedPlan::find_in`）。
    ///
    /// 順序は **timestamp 昇順、同値はバッファ位置**（`readAllAuditShards` がファイル名順で
    /// 連結しているので、位置はシャード名順 × シャード内追記順になる）。安定ソートなので、
    /// 同一 timestamp の相対順は読み取り順のまま残る。
    ///
    /// タイムスタンプ行かイベント行を欠くブロック、閉集合外のイベント名を名乗るブロックは
    /// **黙って読み飛ばす** — 通常読取は fail-closed しないという規約（03 §6.4）であり、
    /// 生きている台帳を 1 行の破損で読めなくしないためである。
    #[must_use]
    pub fn find_in(buffer: &str) -> OrderedAuditEvents {
        let mut records: Vec<AuditEventRecord> = buffer
            .split(BLOCK_SEPARATOR)
            .enumerate()
            .filter_map(|(position, block)| record_of(block, position))
            .collect();
        records.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
        OrderedAuditEvents(records)
    }
}

/// ブロック 1 つから順序の材料を取り出す（欠落・閉集合外は `None`）。
fn record_of(block: &str, position: usize) -> Option<AuditEventRecord> {
    let mut timestamp = None;
    let mut event = None;
    for line in block.lines() {
        if timestamp.is_none()
            && let Some(value) = line.strip_prefix(TIMESTAMP_PREFIX)
        {
            timestamp = Some(value.to_string());
        }
        if event.is_none()
            && let Some(value) = line.strip_prefix(EVENT_PREFIX)
        {
            event = EventType::parse(value);
        }
    }
    Some(AuditEventRecord {
        timestamp: timestamp?,
        event: event?,
        position,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(timestamp: &str, event: &str) -> String {
        format!("\n## H\n**Timestamp**: {timestamp}\n**Event**: {event}\n")
    }

    fn ledger(blocks: &[String]) -> String {
        format!("{}{BLOCK_SEPARATOR}", blocks.join(BLOCK_SEPARATOR))
    }

    fn names(ordered: &OrderedAuditEvents) -> Vec<&'static str> {
        ordered.iter().map(|r| r.event().as_str()).collect()
    }

    #[test]
    fn events_come_back_in_timestamp_order_not_buffer_order() {
        // 辞書順で後ろのシャードに古いイベントがいる、という素朴読みが壊れる形。
        let buffer = ledger(&[
            block("2026-08-21T09:00:05Z", "STAGE_COMPLETED"),
            block("2026-08-21T09:00:01Z", "HUMAN_TURN"),
        ]);
        let ordered = OrderedAuditEvents::find_in(&buffer);
        assert_eq!(names(&ordered), ["HUMAN_TURN", "STAGE_COMPLETED"]);
        assert_eq!(
            ordered.latest().map(AuditEventRecord::event),
            Some(EventType::StageCompleted),
            "最新はバッファ末尾ではなく時刻順の最後"
        );
    }

    #[test]
    fn the_same_second_is_broken_by_buffer_position() {
        let buffer = ledger(&[
            block("2026-08-21T09:00:00Z", "HUMAN_TURN"),
            block("2026-08-21T09:00:00Z", "GATE_APPROVED"),
            block("2026-08-21T09:00:00Z", "STAGE_COMPLETED"),
        ]);
        let ordered = OrderedAuditEvents::find_in(&buffer);
        assert_eq!(
            names(&ordered),
            ["HUMAN_TURN", "GATE_APPROVED", "STAGE_COMPLETED"],
            "同一秒は読み取り順のまま"
        );
        assert_eq!(
            ordered
                .iter()
                .map(AuditEventRecord::position)
                .collect::<Vec<_>>(),
            [0, 1, 2]
        );
    }

    #[test]
    fn a_tie_across_shards_still_yields_one_order() {
        // 通常読取は fail-closed しない（authority 比較の同秒 fail-closed は orchestration 側）。
        let first_shard = ledger(&[block("2026-08-21T09:00:00Z", "GATE_APPROVED")]);
        let second_shard = ledger(&[block("2026-08-21T09:00:00Z", "HUMAN_TURN")]);
        let ordered = OrderedAuditEvents::find_in(&format!("{first_shard}\n{second_shard}"));
        assert_eq!(names(&ordered), ["GATE_APPROVED", "HUMAN_TURN"]);
    }

    #[test]
    fn a_block_missing_its_materials_is_skipped_rather_than_failing_the_read() {
        let buffer = ledger(&[
            "\n## H\n**Timestamp**: 2026-08-21T09:00:00Z\n".to_string(), // Event 行が無い
            "\n## H\n**Event**: HUMAN_TURN\n".to_string(),               // Timestamp 行が無い
            block("2026-08-21T09:00:01Z", "GATE_APPROVED"),
        ]);
        let ordered = OrderedAuditEvents::find_in(&buffer);
        assert_eq!(names(&ordered), ["GATE_APPROVED"]);
        assert_eq!(ordered.len(), 1);
    }

    #[test]
    fn an_event_name_outside_the_closed_set_is_skipped() {
        let buffer = ledger(&[
            block("2026-08-21T09:00:00Z", "NOT_A_REAL_EVENT"),
            block("2026-08-21T09:00:01Z", "HUMAN_TURN"),
        ]);
        assert_eq!(names(&OrderedAuditEvents::find_in(&buffer)), ["HUMAN_TURN"]);
    }

    #[test]
    fn a_second_timestamp_line_inside_a_block_cannot_shadow_the_first() {
        // 描き手は自分の Timestamp を最初に書き、どの読み手も最初の一致を採る。
        let buffer = ledger(&[
            "\n## H\n**Timestamp**: 2026-08-21T09:00:00Z\n**Event**: HUMAN_TURN\n\
             **Timestamp**: 1999-01-01T00:00:00Z\n"
                .to_string(),
        ]);
        let ordered = OrderedAuditEvents::find_in(&buffer);
        assert_eq!(
            ordered.latest().map(AuditEventRecord::timestamp),
            Some("2026-08-21T09:00:00Z")
        );
    }

    #[test]
    fn an_empty_ledger_reads_as_empty() {
        let ordered = OrderedAuditEvents::find_in("");
        assert!(ordered.is_empty());
        assert_eq!(ordered.len(), 0);
        assert_eq!(ordered.latest(), None);
    }

    #[test]
    fn a_record_renders_its_material() {
        let buffer = ledger(&[block("2026-08-21T09:00:00Z", "HUMAN_TURN")]);
        let ordered = OrderedAuditEvents::find_in(&buffer);
        assert_eq!(
            ordered.latest().map(ToString::to_string).as_deref(),
            Some("2026-08-21T09:00:00Z HUMAN_TURN @0")
        );
    }
}
