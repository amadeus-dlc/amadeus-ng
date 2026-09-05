# U2 設計本文の是正記録

2026-09-05。対象は `functional-design/entities.md`、`rules.md`、`functional-spec.md`、
`traceability.json` の現行本文と本記録のみ。正式なレビュー判定・ステージ承認・完了レシートではない。
質問回答、pending-revision、Review の過去記録、audit / state / memory は変更していない。

## 根拠と優先順位

観測互換を最優先し、`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md`
の優先順位に従った。参照した正本は同ディレクトリの
`aggregate-commands.md`、`aggregate-references.md`、
`domain-persistence-neutrality.md`、`cqrs-boundaries.md`、`gateway-taxonomy.md`。
過去の設計本文やコード冒頭の古い説明を、後続裁定より上位には扱っていない。

| 後続裁定・観測 | 本文へ反映した内容 |
|---|---|
| Intent / IntentExecution 分離、ID 参照 | 依頼・定義参照・静的計画は Intent、実行状態は IntentExecution。両 ID は別型の UUIDv7 |
| 2026-08-29〜08-30 の集約コマンド・永続化中立 | 1コマンド1イベント、genesis は対、domain の serde / ESA trait / memento 双子型を撤去 |
| 2026-09-02 のイベント ID | 自前 UUIDv7 id と aggregate_id を分離。seq_nr / occurred_at は封筒から apply 引数へ渡す |
| 2026-09-05 の再生方式訂正 | 最新 snapshot の状態を基底に、通番がそれより大きい差分だけを適用。全履歴の常時再生を再導入しない |
| Quint v2.1 / v2.2 | forward の介在は in-scope 条件付き。initialization は誕生時完了 |
| Quint v2.3〜v2.7 | 再 park、隔離実行、skeleton stance、レビュー・昇格受領証、park中/Completedでの自律切替を反映 |
| human presence | モデル外の条件として HumanTurns / last_gate_resolution_at とガードを明記 |

`aggregate-commands.md` に残る構築 API の制約は、同ファイルの2026-09-05訂正が明記する
「保存済みスナップショットの状態復元を禁止しない」という範囲で適用する。
新しいドメイン双子型や永続化属性を導入する許可には広げていない。

## 所見ごとの是正

対象は functional-spec.md 末尾の2026-09-05レビュー。元の所見・Status・Verdictは保存したまま、
以下に文書修正を記録する。ここでの是正は正式なレビュー所見のクローズを意味しない。

| 所見 | 修正 |
|---|---|
| 8 | stale_report を Result<(), CommandError> に訂正。冪等完了応答は呼出側、イベントなし |
| 9 | PlanAction 検査先を modules/core/command/domain/src/orchestration に変更。不在・検索エラーは失敗 |
| 10 | 旧10ファイル一覧を B3 の過去実績と明示。現行影響範囲は変更時に検索し直す |
| 11 | Quint v2.7 の状態・会計・指令射影とモデル外条件を rules 第3節へ整理 |
| 12 | next_decision の &Intent は skeleton 判断に使用すると訂正。未使用の定義引数という注記を撤去 |
| 13 | YAML 正本の型集合と既存型の所有を同期。壊れた ER 図を同じ型集合の所有関係表へ置換 |
| 18 | backward は target+1 以降を Pending、target は InProgress と分離 |
| R-21 | 集約・ID・静的計画・実行状態の所有を更新。WorkflowDefinition を通常の ES 集約とする |
| R-22 | 16変種・痩身後ペイロード・イベントID・封筒・DTO・applyの失敗境界・読取版を同期 |
| R-23 | complete_stage / StageCompleted の旧経路を現行手順から撤去。誕生時完了と縮退形を明記 |
| R-24 | コマンド別ガード表を作成。再 park、自律切替、受領証と承認前提、全 jump の受領証リセットを同期 |
| 追加確認 | DefinitionRevisionの導出を旧アダプタ計算からCompiledDefinition自身のof_contentへ訂正（ADR-008改訂2026-09-02） |

## 照合した API と境界

以下はリポジトリルート基準の実在ファイル。

- `modules/core/command/domain/src/orchestration/intent.rs`:
  `create` の計画解決・検査、定義 ID / revision、Created への材料搭載。
- `modules/core/command/domain/src/orchestration/intent_execution.rs`:
  `start`、`From<(Started, DateTime<Utc>)>`、`new`、`replay`、
  `apply_event`、`with_version`、`stale_report`、`next_decision`、
  通常コマンド・レビュー会計・昇格・自律切替のガード。
- `modules/core/command/domain/src/orchestration/intent_execution_event.rs` と同名サブディレクトリ:
  16変種とペイロード。方向・次位置・revision_count を再生側が導出すること。
- `modules/core/command/domain/src/orchestration/stage_entry.rs` / `stage_key.rs`:
  静的計画と実行の添字帳の分離、計画検査。
- `modules/core/command/domain/src/workflow_definition/workflow_definition.rs`:
  define / redefine / replay、既存6述語と PlanAction の所有。
- `modules/core/command/interface-adapter/src/orchestration/dto/intent_execution_dto.rs`:
  serde は DTO、検査付き to_domain は IntentExecution::new を通る。
- `modules/core/command/interface-adapter/src/orchestration/intent_execution_repository_impl.rs`:
  最新 snapshot の基底、後続差分、封筒・通番・aggregate_id 検査、with_version。
- `modules/core/command/use-case/src/orchestration/port/intent_execution_repository.rs`:
  store はイベントと適用後集約を受け Result<(), RepositoryError>。継続書込は版の再読込が必要。
- `formal/orchestration/engine_loop.qnt` と
  `modules/core/command/domain/tests/engine_loop_conformance.rs`:
  v2.7 の射影とガードの範囲。

## 検証

本是正は文書だけを変更した。全ワークスペーステストは親作業で実測済みであり、ここでは再実行していない。
過去 Review の167件の実測も当時の結果として保持し、本編集の再実行結果とは呼ばない。

以下の検査を本編集後に実行し、全て成功した。

| 検査 | 結果 |
|---|---|
| PyYAML による正本2件の構文検査 | PASS |
| エンティティの関係先と規則の対象型参照 | PASS |
| BR ID の一意性・traceability の対応先 | 25件の規則、8要求の参照が全て解決 |
| イベント一覧と現行 enum の網羅対応 | 16変種が完全一致 |
| 現行本文・是正記録の相対 Markdown リンク | 全て実在 |
| Review 追記部の編集前後の文字列比較 | 完全一致 |
| PlanAction の現配置・ファサード | 検索先の実在を確認、orchestration の定義・再輸出0件 |
| WorkflowDefinition の述語 | 残す6件が実在、撤去2件の公開定義なし |
| git diff --check、置換文字検査 | PASS |

Mermaid は新規生成・変更せず、存在しない型を指した図を表へ置換した。

## 残る差異と判断の限界

`next_decision(&Intent, &NextRequest) -> NextDecision` は、現行実装で入口の ID 不一致を Err にしない。
`skeleton_gate_stage` は ID・計画長の不一致を None にするため、単に古いメソッド名が残った問題とは異なる。
`aggregate-references.md` の「受け取り側で照合して不一致を Err で拒否する」という原則は維持し、
この API を無条件に原則適合とは記録しない。今回、新たな破損・誤判断の実行例は再現しておらず、
重大な実装不具合を新規認定したものでもない。クエリの参照束縛をどの境界で保証するかは、
所有範囲を越える API 整理の残件として明示した。

現行の呼出経路は追加で確認した。`modules/core/read-model-updater/src/read_tables.rs` は
`intent.id() == execution.intent_id()` で対応するIntentを引き、存在しなければIntentUnavailableを返す。
その同じIntentをNextAnswerRowへ渡すため、現在のRMU経路で別IDが渡るとは認定しない。
この利用経路の束縛と、公開next_decision自体に不一致Errがないという規律上の差異は区別する。

コード冒頭には「version を持たない」「panic しない」「memento」「定義引数不要」といった
旧世代の説明が残るが、本是正は実署名・実処理と後続裁定に照合した。
これら所有範囲外のコメントを、現行本文へ取り込んだり今回編集したりはしていない。
