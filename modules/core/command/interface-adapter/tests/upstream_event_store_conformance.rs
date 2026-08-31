//! 本家 event-store-adapter-rs v3.0.0 への適合テスト (ADR-010 Conformist / B7)。
//!
//! 型が揃うことは適合の証明にならない。**本家のイベントストアにこの層の永続化 DTO を
//! 実際に永続化し、スナップショット + リプレイでドメインへ復元できる**ところまでを固定する。
//!
//! # 両バックエンドへ拡張した理由 (オーナー監査 2026-08-31)
//!
//! このファイルは長らく memory バックエンド (`EventStoreForMemory`) だけで走っており、
//! **実 SQLite (`EventStoreForSqlite`) を 1 度も通していなかった**。オーナー監査
//! 2026-08-31「リポジトリのテストは必ず `EventStoreForSqlite` を実際に通したかチェックしろ。
//! 全部だ」でこれは違反と確定した — 本家の意味論はバックエンドごとに別実装なので、memory が
//! 緑でも SQLite が緑である保証はどこにも無い。そこで各検証を試験装置 ([`StoreFixture`]) 越しの
//! ジェネリック関数へ切り出し、**同一の適合検証を memory と SQLite の両方で走らせる**形にした
//! (`intent_repository_contract.rs` と同じ作法)。
//!
//! **B12 改訂 9 で検証対象が adapter の面へ移った**ため、このテストも domain からこの層へ移設した。ドメインは永続化知識から中立になり
//! (`coding-rules/domain-persistence-neutrality.md`)、本家ストアに載るのは DTO だからである。
//! したがって検証の主語も「ドメイン型が本家契約を満たす」から「**この層の DTO が本家契約を
//! 満たし、往復でドメインへ戻る**」へ変わった — 往復の両端でドメイン値が一致することを
//! 見るので、確認している性質そのものは落ちていない。
//!
//! v3 で `Event` / `Aggregate` trait は消え、ストアに載るのはライブラリ trait を一切実装しない
//! 素の serde 型になった。改訂 9 以降その型は**この層の DTO** であり、ドメイン型ではない。
//! 境界を越えるメタデータは `EventEnvelope` / `SnapshotEnvelope` が運ぶ。
//! ここで固定する本家の意味論は次の 4 つである:
//!
//! - `persist_event`（イベントのみ）は `seq_nr == 1` を**受理しない** — 新規作成は
//!   `persist_event_and_snapshot` である (`ContractViolation`)
//! - `persist_event_and_snapshot` は `seq_nr == 1` ⇔ `expected_version == 0` の対応を要求し、
//!   崩れた呼出しを `ContractViolation` で拒否する
//! - `get_events_by_id_since_seq_nr` の `seq_nr` は**その番号を含む**
//! - 楽観 version はストアが採番し、`SnapshotEnvelope::version()` が正本である
//!   (集約 payload には載らない)

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{DateTime, TimeDelta, Utc};
use core_command_domain::orchestration::{
    Created, Intent, IntentExecution, IntentExecutionEvent, IntentExecutionId, IntentId,
    StageDisplay, StageEntry, StartRequest, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};
use core_command_domain::workspace::{CheckboxState, SpaceName, StorePath};
use core_command_interface_adapter::orchestration::{
    IntentExecutionAggregateKeyDto, IntentExecutionDto, IntentExecutionEventDto,
    IntentExecutionMemoryStore, IntentExecutionSqliteStore,
};
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::{EventStore, EventStoreWriteError};
use tempfile::TempDir;

/// 我々が封筒に載せる型判別子 (Repository が書く値と同じ綴り)。
const MANIFEST: &str = "intent-execution-event/1";

/// 新規作成の `expected_version` (本家 v3 の規約 — BR2.6)。
const CREATE_EXPECTED_VERSION: usize = 0;

const RAW_ID: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

/// 適合テストが試験対象のバックエンドを開くための唯一の抽象。
///
/// 本家のイベントストアは memory / SQLite で別実装なので、意味論の適合も**別々に**確かめる
/// 必要がある。バックエンドごとの差 (揮発か実ファイルか) はこの試験装置に閉じ、適合検証
/// そのものは [`conformance`] のジェネリック関数が持つ。
trait StoreFixture {
    /// 試験対象のストア (型引数は v3 の並びで `AID` / `A` / `P`。違うのは格納先だけ)。
    type Store: EventStore<
            AID = IntentExecutionAggregateKeyDto,
            A = IntentExecutionDto,
            P = IntentExecutionEventDto,
        >;

    /// **空のストア**を開く (呼ぶたびに独立した空のストア)。
    fn open(&self) -> Self::Store;
}

/// 本家 memory バックエンドの試験装置。
struct MemoryFixture;

impl StoreFixture for MemoryFixture {
    type Store = IntentExecutionMemoryStore;

    fn open(&self) -> IntentExecutionMemoryStore {
        IntentExecutionMemoryStore::new()
    }
}

/// 呼ぶたびに**別の SQLite ファイル**へストアを開く試験装置。
struct SqliteFixture {
    /// 一時ディレクトリは試験装置が生きているあいだ保持する (drop で配下ごと消える)。
    root: TempDir,
}

impl SqliteFixture {
    fn new() -> SqliteFixture {
        SqliteFixture {
            root: tempfile::tempdir().expect("一時ディレクトリ"),
        }
    }

    /// まだ何も置かれていない場所を用意する (`intents/` は upstream の既存ディレクトリ —
    /// ストアは作らない)。
    fn fresh_path(&self) -> StorePath {
        let workspace = tempfile::Builder::new()
            .prefix("workspace-")
            .tempdir_in(self.root.path())
            .expect("open ごとの一時ディレクトリ")
            .keep();
        let path = StorePath::for_space(&workspace.join("aidlc"), &SpaceName::default());
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir を持つ"))
            .expect("intents/ を先に作る");
        path
    }
}

impl StoreFixture for SqliteFixture {
    type Store = IntentExecutionSqliteStore;

    fn open(&self) -> IntentExecutionSqliteStore {
        IntentExecutionSqliteStore::new(self.fresh_path().as_path()).expect("ストアは開ける")
    }
}

fn intent_id() -> IntentId {
    IntentId::parse(RAW_ID).unwrap()
}

fn at(offset_secs: i64) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-08-23T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
        + TimeDelta::seconds(offset_secs)
}

fn slug(s: &str) -> StageSlug {
    StageSlug::parse(s).unwrap()
}

/// 合成計画の表示属性。
fn display(number: &str, name: &str) -> StageDisplay {
    StageDisplay::new(StageNumber::parse(number).unwrap(), name, "orchestrator").unwrap()
}

/// 合成計画の走査結果。
fn scan() -> WorkspaceScan {
    WorkspaceScan::new(
        BrownfieldGreenfield::Greenfield,
        "Unknown",
        "Unknown",
        "Unknown",
    )
    .unwrap()
}

/// initialization 1 ステージ + ゲート付き 2 ステージの合成計画 (BR2.5 と同じ流儀)。
fn stages() -> Vec<StageEntry> {
    vec![
        StageEntry::new(
            slug("state-init"),
            PhaseId::Initialization,
            PlanAction::Execute,
            false,
            display("0.1", "State Init"),
        ),
        StageEntry::new(
            slug("intent-capture"),
            PhaseId::Ideation,
            PlanAction::Execute,
            false,
            display("1.1", "Intent Capture"),
        ),
        StageEntry::new(
            slug("requirements-analysis"),
            PhaseId::Inception,
            PlanAction::Execute,
            false,
            display("2.1", "Requirements Analysis"),
        ),
    ]
}

fn intent() -> Intent {
    Intent::from(Created::new(
        intent_id(),
        WorkflowDefinitionId::parse("claude").unwrap(),
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
        StartRequest::new("mvp", "本家ストアへの適合を確かめる"),
        stages(),
        scan(),
    ))
}

fn execution_id() -> IntentExecutionId {
    IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap()
}

fn genesis() -> (IntentExecution, IntentExecutionEvent) {
    IntentExecution::start(execution_id(), &intent(), at(0))
}

/// commit 済みの集約とイベントから封筒を組む (Repository と同じ手順 — B7 裁定 3)。
///
/// 通番も発生時刻も**適用後の集約**が持っている。ドメインは封筒を作らない。
fn envelope(
    aggregate: &IntentExecution,
    event: &IntentExecutionEvent,
) -> EventEnvelope<IntentExecutionAggregateKeyDto, IntentExecutionEventDto> {
    EventEnvelope::new(
        IntentExecutionAggregateKeyDto::of(aggregate.id()),
        aggregate.seq_nr(),
        *aggregate.last_updated_at(),
        IntentExecutionEventDto::of(event),
    )
    .with_manifest(MANIFEST)
}

/// 集約をストアへ載せる形 (スナップショット行の payload)。
fn snapshot_of(aggregate: &IntentExecution) -> IntentExecutionDto {
    IntentExecutionDto::of(aggregate)
}

/// 再水和の結果 (本家 v3 の移行ガイド §3 と同じ形 — 集約 + ストアが載せた版)。
struct Replayed {
    aggregate: IntentExecution,
    version: usize,
}

/// 再構成 — 本家 example (`user_account_repository.rs`) と同型: スナップショット封筒
/// (版の正本 + ある時点の集約) を基底に、その通番より後のイベントを差分再生する
/// (オーナー裁定 2026-08-30)。
///
/// バックエンドを型引数に取るので、memory と SQLite に**同一の手順**を課せる。
async fn find_by_id<S>(store: &S, id: &IntentExecutionId) -> Option<Replayed>
where
    S: EventStore<
            AID = IntentExecutionAggregateKeyDto,
            A = IntentExecutionDto,
            P = IntentExecutionEventDto,
        >,
{
    let key = IntentExecutionAggregateKeyDto::of(id);
    let snapshot = store.get_latest_snapshot_by_id(&key).await.unwrap()?;
    let version = snapshot.version();
    let base = snapshot.aggregate().to_domain().unwrap();
    let envelopes = store
        .get_events_by_id_since_seq_nr(&key, base.seq_nr() + 1)
        .await
        .unwrap();
    let mut events = Vec::with_capacity(envelopes.len());
    for envelope in &envelopes {
        assert_eq!(envelope.manifest(), MANIFEST, "manifest は往復する");
        let event = envelope.payload().to_domain().unwrap();
        events.push((envelope.seq_nr(), *envelope.occurred_at(), event));
    }
    Some(Replayed {
        aggregate: IntentExecution::replay(base, events).with_version(version),
        version,
    })
}

/// 適合検証の本体 — **バックエンドを問わない**。試験装置が開いたストアに同じ約束を課す。
mod conformance {
    use super::{
        CREATE_EXPECTED_VERSION, CheckboxState, EventEnvelope, EventStoreWriteError,
        IntentExecutionAggregateKeyDto, MANIFEST, StoreFixture, at, envelope, execution_id,
        find_by_id, genesis, intent, snapshot_of,
    };
    use event_store_adapter_rs::types::EventStore;

    pub(crate) async fn an_unknown_aggregate_has_no_snapshot<F: StoreFixture>(fixture: &F) {
        let store = fixture.open();
        assert!(find_by_id(&store, &execution_id()).await.is_none());
    }

    pub(crate) async fn the_creation_event_is_refused_by_the_event_only_write<F: StoreFixture>(
        fixture: &F,
    ) {
        // v3 は `is_created()` を持たず、create / update の分岐を `seq_nr == 1` から導出する。
        // イベントのみの書込 API は新規作成を受け付けない (BR2.2)。
        let mut store = fixture.open();
        let (aggregate, started) = genesis();
        let error = store
            .persist_event(envelope(&aggregate, &started), CREATE_EXPECTED_VERSION)
            .await
            .expect_err("seq_nr == 1 は persist_event では書けない");
        assert!(
            matches!(error, EventStoreWriteError::ContractViolation(_)),
            "実際: {error:?}"
        );
    }

    pub(crate) async fn a_creation_with_a_non_zero_expected_version_violates_the_contract<
        F: StoreFixture,
    >(
        fixture: &F,
    ) {
        // BR2.6: seq_nr == 1 ⇔ expected_version == 0。対応が崩れる呼出しは契約違反である。
        let mut store = fixture.open();
        let (aggregate, started) = genesis();
        let error = store
            .persist_event_and_snapshot(envelope(&aggregate, &started), snapshot_of(&aggregate), 1)
            .await
            .expect_err("seq_nr == 1 に版 1 は対応しない");
        assert!(
            matches!(error, EventStoreWriteError::ContractViolation(_)),
            "実際: {error:?}"
        );
    }

    pub(crate) async fn the_aggregate_survives_a_snapshot_and_replay_round_trip_through_the_upstream_store<
        F: StoreFixture,
    >(
        fixture: &F,
    ) {
        let mut store = fixture.open();

        // 1. genesis — `seq_nr == 1` なので create 経路へ入る (expected_version = 0)。
        let (aggregate, started) = genesis();
        store
            .persist_event_and_snapshot(
                envelope(&aggregate, &started),
                snapshot_of(&aggregate),
                CREATE_EXPECTED_VERSION,
            )
            .await
            .unwrap();

        let restored = find_by_id(&store, &execution_id()).await.unwrap();
        assert_eq!(
            restored.aggregate,
            aggregate.with_version(1),
            "genesis がそのまま戻る (版だけストアが刻む)"
        );
        assert_eq!(restored.aggregate.seq_nr(), 1);
        assert_eq!(restored.aggregate.last_updated_at(), &at(0));
        assert_eq!(restored.version, 1, "最初の版はストアが 1 で採番する");

        // 2. スナップショット同時更新 — 本家が version を 1 つ進める。
        let mut aggregate = restored.aggregate;
        let completed = aggregate.complete_stage(&intent(), at(1)).unwrap();
        store
            .persist_event_and_snapshot(
                envelope(&aggregate, &completed),
                snapshot_of(&aggregate),
                restored.version,
            )
            .await
            .unwrap();

        let restored = find_by_id(&store, &execution_id()).await.unwrap();
        assert_eq!(restored.aggregate.seq_nr(), 2);
        assert_eq!(restored.version, 2, "楽観 version はストアが採番する");
        assert_eq!(
            restored.aggregate.cursor(),
            restored.aggregate.stage_index(1).unwrap()
        );
        assert_eq!(restored.aggregate.last_updated_at(), &at(1));

        // 3. ジャーナルだけへの追記 — スナップショットは進まないので、復元はリプレイを通る。
        let mut aggregate = restored.aggregate;
        let opened = aggregate
            .open_gate(&intent(), vec!["docs/x.md".to_string()], at(2))
            .unwrap();
        store
            .persist_event(envelope(&aggregate, &opened), restored.version)
            .await
            .unwrap();

        let restored = find_by_id(&store, &execution_id()).await.unwrap();
        assert_eq!(
            restored.aggregate.seq_nr(),
            3,
            "スナップショットの先をリプレイで追いつく"
        );
        assert_eq!(
            restored
                .aggregate
                .checkbox(restored.aggregate.stage_index(1).unwrap()),
            Some(CheckboxState::AwaitingApproval)
        );
        assert_eq!(restored.aggregate.last_updated_at(), &at(2));
        assert_eq!(restored.version, 3, "イベントのみの書込でも版は進む");

        // 4. もう 1 度スナップショット同時更新 — 直前のリプレイ結果から続けられる。
        let mut aggregate = restored.aggregate;
        let approved = aggregate.approve_gate(&intent(), None, at(3)).unwrap();
        store
            .persist_event_and_snapshot(
                envelope(&aggregate, &approved),
                snapshot_of(&aggregate),
                restored.version,
            )
            .await
            .unwrap();

        let restored = find_by_id(&store, &execution_id()).await.unwrap();
        assert_eq!(restored.aggregate.seq_nr(), 4);
        assert_eq!(restored.version, 4, "版は書込のたびにストアが 1 つ進める");
        assert_eq!(
            restored.aggregate.cursor(),
            restored.aggregate.stage_index(2).unwrap()
        );
        assert_eq!(
            restored
                .aggregate
                .checkbox(restored.aggregate.stage_index(1).unwrap()),
            Some(CheckboxState::Completed)
        );
        assert_eq!(
            restored
                .aggregate
                .approved(restored.aggregate.stage_index(1).unwrap()),
            Some(true)
        );
        assert_eq!(restored.aggregate.last_updated_at(), &at(3));
    }

    pub(crate) async fn the_journal_keeps_every_envelope_in_order_with_its_metadata<
        F: StoreFixture,
    >(
        fixture: &F,
    ) {
        let mut store = fixture.open();
        let id = execution_id();

        let (mut aggregate, started) = genesis();
        store
            .persist_event_and_snapshot(
                envelope(&aggregate, &started),
                snapshot_of(&aggregate),
                CREATE_EXPECTED_VERSION,
            )
            .await
            .unwrap();
        let completed = aggregate.complete_stage(&intent(), at(1)).unwrap();
        store
            .persist_event_and_snapshot(
                envelope(&aggregate, &completed),
                snapshot_of(&aggregate),
                1,
            )
            .await
            .unwrap();

        // `seq_nr` 引数はその番号を含む (本家 `fetch_events_since` は `>=`)。
        let events = store
            .get_events_by_id_since_seq_nr(&IntentExecutionAggregateKeyDto::of(&id), 1)
            .await
            .unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(
            events.iter().map(EventEnvelope::seq_nr).collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(
            events
                .iter()
                .map(EventEnvelope::occurred_at)
                .collect::<Vec<_>>(),
            [&at(0), &at(1)],
            "発生時刻はドメイン供給値のまま往復する"
        );
        assert!(
            events
                .iter()
                .all(|envelope| envelope.manifest() == MANIFEST),
            "manifest は封筒がそのまま運ぶ"
        );
        assert!(
            events
                .iter()
                .all(|envelope| envelope.aggregate_id() == &IntentExecutionAggregateKeyDto::of(&id))
        );

        let tail = store
            .get_events_by_id_since_seq_nr(&IntentExecutionAggregateKeyDto::of(&id), 2)
            .await
            .unwrap();
        assert_eq!(
            tail.iter().map(EventEnvelope::seq_nr).collect::<Vec<_>>(),
            [2]
        );
    }
}

/// 1 つの適合検証を **memory / SQLite の両バックエンド**へ展開する。
///
/// 生成されるのは `<検証名>::memory` と `<検証名>::sqlite` の対で、テスト名にバックエンドが
/// 現れるので、どちらで落ちたかが実行結果からそのまま読める。
macro_rules! conformance_tests {
    ($($name:ident),* $(,)?) => {
        $(
            mod $name {
                use super::{MemoryFixture, SqliteFixture, conformance};

                #[tokio::test]
                async fn memory() {
                    conformance::$name(&MemoryFixture).await;
                }

                #[tokio::test]
                async fn sqlite() {
                    conformance::$name(&SqliteFixture::new()).await;
                }
            }
        )*
    };
}

conformance_tests!(
    an_unknown_aggregate_has_no_snapshot,
    the_creation_event_is_refused_by_the_event_only_write,
    a_creation_with_a_non_zero_expected_version_violates_the_contract,
    the_aggregate_survives_a_snapshot_and_replay_round_trip_through_the_upstream_store,
    the_journal_keeps_every_envelope_in_order_with_its_metadata,
);
