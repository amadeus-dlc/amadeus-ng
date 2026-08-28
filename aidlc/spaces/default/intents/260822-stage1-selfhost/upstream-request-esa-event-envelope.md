# event-store-adapter-rs への要望 — EventEnvelope の導入（v3 提案）

> **結果（2026-08-29 追記）**: 本家 v3.0.0（2026-08-28 リリース）が本要望の方向で実装・
> リリースされた（`Event` / `Aggregate` trait を廃し `EventEnvelope<AID, P>` /
> `SnapshotEnvelope<A>` を導入 — 下記「本家が決めるべき設計論点」の 4 点すべてに回答する形）。
> amadeus-ng は B7（`bolt/b7-esa-v3-event-envelope`）で `=3.0.0` へ乗り換え済み。詳細は
> [`developer-report-1.md`](construction/esa-v3-migration/developer-report-1.md) と
> [`decisions.md` ADR-010 2026-08-29 追記](inception/domain-design/decisions.md) を参照。
>
> amadeus-ng（ADR-010 で v2.0.0 へ Conformist 乗り換え済み）からの設計改善要望。
> 本家の新 intent の初期記述としてそのまま貼れる形にしてある。**最終設計は本家の
> intent（inception/construction）で決めるべきもの**で、以下の形はあくまで提案。
> 根拠はすべて実測（pekko ソース確認・amadeus-ng の乗り換えで実際に踏んだ摩擦）。

## 一文要約

**メタデータと EventPayload を分離した `EventEnvelope` をストアの型として導入し、
ドメインイベントへの trait 要求を最小化してほしい。** 破壊的変更（v3 級）になるが、
現行 `Event` trait の構造的な問題を根本から解消できる。

## 現行設計の問題 — メタデータがドメインイベントに癒着している

v2.0.0 の `Event` trait は、メタデータのアクセサ（`id` / `aggregate_id` / `seq_nr` /
`occurred_at` / `is_created`）と直列化境界（`Serialize + Deserialize`）を**ドメインイベント型
自身に**要求する。その帰結として、利用側は必然的に「封筒 + payload enum」を手作りする
（amadeus-ng の実例: `WorkflowExecutionEvent { id, schema_version, occurred_at, payload }` —
コメントで文字どおり「封筒」と呼んでいる）。

利用側（amadeus-ng の乗り換え Bolt）で実際に踏んだ摩擦 5 点:

1. **メタデータの二重化と照合義務** — 封筒ごと serde で journal の payload 列に書かれるため、
   `aid` / `seq_nr` が**列と payload JSON の両方**に存在する。レビューで「行と復号イベントの
   照合が無い」と指摘され、照合コードを足すことになった。envelope 設計なら列だけが正で、
   照合そのものが不要。
2. **`is_created()` の押し付け** — 「genesis かどうか」はスナップショット行の create/update
   ルーティングという**バックエンド実装詳細**で、ドメインの関心ではない。
3. **イベント ID の発明** — `Event::ID` の要求のため `(aggregate_id, seq_nr)` の組を包む
   ID 型を新設したが、これは導出可能なメタデータであってドメインが持つ理由のない型。
4. **serde / chrono のドメイン侵入** — trait がイベント型全体に `Serialize` /
   `DateTime<Utc>` を要求するため、ドメイン層に serde と chrono が入る。
5. **読取側の再発明** — 横断読取が `(カーソル, イベント)` のタプルを返す形になり、
   これは envelope の貧者版。

## 先行設計の実測 — pekko-persistence `PersistentRepr`

`apache/pekko` の `persistence/src/main/scala/org/apache/pekko/persistence/Persistent.scala`
（実測）:

```scala
final case class PersistentImpl(
    payload: Any,              // ドメインイベント。trait 境界なし
    sequenceNr: Long,          // 封筒が識別を持つ
    persistenceId: String,
    manifest: String,          // スキーマ進化の口（event adapter manifest）
    deleted: Boolean,          // deprecated 方向
    sender: ActorRef,          // deprecated 方向
    writerUuid: String,        // 書き手重複によるリプレイ不整合の検知
    timestamp: Long,           // 「格納時刻」— journal が刻印
    metadata: Option[Any])     // 拡張スロット（Replicated ES 等が使う）
```

要点: **`payload: Any` — ドメインイベントはライブラリの trait を 1 つも実装しない**。
直列化は manifest 経由でシリアライザ基盤の仕事。`is_created` 相当は存在しない
（journal は sequenceNr 範囲でリカバリするため不要）。

## 提案の骨子（たたき台 — 設計は本家に委ねる）

```rust
/// ストアが所有する封筒。メタデータの運搬・直列化はストアの仕事。
pub struct EventEnvelope<P> {
    aggregate_id: String,       // 型付けの方法は本家判断（AggregateId trait 維持でも可）
    seq_nr: usize,
    occurred_at: DateTime<Utc>, // または「格納時刻」— 意味論は下記論点 2
    manifest: String,           // スキーマ進化の口（バージョン付け方式は本家判断）
    payload: P,                 // ドメインイベント本体
}

// ドメインイベントへの要求を最小化:
//   P: Serialize + DeserializeOwned + Send + Sync + 'static  程度まで下げる
// is_created は封筒の seq_nr == 1 から導出（trait メソッドを廃止）
```

- `EventStore` の API は封筒を受け渡す形へ（`persist(envelope)` / 読取は封筒を返す）
- ストレージ列とメタデータが 1:1 になり、payload 列にはドメインの中身だけが入る
- 検討に値する pekko 由来の追加要素（必須ではない）: `writerUuid`（重複書き手の検知 —
  楽観 version とは別の防御）、`metadata: Option<...>`（拡張スロット）

## 本家が決めるべき設計論点

1. **`seq_nr` の所有** — pekko は永続化層が採番する。amadeus-ng は「ドメインが数え、封筒が
   運ぶ」（集約 = FSM で seq_nr はイベント適用回数）。ライブラリとしてどちらを既定にするか、
   両対応にするか。
2. **時刻の意味論** — pekko の `timestamp` は「格納時刻」で journal が刻印する。
   「発生時刻」をドメインが渡す設計とどちらを採るか（両方持つ選択肢もある）。
3. **`Aggregate` trait（スナップショット側）** — 同じ癒着（serde / chrono / メタデータ
   アクセサの要求）がスナップショット側にもある。envelope 化を journal だけにするか、
   snapshot 側（`SnapshotEnvelope` は既に存在する）も揃えるか。
4. **移行** — 4 バックエンド（DynamoDB / Bigtable / SQLite / memory)の格納形式が変わる。
   v3 の破壊的変更として一括か、段階的か。

## 期待する効果

- 利用側のドメイン層から serde / chrono / ライブラリ trait への依存が消える
  （Conformist の対価が「ほぼゼロ」になる）
- メタデータ二重化と照合義務の解消
- `is_created` / イベント ID 型の発明といった「バックエンド都合のドメイン侵入」の解消
- スキーマ進化の口（manifest）がストア側に定位置を持つ

## 参考資料

- pekko: `apache/pekko` `persistence/.../Persistent.scala`（本文書の実測元）
- amadeus-ng の乗り換え実測: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/esa-v2-migration/developer-report-1.md`（trait 実測・§7 設計質問）/ `developer-report-2.md`（照合コードの追加経緯 §3.3、CodeRabbit #500/#466）
- amadeus-ng ADR-010（Conformist 採用の経緯と対価の記録）
