# Domain Design — 設計裁定の質問

requirements.md の Open questions（O1〜O3）と設計監査 E束のうち、コンポーネント設計を
左右する未裁定4点。R1（PlanAction 所有一本化）・R2（畳み込み移設）は裁定済み（DECIDED）の
ため質問せず、components.md / decisions.md に反映する。

## Q1. AuditLedger の位置づけ（O2 / B-1 の中核裁定）

現仕様（11号 §2.1）は AuditLedger を独立集約と宣言するが、audit-first 遷移は同一ロック区間で
監査追記と状態書込を行うため「1トランザクション1集約」の DDD 規範と緊張する（設計監査 V1）。

- A. **WorkflowExecution のイベントログとして再分類**: 集約コマンドが発行イベント列を返し、
  audit-first 区間で台帳へ追記する（C16 の解でもある）。`AuditLedgerRepository` は
  イベントストア面（append + 位置付き読取）として設計。B9「状態はキャッシュ、真実源は監査」
  の当然の帰結で、1トランザクション1集約が成立（監査 E束推奨）
- B. peer 集約のまま維持: 現仕様どおり2集約とし、同一区間の2集約更新を仕様上の明示例外として記録
- X. Other (please specify)

[Answer]: A. WorkflowExecution のイベントログとして再分類（集約コマンドがイベント列を返し audit-first 区間で台帳追記。AuditLedgerRepository はイベントストア面。1トランザクション1集約が成立）

## Q2. next_decision（21分岐ラダー）の層配置（O1 / Issue 3-C の前提裁定）

- A. **ドメイン層の純粋関数**: orchestration のドメインサービス
  `next_decision(&WorkflowExecution, &WorkflowDefinition, …) → NextDecision`。
  分岐ラダーの契約正本は `engine_loop.qnt`（ドメインの状態機械）でありモデル対応が保てる。
  `Next` ユースケースは I/O 調達（Repository で load）と directive 組立だけを持ち、
  I8（読取専用の参照渡し）とも整合
- B. ユースケース層に置く: ドメインは述語だけ提供し、21分岐はユースケースが編む
  （モデル↔実装の対応が2層に割れる）
- X. Other (please specify)

[Answer]: A（精密化のうえ確定・統一ルール）: ① next_decision は WorkflowExecution のクエリメソッド（&self、&WorkflowDefinition 引数） ② 状態遷移は &mut self コマンド（現行踏襲、typestate 不採用 — Rust の排他借用が安全性を担保、ITF リプレイ/PBT と整合） ③ ユースケースは進行管理・フロー制御のみ（ビジネスロジック禁止） ④ R2 の畳み込み移設先も集約メソッド。オーナー明言:「集約は FSM。状態としてのデータと状態遷移のための振る舞いは同じ型に閉じ込める。これは統一ルールです。横展開する考え方」

## Q3. 最小フック4本（FR5）の実装形態

- A. **マルチコールバイナリのサブコマンド**: `aidlc` の1バイナリに hook 動詞を持たせる
  （upstream の `.ts` フック → バイナリ呼出への写像は逸脱台帳 #1 の綴り写像に追記）。
  単一バイナリ配布（A1）と整合し、フックは thin な CLI 面 + ユースケース共有
- B. `harness-claude` 側の独立実行体として分離（バイナリが増える）
- X. Other (please specify)

[Answer]: A. マルチコールバイナリのサブコマンド（aidlc 1バイナリに hook 動詞。逸脱台帳 #1 の写像に追記）

## Q4. StateFile 所有の一本化（O3 / B-2 の前提確認）

- A. **WorkflowExecution が集約ルート**（identity = intent）で、StateFile（`aidlc-state.md`）は
  永続化媒体。`state_file_io` は `WorkflowExecutionRepositoryImpl` の内部部品に確定。
  01号 §3 の集約候補表から StateFile を落とす（B束 FR8.2 の修正と同時に実施）
- B. StateFile を独立集約として残す（3通りの所有主張を仕様上調停する別案が要る）
- X. Other (please specify)

[Answer]: A. WorkflowExecution が集約ルート（identity = intent）、StateFile は永続化媒体。state_file_io は WorkflowExecutionRepositoryImpl の内部部品に確定。01号の集約候補表から StateFile を落とす（FR8.2 と同時）

## Q5. 永続化パラダイム（チャット議論で確定 — Mode: chat）

[Answer]: イベントソーシングを採用する。j5ik2o/event-store-adapter-rs を前提とし、その型と API（persist_event_and_snapshot / get_latest_snapshot_by_id + get_events_by_id_since_seq_nr + replay、version 楽観ロック）に従う。SQLite 版 EventStore 実装を新規作成（本家には無いため。async trait を本家と同形でローカル定義し、将来の本家 feature 化/SQLite 実装貢献で合流可能な形を保つ）。当初案の「監査先行 WAL + 同期プロジェクション」「集約がイベント列を返すハイブリッド」はオーナー指摘（中途半端な設計の排除・1コマンド1イベントの規律）により棄却。

## Q6. イベント規律（Mode: chat）

[Answer]: 1コマンド1イベント（絶対）。集約コマンドは aggregate.method(p1, p2) → Result<DomainEvent, DomainError> で単一イベントを返す。upstream 監査台帳の複数行（GATE_APPROVED + STAGE_COMPLETED + フェーズ境界トリオ等）は1ドメインイベントからプロジェクションが描画する N 行であり、ドメインイベント語彙（コマンドと1:1）と upstream 監査行語彙（86語）は別物。リプレイは decide/apply 分離の apply（apply_event）で畳む。

## Q7. ストアとリードモデル（Mode: chat）

[Answer]: Repository → EventStoreImpl(sqlite client) → SQLite ← Read Model Updater（プロジェクション）。ジャーナルとスナップショットは SQLite（ライブラリのテーブル構造を SQLite に写す）。aidlc-state.md・監査シャード等の upstream 互換ファイルはすべてリードモデル（投影）。RMU はプロセス内の差分処理関数（AWS Lambda 型）— チェックポイント（ウォーターマーク）を SQLite に永続化し、コマンド末尾（Tx コミット後・プロセス終了前）に同期でキャッチアップ。クラッシュ時は次回呼出の差分処理が冪等に修復（真実源はジャーナル = B9 維持）。常駐プロセスなし。

## Q8. async（Mode: chat）

[Answer]: tokio を導入し async 対応は初期化（composition root）から行う。コントローラ・ユースケースは async、ドメイン（集約）は純粋・同期のまま。将来のウェブサーバ実装も見据えた判断。event-store-adapter-rs crate の直接依存は現状 AWS SDK + tonic + Bigtable クライアントがハード依存のため見送り、trait を同形でローカル定義する（本家が feature 化されたら乗り換え）。

## Q9. ロック機構の帰結（Mode: chat）

[Answer]: mkdir ロック機構は退役。ストア書込は SQLite Tx + 楽観 version、リードモデル書込は冪等生成 + 単一ファイル原子性（tmp+rename）、チェックポイントは Tx 内更新で足りる。FsWorkspaceLock・WorkspaceLock ポート・LockProtocol・reap_eligible は退役し、state_file_io は RMU の投影ライタ部品に転生。audit_lock.qnt は「ジャーナル/スナップショット/version/チェックポイント協定」（version 競合拒否・チェックポイント単調性・投影冪等性）の検証モデルへ改訂。逸脱台帳に並行制御置換（ロック dir 非生成）を登録。マルチクローン（複数 clone/worktree の並行利用。台帳はクローン別ファイルを git で交換する upstream 設計）は、stage-1 では単一クローン運用とし、書込は自分の SQLite のみ・他クローンのシャードは読み取り専用の外部ストリームとして読み側で合流。完全な複数クローン再水和の意味論は後続 intent へ。

## Consolidated Summary Confirmation

初回（2026-08-22T08:2x、Q1〜Q9）の確認は `Looks correct` で確定済み。以下は requirements-analysis 改訂後の
再入（成果物は Keep、ADR-005 のみ再エクスポート禁止裁定で改訂）に伴う再確認。

- イベントソーシング採用（event-store-adapter-rs 前提、SQLite 版 EventStore を新規実装、trait は本家同形をローカル定義）
- 1コマンド1イベント（絶対）。ドメインイベント語彙はコマンドと1:1、upstream 監査行はプロジェクションが描画する別語彙
- SQLite がジャーナル+スナップショット+チェックポイントのストア。upstream 互換ファイル（状態ファイル・監査シャード）はすべてリードモデル
- RMU はチェックポイント付きのプロセス内差分関数（Lambda 型、常駐なし、冪等キャッチアップ）
- async は初期化から（tokio）。ドメインは純粋・同期のまま
- ロック機構（mkdir/スタンプ/reap）は退役、SQLite Tx + 楽観 version へ置換。audit_lock.qnt は新協定の検証モデルへ改訂。逸脱台帳へ登録
- 集約 = FSM 統一ルール（Q2）・WorkflowExecution が集約ルート（Q4）は維持。Q1（イベントログ化）は本 ES 採用に吸収
- 波及: requirements.md FR1.1/1.2 の合格基準は後方ジャンプで改訂済み（FR1.1〜1.3・FR3.3・FR8.1・NFR1 注記・NFR3・§7 O1〜O3）
- 再入で追加: ADR-005（PlanAction 所有一本化）を re-export 併用から**完全移動**へ改訂（オーナー裁定 2026-08-22「利便性のための再エクスポートはどこでも禁止」— coding-rules/module-visibility.md 追補。components.md の R1 注記・design-audit R1・requirements.md FR8.3 も同文言に統一）
- 再入で追加: 上記以外の components.md / decisions.md / traceability.json は Keep（内容不変）

Does this all look correct before I regenerate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
