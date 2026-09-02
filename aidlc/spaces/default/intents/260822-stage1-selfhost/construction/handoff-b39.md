# ハンドオフ — 是正 Bolt 2 前半 / b39: RMU の構造化投影（`read_*` 表）と `Started` の自己完結化（2026-09-02）

設計書: [`b39-rmu-read-tables/design.md`](b39-rmu-read-tables/design.md)。正本: 仕様 11 §4.1、ADR-011、
`aggregate-commands.md`（genesis イベントは集約 id と材料を運ぶ）、`cqrs-boundaries.md` 規則 6 追記。

## やったこと（3 スライス）

- **A（ドメイン）**: `Started { id, intent_id, stages }`。`IntentExecution: From<(Started, DateTime<Utc>)>` が
  唯一の genesis 状態導出（`new` 完全コンストラクタへ委譲 — 構造体リテラルは `new` の 1 箇所）。
  `start` はそれを通る。追加クエリ `first_in_scope_of_phase(phase)`（実効プランで判断）。両側 `StartedDto`
  を同一ワイヤ形式で更新（`StageEntryDto` は各側 `intent_dto.rs` の `pub(super)` を共有）。
  **ローカルの `.aidlc-store.sqlite` は旧 `Started` を復号できなくなる（`Corrupt`）— 再鋳造で対応。**
- **B（RMU 定義ストリーム）**: `DefinitionEntry`、`JournalBatch::new(executions, intents, definitions, scanned_to)`、
  読む側の定義 DTO（`WorkflowDefinitionEventDto` / `DefinedDto` / `RedefinedDto` / `definition_content_dto.rs`
  に子 5 型同居 / `kinds_codec.rs`）。`decode_definition_row`（`Defined` の id と行 `aid` の照合、
  `Redefined` は行 `aid` が id）。「暫定の読み飛ばし」撤去。app の横断適合テストで両側のバイト一致を固定。
- **C（投影核・SQL・Tx）**: `read_tables`（`ReadTables::project(&JournalBatch)` 純粋核、行型 13、
  `RequestKind`、`ReadTablesError { MissingGenesis, IntentUnavailable }`、内部 `spelling` / `json_column` /
  `sql` / `stage_lookup`）。`JournalReader::advance_checkpoint(projection, to, &ReadTables)` が行の全差し替えと
  チェックポイント前進を 1 Tx（`BEGIN IMMEDIATE`）。`JournalReaderImpl::open` が `CREATE TABLE IF NOT EXISTS read_*`。
  `catch_up` は差分非空のとき全履歴（チェックポイント ZERO なら差分 = 全履歴）から再計算する。

## 設計上の要点（初見向け）

- 行の値はすべて集約のクエリの写し。RMU が持つのは「どのキーでどのクエリを呼ぶか」の列挙だけ。
- 全再計算 + 全差し替え（差分投影ではない）。ジャーナルは 1 ワークスペース分で小さい。増分化は必要になった時点で。
- Markdown 面（`aidlc-state.md` / 監査シャード）は従来どおり差分投影で、単一 intent 契約（`MixedIntents`）も従来どおり。
  構造化面は複数 intent / 複数実行をキーで自然に扱う。

## 次（b40 — Bolt 2 後半、`read-model-spec.md` §4.3〜4.5）

1. `read_run_stage`（run-stage 指示の材料一式。パスはハーネス相対、`directive_digest` / `route_digest`）、
   `read_scope_change`、`read_config_current`。
2. steering: `SteeringPlan::pack`（クエリ側 `steering_plan.rs` の分割・パック約 120 行）を RMU の投影ヘルパへ移設、
   `read_steering_plan` / `read_steering_part`、参照入力（memory 規則ファイル）のダイジェスト比較リフレッシュ
   （`ProjectionTargets` に memory_dir 相当の入力を足す — 読取入力なので型を分ける）。
3. `read_definition` の `default_scope` / stock 判定は Bolt 3 の必要に応じて（upstream 定数 `DEFAULT_SCOPE = "classic"`
   はクエリ側 `scope_resolution.rs` にある）。

その後 **#7 キュー 2b（#85 = A、非ゲート完了パイプラインの撤去）** → **Bolt 3（クエリ側縮小）**。
