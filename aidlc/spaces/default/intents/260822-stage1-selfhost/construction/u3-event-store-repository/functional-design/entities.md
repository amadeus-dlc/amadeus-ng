# entities — U3 イベントストアと IntentExecutionRepository

> 2026-09-05 是正。現行の正本は下の YAML。末尾の Review は当時の記録として保存し、現在の署名や保証を定めるものとは扱わない。
> 出典: `../../../inception/units-generation/unit-of-work.md`、`../../../inception/units-generation/unit-of-work-story-map.md`、
> `../../../inception/requirements-analysis/requirements.md`（FR1 / FR1.2 / FR1.3 / NFR3）、
> `../../../inception/contract-design/contract-summary.md`（C3 / C6、B8・B13 追記）、
> `../../../inception/domain-design/decisions.md`（ADR-001 / ADR-007 / ADR-010）、
> `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` の gateway-taxonomy / upstream-contracts / domain-persistence-neutrality / error-handling / cqrs-boundaries。
> 最新スナップショットとそれ以降の差分イベント集合でリプレイする、という確定方針を適用する。

## 1. エンティティ（正本）

```yaml
entities:
  - name: IntentExecutionRepository
    kind: port-trait
    layer: core-command-use-case
    description: "IntentExecution を自集約の IntentExecutionId で検索し、イベントと適用後集約を永続化する。ポートは静的束縛の async fn。Send / Sync は要求しない"
    attributes:
      - { name: find_by_id, type: "async fn(&self, &IntentExecutionId) -> Result<IntentExecution, RepositoryError<IntentExecutionId>>", required: true }
      - { name: store, type: "async fn(&mut self, &IntentExecutionEvent, &IntentExecution) -> Result<(), RepositoryError<IntentExecutionId>>", required: true }
    constraints:
      - "版は返却する集約が保持する。RehydratedWorkflowExecution / RehydratedIntentExecution や裸の expected_version 引数は公開しない"
      - "呼出側は同じコマンドが返したイベントとその適用後集約を渡す。独立した参照引数なので、この対応は型だけでは保証されない"
      - "実装は保存前に event.aggregate_id() と aggregate.id() を照合し、不一致を Corrupt として I/O 前に拒否する"
      - "store は借用した集約の版を更新しない。続けてコマンドを保存するなら find_by_id で再取得する"

  - name: RepositoryError
    kind: error-enum
    layer: core-command-use-case
    description: "Repository 共通の RepositoryError<Id>。Display は材料のみ、Corrupt の診断原因は Error::source で連鎖する"
    attributes:
      - { name: NotFound, type: "{ id: Id }", required: true }
      - { name: Conflict, type: "{ expected: usize, actual: usize }", required: true }
      - { name: Io, type: "{ kind: std::io::ErrorKind, path: Option<PathBuf> }", required: true }
      - { name: Corrupt, type: "{ id: Id, seq_nr: Option<usize>, source: Box<dyn Error + Send + Sync> }", required: true }
    constraints:
      - "書込側の CorruptCause は公開契約に存在しない。CorruptDetail / DtoDecodeError はアダプタの診断詳細"

  - name: IntentExecution
    kind: aggregate
    layer: core-command-domain
    description: "U2 所有。IntentExecutionId で識別し、Intent は IntentId で参照する。seq_nr、last_updated_at、読取済み version を保持する"
    constraints:
      - "version は usize の不透明トークン。seq_nr から導出・算術せず、ストアの行から読んだ値を with_version で保持する"
      - "再構成は検査付きコンストラクタと replay(base, delta) を使う。ドメインは serde / EventStore trait / 永続化 DTO を所有しない"

  - name: IntentExecutionEvent
    kind: domain-event
    layer: core-command-domain
    description: "U2 所有。イベント識別子と aggregate_id を持つ。輸送封筒の通番・発生時刻は適用後集約から得る"
    constraints:
      - "同じ Rust 型でも別実行のイベントは構成可能。保存時の ID 照合は実装の責務であり、型保証とは呼ばない"

  - name: IntentExecutionRepositoryImpl
    kind: gateway-impl
    layer: core-command-interface-adapter
    description: "本家 event-store-adapter-rs =3.0.0 のストアを型引数 S として単一所有。SQLite と memory は同じ Repository 手順を使う"
    attributes:
      - { name: store, type: "S: EventStore<AID = IntentExecutionAggregateKeyDto, A = IntentExecutionDto, P = IntentExecutionEventDto>", required: true }
      - { name: location, type: "Option<StorePath>", required: true }
      - { name: strategy, type: SnapshotStrategy, required: true }
    constraints:
      - "open は SQLite、in_memory は本家 memory を選ぶ。書込は &mut self、独自の内部可変性を追加しない"
      - "新規作成は seq_nr == 1 と expected_version == 0。本家が version を採番する。旧 FIRST_STORED_VERSION の写しへの注入は行わない"
      - "初回は必ず persist_event_and_snapshot。以後は設定間隔で同関数、それ以外は persist_event。Tx と CAS は本家が所有する"

  - name: SnapshotStrategy
    kind: configuration-value
    layer: core-command-interface-adapter
    description: "Repository 内部設定。every(NonZeroUsize) で指定し、既定は 10"
    constraints:
      - "seq_nr が間隔の倍数ならスナップショットを更新する。初回必須の分岐は Repository が担う"
      - "イベントのみ保存しても行の version は進むが、基底 payload と snapshot.seq_nr は更新しない"

  - name: IntentExecutionDto
    kind: persistence-dto
    layer: core-command-interface-adapter
    description: "ある時点の集約の永続化表現。正確な属性は modules/core/command/interface-adapter/src/orchestration/dto/intent_execution_dto.rs を参照"
    constraints:
      - "serde は DTO に閉じる。to_domain が検査付き再構成コンストラクタを通す。旧 domain memento / from_state 経路ではない"
      - "payload に楽観 version を保存しない。版の正本は SnapshotEnvelope::version()"
  - name: IntentExecutionEventDto
    kind: persistence-dto
    layer: core-command-interface-adapter
    description: "イベントの永続化表現。aggregate_id を含む payload を本家 EventEnvelope に載せる。manifest は intent-execution-event/1"
  - name: IntentExecutionAggregateKeyDto
    kind: persistence-dto
    layer: core-command-interface-adapter
    description: "本家ストアへ渡す集約キー。ドメイン ID をストア trait に直接結合しない"
  - name: StorePath
    kind: value-object
    layer: core-command-domain
    description: "workspace 所有。<aidlc root>/spaces/<space>/intents/.aidlc-store.sqlite を表す。親ディレクトリは open 時に作成しない"

  - name: JournalReader
    kind: port-trait
    layer: core-read-model-updater
    description: "U4 所有。全集約横断の読取と投影の公開・チェックポイントを担う。完全な署名は modules/core/read-model-updater/src/orchestration/journal_reader.rs が正本"
    attributes:
      - { name: events_after, type: "async fn(&self, GlobalSeqNr) -> Result<JournalBatch, JournalReadError>", required: true }
      - { name: checkpoint, type: "async fn(&self, &ProjectionName) -> Result<GlobalSeqNr, JournalReadError>", required: true }
      - { name: advance_checkpoint, type: "async fn(&mut self, &ProjectionName, GlobalSeqNr, &ReadTables) -> Result<(), JournalReadError>", required: true }
    constraints:
      - "上記は U3 との接点の抜粋。公開計画・復旧・steering の操作を含む全 API を U3 に再定義しない"
      - "Repository の差分再生と RMU の全履歴投影は別の用途。コマンド側から RMU に依存しない"
  - name: JournalReaderImpl
    kind: gateway-impl
    layer: core-read-model-updater
    description: "U4 所有。本家 journal を別接続から読み、投影の保存・チェックポイントを管理する"
  - name: JournalReadError
    kind: error-enum
    layer: core-read-model-updater
    description: "U4 所有。Io / Corrupt / CheckpointRegression。RMU の CorruptCause は RepositoryError の source 契約と別物"
  - name: GlobalSeqNr
    kind: value-object
    layer: core-read-model-updater
    description: "U4 所有。u64。追記専用 journal の rowid を横断カーソルとし、ZERO は未読"
  - name: ProjectionName
    kind: value-object
    layer: core-read-model-updater
    description: "U4 所有。投影識別子。検証済みの名前をチェックポイント操作へ渡す"

  - name: IntentId
    kind: value-object
    layer: core-command-domain
    description: "Intent 集約の UUIDv7。実行集約の IntentExecutionId とは別の型"
    constraints:
      - "小文字36字、UUID version 7 / variant 10xx。旧 kebab 識別子は受理しない"
  - name: IntentDirName
    kind: value-object
    layer: core-command-domain
    description: "記録ディレクトリ名。IntentId とは別のパスセグメント"
    constraints:
      - "正規表現 ^[0-9]{6}-[a-z0-9]+(?:-[a-z0-9]+)*$、全体64字以下"
      - "空区間、連続ハイフン、末尾ハイフンを拒否。正規化しない。予約ラベル拒否は生成側の責務"

  - name: JournalProtocolModel
    kind: quint-model
    layer: formal
    description: "formal/orchestration/journal_protocol.qnt。単一集約、writer 2、投影1。毎イベント更新（間隔1）の構成を検証する"
    attributes:
      - { name: vars, type: "journalLen / snapVersion / snapSeq / checkpoint / readModelSeq / loadedVersion / lastAction / lastActor / prev*", required: true }
      - { name: invariants, type: "conflict_rejected / snapshot_tracks_journal / version_equals_journal / checkpoint_monotone / checkpoint_bounded / projection_idempotent / truth_is_journal / no_lost_update", required: true }
    constraints:
      - "snapSeq == journalLen は間隔1限定。既定10の一般的な保証ではない"
      - "ITF再生先 modules/app/aidlc/tests/journal_protocol_conformance.rs が every(1) を明示する。既定10と任意Nの差分再生は別の実装テストで検証する"

  - name: RetiredLockMachinery
    kind: retirement-list
    layer: historical
    description: "ADR-007で退役したWorkspaceLock/FsWorkspaceLock/LockProtocol/LockIdentity/ProcessProbe/audit_lockと専用lint。現行の投影公開など別責務の排他とは区別する"

relationships:
  - { from: IntentExecutionRepositoryImpl, to: "本家 EventStoreForSqlite / EventStoreForMemory", description: "単一所有。Tx、CAS、journal / snapshot DDL は本家の責務" }
  - { from: IntentExecutionRepositoryImpl, to: IntentExecution, description: "DTO復元 → 最新基底以降の差分検査 → replay → 読取済み版を保持" }
  - { from: JournalReaderImpl, to: "本家 journal / U4の投影表", description: "横断読取と投影公開。U3の書込側DTOを共用せずU4専用DTOで復号" }
  - { from: JournalProtocolModel, to: "IntentExecutionRepositoryImpl + JournalReaderImpl", description: "app/aidlc の ITF 適合テスト。モデルの構成と毎回更新の設定を揃える" }
```

## 2. 要約と旧設計との対応

ポートは `core-command-use-case` の `IntentExecutionRepository`、実装と永続化 DTO は `core-command-interface-adapter` に置く。
版は集約そのものが運び、`store(event, aggregate)` はその版を期待値として渡す。
初回必須・既定10イベントごとの基底更新と、最新基底以降の差分再生を組み合わせる。
型だけでは別実行のイベント混入を防げないため、書込境界でも ID を照合する。

`JournalReader` 一式は U4 の RMU 所有であり、旧 use-case / adapter 配置表は失効した。
旧 `WorkflowExecutionRepository`、再水和専用の器、domain の serde-memento、自前 EventStore / SQL / ワイヤ型は現行設計へ戻さない。
旧 mkdir ロックの退役は ADR-007 の履歴であり、登録簿の操作を復活させた `within_write_transaction` で扱うことはない。
FR1 は U3 が文書上の集約対応を持ち、FR1.1 の監査投影の実装担当は U4 のままである。

## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T08:40:40Z
**Iteration:** 1（advisory, unit: u3-event-store-repository）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Critical | `rules.md` BR1.2 / BR1.3 / BR2.3、`functional-spec.md` §3.1 手順1・§3.2 手順3 | 楽観 version の算出式が、既に実装済みの U2 `WorkflowExecution` API の実挙動と矛盾する。実コード（`modules/core/domain/src/orchestration/workflow_execution.rs`）を確認した: `version()` の doc コメント（287-290 行）は「楽観 version（集約の遷移では変わらない — Repository が `with_version` で載せる）」と明記し、`apply_event`（744-759 行）は `self.seq_nr` のみを更新して `self.version` には一切触れない（テスト `with_version_replaces_only_the_optimistic_version`、1889-1894 行がこの契約を固定）。genesis では `start_from_plan_unchecked`（214 行）が `version: 0` を無条件にセットする一方、返す `WorkflowExecutionEvent` は `seq_nr: 1`（191 行）。BR1.3 は「期待 version = `aggregate.version() − 1`（= `event.seq_nr − 1`）」と両式を同値主張するが、genesis では `aggregate.version() − 1 = 0u64 − 1` で **u64 アンダーフロー**（デバッグビルドは panic、リリースビルドは `u64::MAX` へ wrap）となり、`event.seq_nr − 1 = 0` と一致しない。非 genesis でも、`find_by_id` がロードした `version`（Repository が `with_version` で載せた値 `V`）は `apply_event` を挟んでも不変のため、store 時点の `aggregate.version()` は常に「現在 DB に永続化済みの version」そのもの（`V`）であり、そこから `− 1` した `V−1` を `expected` として `UPDATE … WHERE version = :expected` に渡すと、実際の行は `version = V` なので必ず影響行 0 になり、実際には競合が無いにもかかわらず**毎回** `Conflict { expected: V-1, actual: V }` を返す。つまり本 Unit の中核機構（FR1.2 の主たる合格基準である store の楽観ロック）は、記述どおりに実装すると genesis で panic/暴走し、以降の通常書込みも恒常的に偽陽性の競合エラーになる。 | `expected` の算出を `event.seq_nr() − 1`（`aggregate.version()` を経由しない）に一本化するか、`aggregate.version()` を使うなら `− 1` を外して `expected = aggregate.version()` とし、書込み成功後に `with_version(expected + 1)`（または `event.seq_nr()`）で更新後の集約に載せ替える手順を明記する。あわせて BR1.2 / `functional-spec.md` §3.2 の「適用 1 件ごとに version + 1」の記述も、`apply_event` 自体は version を変更しないという実コード契約を踏まえ「replay ループ終了後に Repository が明示的に `with_version` を呼ぶ」と書き改める。 |
| 2 | Major | `entities.md`（EventStore の `persist_event` / `get_events_by_id_since_seq_nr`）と `contract-summary.md` C3 の対応 | 承認済み契約 C3（`inception/contract-design/contract-summary.md` 97-135 行）の `EventStore<AID, A, E>` trait は `persist_event(&mut self, event: &E, version: usize)` / `get_events_by_id_since_seq_nr(&self, aid: &AID, seq_nr: usize)` のように数値パラメータを **`usize`** で定義しているが、本設計の `entities.md`（32 行・35 行）は同じメソッドを **`u64`** で定義している。`rules.md` BR1.1 は「trait の形は C3 のコードを正とし、型名だけ本設計…に具体化」と明言しており、原子型そのものの変更は想定されていない。C3 の所有者は使う側のユースケース層（U5/U6、`contract-summary.md` §3）であり、U3 は「準拠」する側 — U3 が無断で型を変えると、U5/U6 が C3 どおり `usize` で実装した場合に trait 実装がコンパイルエラー（型不一致）になるリスクがある（実ドメイン型 `seq_nr`/`version` が `u64` である事実 — `workflow_execution.rs:84-85` — に照らせば `u64` への変更自体は妥当と思われるが、無言の変更である点が問題）。 | `entities.md` に「C3 の `usize` を実装済みドメイン型（`u64`）に合わせて具体化した」という一文を明記するか、C3 側を `u64` に合わせて改訂する（改訂は所有者 U5/U6 側で）。いずれかを選び、無言の型変更を残さない。 |
| 3 | Major | `entities.md`（`WorkflowExecutionRepositoryImpl.store: SqliteEventStore`）、`functional-spec.md` §2 | `WorkflowExecutionRepository::store` は C3 で `&self`（`entities.md` 23 行も同じ）だが、内部で使う `EventStore::persist_event_and_snapshot` は `&mut self`（`entities.md` 33 行、C3 も同じ）。`WorkflowExecutionRepositoryImpl` が保持する `store` フィールドの型は `entities.md` 174 行で単なる `SqliteEventStore`（値型・ラッパーなし）としか書かれておらず、`&self` メソッドの中から `&mut self` メソッドを呼ぶために必要な内部可変性の機構（`Mutex` / `RefCell` / `Cell` 等）がどこにも記載されていない（`entities.md` / `rules.md` / `functional-spec.md` 全文を検索しても `Mutex` / `RefCell` / `Cell` / interior は 0 件）。この点は上流の contract-design レビュー（`contract-summary.md` の `## Review` Finding #3、Minor）が「functional-design（U3）で `WorkflowExecutionRepositoryImpl` 内の `EventStore` 保持方法（`tokio::sync::Mutex` 等）を明記する」と名指しで本ステージへ申し送っていたが、本設計では未対応のまま残っている。`within_write_transaction`（`&mut self` — `functional-spec.md` §2）を含め、同種の問題が複数メソッドに連鎖する。 | `SqliteEventStore` の内部可変性戦略（例: 内部で `tokio::sync::Mutex<rusqlite::Connection>` を持たせて trait メソッド自体を `&self` で実装する、あるいは `WorkflowExecutionRepositoryImpl` が `store: tokio::sync::Mutex<SqliteEventStore>` を保持する）を `entities.md` に明記する。 |

> （2026-08-23 追記: 所見 3 は前提ごと失効した。`WorkflowExecutionRepository::store` を `&mut self` に是正したことで、`&self` から `&mut self` を呼ぶための内部可変性の機構そのものが不要になった（オーナー裁定 2026-08-23、正本 `coding-rules/interior-mutability.md` / `coding-rules/command-query-separation.md` を新設。委任 8 で実装是正・本文同期済み）。）

> （2026-08-27 追記 — Bolt B6 / ADR-010 による失効: 所見 1・2 は**前提ごと失効した**。所見 1 が指摘した
> 「楽観 version の算出式」は、`version` が**ストアの採番する不透明トークン**になったことで式そのものが
> 無くなった（`with_version` は削除、`version == seq_nr − 1` の前提検査も削除 — BR5.3 / ADR-010 追記 (1)）。
> 所見 2 が指摘した「C3 の `usize` と本設計の `u64` の食い違い」は、**本家の `usize` へ戻す**ことで解消した
> — 借り物の契約を我々のドメイン型に合わせて書き換えていたこと自体が `coding-rules/upstream-contracts.md`
> 違反であり、所見 2 の指摘の向きが正しかった。あわせて `Validation Tool Results` の
> 「`WorkflowExecutionSnapshot` の実フィールド数 = 16 属性」は **17 属性**（`last_updated_at` 追加）に、
> 「C6 楽観 version 制約の genesis 経路」は「Gateway が写しに初期 version 1 を載せる」に変わっている。）

### Validation Tool Results

| Tool / Check | Result | Interpretation |
|---|---|---|
| `bun .claude/tools/aidlc-sensor-traceability.ts --stage functional-design --output-path .../traceability.json` | `"pass":false`、`gaps:[]`、`orphans:[]`、`missing_from_table:[]`、`invalid_entries:[]`、`invalid_targets:[]`、`missing_from_upstream_ids` に FR1/FR2/…/FR9 系 36 件 | ブリーフの想定どおり構造的なノイズ（U3 は FR1.2/FR1.3/NFR3 のみ担当、`upstream_ids` はその 3 件のみを列挙）。実害となる `gaps`/`orphans`/`invalid_targets`/`invalid_entries` はすべて空 — 実質合格 |
| `bun .claude/tools/aidlc-sensor-required-sections.ts --stage functional-design --output-path <各 md>`（entities/rules/functional-spec の 3 回） | 3 ファイルとも `"pass":true`（H2 見出し 2 / 2 / 8 本） | 合格 |
| `grep -rnE 'WorkspaceLock\|FsWorkspaceLock\|LockProtocol\|LockIdentity\|reap_eligible\|OwnerStamp\|AcquireBudget\|LockGuard\|process_alive\|ProcessProbe\|audit_lock\|reap-decision-locality' modules tools scripts formal .github Cargo.toml`（BR3.1 の合格条件そのものを現状コードに事前実行） | 255 件・16 ファイル、すべて `entities.md` `RetiredLockMachinery` の列挙（use-case workspace mod / adapter fs_workspace_lock・process_probe / domain lock_protocol・lock_identity / infra-io process_probe / formal audit_lock.qnt / lint check.rs・赤例 / quint-gate.sh / テスト）で説明が付く | 退役対象の網羅性を確認 — 抜け漏れなし |
| `WorkflowExecution::version()` / `apply_event` / `start_from_plan_unchecked` の実コード確認（`modules/core/domain/src/orchestration/workflow_execution.rs`） | doc コメント・実装・専用テストが「version は集約の遷移では不変、Repository が `with_version` で載せる」契約を明示 | 所見1（Critical）の直接根拠 |
| C3（`contract-summary.md`）の `EventStore` trait シグネチャと `entities.md` の対比 | `usize`（C3）↔ `u64`（`entities.md`） | 所見2（Major）の直接根拠 |
| `entities.md`/`rules.md`/`functional-spec.md` 全文検索 `mutex`/`refcell`/`cell<`/`interior`（大小無視） | 0 件 | 所見3（Major）の直接根拠 |
| `WorkflowExecutionSnapshot` の実フィールド数（`workflow_execution_snapshot.rs`） | 16 属性、`entities.md` の列挙（intent_id〜version）と完全一致 | 合格（BR5.2 / U2 pending-revision 9 の反映を確認） |
| C6 楽観 version 制約の genesis 経路（`contract-summary.md` `## Review` Major 所見1 — UPDATE-only では初回 store が誤って Conflict）の反映確認 | `rules.md` BR1.3 / BR2.3 が `expected == 0` の場合を `INSERT` 経路に分岐しており、この所見自体は解消済み | 合格（ただし所見1 の `aggregate.version()` 由来の別経路のバグが新たに存在） |

### Summary

上流成果物（unit-of-work・requirements・contract-summary・decisions・U2 code-summary/pending-revision）の読み込みと突合は丁寧で、retirement 対象の網羅性（BR3.1 grep 255 件・16 ファイルすべて説明可能）、traceability/required-sections センサーの合格、C6 genesis 経路（contract-design レビュー Major 所見1）の解消、`WorkflowExecutionState` の 16 属性の実コード一致などは高品質である。しかし本 Unit の中核機構である store の楽観バージョン制御（FR1.2 の主たる合格基準）が、既に実装済みの U2 `WorkflowExecution::version()` の実挙動（集約の遷移では不変、Repository が明示的に `with_version` で載せ替える）と矛盾する式（`aggregate.version() − 1`）に基づいており、記述どおり実装すると genesis で u64 アンダーフロー（panic/wrap）、以降の通常書込みは恒常的に偽陽性 Conflict になる（Critical 所見1）。加えて、承認済み契約 C3 との無言の型変更（Major 所見2）と、`&self`/`&mut self` の食い違いに対する内部可変性戦略の欠落（Major 所見3 — 契約レビューで名指しで申し送られていたにもかかわらず未対応）がある。Critical 1 件があるため advisory の閾値（Critical 0）を満たさず、NOT-READY と判定する。承認ゲートでは、特に所見1（バージョン算出式）を実装着手前に必ず是正するよう優先度高く扱われたい — U2 の code-generation で先例（上流設計の欠陥を実装エージェントが独自に発見・自力修正した経緯、`code-summary.md` `## Review` 所見1 参照）があり、同じパターンの再発（設計ゲートの欠陥を実装側が肩代わりする）は避けるべきである。
