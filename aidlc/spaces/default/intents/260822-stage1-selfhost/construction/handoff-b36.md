# park ハンドオフ — b36 途中（2026-09-02）

オーナー指示 `park` による一時停止の記録。再開時はこのファイルから読み始める。

## 現在地

- **main**: b35 まで着地済み（b31 #81 / b32 #83 / b33 #84 / b34 #86 / b35 #87 — すべてマージ済み）
- **作業ブランチ**: `bolt/b36-compiled-definition`（push 済み・PR 未作成）。WIP は
  `ffd6d077`（コード）+ `9c4e603e`（canon）で保全済み

## b36（CompiledDefinition 昇格）— 実装完了・最終ゲート未走

**裁定**（オーナー 2026-09-02、対話で段階確定）:

1. **配布束は同一システムのドメインモデル** —「クライアントをリポジトリに、クライアントが
   扱うデータを集約に昇格」。外部システムクライアント／暫定の足場という位置づけは消滅
2. **集約は自前の ID 型を持つ** — `CompiledDefinitionId` 新設（「集約やリポジトリとは何かを
   考えろ。これはこのシステム固有の知識ではない」）
3. **store は他リポジトリと同形の (イベント, 集約) 対** —「他のリポジトリの契約みろ」。
   genesis `CompiledDefinition::compile` が対を返し、`CompiledDefinitionEvent::Compiled` が
   内容全量を運ぶ（Defined の鏡）
4. **store は実装済みのまま残す**（TODO 化しない — オーナー確認済み。todo!/unimplemented! は
   deny lint。golden バイト往復が毎 CI の回帰ガードとして機能する）
5. **`IntentRepository::store` の `occurred_at` 引数は削除**（「他のリポジトリではないよ」）—
   `Intent.created_at` 化・`From<(Created, occurred_at)>`（b30 の Defined 対の鏡）で実装済み

### 完了済み（WIP コミットに含まれる）

- domain: `CompiledDefinition`（集約）/ `CompiledDefinitionId`(+Error) /
  `CompiledDefinitionEvent::Compiled`（ハイブリッド族）/ genesis `compile`（対）/
  再構成 `new`。`StageNode.produces_kinds` を**文書順保持の Vec ペア**へ（BTreeMap が
  順序を握り潰していた実欠陥 — 書き手実装が発見）
- use-case: `CompiledDefinitionRepository`（find_by_id/store 両動詞・対契約）、
  `DefineWorkflowUseCase`（2 集約 2 ID の正規形）、`DefineWorkflowError` 改形、
  スタブ `InMemoryCompiledDefinitionRepository`
- adapter: `CompiledDefinitionRepositoryImpl`（旧 ClientImpl 改形。**store は dist と
  バイト完全一致** — FIELD_ORDER 28 emit 構造体・grid 手組み・scopes frontmatter・
  harness 最小形。golden バイト往復テスト緑）、`kinds_codec`（順序保存 map codec）
- Intent 時刻化: created_at フィールド・両側 IntentDto の wire 追随（逐語リテラル更新済み）・
  封筒時刻は集約から・波及フィクスチャ全修正
- runtime: 両 ID 鋳造 + ensure_defined 配線
- canon: gateway-taxonomy（§1 再是正・§2 行・§3 決着・§5 行）/ cqrs-boundaries（規則 7 —
  配布束は集約、stage-graph.json は「二つの顔」）

### 検証状況

- workspace コンパイル緑・RMU lib 161 緑・golden バイト往復緑・impl/use-case 単体緑
- **全ゲートチェーン（fmt/clippy/lint/test 全走/coverage 相対）は park により中断 — 未完走**。
  直前の走で残っていた failure は RMU 逐語リテラル 7 本のみで、修正済み（161 緑で確認）

### 再開手順

1. `bolt/b36-compiled-definition` を checkout し本ファイルと WIP 2 コミットを確認
2. 全ゲートチェーン（fmt --all → clippy workspace → cargo lint → cargo test --workspace →
   coverage --base origin/main）を完走させる
3. 緑なら PR（タイトル案: 「b36: CompiledDefinition 昇格 — 配布束を集約に、Client を
   リポジトリに」。本文: 裁定 5 点・バイト忠実 store・produces_kinds 順序修正・
   occurred_at 統一）→ 収束 → merge queue
4. マージ後: #79（§1-1 の「リードモデル」宣言の読み替えを追記）/ #80（重複の位置づけ更新）/
   #85 とキュー（#74 park 本体 — b36 に割り込まれて未着手・#73 ほか）の選択に戻る

## 積み残し（変わらず）

#70 / #71 / #72 / #73 / **#74（park 本体 — b36 割り込みで未着手）** / #75 相当なし /
#76 済 / #77（doctor 本体）/ #82（coverage ジッタ — 対応案 1 の優先度上げ提案済み）/
#85（存置 or 撤去裁定待ち）
