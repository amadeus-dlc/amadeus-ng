//! 時計 — **横断機構の注入シームであって Gateway ではない** (clean-architecture: 時計は
//! Infrastructure が所有する機構。aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。
//!
//! どのユースケースもこの trait を消費しない。存在理由は、時刻に依存する Gateway の挙動
//! (イベント記録時刻の押印など) を、実時間の経過に頼らず決定的に検証できるようにすること
//! だけである。したがってアプリ境界のポートとして use-case 層には置かず、実装と同じ
//! アダプタ層に閉じ込める。
//!
//! 単位は `chrono::DateTime<Utc>` — ドメインイベントの `occurred_at` と集約の
//! `last_updated_at` が本家 event-store-adapter-rs の契約でこの型だからである (ADR-010)。
//! 自前の epoch ミリ秒と ISO 8601 整形はここで役目を終えた (NFR4.1 の再検討)。
//!
//! 実装は [`SystemClock`](crate::SystemClock)(実時計) と [`FakeClock`](crate::FakeClock)
//! (テスト用の制御可能な偽時計)。

use chrono::{DateTime, Utc};

/// 現在時刻の抽象。テストで fake を注入するための唯一の時刻源。
pub trait Clock {
    /// 現在の UTC 時刻。記録時刻の押印と経過時間の算出はこの値で行う。
    #[must_use]
    fn now(&self) -> DateTime<Utc>;
}
