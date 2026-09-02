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

- #70 / #71 / #72 / #73 / **#74（park 本体 — b36 割り込みで未着手）** / #75 相当なし
- #76 済 / #77（doctor 本体）/ #82（coverage ジッタ — 対応案 1 の優先度上げ提案済み）
- #85（存置 or 撤去裁定待ち）

## 再開（2026-09-02、同日）

再開手順 1〜3 を実施した。本節が最新の現在地であり、上の「再開手順」は消化済み。

### 実施したこと

1. **ブランチ整理**: b34 起点で b35 の未 squash コミットを含んでいたため、`git rebase --onto
   origin/main cf2390be` で b36 の 4 コミットだけを `origin/main`（b35 #87 squash 済み）へ移植
   （ツリーは旧 tip と同一。監査シャードのみ差分）。
2. **ゲートチェーン完走**: fmt / clippy / `cargo lint` / `cargo test --workspace`（48 スイート
   1459 本）緑。coverage は絶対床（90%）は通過、**相対ゲートが当初赤**（head 98.69% < base
   98.97%）— 旧ポート 3 ファイル（100% カバー）の削除で分母が減った分と、`store` 経路の未通過
   分岐が原因。→ テスト追加と防御分岐の共通化で **head 98.974% ≥ base 98.971%** に回復し緑（同日、5 ゲート全緑）。
3. **レビュー是正（Fable メインセッション）**:
   - clippy `disallowed_methods`: `store` の書き手が `serde_json::to_string(_pretty)` を直接
     呼んでいた（BR1.7 違反）→ `canon_json::to_value` + `serialize(ContractPretty)` の 1 経路へ。
     grid の手組み文字列も `Serialize` 実装（挿入順保存 — BR1.8）へ置換。golden 往復は緑のまま。
   - `CompiledDefinition::new`（genesis 以外の第 2 の構築口 — aggregate-commands「再構成の形」
     違反）を撤去し、`Intent` / `WorkflowDefinition` と同型の `From<Compiled>` に統一。Repository
     の読取経路は復号内容を `Compiled` に束ねてこの変換を通す。
   - 「`DefinitionArtifactsClient`」「暫定の足場」の古い記述（use-case ファサード・アダプタ doc・
     テスト名・canon 2 箇所）を b36 裁定に追随。仕様 12 §2.1 / 01 §3 の集約表に
     `CompiledDefinition` を追加（Repository 名は集約表から取る規則の根拠）。
   - 合成ルートの 2 ID を同じ源 `harness_name` から鋳造（doc の主張と実装を一致）。
   - 旧セマンティクス前提の結合テスト 2 本を Repository 契約に追随（ID 不一致 = `NotFound`、
     `Corrupt` の `Display` は分類・材料を漏らさない）。
   - coverage 回復のテスト追加: scope ファイルの書出し/掃除と往復、対の不一致拒否、I/O 失敗
     3 形、削除失敗（unix 権限）、`kinds_codec` 順序往復、`CompiledDefinitionId::try_from`、
     `DefineWorkflowError::from`、`PlanAction::flipped`。`store_corrupt` / `revision_failure` の
     共通化で防御分岐を 1 箇所ずつに畳んだ。

### PR #88 の収束中に加わった裁定（2026-09-02）

- **集約は FSM**（オーナー「CompiledDefinition これ状態遷移しないってこと？」→「今の実装は途中段階か。
  推奨で早めに修正したほうがいい」）: `recompile` / `register_scope` / `apply_plugin_selection` の
  3 遷移とガード、decide / apply 分離、`replay` なし（媒体がスナップショット）。12 §2.1・
  aggregate-commands 適用例に記録。
- **内容版はドメインが導出**（推奨 A で進行、ADR-008 改訂として decisions.md に追記）:
  `DefinitionRevision::of_content`。Repository の生 JSON ハッシュを撤去。golden の
  `the_shipped_revision_is_reproducible_from_the_pinned_bytes` は 2 回読取の同値比較なので固定値なし。
- **系譜照合は受け手の集約が持つ**: `WorkflowDefinition::define(id, &bundle, at)` /
  `redefine(&bundle, at)` + `LineageMismatch`。
- Bugbot 2 件・CodeRabbit 7 件は返信・resolve 済み（系譜照合 1 件は FSM 着地後に閉じる）。
  `store` はファイル単位の原子的書込、既存 `harness.json` の付随キーと scope `.md` の本文を保持、
  override 先へ書込、一覧失敗は `Io`。

### 次

PR 作成 → 収束ルール（project.md Corrections）で畳む → merge queue。マージ後は上の
「再開手順 4」（#79 / #80 追記、#85 とキューの選択）へ。
