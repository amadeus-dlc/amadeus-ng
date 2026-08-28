# rules — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> Functional Design（Construction 3.1）成果物（Unit: U3、kind: library、Bolt: B5）。出典: `entities.md`（同ディレクトリ）、`../../../inception/requirements-analysis/
> requirements.md`（FR1.2 / FR1.3 / NFR1 / NFR3）、`../../../inception/contract-design/contract-summary.md`（C3 / C6）、`../../../inception/domain-design/
> decisions.md`（ADR-001 / 003 / 006 / 007）、`docs/adr/0003-quint-operations.md`（Quint DoD）、確認質問 `functional-design-questions.md`（Q1〜Q4 = A、P1〜P7）。
>
> 下の fenced `yaml` が正本。BR1.x = ポート契約、BR2.x = SQLite ストアと Repository 実装、BR3.x = ロック退役と検証モデル、BR4.x = U2 の是正、
> BR5.x = 仕様同期と合格条件。

> ## ⚠ 部分失効（2026-08-27 / ADR-010・Bolt B6 — event-store-adapter-rs v2.0.0 へ乗り換え）
>
> **自前ストアを前提にした規則は失効した**（YAML 内に個別の失効注記を入れてある）:
> BR1.1 の「trait 3 本」と `u64` 化、BR1.2 の `with_version`、BR1.3 の `version == seq_nr − 1` 前提と
> SQL 手順、BR2.1 の `PRAGMA user_version` と `busy_timeout`、BR2.2 / BR2.3（DDL と Tx 手順）、
> **BR2.4（`within_write_transaction`）は口ごと削除され代替は未定 — U7 で裁定**、BR2.5（ワイヤ形式）、
> BR2.7 の InMemory 2 型、BR3.5 の ITF 再生先。
>
> **不変**: BR1.4（`JournalReader` の意味論）、BR3.x のロック退役、BR4.x（U2 の是正）、BR5.x の合格条件、
> Quint モデル `journal_protocol.qnt`（**1 文字も変えずに通った**）。

## 1. 規則（正本）

```yaml
rules:
  # --- BR1: ポート契約（ユースケース層、C3） ---
  - id: BR1.1
    statement: "【2026-08-27 改訂 / ADR-010】C3 の trait **2 本**（WorkflowExecutionRepository / JournalReader）とエラー型を core-use-case::orchestration に置く。~~EventStore<AID, A, E>~~ は**失効** — イベントストアの契約は本家 `event_store_adapter_rs::types::EventStore` が正本であり、我々は定義せず実装する側に回る（Conformist）。メソッドは async fn（AFIT、Rust 2024）、dyn は使わない、Send / Sync 境界は要求しない。~~数値パラメータ（seq_nr / version）は C3 の usize ではなく実ドメイン型に合わせた u64~~ → **失効・撤回**: 本家の `usize` に戻した（借り物の契約を書き換えていたこと自体が coding-rules/upstream-contracts.md 違反）。Repository 実装は本家ストアを型引数 S として直接所有し、`store` は `&mut self` で素直に実装する（内部可変性は使わない — coding-rules/interior-mutability.md、オーナー裁定 2026-08-23、委任 8 で是正）"
    category: policy
    applies_to: [WorkflowExecutionRepository, EventStore, JournalReader]
    trigger: "ポート定義"
    logic: "trait の形は C3 のコードを正とし、型名だけ本設計（IntentId / WorkflowExecution / WorkflowExecutionEvent / GlobalSeqNr / ProjectionName）に具体化"
    violation: "dyn・Box<dyn Error>・外部エラークレートが現れればレビューで差し戻し"
    source: "C3, ADR-006, use-case-rules §2, error-handling.md"
  - id: BR1.2
    statement: "【2026-08-27 改訂 / ADR-010】find_by_id は集約を完全に再構成して返す: 最新スナップショット（本家 get_latest_snapshot_by_id。復号は serde だが `#[serde(try_from)]` で memento を経由するので from_state の検査点を必ず通る）→ その seq_nr **以降**のイベント（本家 get_events_by_id_since_seq_nr は**指定番号を含む**ので自分自身は読み飛ばす）を昇順 apply_event。~~with_version(snapshot.version) → replay 後に Repository が明示的に with_version(最後に適用した seq_nr) を載せる（version = 永続化済みイベント数 = 最後の seq_nr）~~ → **失効**: `with_version` は削除された。**version はストアが載せた値をそのまま保つ**のであって Repository が seq_nr から載せ直すことはしない（BR5.3 — version は不透明トークン）。集約が無ければ NotFound、復号不能／from_state が Err なら Corrupt（部分データは返さない）"
    category: validation
    applies_to: [WorkflowExecutionRepository]
    trigger: "再水和"
    logic: "IF snapshot なし AND journal なし THEN NotFound; IF snapshot なし AND journal あり THEN Corrupt(MissingSnapshot); ELSE decode → from_state → replay"
    violation: "テスト（ラウンドトリップ・欠落・破損）で検出"
    source: "FR1.3, C3 ①, ADR-001, P2"
  - id: BR1.3
    statement: "【2026-08-27 改訂 / ADR-010】store は『1 コマンドが返した単一イベント』と『適用後の集約』を同一 Tx で永続化する（**Tx を張るのは本家**であり、我々は接続も Tx も持たない）。期待 version = aggregate.version()（**ストアが前回載せた不透明トークン**）。~~これは event.seq_nr() − 1 と一致しなければならず、不一致は Corrupt(SequenceGap)~~ / ~~書込後の snapshot.version = event.seq_nr()~~ → **失効**: version を seq_nr から導く前提そのものが BR5.3 で否定された（前提検査からも削除済み）。残る前提検査は event と aggregate の identity 一致・seq_nr 一致・seq_nr ≥ 1 の 3 点。genesis（`Event::is_created()` が真）では Gateway が**ストアへ渡す写しにだけ**初期 version 1 を載せる（呼出側の集約は動かない — ADR-010 追記 (1)）。競合は本家が返し、我々は Conflict { expected, actual } へ写す（actual は競合時のみ get_latest_snapshot_by_id を 1 回読み直して得る — 本家は整形済み文字列しか返さないので文言は解析しない）。Conflict 以外は再試行しない（再試行はユースケースが再水和して 1 回）。store は引数の集約を変更しない（&）ため、呼出側が続けて store するには再水和が要る（1 コマンド 1 プロセスの CLI では起きない）"
    category: validation
    applies_to: [WorkflowExecutionRepository, EventStore]
    trigger: "store"
    logic: "【2026-08-27 訂正 / ADR-010】前提検査は identity 一致（event.intent_id == aggregate.intent_id）・event.seq_nr == aggregate.seq_nr・event.seq_nr ≥ 1 の 3 点のみ（~~aggregate.version() == event.seq_nr − 1 の検査~~ は statement のとおり削除済み — 本行が旧手順のまま残っていたのを是正）。expected = aggregate.version()（ストアが前回載せた不透明トークン、genesis は Gateway が写しに載せる FIRST_STORED_VERSION）。Tx 内の journal INSERT / snapshot INSERT-or-UPDATE / CAS 判定は**すべて本家 persist_event_and_snapshot の内部**で行われ、我々は 1 回呼ぶだけ（影響 0 行時の actual 読み直しも本家がエラーで返した後に get_latest_snapshot_by_id で我々が行う）"
    violation: "競合テスト（2 つの再水和 → 片方 store → もう片方 store が Conflict）で検出"
    source: "FR1.2, C3, C6 制約 (1)(2), ADR-007, P3"
  - id: BR1.4
    statement: "JournalReader: events_after(after) は全集約横断の通番が after より大きい行を昇順で返す（2026-08-27 補足 / ADR-010: 通番の実体は**本家 journal 表の rowid**。専用の AUTOINCREMENT 列は無い）。checkpoint(name) は未登録なら GlobalSeqNr::ZERO。advance_checkpoint(name, to) は to < 現在値なら CheckpointRegression、同値は no-op、増加は UPSERT。チェックポイントの巻き戻しは行削除（再生成）だけ"
    category: validation
    applies_to: [JournalReader]
    trigger: "投影の差分読取"
    logic: "C6 checkpoint 制約 (3)"
    violation: "単調性テストで検出"
    source: "C3, C6, ADR-003"
  - id: BR1.5
    statement: "【2026-08-27 改訂 / ADR-010】エラー型（RepositoryError / ~~EventStoreError~~ → **JournalReadError** / CorruptCause）は手実装 enum、Display は材料のみ、std::error::Error 手実装、thiserror / anyhow 不使用（**申し送り**: 本家経由で thiserror が推移依存に入った — 我々が直接使わない方針は不変だが、正本への注記が要る）。Io は ErrorKind と path を保持（監査 C24）。~~EventStoreError → RepositoryError の写像は変種同名（Schema / CheckpointRegression は Repository 面に出ないので Corrupt(SchemaVersion) / 内部扱い）~~ → **失効**: 本家の EventStoreWriteError / EventStoreReadError を RepositoryError へ写す。`Schema` 変種は廃止（本家は PRAGMA user_version を使わない）、`CorruptCause` は 6 → 4 分類（UnknownEventType / SchemaVersion を削除）"
    category: policy
    applies_to: [RepositoryError, EventStoreError]
    trigger: "エラー定義"
    logic: "coding-rules/error-handling.md のルールどおり"
    violation: "cargo lint 候補・レビュー"
    source: "coding-rules/error-handling.md, C3"

  # --- BR2: SQLite ストアと実装（アダプタ層） ---
  - id: BR2.1
    statement: "【2026-08-27 改訂 / ADR-010】ストアファイルは StorePath::for_space(aidlc_root, &SpaceName) = `<aidlc root>/spaces/<space>/intents/.aidlc-store.sqlite`（Q1 = A。**不変**）。open は create-if-missing（表と索引は**本家が冪等に作る**）。~~PRAGMA user_version を検査（0 → スキーマ作成して 1 に、1 → そのまま、それ以外 → Schema { found, supported: 1 }）~~ → **失効**: 本家は user_version を使わない。版の固定は `event-store-adapter-rs = \"=2.0.0\"` の完全固定 + スキーマガードテストが担う。~~busy_timeout 5000ms~~ → **未決（U7 で裁定）**: 本家の接続には設定できず（接続を露出しない）SQLite 既定の 0ms になる — 並行書込は待たずに SQLITE_BUSY（Io { kind: WouldBlock }）。BR2.1 の実質的な後退であり、単一プロセス前提の現状は受容して U7 の並行モデルと併せて再裁定する。我々が開く JournalReaderImpl の接続には 5000ms を設定済み。journal_mode は既定（WAL は使わない — 付随ファイルを増やさない）"
    category: policy
    applies_to: [WorkflowExecutionRepositoryImpl, StorePath]   # 2026-08-27: EventStoreImpl は削除済み（ADR-010）
    trigger: "ストアの open"
    logic: "親ディレクトリが無ければ Io(NotFound)（intents/ は upstream の既存ディレクトリ — 作らない）"
    violation: "テスト（空 DB の初期化・~~user_version 不一致~~ → **スキーマガード突合**（2026-08-27 / ADR-010）・親 dir 欠落）で検出"
    source: "Q1 = A, deviations # 4, NFR1"
  - id: BR2.2
    statement: "【2026-08-27 全面改訂 / ADR-010】~~スキーマは C6 の DDL を逐語で使う（journal / snapshot / checkpoint、UNIQUE(aggregate_id, seq_nr)）~~ → **失効**: 我々は DDL を発行しない。`journal` / `snapshot` は**本家が冪等に作る**（正本は upstream）。我々が作る表は `amadeus_projection_checkpoint`（projection TEXT PRIMARY KEY, last_global_seq INTEGER NOT NULL）**1 つだけ**で、本家の表と名前が衝突しないよう接頭辞を付ける。本家スキーマへの結合はピン `=2.0.0` と**スキーマガードテスト**（journal の DDL 文字列・一意索引・rowid 前提の実測突合）で守る。列は増やさない（revision_count は snapshot payload 内 — P4）"
    category: policy
    applies_to: ["【失効】EventStoreImpl", JournalReaderImpl]   # 2026-08-27: 表を作るのは本家と JournalReaderImpl（ADR-010）
    trigger: "初期化"
    logic: "【2026-08-27 訂正 / ADR-010】statement のとおり我々は DDL を発行しない（本行が旧手順「DDL を定数として埋め込み」のまま残っていたのを是正）。スキーマガードテストが `sqlite_master`（`sql` 列）から本家 `journal` テーブルと一意索引 `journal_aid_seq_nr_idx` の DDL を実測し、ピン留めした期待値と逐語比較する（`PRAGMA table_info` ではない）。`amadeus_projection_checkpoint` は我々が `CREATE TABLE IF NOT EXISTS` で冪等に作る"
    violation: "C6 との乖離はレビューで差し戻し（contract-summary を改訂するなら契約改訂として記録）"
    source: "C6"
  - id: BR2.3
    statement: "【2026-08-27 全面失効 / ADR-010】~~書込はすべて BEGIN IMMEDIATE で始める Tx（書込ロック先取り）。persist_event_and_snapshot の Tx 内順序: expected = aggregate.version()、new_version = event.seq_nr()（= expected + 1 を検査）。(1) journal INSERT（UNIQUE 違反 → rollback + Conflict）、(2) expected == 0 なら snapshot INSERT(version = new_version)（既存行があれば rollback + Conflict）、それ以外は UPDATE … WHERE aggregate_id = ? AND version = expected（影響 0 行 → 現在 version を SELECT して rollback + Conflict）、(3) COMMIT。persist_event(event, version) は (1) のみ~~ → **すべて本家の内部**になった。我々は `persist_event_and_snapshot(event, aggregate)` を 1 回呼ぶだけで、接続も Tx も持たない。本家の作法は実測で 2 つ: genesis（`Event::is_created()` が真）は CAS をせず渡された version をそのまま初期値に記録する（だから Gateway が写しに 1 を載せる）、更新は `WHERE version = aggregate.version()` の CAS を張り通れば version + 1 を記録する"
    category: validation
    applies_to: ["【失効】EventStoreImpl"]   # 2026-08-27: Tx 手順は本家の内部（ADR-010）
    trigger: "store"
    logic: "【2026-08-27 訂正 / ADR-010】statement のとおり Tx はすべて本家の内部（我々は rusqlite の Transaction を直接扱わない）。本行が旧手順「rusqlite の Transaction … 成功経路だけ commit」のまま残っていたのを是正。`JournalReaderImpl` の別接続は登録簿ではなく横断読取専用で、Tx は張らない"
    violation: "Conflict テスト（2 つの再水和 → 片方 store → もう片方 store が Conflict）で検出。~~同時 2 接続テスト（busy_timeout 内に直列化）~~ → 失効: 本家の接続には busy_timeout を設定できず（BR2.1）、並行書込の直列化は保証されない。単一プロセス前提の現状は受容し U7 の並行モデルと併せて再裁定する"
    source: "FR1.2, C6 制約 (1), ADR-007, Q3 = A"
  - id: BR2.4
    status: superseded (2026-08-27 / ADR-010) — 代替は未定、U7 で裁定する
    statement: "【2026-08-27 全面失効 / ADR-010・Bolt B6】~~within_write_transaction(f: FnOnce(&Transaction) -> Result<T, EventStoreError>) を EventStoreImpl が公開する。BEGIN IMMEDIATE … f … COMMIT。intents.json（登録簿）の read-modify-write を行う処理（U7 の birth / archive）はこの中で実行し、これを登録簿の唯一の直列化機構とする（Q2 = A）~~ → **口ごと削除した**。本家 EventStoreForSqlite は Connection を内部保持し from_connection は private、transaction() も persist_* の内部でしか使われないため、**本家経由では実現できない**（ADR-010 が調査済み）。ADR-010 は「登録簿 intents.json をやめてジャーナルと同じ DB のテーブルにし、RMU の投影対象にする」を筋と書いている（リードモデルをコマンド側が Tx で守る構造自体が CQRS の境界に反する — coding-rules/cqrs-boundaries.md）が、U7 の設計に踏み込むため**本 Bolt では裁定していない**。**『解決済み』ではなく未決である。** 他の相互排他機構（mkdir / flock）を導入しない方針は維持する"
    category: policy
    applies_to: ["【失効】EventStoreImpl", "U7（裁定先）"]
    trigger: "登録簿の変更"
    logic: "stage-1 は単一クローン。~~同一ホストの並行 CLI は busy_timeout 内に直列化~~ → 失効（本家の接続に busy_timeout を設定できないため。U7 の並行モデルと併せて再裁定）"
    violation: "別機構が現れればレビューで差し戻し"
    source: "Q2 = A, 11 号 §10 未決事項（**再び未決へ差し戻し** — 2026-08-27 / ADR-010）, ADR-007"
  - id: BR2.5
    status: superseded (2026-08-27 / ADR-010)
    statement: "【2026-08-27 全面失効 / ADR-010】~~ワイヤ形式: journal.payload は EventPayloadWire の正準 JSON（canon-json）、snapshot.payload は StateWire（16 属性）の正準 JSON。復号は parse-don't-validate（未知フィールド・未知 type・型不一致・範囲外は Corrupt）。schema_version = 1 以外は Corrupt(SchemaVersion)~~ → **ワイヤ構造体ごと削除**。payload は**本家が serde_json::to_vec で書く**（我々のコードは呼んでいない）。未知の変種も対応外の版も復号失敗 = Corrupt(UndecodablePayload) に畳まれ、CorruptCause::UnknownEventType / SchemaVersion は削除された。**ストアの payload は契約 JSON（BR1.7 / canon-json）ではない** — 契約 JSON の射程は upstream 観測面（監査行・状態ファイル・directive）に限られる（この射程は coding-rules 正本への追記候補）。StateWire が担っていた**値域検査（JSON の正確整数域 2^53 超の拒否）も無くなった**（ストアファイルは upstream 非観測なので実害は無いと判断）。固定トークンの upstream 綴りはドメイン型の serde 表現として維持される"
    category: policy
    applies_to: ["【失効】EventPayloadWire", "【失効】StateWire"]
    trigger: "符号化・復号"
    logic: "~~serde 構造体は adapter に閉じる（ドメインは serde を知らない — ADR-004）~~ → **失効**: 本家 trait が serde 境界を要求するためドメイン型が serde を持つ。ただし集約の復号は memento（WorkflowExecutionState）を経由するので from_state の検査点は 1 か所のまま保たれている（オーナー裁定 2026-08-27）"
    violation: "~~ラウンドトリップ PBT~~ → 集約の serde 往復テストと『改竄 JSON が from_state で弾かれる』テストで検出"
    source: "components PersistenceGateways, ADR-001, U1 canon-json, ADR-010"
  - id: BR2.6
    statement: "【2026-08-27 改訂 / ADR-010】時刻: occurred_at は呼出側（ユースケース）が渡した値を素通し（型は `chrono::DateTime<Utc>` — 本家 trait の要求。**NFR4.1 依存最小化の再検討対象**）。~~updated_at は Clock 機構（core_interface_adapter::clock、Fake 付き）から取る。Repository / EventStore は Clock を注入される~~ → **失効**: `amadeus_projection_checkpoint` から updated_at 列を落とし、本家の snapshot の last_updated_at は**イベントの occurred_at から来る**（本家の作法。ストアは時計を持たない）。したがって **Repository も JournalReader も Clock を持たない**。Clock / SystemClock / FakeClock は残置したが現在利用者はいない（ユースケース着手時に注入シームとして使われる想定）。Clock は Gateway ではないという分類は不変"
    category: policy
    applies_to: [WorkflowExecutionRepositoryImpl, JournalReaderImpl]   # 2026-08-27: EventStoreImpl は削除済み（ADR-010）
    trigger: "書込"
    logic: "gateway-taxonomy §1"
    violation: "レビュー"
    source: "P5, gateway-taxonomy §1"
  - id: BR2.7
    statement: "【2026-08-27 改訂 / ADR-010】~~InMemoryEventStore / InMemoryWorkflowExecutionRepository を先に書き~~ → **テストダブル型は置かない**。`WorkflowExecutionRepositoryImpl::in_memory()`（本家 memory バックエンド）と `open()`（SQLite）の**両バックエンド**に、同じ契約テスト群（ラウンドトリップ・Conflict・NotFound・Corrupt・チェックポイント単調性・events_after 順序）をジェネリック関数で共有して実行する。**実装コードが 1 行も違わないからこそ同じ約束を課せる**という規則の趣旨は、むしろ強くなった"
    category: validation
    applies_to: [WorkflowExecutionRepositoryImpl, JournalReaderImpl]
    trigger: "TDD"
    logic: "契約テストは `fn contract_<case><R: WorkflowExecutionRepository>(repo: R)` の形で 1 度書く"
    violation: "片方だけ通るテストが残れば差し戻し"
    source: "gateway-taxonomy §6, C3 ④"
  - id: BR2.8
    statement: "Repository 実装はドメイン型の公開 API（WorkflowExecution::state / from_state / apply_event / version、WorkflowExecutionEvent の new / アクセサ）だけを使う。集約の private フィールドに触る逃げ道（pub(crate) の昇格・serde derive の追加）を作らない"
    category: policy
    applies_to: [WorkflowExecutionRepositoryImpl]
    trigger: "実装"
    logic: "field-visibility.md / module-visibility.md"
    violation: "cargo lint（no-public-fields）とレビュー"
    source: "coding-rules, ADR-004"

  # --- BR3: ロック退役と検証モデル ---
  - id: BR3.1
    statement: "ADR-007 の退役対象をすべて削除する（entities.md RetiredLockMachinery の列挙: use-case workspace mod、adapter fs_workspace_lock / process_probe、domain lock_protocol / lock_identity / reap_eligible / LockError、infra-io process_probe、fs_workspace_lock_test、md5 依存、audit_lock.qnt + fixtures + audit_lock_conformance.rs + quint-gate の該当ステップ、lint ルール reap-decision-locality と赤例テスト）。後方互換の型エイリアス・deprecated 残置は作らない"
    category: policy
    applies_to: [RetiredLockMachinery]
    trigger: "Bolt B5"
    logic: "削除後 `grep -rnE 'WorkspaceLock|FsWorkspaceLock|LockProtocol|LockIdentity|reap_eligible|OwnerStamp|AcquireBudget|LockGuard|process_alive|ProcessProbe|audit_lock|reap-decision-locality' modules tools scripts formal .github Cargo.toml` = 0 件"
    violation: "残存 1 件で差し戻し"
    source: "ADR-007, オーナー裁定（後方互換コードは残さない）"
  - id: BR3.2
    statement: "ロック dir（`.aidlc-lock/` 等）を生成するコードを残さない。並行制御は ~~BR2.3 / BR2.4 の~~ SQLite Tx + 楽観 version だけ（2026-08-27 補足 / ADR-010: Tx を張るのは**本家**であり、BR2.3 / BR2.4 は失効した。**登録簿の直列化だけは代替が未定であり U7 で裁定する**）"
    category: policy
    applies_to: [WorkflowExecutionRepositoryImpl]   # 2026-08-27: EventStoreImpl は削除済み（ADR-010）
    trigger: "Bolt B5"
    logic: "deviations # 4 (b)"
    violation: "grep `.aidlc-lock` = 0 件（research/ を除く）"
    source: "ADR-007, deviations # 4"
  - id: BR3.3
    statement: "新モデル `formal/orchestration/journal_protocol.qnt`（Q4 = A）を書く: 1 集約・writer 2・投影 1 の抽象。var: journalLen / snapVersion / snapSeq / checkpoint / readModelSeq / loadedVersion（writer ごと）/ lastAction / lastActor + prev*。action: load(w)（loadedVersion[w] := snapVersion）、store_ok(w)（loadedVersion[w] == snapVersion のときだけ: journalLen+1・snapVersion+1・snapSeq = journalLen）、store_conflict(w)（loadedVersion[w] != snapVersion: 状態不変）、catchup（readModelSeq := journalLen、checkpoint := journalLen）、crash（状態不変のマーカー — Tx 済み・投影未反映）、idle。invariant: conflict_rejected（store_conflict は journal / snapshot を変えない）、snapshot_tracks_journal（snapSeq == journalLen）、version_equals_journal（snapVersion == journalLen）、checkpoint_monotone（checkpoint >= prevCheckpoint）、checkpoint_bounded（checkpoint <= journalLen）、projection_idempotent（prevCheckpoint == prevJournalLen のときの catchup は readModelSeq を変えない）、truth_is_journal（readModelSeq <= journalLen）、no_lost_update（store_ok は prevLoadedVersion[actor] == prevSnapVersion のときのみ）。witness: w_conflict / w_crash_then_catchup / w_interleaved_writers / w_idempotent_catchup"
    category: validation
    applies_to: [JournalProtocolModel]
    trigger: "Bolt B5"
    logic: "状態遷移レベル（prev → current）の不変条件を併置する（audit_lock v2 の教訓）"
    violation: "quint typecheck / run 失敗、mutation 未検出で差し戻し"
    source: "FR1.2, NFR3, ADR-007, Q4 = A, ADR 0003 DoD"
  - id: BR3.4
    statement: "Quint DoD: (1) named invariant ごとに対応するガード・遷移を壊した変異モデルが violation になることを確認して記録（code-summary）、(2) 状態遷移レベル不変条件の併置、(3) in-module witness を負形式 run（`--invariant \"not(w_x)\"` で violation = pass）で CI ゲートに載せる。`scripts/quint-gate.sh` の audit_lock ステップを journal_protocol のステップ（typecheck / invariants run / witness 4 本）に置換"
    category: validation
    applies_to: [JournalProtocolModel]
    trigger: "Bolt B5"
    logic: "ADR 0003 決定 4 / 7"
    violation: "ゲートが赤なら PR を戻す"
    source: "ADR 0003, team.md Testing Posture"
  - id: BR3.5
    statement: "ITF 準拠: `tests/conformance/fixtures/journal_protocol/*.itf.json` を採取し、`modules/core/interface-adapter/tests/journal_protocol_conformance.rs` が ~~InMemoryEventStore~~ → **`WorkflowExecutionRepositoryImpl` + `JournalReaderImpl`**（2026-08-27 改訂 / ADR-010）+ フェイク投影（readModelSeq を持つだけ）に再生する。**モデルは 1 文字も変えずに通った**（乗り換えの意味論的な検収）。lastAction × lastActor 駆動: load → find_by_id で version を記録、store_ok → store（Ok を要求）、store_conflict → store（Err(Conflict) を要求）、catchup → events_after + advance_checkpoint、crash → 何もしない、idle → 何もしない。各ステップで状態射影（journalLen = 全イベント数、snapVersion / snapSeq、checkpoint、readModelSeq）を突合。fixture は 6 本以上・全アクション網羅"
    category: validation
    applies_to: [WorkflowExecutionRepositoryImpl, JournalReaderImpl, JournalProtocolModel]
    trigger: "Bolt B5"
    logic: "engine_loop_conformance / 旧 audit_lock_conformance と同じ型（ADR 0003 決定 5）"
    violation: "再生不一致はモデルか実装の欠陥 — どちらかを直す"
    source: "FR1.2 合格, NFR3"

  # --- BR4: U2 の是正（B3 の申し送り） ---
  - id: BR4.1
    statement: "IntentId::parse を UUIDv7 形式の検証に改める（小文字 36 字、version nibble 7、variant 10xx）。kebab の受理は廃止。IntentIdError は Empty / Length / Format / Version / Variant。既存テスト・ITF（engine_loop_conformance）・ゴールデンの IntentId リテラルを UUIDv7（例: intents.json 実データ `01a02785-1bd8-76eb-aeea-5aa303ebd5b6`）へ置換"
    category: policy
    applies_to: [IntentId]
    trigger: "Bolt B5"
    logic: "01 号 §3.3 / 11 号 §2.2 の規範どおり（オーナー裁定 2026-08-23）"
    violation: "kebab を受理するテストが残れば差し戻し"
    source: "U2 pending-revision 8, U9 FD Q2 = A"
  - id: BR4.2
    statement: "IntentDirName を core_domain::workspace に新設（`^[0-9]{6}-[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$`、64 字以下）。予約ラベルの拒否は birth（U7）の責務で本型は形式だけを保証"
    category: policy
    applies_to: [IntentDirName]
    trigger: "Bolt B5"
    logic: "11 号 §2.2"
    violation: "レビュー・ユニットテスト"
    source: "U2 pending-revision 8, 11 号 §2.2"
  - id: BR4.3
    statement: "WorkflowExecutionSnapshot → WorkflowExecutionState、WorkflowExecutionSnapshotBuilder → WorkflowExecutionStateBuilder、SnapshotError → StateError、snapshot() → state()、from_snapshot() → from_state()、ファイル名 workflow_execution_snapshot.rs → workflow_execution_state.rs。旧名の再エクスポート・alias は残さない。rustdoc / 10 号 §2.1 の『現行コード名』注記を除去"
    category: policy
    applies_to: [WorkflowExecutionState]
    trigger: "Bolt B5"
    logic: "U2 pending-revision 9（メソッドも改名 — B4 統合時の裁定、ゲートでオーナー確認）"
    violation: "grep `Snapshot` が core-domain::orchestration に残れば差し戻し（C6 の `snapshot` テーブル名はアダプタ層のみ）"
    source: "U2 pending-revision 9, 10 号 §2.1"

  # --- BR5: 仕様同期と合格条件 ---
  - id: BR5.1
    statement: "仕様同期（文書）: 10 号 §6 I14 と 11 号 §6 W1〜W5 を journal_protocol の不変条件（J1 conflict_rejected / J2 snapshot_tracks_journal / J3 checkpoint_monotone / J4 projection_idempotent / J5 truth_is_journal / J6 no_lost_update）へ差し替え、E4 定義名を新モデルに。01 号 §3.3 の代表不変条件と §6 第一陣の項目、11 号 §2.2 LockIdentity 行（退役）・§3 / §4 の ProcessProbe（退役）・§8 Quint 記録（audit_lock → journal_protocol の経緯）・§10 未決 2 件（Q1 / Q2 で確定）、`deviations.md` # 4 のパス `aidlc/spaces/<space>/intents/.aidlc-store.sqlite` 確定、coding-rules tell-dont-ask.md の reap 例と README の lint ルール列（reap-decision-locality を除去）、gateway-taxonomy §1 の機構モジュール例（process_probe 除去）、10 号 §3 / 11 号 §3 の Repository 実装欄（EventStoreImpl / within_write_transaction）。いずれも出典注記つき"
    category: policy
    applies_to: [SpecDocument, CodingRule]
    trigger: "Bolt B5"
    logic: "B4 の作法（security-design U9 §2）を踏襲"
    violation: "BR3.1 の grep に docs/specs/*.md（research 除く）と coding-rules を加えて 0 件（履歴注記を除く）"
    source: "B4 申し送り, code-summary U9 §7"
  - id: BR5.2
    statement: "合格 = (a) cargo test --workspace 全緑（契約テスト両実装・PBT ラウンドトリップ・Conflict・クラッシュ再構成（store 後にプロセスを落としたと見なし新接続で find_by_id → 同一状態）・ITF journal_protocol）、(b) coverage 90% 床維持、(c) quint-gate 緑（journal_protocol の invariants + witness、mutation 記録）、(d) cargo audit 緑（rusqlite / tokio 追加後）、(e) BR3.1 / BR3.2 / BR4.3 の grep 0 件、(f) cargo lint 自己テスト緑（ルール削除後）、(g) CI 4 ジョブ + CI Success 緑、(h) 逸脱台帳 # 4 のパス確定"
    category: validation
    applies_to: [WorkflowExecutionRepositoryImpl, JournalReaderImpl, JournalProtocolModel]   # 2026-08-27: EventStoreImpl は削除済み（ADR-010）
    trigger: "Bolt B5 の PR"
    logic: "unit-of-work U3 合格 + NFR2 / NFR4"
    violation: "PR を戻す"
    source: "unit-of-work U3, NFR2, NFR3, NFR4"
```

## 2. 規則の要約

| ID | 区分 | 一言 | 出典 |
|---|---|---|---|
| BR1.1 | policy | ~~ポート 3 本~~ → **ポート 2 本**（2026-08-27 / ADR-010）+ エラーをユースケース層に（async、dyn なし）。数値は本家に従い `usize` | C3 / ~~ADR-006~~ → ADR-010 |
| BR1.2 | validation | find_by_id = スナップショット + 差分 replay、NotFound / Corrupt の境界。~~`with_version`~~ は削除（2026-08-27 / ADR-010 — version はストアが載せた値のまま） | FR1.3 |
| BR1.3 | validation | store = 同一 Tx（張るのは本家）+ 楽観 version（不透明トークン）、Conflict の定義。~~`version == seq_nr − 1`~~ の前提検査は削除（2026-08-27 / ADR-010） | FR1.2 / C6 |
| BR1.4 | validation | JournalReader の順序とチェックポイント単調性 | C3 / C6 |
| BR1.5 | policy | エラーは材料のみ・手実装。`EventStoreError` → **`JournalReadError`**（2026-08-27 / ADR-010） | error-handling.md |
| BR2.1 | policy | ストアパス（space 単位）・open。~~user_version~~ は廃止、~~busy_timeout~~ は**未決（U7 で裁定）**（2026-08-27 / ADR-010） | Q1 = A |
| BR2.2 | policy | ~~スキーマ = C6 逐語~~ → **本家 2 表 + 我々の `amadeus_projection_checkpoint` 1 表**、ピン `=2.0.0` + スキーマガードテスト（2026-08-27 / ADR-010） | C6 |
| BR2.3 | validation | ~~BEGIN IMMEDIATE Tx の書込手順~~ → **失効。本家の内部**（2026-08-27 / ADR-010） | FR1.2 / Q3 = A |
| BR2.4 | policy | ~~within_write_transaction で intents.json を直列化~~ → **失効。口ごと削除され代替は未定 — U7 で裁定**（2026-08-27 / ADR-010） | Q2 = A（**再び未決へ**） |
| BR2.5 | policy | ~~ワイヤ形式（正準 JSON、parse-don't-validate）~~ → **失効。payload は本家が serde で書く（契約 JSON ではない）**（2026-08-27 / ADR-010） | ADR-001 / U1 |
| BR2.6 | policy | occurred_at は呼出側。~~updated_at は Clock~~ → **失効。時刻はイベントから来るのでストアは Clock を持たない**（2026-08-27 / ADR-010） | P5 |
| BR2.7 | validation | ~~InMemory 先行~~ → **両バックエンド**（本家 memory / SQLite）+ 契約テスト共有（2026-08-27 / ADR-010） | gateway-taxonomy §6 |
| BR2.8 | policy | 実装はドメイン公開 API だけを使う | coding-rules |
| BR3.1 | policy | ロック系の全削除（grep 0 件） | ADR-007 |
| BR3.2 | policy | ロック dir を生成しない | deviations # 4 |
| BR3.3 | validation | journal_protocol.qnt のモデル仕様 | Q4 = A / ADR 0003 |
| BR3.4 | validation | Quint DoD と quint-gate の置換 | ADR 0003 |
| BR3.5 | validation | ITF 準拠（~~InMemory~~ → `WorkflowExecutionRepositoryImpl` + `JournalReaderImpl`、2026-08-27 / ADR-010 + フェイク投影） | FR1.2 / NFR3 |
| BR4.1 | policy | IntentId = UUIDv7 | U2 pending 8 |
| BR4.2 | policy | IntentDirName 新設 | 11 号 §2.2 |
| BR4.3 | policy | Snapshot → State 改名（型・メソッド・ファイル） | U2 pending 9 |
| BR5.1 | policy | 仕様・正本の同期（不変条件表・退役・パス確定） | B4 申し送り |
| BR5.2 | validation | 合格条件 | U3 合格 / NFR2〜4 |
