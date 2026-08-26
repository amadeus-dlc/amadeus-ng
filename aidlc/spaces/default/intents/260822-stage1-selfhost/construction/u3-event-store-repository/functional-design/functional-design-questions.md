# functional-design-questions — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> Functional Design（Construction 3.1）の質問票（Unit: U3、kind: library、Bolt: B5、規模 L）。出典: `../../../inception/units-generation/unit-of-work.md`
> （U3 の責務・境界・合格）、`../../../inception/units-generation/unit-of-work-story-map.md`（FR1.2 / FR1.3 / NFR3）、`../../../inception/requirements-analysis/
> requirements.md`（FR1.2 / FR1.3 / NFR1 / NFR3）、`../../../inception/domain-design/components.md`（PersistenceGateways）、`../../../inception/domain-design/
> decisions.md`（ADR-001 / 003 / 004 / 006 / 007 / 008）、`../../../inception/contract-design/contract-summary.md`（C3 / C6）、Bolt B3 の実装
> （`../../u2-domain-es-core/code-generation/code-summary.md` §4 / §7）と U2 機能設計 pending-revision（項目 8 / 9）、Bolt B4 で改訂した仕様（10 号 §2.1 / §3、
> 11 号 §2.1 / §10、`deviations.md` # 4）。
>
> 質問は、**成果物を生成する前にオーナー裁定が要る基盤選択 4 点**に絞る（永続化・並行制御の根本裁定は生成前に対話で確定する — project.md Corrections）。

## 質問

### Q1. SQLite ストアファイル（ジャーナル / スナップショット / チェックポイントを入れる 1 ファイル）の置き場所

`deviations.md` # 4 は「`.aidlc-store.sqlite` 相当（最終パスは U3 で確定）」と記録している。候補:

- A. **space 単位で 1 ファイル** `aidlc/spaces/<space>/intents/.aidlc-store.sqlite`（推奨） — 既存の `.gitignore`（`aidlc/spaces/*/intents/.aidlc-*`）で
  そのまま git 管理外。`intents.json`（intent 登録簿）と同じディレクトリに置け、space 内の全 intent のジャーナルを 1 つの DB に入れるので C6 の
  `global_seq_nr`（投影のチェックポイント単位）が space 全体で単調になる。Q2 の A とも相性がよい。
- B. **intent 単位** `<record>/.aidlc-store.sqlite`（既存 ignore `aidlc/spaces/*/intents/*/.aidlc-*` で git 管理外） — intent 間で完全独立だが、
  `intents.json` の直列化（Q2）と space 横断のチェックポイントには別の仕組みが要る。
- X. Other (please specify)

[Answer]: A

### Q2. `intents.json`（intent 登録簿、SQLite ジャーナルの外にある共有ファイル）の同時書込防止の機構（11 号 §10 の未決事項 — U3 で確定）

mkdir ロックが退役（ADR-007）したあと、登録簿の read-modify-write（intent の birth / archive）を直列化する機構が未定:

- A. **同じ SQLite DB のトランザクションを唯一の相互排他に使う**（推奨、Q1 = A 前提） — 登録簿を変更する処理は `BEGIN IMMEDIATE`（書込ロックを
  取る開始）で Tx を開き、その中で `intents.json` を読んで書き戻す。新しい機構を増やさず、集約の保存と同じロックに乗る。stage-1（単一クローン）の
  実態にも十分。
- B. **登録簿自体も SQLite のテーブルに載せ、`intents.json` をリードモデル（投影）にする** — 本質的だが intent 生成（U7）の設計に踏み込む。本 Unit では
  範囲外とし、U7 の設計で扱う。
- C. **OS のファイルロック（`flock`）を別機構として追加** — 並行制御が 2 機構になる（ADR-007 の「中途半端」を再導入）。
- X. Other (please specify)

[Answer]: A

### Q3. SQLite ドライバ（Rust クレート）と非同期（async）の形

ADR-006 は「tokio を初期化から、ドメインは同期」を裁定済み。ドライバは未選定（ワークスペース依存に SQLite 関連は無し）:

- A. **`rusqlite`（SQLite 同梱 `bundled` フィーチャ、同期 API）を採用し、Repository の `async fn` の内部で同期呼出**（推奨） — 成熟・最小依存
  （`rusqlite` + `libsqlite3-sys`、ネットワーク依存なし）。ワンショット CLI + tokio `current_thread` では ms 単位のブロッキングは許容。`cargo audit`
  の対象に入る。
- B. **`sqlx`（非同期ドライバ、`sqlite` フィーチャ）** — 本来の async だが依存ツリーが大きく（tokio-rt / sqlx-core / クエリマクロ）、ビルドと監査面が
  不釣り合い。
- C. **`rusqlite` + `tokio::task::spawn_blocking`（別スレッドで同期 I/O）** — 将来のウェブサーバ向け。今は過剰で、`Send` 制約が Repository の型に
  波及する。
- X. Other (please specify)

[Answer]: A

### Q4. Quint モデルの扱い — ADR-007「`audit_lock.qnt` を協定モデルへ改訂」の具体形

- A. **新モデル `formal/orchestration/journal_protocol.qnt`（集約 `WorkflowExecution` の永続化協定: version 競合拒否・チェックポイント単調性・投影冪等性・
  真実源はジャーナル）を書き、`audit_lock.qnt`・`LockProtocol`・ITF テスト `audit_lock_conformance.rs`・fixtures・CI ゲートの該当ステップを削除する**
  （推奨） — 退役機構の名前を残さない（「痛みが伴っても本質な姿」）。新モデルの ITF 準拠は InMemory の EventStore / Repository に対して回す。
- B. **ファイル名 `formal/workspace/audit_lock.qnt` を保ったまま中身を協定モデルに書き換える** — ADR の字義どおりだが、名前（lock）と中身
  （ジャーナル協定）が乖離し、11 号 §6 の W1〜W5 の定義名も旧名のまま参照され続ける。
- X. Other (please specify)

[Answer]: A

## 前提（確認事項）

- P1. 範囲: (a) ユースケース層に C3 の trait 3 本（`WorkflowExecutionRepository` / `EventStore<AID, A, E>` / `JournalReader`）とエラー型、(b) アダプタ層に
  SQLite EventStore（C6 の 3 テーブル、`persist_event_and_snapshot` は同一 Tx + 楽観 version 条件付き書込）、`WorkflowExecutionRepositoryImpl`
  （store / find_by_id）、`InMemoryWorkflowExecutionRepository`（先行）、(c) ロック退役 — `WorkspaceLock` ポート（use-case）、`FsWorkspaceLock`
  （adapter）、`LockProtocol` / `LockIdentity` / `reap_eligible`（domain workspace）、`ProcessProbe` 機構（reap 専用だったため）、`cargo lint` の
  `reap-decision-locality` ルールと赤例テスト、`fs_workspace_lock_test`、Quint / ITF（Q4）、(d) U2 の是正 — `IntentId` を UUIDv7 形式に、`IntentDirName` を
  domain workspace に新設、`WorkflowExecutionSnapshot` → `WorkflowExecutionState`（`state()` / `from_state()`）、(e) 仕様の同期 — 10 号 §6 I14 /
  11 号 §6 W1〜W5 / 01 号 §3.3 の不変条件を協定モデルへ差し替え、`deviations.md` # 4 のパス確定、coding-rules（tell-dont-ask / README）の reap 記述更新。
  投影（RMU）は U4、intent 生成は U7 で扱う。
- P2. 再水和: `find_by_id` = 最新スナップショット + それ以降のイベント replay（ADR-001: スナップショットは毎 store で更新するので通常 replay は 0 件）。
  集約が無ければ `NotFound`、ジャーナル行があるのにスナップショットが無い／復号不能なら `Corrupt`（部分データは返さない — C3 ①）。`from_started`
  相当の入口は不要（U2 申し送り D6 の「要るなら」は不要と判定）。
- P3. 楽観 version: `store` は `journal INSERT` + `snapshot UPDATE … WHERE version = :expected` を同一 Tx で行い、影響行 0（または `UNIQUE(aggregate_id,
  seq_nr)` 違反）なら `Conflict { expected, actual }`。再試行（再水和して 1 回）はユースケース側（U5）。`Conflict` 以外は再試行しない（C3 ③）。
- P4. `revision_count` は C6 の snapshot `payload`（集約の正準 JSON = `WorkflowExecutionState` 全 16 属性）に含まれるため、列追加は不要（U2 申し送りの
  「1 列追加」は payload 内で満たす）。ペイロードの正準 JSON は U1 の canon-json。
- P5. 時刻: イベントの `occurred_at` は呼出側（ユースケース）が渡す。テーブルの `updated_at` は `Clock` 機構（adapter の機構モジュール）を Impl に注入して
  取る。Gateway には数えない。
- P6. 依存追加: `rusqlite`（Q3 = A の場合）と `tokio`（`rt`、current_thread、ADR-006）をワークスペース依存に追加。`cargo audit` が CI で検査する（NFR4）。
- P7. マルチクローン: stage-1 は単一クローン運用。ジャーナル（git 管理外）は交換せず、他クローンの監査シャードは読み取り専用の外部入力（U4）。
  ジャーナルを持たないクローンでの再構成は後続 intent。

## Consolidated Summary Confirmation

- Q1〜Q4 の裁定（ストアの置き場所・登録簿の直列化・ドライバ・Quint モデル）を entities / rules / functional-spec に写す。
- U3 = EventStore trait 群 + SQLite 実装 + Repository 実装（InMemory 先行）+ ロック退役 + U2 是正（IntentId UUIDv7 / IntentDirName / State 改名）+ 仕様同期。
- 成果物は entities.md / rules.md / functional-spec.md / traceability.json。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
