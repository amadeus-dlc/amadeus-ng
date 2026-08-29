# command/domain 全量監査 — 上流由来・読取専用・イベント非出現の型の分類（2026-08-29）

発端: オーナー裁定 ③「`WorkflowDefinition` の『読取モデル集約』は語義矛盾（リードモデル =
投影とコマンド側の整合性境界 = 集約は別物。読取専用・イベント無し・ジャーナル無しは集約では
ない）」。分類基準はオーナー指示どおり: **ジャーナルのイベント payload / 集約状態に現れる型が
正当な command 語彙**。

前提の確定裁定: ① `state()` → `snapshot()`（B12 委任済み）。② ~~`Intent::new` はイベントを返さなくてよい~~ → **上書き（2026-08-30 オーナー裁定）**: `Intent` は集約であり、genesis は `Intent::create(...) -> (Intent, IntentEvent::Created)` の対を返す（B12 改訂 8）。
リードモデルが欲しければ **`core/query/interface-adapter`** に置く（コントローラ・ゲートウェイ
を配置可。クエリ側ユースケース層は必要と立証されるまで作らない — GraphQL では不要が多い）。

## A. 正当（command 語彙 — イベント payload / 集約状態 / その部品）

orchestration: `IntentExecution` / `IntentExecutionId` / `IntentExecutionEvent`（全 payload 構造体）/
`IntentExecutionSnapshot`（私有）/ `Intent` / `IntentId` / `StageEntry`（slug・phase・plan_action・
conditional・display の 5 成分）/ `StageDisplay` / `WorkspaceScan` / `AutonomyMode` / `Status` /
`PhaseBoundary` / `JumpDirection` / `StageIndex` / `Verdict` / `SkeletonStance` / 各エラー enum /
`event_manifest`（直列化型判別子）。集約 API の入出力型（`NextDecision` / `NextRequest` /
`EngineSignal`）もクエリ戻り値として正当。

workflow_definition のうちイベントに現れるもの: `WorkflowDefinitionId` / `DefinitionRevision`
（`Started` の系譜参照）/ `StageSlug` / `PlanAction` / `PhaseId` / `StageNumber`（StageDisplay 部品）/
`BrownfieldGreenfield`（WorkspaceScan 部品）。

## B. 要再配置候補（ハーネス成果物の束ね・派生 — イベントにも集約状態にも現れない）

`WorkflowDefinition` / `StageGraph` / `StageNode`（+ Builder）/ `ScopeGrid` / `ScopeMetadata` /
`StageMode` / `ReviewClass` / `RuleScope` / `SkeletonDefault` / `ReviewCapValue` / `ConsumeDecl` /
`RuleInContext` / `SensorRef`（+ 対応する Unknown* エラー群）。

**使用実測（非テスト・非 doc）は 2 箇所だけ**:

1. `Intent::resolve(definition: &WorkflowDefinition, ...)`（intent.rs:126）— 誕生時の計画解決。
   `node.execution() == ExecutionKind::Conditional` で conditional フラグを導出（結果は
   `StageEntry.conditional` に焼き込まれ、以後 definition は不要）。`ExecutionKind` は
   ここで一時的に読まれるだけで保存されない。
2. `IntentExecution::next_decision` の同一性ガード（intent_execution.rs:1040）—
   `definition.id() != intent.definition_id()` の **id 比較 1 箇所のみ**。判断本体は
   self + intent で完結。**パラメータ除去可能**（intent が definition_id を持つため、
   ガードの意味は「呼出側が正しい定義を渡したか」だが、渡された定義を他に使わないなら
   ガード自体が不要になる）。

イベント/スナップショット/StageEntry のソースに現れる `WorkflowDefinition` / `StageNode` は
**すべて doc コメントの言及**（BR2.2「リプレイは WorkflowDefinition を要さない」の説明文）。

ポート: `WorkflowDefinitionRepository`（command/use-case）+ `Impl` / `InMemory`
（command/interface-adapter）。**Repository は集約専用**（gateway-taxonomy）なので、集約でない
以上は命名違反。実体は上流ハーネス成果物（stage-graph.json / scope-grid.json / scope カタログ /
harness.json）の読取。

## C. 別件の観測（本監査の基準では B に見えるが、既裁定があるもの — 裁定要否のフラグのみ）

- `workspace/audit_events`（EVENT_HEADINGS 86 語等）・`audit_field`・`audit_ordering`・
  `state_field_value`・`state_version`・`bolt_refs`・`shard_name`・`clone_id`・`space_name`・
  `intent_dir_name`: 投影（RMU）が使う読取側語彙だが、**B9 オーナー裁定**（shared 解体 —
  「語彙は domain へ。ドメインの pub 型がそのまま公開言語」）で意図的に配置されたもの。
  上流成果物データではないため今回の再配置候補には含めない。将来 query 側が育った時点で
  再評価。
- `directive_schema`（DirectiveKind）: CLI 出力の Published Language。同じく B9 で domain へ。
  同上の扱い。
- `StorePath` / `CheckboxState`: イベント・スナップショットに現れるため A。

## specs/12 の同時監査

- §2.1: `WorkflowDefinition` を「**読取モデル集約**」と記述 — 語義矛盾（オーナー判定）。
- L37: 「`ScopeGrid` と `StageNode` は値オブジェクト。`StageDefinition`（stage file）と
  `AgentPersona` は**スライス 2 の集約**」— 同じ病理（読取専用の上流文書を集約と呼ぶ）。
  スライス 2 は未実装なので実装被害はまだ無い。

## 修正方針の選択肢（論点 1〜3、オーナー裁定待ち）

**論点 1 — 置き場所と呼称**:
- 案 A（推奨）: B 群を **command/interface-adapter の所有へ移す**（誕生時入力の解析・束ねは
  上流成果物クライアントの内部表現）。`Intent::resolve` の入力は domain 所有の**最小材料型**
  （例: 解決済みステージ行の列 — slug・phase・conditional・display・スコープ別 plan_action）へ
  差し替え、domain から B 群への参照を消す。`next_decision` の definition パラメータは除去。
  クエリ側で定義データの表示が要る日が来たら core/query/interface-adapter のゲートウェイが
  同じ成果物を読む（オーナー裁定どおり）。
- 案 B: B 群を domain に残し「参照データの値（集約ではない）」へ再分類、名前だけ直す。
  移動なしで安いが、「配置から再考」というオーナー指示に応えない。

**論点 2 — ポートの再命名**: Repository を剥がし、上流システム名を冠したクライアントへ
（例: `HarnessArtifactsClient` — 読む対象がハーネス成果物 4 ファイルであることを名で言う）。
ポートは（将来の intent-create）ユースケース層、実装は interface-adapter（DIP 不変）。

**論点 3 — specs/12 の書き直し**: 「読取モデル集約」の語を除去し「ハーネス成果物の参照
データ（値）」へ。スライス 2 の `StageDefinition` / `AgentPersona` からも集約の語を剥がす。
ADR-008（系譜 ID + 内容版）の実質は維持。

**実施タイミングの推奨**: B12（分割・改名）とは分離し、**B13** として実施。B12 内の
`Intent::resolve(&WorkflowDefinition)` は暫定のまま着地させ、B13 で入力型を差し替える
（B12 は既に 6 改訂を吸収しており、これ以上の同時変更は検収可能性を下げる）。

---

# 裁定（2026-08-29・オーナー）— 論点 1〜3 はすべて次で確定

- **呼称**: 「読取モデル集約」（および「読取専用集約」）の呼び方を**廃止**。**集約に統一**。
  CQRS のリードモデルと紛らわしい造語だった。
- **`WorkflowDefinition` は集約である**。したがって変異が本システムのスコープに入った時点で、
  状態遷移はイベントを吐かなければならない（`aggregate-commands.md` がそのまま適用される）。
  現スコープでは本システムから変異させないためコマンド未実装 — 規則適合は空虚に成立。
- **実ファイル（stage-graph.json / scope-grid.json / scope カタログ）がこの集約のリード
  モデル**である。現スコープでは upstream のコンパイラがそれを生成する（投影の役割を
  upstream ツールが担っている）。実ファイルは現状この集約の唯一の永続表現も兼ねるため、
  `WorkflowDefinitionRepository` がそこから再構成するのは自集約の I/O であり違反ではない。
- **配置は一旦現状維持**（B 群の command/domain 残留・ポート名 `WorkflowDefinitionRepository`
  維持 — 集約なので gateway-taxonomy「集約名 + Repository」に**適合**しており、改名論点は消滅）。
- 呼称の残存 3 箇所（specs/12 §2.1・aggregate-commands.md 射程・components.md）は
  是正済み（2026-08-29）。

本監査の A/B/C 分類は事実の記録として残すが、B 群の「要再配置候補」という結論は上記裁定で
**上書き**（現状維持）。将来 WorkflowDefinition の変異（プラグイン導入・再コンパイル等）が
要件化した時点で、ジャーナル導入・実ファイルの投影化・イベント設計を改めて裁定する。

## 追補（2026-08-29・オーナー指示による実測確認）— 「作ったら不変」は誤り

`WorkflowDefinition` の実ファイルは**更新される**:

1. **実行時変異**: コンポーザが承認済みカスタムスコープを登記へ追記
   （`scopes/aidlc-<name>.md` 新規 + `scope-grid.json` エントリ追加 — SKILL.md が公認の
   書込経路と明記。mid-workflow の compose でも発火）。
2. **再コンパイル**: `aidlc-graph.ts compile` が stage-graph.json を再生成（繰り返し前提の
   採番設計）。
3. モデル自身が変化前提: `WorkflowDefinitionId` = 内容が変わっても不変の系譜 ID、
   `DefinitionRevision` = 内容 sha256 の値属性、`Started` が実行ごとに両方をピン（ADR-008）。
   specs/12 の「構築後 immutable」はプロセス 1 起動内のインスタンス不変の意。

**含意**: スコープ合成は現に起きている状態遷移だが、ドメインイベント無しのファイル追記で
実現されている（監査痕跡はエージェント作業記録のみ）。オーナー裁定「集約としてイベントを
吐くべき」は将来要件ではなくこの欠落の指摘であり、スコープ合成をコマンド側へ取り込む時点で
`ScopeComposed` 等のイベント設計 + 実ファイルの投影化が必須になる（後続 intent の設計課題
として申し送り）。

## 設計方針の承認（2026-08-29・オーナー「ぜひそういう設計にしてほしい」）

`WorkflowDefinition` の将来形は**オーナー承認済みの設計方針**として確定（裁定待ちの申し送り
から格上げ）:

- `&mut self` コマンド（例: `compose_scope(&mut self, ...) → Result<WorkflowDefinitionEvent, _>`
  → `ScopeComposed`）で状態遷移し、1 コマンド 1 イベント（`aggregate-commands.md`）。
- `WorkflowDefinitionEvent` の語彙、自前ジャーナル（aid `WorkflowDefinition-{id}`）と
  楽観 version。
- **実ファイル（stage-graph.json / scope-grid.json / scope カタログ）は RMU の投影**として
  描かれるリードモデルになる。
- `DefinitionRevision` は従来どおり Repository / 投影側が内容から計算して付与
  （ADR-008「ドメインは計算しない」は維持）。

実施時期: stage-1 スコープ外。スコープ合成（コンポーザの登記書込）をエンジンのコマンド側へ
取り込む後続 intent で実装する。それまでの現状（find のみ・実ファイルが永続表現を兼ねる）は
経過状態として維持。
