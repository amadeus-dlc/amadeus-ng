# rules — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> Functional Design（Construction 3.1）成果物（Unit: U3、kind: library、Bolt: B5）。出典: `entities.md`（同ディレクトリ）、`../../../inception/requirements-analysis/
> requirements.md`（FR1.2 / FR1.3 / NFR1 / NFR3）、`../../../inception/contract-design/contract-summary.md`（C3 / C6）、`../../../inception/domain-design/
> decisions.md`（ADR-001 / 003 / 006 / 007）、`docs/adr/0003-quint-operations.md`（Quint DoD）、確認質問 `functional-design-questions.md`（Q1〜Q4 = A、P1〜P7）。
>
> 下の fenced `yaml` が正本。BR1.x = ポート契約、BR2.x = SQLite ストアと Repository 実装、BR3.x = ロック退役と検証モデル、BR4.x = U2 の是正、
> BR5.x = 仕様同期と合格条件。

## 1. 規則（正本）

```yaml
rules:
  # --- BR1: ポート契約（ユースケース層、C3） ---
  - id: BR1.1
    statement: "C3 の trait 3 本（WorkflowExecutionRepository / EventStore<AID, A, E> / JournalReader）とエラー型を core-use-case::orchestration に置く。メソッドは async fn（AFIT、Rust 2024）、dyn は使わない、Send / Sync 境界は要求しない。数値パラメータ（seq_nr / version）は C3 の usize ではなく実ドメイン型に合わせた u64（C3 改訂提案として所有者 U5 / U6 へ申し送り — 無言の変更にしない）。Repository 実装は EventStore を直接所有し、`store` は `&mut self` で素直に実装する（内部可変性は使わない — coding-rules/interior-mutability.md、オーナー裁定 2026-08-23、委任 8 で是正）"
    category: policy
    applies_to: [WorkflowExecutionRepository, EventStore, JournalReader]
    trigger: "ポート定義"
    logic: "trait の形は C3 のコードを正とし、型名だけ本設計（IntentId / WorkflowExecution / WorkflowExecutionEvent / GlobalSeqNr / ProjectionName）に具体化"
    violation: "dyn・Box<dyn Error>・外部エラークレートが現れればレビューで差し戻し"
    source: "C3, ADR-006, use-case-rules §2, error-handling.md"
  - id: BR1.2
    statement: "find_by_id は集約を完全に再構成して返す: 最新スナップショット → from_state → with_version(snapshot.version) → seq_nr より後のイベントを昇順 apply_event → replay 後に Repository が明示的に with_version(最後に適用した seq_nr) を載せる（apply_event は version を変えない — B3 実装契約。version = 永続化済みイベント数 = 最後の seq_nr）。集約が無ければ NotFound、ジャーナル行があるのにスナップショットが無い／復号不能／from_state が Err なら Corrupt（部分データは返さない）"
    category: validation
    applies_to: [WorkflowExecutionRepository]
    trigger: "再水和"
    logic: "IF snapshot なし AND journal なし THEN NotFound; IF snapshot なし AND journal あり THEN Corrupt(MissingSnapshot); ELSE decode → from_state → replay"
    violation: "テスト（ラウンドトリップ・欠落・破損）で検出"
    source: "FR1.3, C3 ①, ADR-001, P2"
  - id: BR1.3
    statement: "store は『1 コマンドが返した単一イベント』と『適用後の集約』を同一 Tx で永続化する。期待 version = aggregate.version()（find_by_id が載せた、永続化済みの最後の seq_nr。genesis は 0）。これは event.seq_nr() − 1 と一致しなければならず、不一致は呼出側の不整合として Corrupt(SequenceGap)（u64 の減算は seq_nr ≥ 1 を先に検査してから行う）。書込後の snapshot.version = event.seq_nr()。スナップショットの現在 version が期待と一致しなければ Conflict { expected, actual }、ジャーナルの UNIQUE(aggregate_id, seq_nr) 違反も Conflict。Conflict 以外は再試行しない（再試行はユースケースが再水和して 1 回）。store は引数の集約を変更しない（&）ため、呼出側が続けて store するには再水和が要る（1 コマンド 1 プロセスの CLI では起きない）"
    category: validation
    applies_to: [WorkflowExecutionRepository, EventStore]
    trigger: "store"
    logic: "前提検査: event.intent_id == aggregate.intent_id、event.seq_nr == aggregate.seq_nr、event.seq_nr ≥ 1、aggregate.version() == event.seq_nr − 1（不一致は Corrupt(SequenceGap) — 呼出側バグ）。expected = aggregate.version()。Tx 内: journal INSERT; expected == 0 なら snapshot INSERT（既存行があれば Conflict）、それ以外は snapshot UPDATE … SET version = event.seq_nr … WHERE version = expected（影響 0 行なら actual を読んで Conflict）"
    violation: "競合テスト（2 つの再水和 → 片方 store → もう片方 store が Conflict）で検出"
    source: "FR1.2, C3, C6 制約 (1)(2), ADR-007, P3"
  - id: BR1.4
    statement: "JournalReader: events_after(after) は global_seq_nr > after の行を昇順で返す（全集約横断）。checkpoint(name) は未登録なら GlobalSeqNr::ZERO。advance_checkpoint(name, to) は to < 現在値なら CheckpointRegression、同値は no-op、増加は UPSERT。チェックポイントの巻き戻しは行削除（再生成）だけ"
    category: validation
    applies_to: [JournalReader]
    trigger: "投影の差分読取"
    logic: "C6 checkpoint 制約 (3)"
    violation: "単調性テストで検出"
    source: "C3, C6, ADR-003"
  - id: BR1.5
    statement: "エラー型（RepositoryError / EventStoreError / CorruptCause）は手実装 enum、Display は材料のみ、std::error::Error 手実装、thiserror / anyhow 不使用。Io は ErrorKind と path を保持（監査 C24）。EventStoreError → RepositoryError の写像は変種同名（Schema / CheckpointRegression は Repository 面に出ないので Corrupt(SchemaVersion) / 内部扱い）"
    category: policy
    applies_to: [RepositoryError, EventStoreError]
    trigger: "エラー定義"
    logic: "coding-rules/error-handling.md のルールどおり"
    violation: "cargo lint 候補・レビュー"
    source: "coding-rules/error-handling.md, C3"

  # --- BR2: SQLite ストアと実装（アダプタ層） ---
  - id: BR2.1
    statement: "ストアファイルは StorePath::for_space(aidlc_root, &SpaceName) = `<aidlc root>/spaces/<space>/intents/.aidlc-store.sqlite`（Q1 = A）。open は create-if-missing、PRAGMA user_version を検査（0 → スキーマ作成して 1 に、1 → そのまま、それ以外 → Schema { found, supported: 1 }）、busy_timeout 5000ms、journal_mode は既定（WAL は使わない — 付随ファイルを増やさない）"
    category: policy
    applies_to: [EventStoreImpl, StorePath]
    trigger: "ストアの open"
    logic: "親ディレクトリが無ければ Io(NotFound)（intents/ は upstream の既存ディレクトリ — 作らない）"
    violation: "テスト（空 DB の初期化・user_version 不一致・親 dir 欠落）で検出"
    source: "Q1 = A, deviations # 4, NFR1"
  - id: BR2.2
    statement: "スキーマは C6 の DDL を逐語で使う（journal / snapshot / checkpoint、UNIQUE(aggregate_id, seq_nr)）。追加するのは journal(aggregate_id, seq_nr) の暗黙 UNIQUE 索引以外に無し。列は増やさない（revision_count は snapshot.payload 内 — P4）"
    category: policy
    applies_to: [EventStoreImpl]
    trigger: "初期化"
    logic: "DDL を定数として埋め込み、テストで C6 のテーブル・列名・型・制約と突合（PRAGMA table_info）"
    violation: "C6 との乖離はレビューで差し戻し（contract-summary を改訂するなら契約改訂として記録）"
    source: "C6"
  - id: BR2.3
    statement: "書込はすべて BEGIN IMMEDIATE で始める Tx（書込ロック先取り）。persist_event_and_snapshot(event, aggregate) の Tx 内順序: expected = aggregate.version()、new_version = event.seq_nr()（= expected + 1 を検査）。(1) journal INSERT（UNIQUE 違反 → rollback + Conflict）、(2) expected == 0 なら snapshot INSERT(version = new_version)（既存行があれば rollback + Conflict）、それ以外は UPDATE … SET version = new_version, seq_nr = new_version, payload, updated_at WHERE aggregate_id = ? AND version = expected（影響 0 行 → 現在 version を SELECT して rollback + Conflict）、(3) COMMIT。persist_event(event, version) は (1) のみ（本 Unit の Repository は使わないが契約として実装・テスト）"
    category: validation
    applies_to: [EventStoreImpl]
    trigger: "store"
    logic: "rusqlite の Transaction（drop で rollback）を使い、成功経路だけ commit"
    violation: "同時 2 接続テスト（busy_timeout 内に直列化）と Conflict テストで検出"
    source: "FR1.2, C6 制約 (1), ADR-007, Q3 = A"
  - id: BR2.4
    statement: "within_write_transaction(f: FnOnce(&Transaction) -> Result<T, EventStoreError>) を EventStoreImpl が公開する。BEGIN IMMEDIATE … f … COMMIT。intents.json（登録簿）の read-modify-write を行う処理（U7 の birth / archive）はこの中で実行し、これを登録簿の唯一の直列化機構とする（Q2 = A）。他の相互排他機構（mkdir / flock）は導入しない"
    category: policy
    applies_to: [EventStoreImpl]
    trigger: "登録簿の変更"
    logic: "stage-1 は単一クローン。同一ホストの並行 CLI は busy_timeout 内に直列化、超過は Io(WouldBlock 相当 — rusqlite の Busy を ErrorKind::WouldBlock に写す)"
    violation: "別機構が現れればレビューで差し戻し"
    source: "Q2 = A, 11 号 §10 未決事項（確定）, ADR-007"
  - id: BR2.5
    statement: "ワイヤ形式: journal.payload は EventPayloadWire の正準 JSON（canon-json）で `{ \"type\": \"<変種名>\", …材料 }`、snapshot.payload は StateWire（16 属性）の正準 JSON。固定トークンは upstream 綴り（CheckboxState の 6 マーク、PlanAction EXECUTE / SKIP、PhaseId の 5 語、AutonomyMode autonomous / gated）、その他の列挙は snake_case。復号は parse-don't-validate（未知フィールド・未知 type・型不一致・範囲外は Corrupt）。schema_version = 1 以外は Corrupt(SchemaVersion)"
    category: policy
    applies_to: [EventPayloadWire, StateWire]
    trigger: "符号化・復号"
    logic: "serde 構造体は adapter に閉じる（ドメインは serde を知らない — ADR-004）。PBT: 任意のイベント / 状態で encode→decode が恒等（正準 JSON はバイト決定的）"
    violation: "ラウンドトリップ PBT で検出"
    source: "components PersistenceGateways, ADR-001, U1 canon-json"
  - id: BR2.6
    statement: "時刻: occurred_at は呼出側（ユースケース）が渡した文字列を素通し。updated_at は Clock 機構（core_interface_adapter::clock、Fake 付き）から取る。Repository / EventStore は Clock を注入されるが、Clock は Gateway ではない"
    category: policy
    applies_to: [EventStoreImpl]
    trigger: "書込"
    logic: "gateway-taxonomy §1"
    violation: "レビュー"
    source: "P5, gateway-taxonomy §1"
  - id: BR2.7
    statement: "InMemoryEventStore / InMemoryWorkflowExecutionRepository を先に書き、SQLite 実装と同じ契約テスト群（ラウンドトリップ・Conflict・NotFound・Corrupt・チェックポイント単調性・events_after 順序）をジェネリック関数で共有して両方に対して実行する"
    category: validation
    applies_to: [InMemoryEventStore, InMemoryWorkflowExecutionRepository, EventStoreImpl]
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
    statement: "ロック dir（`.aidlc-lock/` 等）を生成するコードを残さない。並行制御は BR2.3 / BR2.4 の SQLite Tx + 楽観 version だけ"
    category: policy
    applies_to: [EventStoreImpl]
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
    statement: "ITF 準拠: `tests/conformance/fixtures/journal_protocol/*.itf.json` を採取し、`modules/core/interface-adapter/tests/journal_protocol_conformance.rs` が InMemoryEventStore + フェイク投影（readModelSeq を持つだけ）に再生する。lastAction × lastActor 駆動: load → find_by_id で version を記録、store_ok → store（Ok を要求）、store_conflict → store（Err(Conflict) を要求）、catchup → events_after + advance_checkpoint、crash → 何もしない、idle → 何もしない。各ステップで状態射影（journalLen = 全イベント数、snapVersion / snapSeq、checkpoint、readModelSeq）を突合。fixture は 6 本以上・全アクション網羅"
    category: validation
    applies_to: [InMemoryEventStore, JournalProtocolModel]
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
    applies_to: [EventStoreImpl, WorkflowExecutionRepositoryImpl, JournalProtocolModel]
    trigger: "Bolt B5 の PR"
    logic: "unit-of-work U3 合格 + NFR2 / NFR4"
    violation: "PR を戻す"
    source: "unit-of-work U3, NFR2, NFR3, NFR4"
```

## 2. 規則の要約

| ID | 区分 | 一言 | 出典 |
|---|---|---|---|
| BR1.1 | policy | ポート 3 本 + エラーをユースケース層に（async、dyn なし） | C3 / ADR-006 |
| BR1.2 | validation | find_by_id = スナップショット + 差分 replay、NotFound / Corrupt の境界 | FR1.3 |
| BR1.3 | validation | store = 同一 Tx + 楽観 version、Conflict の定義 | FR1.2 / C6 |
| BR1.4 | validation | JournalReader の順序とチェックポイント単調性 | C3 / C6 |
| BR1.5 | policy | エラーは材料のみ・手実装 | error-handling.md |
| BR2.1 | policy | ストアパス（space 単位）・open・user_version・busy_timeout | Q1 = A |
| BR2.2 | policy | スキーマ = C6 逐語 | C6 |
| BR2.3 | validation | BEGIN IMMEDIATE Tx の書込手順 | FR1.2 / Q3 = A |
| BR2.4 | policy | within_write_transaction で intents.json を直列化 | Q2 = A |
| BR2.5 | policy | ワイヤ形式（正準 JSON、parse-don't-validate） | ADR-001 / U1 |
| BR2.6 | policy | occurred_at は呼出側、updated_at は Clock | P5 |
| BR2.7 | validation | InMemory 先行 + 契約テスト共有 | gateway-taxonomy §6 |
| BR2.8 | policy | 実装はドメイン公開 API だけを使う | coding-rules |
| BR3.1 | policy | ロック系の全削除（grep 0 件） | ADR-007 |
| BR3.2 | policy | ロック dir を生成しない | deviations # 4 |
| BR3.3 | validation | journal_protocol.qnt のモデル仕様 | Q4 = A / ADR 0003 |
| BR3.4 | validation | Quint DoD と quint-gate の置換 | ADR 0003 |
| BR3.5 | validation | ITF 準拠（InMemory + フェイク投影） | FR1.2 / NFR3 |
| BR4.1 | policy | IntentId = UUIDv7 | U2 pending 8 |
| BR4.2 | policy | IntentDirName 新設 | 11 号 §2.2 |
| BR4.3 | policy | Snapshot → State 改名（型・メソッド・ファイル） | U2 pending 9 |
| BR5.1 | policy | 仕様・正本の同期（不変条件表・退役・パス確定） | B4 申し送り |
| BR5.2 | validation | 合格条件 | U3 合格 / NFR2〜4 |
