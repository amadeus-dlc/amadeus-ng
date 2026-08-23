# workflow-definition コンテキスト仕様

> **位置づけ**: コンテキスト別仕様の第 3 号。スライス 1 = **グラフリーダ契約**（コンパイル済み成果物の読取面）に範囲を限定する。`01-domain-model.md` の裁定（B1・B6・B7・B11）と D3/D4/D10、ADR 0001〜0004 に従う。
> **契約コーパス**: upstream `01-workflow-model.md`（主）、`02-orchestration-engine.md` §4-5・`04-stage-protocol.md` §2・`06-sensors.md` §3・`08-memory-rules-learnings.md` §2・`11-plugin-system.md` §7（従）。精密抽出は [`research/workflow-definition-graph-reader.md`](research/workflow-definition-graph-reader.md)（3 入力の形状・読込失敗態度・述語・**証拠格付け [S]/[F]/[G]**）に収録済み。本書は**構造の規範**を担い、逐語の完全列挙は抽出文書と upstream を正とする。
> **状態**: ドラフト（フェーズ A。スライス 1 = グラフリーダ契約のみ。`compileStageGraph`（B6）・エージェントペルソナ・3 ダイヤル（Depth / TestStrategy / Tier）・キーワード推論はスライス 2 以降）
> **策定日**: 2026-08-22

---

## 1. 責務と境界

workflow-definition は「**何を実行しうるか**」の静的定義を所有する。5 Phase / 33 Stage / 11 Scope、エージェントペルソナ、3 ダイヤル、そして唯一の YAML→JSON 変換である `compileStageGraph` がここに属する。

**本スライスの範囲は読取面に限る**。すなわち、ビルド時にコンパイルされた 3 つの入力（`stage-graph.json` / `scope-grid.json` / `<harnessDir>/scopes/aidlc-*.md`）を読み、他コンテキストへ述語として供給するところまでを規範化する。コンパイラ本体（書き）は同じコンテキストの所有物だが、スライス 2 で扱う。

境界の要点（01 の裁定の引き受け）:

- **Published Language の第 1 号**（01 §2）: コンパイル済み `stage-graph.json` / `scope-grid.json` は workflow-definition が orchestration / verification へ公開する契約であり、D6 により upstream 互換で凍結される。**配布物そのものが契約**であって、本コンテキストの内部表現ではない。
- **B1**: scope grid は本コンテキストの**不変の成果物**であり、orchestration は読むだけである。裏返して言えば、`effectivePlanAction` の合成（recompose オーバレイが静的グリッドに勝つ read model）は orchestration の所有物 — 具体的には集約 `WorkflowExecution` の `effective_plan` — であり、本コンテキストが供給するのは**グリッド側の半分**（「このスコープ列でこの slug は EXECUTE / SKIP / 未収載のどれか」の 3 値照会）だけである（§2.3、設計監査 R2）。
- **B6**: `compileStageGraph` は本コンテキストの純粋ドメインサービス。distribution と plugin は共に customer で、失敗時の補償は各呼び出し元の責務。スライス 1 は compile を実装しないため、**書き側の契約**（`FIELD_ORDER` 28 のキー順、`contract-pretty` のバイト体裁）は §10 に前提として記録するに留める。
- **B7**: `review_class` の列挙と契約的意味は verification が正準所有する。グラフノード上の `review_class` は**外部キー参照**であり、本コンテキストは値をそのまま運ぶ。
- **B11**: walking skeleton の stance 解決は orchestration のプロセス。**アンカー計算**（スコープで最初の Construction EXECUTE ステージ）は本コンテキストの純関数 `firstInScopeStageOfPhase` であり、orchestration の recompose ガードがこれを呼ぶ。
- **Quint の適用外**（00-policy A9 / 01 §3.1）: 本コンテキストの不変条件は状態遷移ではなくコンパイル出力の構造的性質なので、E4 は付さず **proptest** で網羅する。

## 2. ドメイン層

### 2.1 集約

| 集約 | ルートと内包 | トランザクション境界 |
| --- | --- | --- |
| `WorkflowDefinition` | **本コンテキストの集約ルート**。識別子 `WorkflowDefinitionId`（`<harnessRoot>/tools/data/harness.json` の `name`。**内容が変わっても不変の系譜 ID**）と内容版 `DefinitionRevision`（`{ stage_graph, scope_grid, scopes }` の正準 JSON の `sha256:`。識別子ではなく**値属性**）を持つ。どちらも Repository 実装が付与し、ドメインは計算しない（ADR-008、Bolt B3 実装 `workflow_definition_id.rs` / `definition_revision.rs`）。3 入力（`stage-graph.json` / `scope-grid.json` / scope カタログ）を束ねた読取モデル集約で、`StageGraph`（コンパイル出力の成果物値。`Vec<StageNode>` の**文書順を保持** ＋ `StageSlug → index` の索引）と `ScopeGrid`、および `ScopeDefinition` 群を**内包する**。構築後は immutable | スライス 1 に変異は無い（読取専用）。3 入力は compile が lockstep で出すため、**束ね直しの単位は常に 3 入力まとめて 1 回**（片方だけ新しい状態は upstream でも想定外）。「返した配列を呼び出し側が mutate してはならない」という upstream のコメント規約は、Rust では所有権と不変参照で構造的に成立する |

内包物の位置づけ:

- `StageGraph` は集約内の**成果物値**（独立した集約ルートではない — グラフだけを単独で load / save する経路を作らない）。
- `ScopeDefinition`（identity ファイルの frontmatter とグリッド 1 列の join。**存在の権威は identity ファイル**で、グリッド列は権威ではない — §3.3）は集約に内包される。
- `ScopeGrid`（グリッドファイル全体）と `StageNode` は値オブジェクト。`StageDefinition`（stage file = frontmatter ＋本文）と `AgentPersona` はスライス 2 の集約。

**`WorkflowDefinition` を集約ルートへ昇格させた理由**（2026-08-22 オーナー裁定）: 第一の理由は**一貫性の単位**である — 3 入力は compile が lockstep で出すため、片方だけ新しい状態は upstream でも想定外であり、束ね直しの単位は常に「3 入力まとめて 1 回」になる。集約とは不変条件を守る一貫性の境界なので、この束が集約である（設計監査 C10）。第二の理由として命名規則がある: Repository は「集約名 + Repository」で名付けるため（[`coding-rules/gateway-taxonomy.md`](../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)）、3 入力を束ねる読取面のポートに名を与えるには束ねた結果そのものが集約でなければならない — `StageGraphRepository` は**ファイル名由来の名前**（格納形式は Repository 実装の内部詳細）であり規則違反になる。

### 2.2 Domain Primitive（E1/E2 の受け皿）

| 型 | 定義 | 強制 |
| --- | --- | --- |
| `StageSlug` | `^[a-z][a-z0-9-]*$`。グラフ内で一意、ファイル名 stem と一致 | E2 |
| `StageNumber` | `"<phaseIndex>.<seq>"` の**文字列**。数値化・正規化しない（`"3.10"` を `3.1` にしない）。順序比較は `numeric_stage_order`（`"P.I"` を整数 2 個に割り、P → I の順で比較。`"1.10" > "1.9"`）のみで行う | E1（構成関数）＋E2 |
| `PhaseId` | 5 値閉集合＋全順序（`initialization=0` … `operation=4`） | E1 |
| `ExecutionKind` | `ALWAYS` / `CONDITIONAL`。**プラン所属（EXECUTE/SKIP）ともゲート軸とも直交**することを型コメントではなく本仕様で明記する | E1 |
| `StageMode` | 5 値閉集合。`agent-team` は**予約 variant として明示的に保持**し、既定経路へフォールスルーさせない（§4 の拒否点） | E1 |
| `PlanAction` | `EXECUTE` / `SKIP`。**グリッド未収載は第 3 の値ではなく `Option` の `None`** で表す（§4 の 3 値）。**所有は本コンテキスト**であり、orchestration（10 §2.2）と 01 §3.1 は参照するだけで再定義も再輸出もしない（ADR-005、Bolt B3 実装） | E1 |
| `ScopeName` | スコープ名。生のまま `join()` に到達してはならないパスセグメント | E2 |
| `ArtifactName` | 成果物**語彙名**（パスではない）。kebab-case | E2 |
| `UnitKind` | `produces_kinds` の値要素（`service` / `ui` / `packaging` / `library` 等）。**マップに無い成果物は全 kind に適用**という既定は写像関数側の規範 | E1 |
| `RulesInContextEntry` | `{path, scope: org\|team\|project\|phase}`。**オブジェクトのまま保持**し文字列配列に潰さない。`path` は `default` スペースへのコンパイル時ピンで、実行時に再解決しない | E1 |
| `SensorResolution` | `{id, path, matches?}`。同上（オブジェクト配列） | E1 |
| `ReviewerMaxIterations` | 正整数。`reviewer` の存在を要求（ADR 0001 決定 4 により整数型で持つ） | E2 |
| `ReviewClass` | 正準は verification 所有（B7 — `none < advisory < adversarial` の low-wins 束、01 §125）。**スライス 1 では verification クレートが未新設のため、ノード欄の 2 値宣言型（adversarial / advisory）と scope frontmatter の 3 値 cap 型を本コンテキストに暫定ホスト**する。verification クレート新設（項目 3 スライス B）で正準型に統合し、本コンテキストは依存参照へ移行する | E1（暫定ホスト → 依存） |
| `DepthLevel` / `TestStrategyLevel` | scope identity frontmatter の設計ダイヤル。**エンジンの決定には影響しない**（助言軸であることを仕様として明記） | E2（パース）＋E5（非影響性） |
| `SkeletonDefault` | scope frontmatter の `skeleton`（スコープ既定の walking-skeleton 姿勢 — stance で上書きされる「既定」なので Switch ではなく Default と命名）。`"on"` / `"off"` の 2 値厳密パース（逐語拒否文言あり — §4） | E2 |

### 2.3 集約の述語面（純関数）

グラフとグリッドを読む面は、集約 `WorkflowDefinition` の**クエリメソッド 6 つ ＋ グリッド照会 1 つ**であり、これがスライス 1 の最小面である。独立したドメインサービスとしては置かない — 状態の所有者の外で判断する Ask 型を避けるため（01 §7.1 原則 2、設計監査 C9、Bolt B3 実装）。

| 述語 | 入力 → 出力 | 意味論の規範 | 対応する upstream |
| --- | --- | --- | --- |
| `is_valid_scope` | (`&str`) → `bool` | `valid_scopes()` に含まれるか。権威は identity ファイルの存在であってグリッド列の有無ではない | `validScopes` の判定面 |
| `valid_scopes` | () → 整列済み `ScopeName` 集合 | **identity ファイルの存在が権威**であり、グリッド列は権威ではない。逐語: *"Scope validity is the .md-presence authority (validScopes), not the grid"* | `validScopes` |
| `scope_metadata` | (`&str`) → `Option<&ScopeMetadata>` | scope identity ファイルの frontmatter（`depth` / `testStrategy` / `skeleton` / `review_cap` 等）。`.md` が無ければ `None`（= 無効スコープ） | scope frontmatter の読取面 |
| `subgraph_for_scope` | (`ScopeName`) → `Result<Vec<&StageNode>, UnknownScope>` | 未知スコープは**拒否**（逐語文言つき）。グリッド列の EXECUTE 集合でグラフを filter し、**`numericStageOrder` でソートして**返す。**ランタイムで topo ソートはしない** — compile のエッジ局所不変条件（F13）により数値順が有効な topo 順であることが保証されている | `subgraphForScope` |
| `stages_in_scope` | (`ScopeName`) → `Vec<(&StageSlug, PhaseId, Option<PlanAction>)>` | **全ステージ**について `(slug, phase, action)` を**文書順**で返す。`action` は静的グリッドの 3 値（recompose サフィックスは合成しない）。未知スコープは空（`subgraph_for_scope` との非対称） | `stagesInScope` |
| `first_in_scope_stage_of_phase` | (`PhaseId`, `ScopeName`) → `Option<&StageNode>` | `subgraph_for_scope` の並びから最初の該当 phase ノード。walking skeleton ゲートアンカーの**導出元**であり、ハードコードしない（B11）。未知スコープは `None` | `firstInScopeStageOfPhase` |
| `grid().action()` | (`ScopeName`, `StageSlug`) → `Option<PlanAction>` | **3 値照会**。列に slug が無ければ `None`（「このグリッドがコンパイルしていないステージ」）で、`SKIP` に畳まない。orchestration の `effectivePlanAction` は「オーバレイ → 本照会」の順で解決する合成読みであり、**畳み込みの責務は呼び出し側 = 集約 `WorkflowExecution`**（`effective_plan`）である（B1 / 設計監査 R2、Bolt B3 実装） | `effectivePlanAction` のグリッド参照部分 |

**2 経路の順序使い分け（本仕様の中核）**: `subgraph_for_scope` は `numericStageOrder` で**再ソート**し、`stages_in_scope` は**文書順**のまま返す。文書順の前進走査そのものは本コンテキストの担い手ではなく、集約 `WorkflowExecution` が `Started` で確定させた `stages`（`StageEntry` 列 = 文書順の解決済み計画）の上で行う（設計監査 R2、Bolt B3 で定義側から削除済み）。upstream ではコンパイラが数値順にソートして emit するため配布データでは両者が一致するが、**2 つの経路そのものは残っている**。したがって読込時に配列を数値順へ正規化してはならない（F2）。文書順インデックスに依存する派生値（`stageIndex` 等）も同じ理由で文書順に従う。

補助の純関数として `numeric_stage_order`（`StageNumber` の全順序）と `stage_graph_drift`（slug 集合の差分 — `missingFiles` は graph→disk で hard fail、`uncompiledStages` は disk→graph で advisory）を置く。後者はセッション開始フックの材料であり、スライス 1 の最小面ではないが、グラフ側の入力はここが供給する。

## 3. 入力 3 種の形状（Published Language の規範）

3 つの入力は `<harnessRoot>/tools/data/`（graph / grid）と `<harnessRoot>/scopes/`（identity）に配置される。パス解決とテストシーム（`AIDLC_STAGE_GRAPH` / `AIDLC_SCOPE_GRID` / `AIDLC_SCOPES_DIR`）は §6 の Gateway が所有する。

### 3.1 `stage-graph.json`

**ルートは配列**（`StageNode[]`）であってオブジェクトではない。配布物（v2.6.40 Claude ハーネス）では要素数 33、`number` は `0.1`…`4.7`、配列順は `numericStageOrder` の昇順。

ノードのフィールドは `FIELD_ORDER` の 28 個で、群ごとの規範は次のとおり。個別フィールドの意味論と典拠は抽出文書 §2.2 の全量表を正とする。

| 群 | フィールド（28） | 読み側の規範 |
| --- | --- | --- |
| 同一性・順序 | `slug` / `number` / `name` / `phase` | `number` は**文字列のまま**保持し、比較は `numeric_stage_order` のみ。`phase` は 5 値閉集合 |
| 適用可否 | `execution` / `condition` | `execution` はステージ著者側の適用可否であり、グリッドの所属判定とは直交。`condition` は自由記述の散文 |
| トポロジ | `lead_agent` / `support_agents` / `mode` / `for_each` / `workspace_requires` | `mode` は 5 値閉集合で `agent-team` を明示的に保持（§4）。`for_each: "unit-of-work"` と `workspace_requires` は任意 |
| 成果物 | `produces` / `optional_produces` / `produces_kinds` / `consumes` | `produces` は語彙名であってパスではない（パス解決はエンジン側）。`optional_produces` は directive の `produces` 解決に**含まれる**。`consumes[]` は `{artifact, required, conditional_on?}` のオブジェクト配列 |
| 依存 | `requires_stage` | 既知 slug のみ・build 時 dedup 済み。エッジ局所順序は F13 |
| レビュー | `reviewer` / `reviewer_max_iterations` / `review_class` | 3 つ揃って B10（レビュアーレシート述語）の入力。`review_class` は verification 所有型への外部キー（B7） |
| スコープ・儀式 | `scopes` / `summary_confirmation` | `scopes` は**グリッドの転置元**。`summary_confirmation` の値域は `"required"` / `"if-present"` の 2 値（配布物の実測は `required` 27 件のみ） |
| センサー | `sensors` / `sensors_applicable` | `sensors_applicable` は `{id, path, matches?}` の**オブジェクト配列**。compile 時の逐語スナップショットであり、実行時に manifest を再オープンしない |
| プラグイン | `plugin` / `enabled` | `plugin` は frontmatter からの逐語コピー。**`enabled` はキーが出ていれば必ず `false`**（有効ノードでは emit されない）。欠損は有効とみなす |
| 散文 | `inputs` / `outputs` | いずれも**文字列**（配列にしない）。記述用途のみで機械可読ではない — 機械可読な出力は `produces` |
| ルール | `rules_in_context` | `{path, scope}` の**オブジェクト配列**（長さ 3 または 4）。`path` は `default` スペースへのコンパイル時ピン |

**受理と拒否の規範**:

- **未知フィールドは無視して受理する**（`deny_unknown_fields` を付けない）。プラグインや将来版が `FIELD_ORDER` を増やしても既存バイナリが読めなくなってはならない（F1）。
- `when` / `required_sections` / 予約 4 キー（`on_failure` / `blocks_on` / `timeout` / `retry`）は**グラフに到達しない**。読み手がこれらの存在を期待してはならない。
- `rules_in_context` と `sensors_applicable` は**コンパイル専用**フィールド（著者は書けない）。
- directive 上では `sensors_applicable` が id の `string[]`、`rules_in_context` が配送済みパスの `string[]` へ**射影される**。グラフ上の型と directive 上の型は別物であり、潰して 1 型にしてはならない（F4）。`stage_file` はグラフに無く `phase` + `slug` から導出される。

### 3.2 `scope-grid.json`

**2 段構造**であり、中間の `"stages"` キーを省略してはならない（レガシー `mapping[scope].stages` 互換のための構造）。

```jsonc
{
  "<scope-name>": {
    "stages": { "<stage-slug>": "EXECUTE" | "SKIP" }
  }
}
```

- 内容は**全ステージの `scopes:` リストの純粋な転置**であり、グラフ閉包も述語適用も行わない。深さ・キーワード・説明はここに入らない（identity ファイル側）。
- **`initialization` の 3 ステージは frontmatter に関係なく全列で EXECUTE** になる（転置の特例 — F12）。
- 配布物では 11 スコープ列（EXECUTE 数は bugfix 7 / classic 26 / enterprise 33 / express 10 / feature 33 / infra 13 / mvp 23 / poc 8 / refactor 8 / security-patch 10 / workshop 26）。
- コンポーザが承認時に追記した **composed 列は逐語で読む**。読んだ後に再コンパイルしてはならない（upstream の明文の指示）。
- emit 側の整列（スコープ名アルファベット昇順、per-scope はステージ数値順）は書き側の契約であり、**読み側は列やキーの出現順に依存しない**（`BTreeMap` で保持してよい）。

### 3.3 scope identity ファイル（`<harnessDir>/scopes/aidlc-<name>.md`）

スコープは**2 ファイルで 1 つ**である。identity ファイルの YAML frontmatter が `name`（必須）、`depth` / `description` / `keywords`、任意で `plugin` / `testStrategy` / `runner` / `skeleton` / `review_cap` / `freeform_default` を供給し、グリッド列が EXECUTE/SKIP を供給する。

- **スコープ存在の権威は identity ファイル**であり、`valid_scopes` はこの集合から作る（F7）。
- frontmatter の検証（逐語拒否文言つき）: `skeleton` は `"on"` / `"off"`、`review_cap` は `adversarial` / `advisory` / `none`、2 ファイル間の `name:` 重複は致命、`plugin:` が `aidlc-` 始まりは拒否、`freeform_default: true` は**有効な**スコープ中 1 つまで。
- **frontmatter パーサは手書き**とする（00-policy R9。汎用 YAML パーサへの置換は寛容パースと逐語拒否文言の契約を静かに変えるため逸脱台帳マター）。

## 4. 読込の失敗態度（規範）

読込失敗の扱いは 3 入力で**意図的に非対称**である。この非対称そのものが観測可能な契約であり、「より厳格にする」方向の改変も逸脱になる。

| # | 事象 | 規範の態度 |
| --- | --- | --- |
| 1 | `stage-graph.json` が読めない | **fatal**。非ゼロ exit ＋ stderr に逐語文言。`AIDLC_STAGE_GRAPH` が設定されているときだけ hint 節が「unset して既定に戻せ」形に切り替わる |
| 2 | `stage-graph.json` が不正 JSON | **fatal**。`Stage graph at <p> is not valid JSON: <err>` 形の逐語文言 |
| 3 | `scope-grid.json` が読めない／不正 | **fatal にしない**。グラフの `scopes[]` から**その場で転置して導出**する（*"callers never see a hard ENOENT for a derivable artifact"*）。ここを厳格化すると新規 fixture ツリーが壊れる |
| 4 | 未知スコープ | **非対称**: `subgraph_for_scope` のみ `Unknown scope: "<s>". Valid scopes: <csv>` で拒否。`first_in_scope_stage_of_phase` は `None`、`stages_in_scope` は空、`scope_metadata` は `None` を返す（設計監査 R2 / C8） |
| 5 | identity ファイルあり × グリッド列なし | **zero-EXECUTE な正当スコープ**。unknown ではなくエラーでもない（`subgraph_for_scope` は空を返す）。`initialization` の 3 ステージは #8 の転置特例で常に EXECUTE なので、zero-EXECUTE は initialization 以外のステージについての記述 |
| 6 | グリッド列あり × identity ファイルなし | **ランタイムから不可視**。列ごと落ちるだけでエラーにしない（join の軸が metadata 側だから） |
| 7 | グリッド列に slug が無い | **3 値の `None`**。`SKIP` に畳まない。畳み込みは呼び出し側 = 集約 `WorkflowExecution` の `effective_plan`（orchestration の `effectivePlanAction`）の責務（設計監査 R2） |
| 8 | `initialization` の 3 ステージ | 全スコープ列で EXECUTE（転置の特例。グリッド側の値がどうであれ、転置規則としてこの結論になる）。**適用点はグリッド側の転置**（`ScopeGrid` — `grid().action()` の供給元。Bolt B3 実装 `scope_grid.rs` の転置述語 `phase == initialization ∨ node.scopes.contains(scope)`、テスト `transposition_puts_initialization_in_every_column`）であり、`stages_in_scope` / `effective_plan` はその結果を読むだけ。二重防御として `WorkflowExecution::start` は initialization が EXECUTE でなければ `InitializationMustExecute` で拒否する（10 §2.1） |
| 9 | `mode: "agent-team"` | **明示的に未実装として拒否**する。既定の実行経路へフォールスルーさせてはならない（upstream の最低要件は `throw "mode agent-team not yet implemented"`） |
| 10 | 上記いずれの失敗でも | **stdout に何も書かない**。half-emitted directive を出さないという orchestration 側の契約（10 §6 I1）を、読込側から破らない |

**逐語文言の採取状態（2026-08-22 更新）**: #1・#2・#4 の 4 文言と scope frontmatter の 2 文言は、抽出時点では参考ツリー 2 本（[F] v2.2.0 / [G] fork）でのみ一致確認済みだったが、**Issue #7 項目 0 のゴールデン採取でピン留め `3c3146cf` の実バイトと 6/6 一致を確認した**（`research/golden-3c3146cf-lib.md` §8.2・§8.3、`research/golden-3c3146cf-graph-dist.md` §3.1）。したがって本節はバイト列まで規範である。付随して確定した実装事実:

- #1 の hint 分岐は `process.env.AIDLC_STAGE_GRAPH` の **truthy 判定**なので、**空文字列の env では既定 hint に落ちる**。一方パス解決 `stageGraphPath()` は `??`（nullish 合体）なので空文字列の env はパスとして採用される — この非対称は upstream に実在する。
- #4 で throw する関数は **2 つ**（`subgraphForScope` と `resolvePlanForScope`）。
- scope `.md` の読取は、ディレクトリの `readdirSync` 失敗は空リストへ劣化するが、**個別 `.md` の読取失敗は素通しで伝播する**（本節の表に無い挙動）。
- `name:` **重複**の拒否文言だけは現実装が upstream 逐語と一致していない（§11 の残件）。

## 5. ユースケース層

**ユースケース**（スライス 1 の範囲、すべて読み取り専用）: `LoadStageGraph`、`LoadScopeCatalog`（グリッド列と identity の join）、`ResolveScopePlan`（`stagesInScope` 相当 — 全ステージの `{slug, phase, action}`）。compile・validate-grid・recompose のためのグラフ CLI 面はスライス 2。

**ポート**: 現行ポートは orchestration 側の **`WorkflowDefinitionRepository`（10 §3）1 本だけ**である。動詞は `find_by_id(&WorkflowDefinitionId)` であり、引数を取らない旧動詞 `find` は**廃止**した（後方互換の併存なし — C4 改訂 2026-08-23 / ADR-008）。1 つのハーネスが提供できる定義は 1 つなので、実装は「探す」のではなく「**要求された id が自分の id か**」を検査し、一致すれば 3 入力を読んで `id` と `revision` を載せた集約を返す。失敗態度は `NotFound { expected, actual }`（id 取り違え — 契約上 fatal）と `HarnessIdentity { path, cause }`（`harness.json` の読取・不正 JSON・`name` 欠落／不正 — 定義 id の供給元が失われる fatal）を §4 の既存の非対称に加える。その実装（`WorkflowDefinitionRepositoryImpl`）が 3 入力の取得（パス解決・env オーバライド・「読めない」と「不正」の区別 — §4 #1/#2）、scope カタログの列挙・読取、および `id` / `revision` の付与を**内部詳細として**持つ。

**将来の内部部品案**（ポートではない・スライス 1 では実装しない）: 実装内部をグラフ取得系とカタログ取得系に分割する案があるが、旧仮名 `...Source` は [`coding-rules/gateway-taxonomy.md`](../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md) のポート造語禁止にあたるため、スライス 2 で分割が実際に要ることになった時点で同規則に沿って命名する。

**他コンテキストへの供給面**（Customer/Supplier の supplier 側）: 供給の実体は **`WorkflowDefinitionRepository` が返す集約 `WorkflowDefinition` の述語面**（§2.3 のクエリメソッド 6 つ ＋ `grid().action()`）と、集約が内包する `StageNode` の読取である。顧客ごとに別名のサービス型を立てない — 個別名を置くと 1 つの集約の一部の操作にポートを 1 つずつ立てることになり、集約単位の境界を名前の上で解体してしまう（設計監査 C9、gateway-taxonomy §3）。

| 顧客 | 使う面 | 契約の要点 |
| --- | --- | --- |
| orchestration（scope 解決ラダー・10 §3） | `is_valid_scope` / `valid_scopes` / `scope_metadata` / `subgraph_for_scope` / `stages_in_scope` / `grid().action()` | `valid_scopes` の権威は identity ファイル。未知スコープ拒否の逐語文言もここから供給する。文書順保持と 2 経路の使い分け（§2.3）を含む |
| orchestration（walking skeleton — B11） | `first_in_scope_stage_of_phase` | ゲートアンカーの導出元。stance 解決そのものは顧客側 |
| verification（B10 の材料） | 集約が内包する `StageNode` の `reviewer` / `review_class` / `reviewer_max_iterations` | `review_class` は verification 所有型への外部キー（B7） |
| verification（B8） | 同じく `StageNode` の `sensors` / `sensors_applicable` | compile 時スナップショットであることを含めて供給する（実行時に manifest を再オープンしない） |

## 6. インターフェイスアダプタ層

- **Gateways**: 集約 `WorkflowDefinition` を 3 入力から再構成する Repository 実装 `WorkflowDefinitionRepositoryImpl`（ポート trait は use-case 層、実装は `XxxRepositoryImpl` — [`coding-rules/gateway-taxonomy.md`](../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。**格納形式がファイルであることは実装の内部詳細**なので型名に技術接頭辞を付けない）。**パス解決とテストシームはここに閉じる** — `<harnessRoot>/tools/data/{stage-graph,scope-grid}.json`、`<harnessRoot>/scopes/`、および `AIDLC_STAGE_GRAPH` / `AIDLC_SCOPE_GRID` / `AIDLC_SCOPES_DIR` のオーバライド（D6 の対象範囲は §11 の未決事項）。JSON コーデックと frontmatter パーサは Gateway の内部部品（01 §7）。キャッシュ戦略（呼び出しごとの明示ロード / `OnceCell` / 注入）は観測不能なので実装の自由（§10）。**I/O 責務はすべてここ**。テストダブル `InMemoryWorkflowDefinitionRepository`（`Impl` 接尾辞は付けない）を最初に用意する — これは 10 §8-3 が挙げる in-memory Gateway 一式の `WorkflowDefinition` 分と同一物である。
- **Presenters**: 読込失敗の stderr 逐語文言と非ゼロ exit。**stdout を汚さない**（§4 #10）。文言は文言カタログ（A3）から引く。
- **Controllers**: `--scope` 等の引数を `ScopeName::parse` に通し、成功した型付き値をユースケースへ渡す（01 §7 の規約）。未知スコープの判定は Controller ではなく述語側（`subgraph_for_scope`）の責務であり、Controller は検証ロジックを持たない。

## 7. インフラストラクチャ層の利用

正準 JSON（A2）・文言カタログ（A3）・ハッシュは純粋部品としてドメイン層からも利用可。ファイル I/O は Gateway のみ。プロファイルの使い分けは ADR 0001 のとおり: ディスク成果物は `contract-pretty`（2 スペース＋末尾改行 — 書き側の契約）、route hash の入力（**グラフノード全体**を挿入順で直列化する非正準形）は `contract-compact`。`tracing` 計装（A10）はユースケース／アダプタ層に置き、グラフ読込は orchestration のターンスパンの子スパンになる。

## 8. 不変条件表（強制手段つき）

E4（Quint）は本コンテキストに**付さない** — 対象が状態遷移ではないため（00-policy A9 / 01 §3.1）。網羅検査は proptest が担う。

| # | 不変条件 | 強制 | 備考 |
| --- | --- | --- | --- |
| F1 | `stage-graph.json` のルートは配列で、要素は 28 フィールド集合。**未知フィールドは無視して受理**（`deny_unknown_fields` 禁止） | E2 | 将来版・プラグインの追加フィールドで読めなくならないこと |
| F2 | 配列の**文書順を保持**し、`subgraph_for_scope`（数値順ソート）と `stages_in_scope`（文書順）の 2 経路を潰さない。読込時に数値順へ正規化しない（文書順の前進走査は集約 `WorkflowExecution` の `stages` 上で行う — §2.3、設計監査 R2） | **E1**（`Vec` ＋別 API） | **暫定規範**（§11 で裁定待ち）。正規化を選ぶと手編集グラフに対する挙動が本家と分岐する |
| F3 | `number` は文字列 `"P.I"` のまま保持し、順序比較は `numeric_stage_order` のみ | E1+E2 | proptest: 全順序性と `"1.10" > "1.9"` |
| F4 | `rules_in_context` / `sensors_applicable` はオブジェクト配列。文字列配列へ潰さない（directive 上の射影形とは別型） | **E1** | 潰すと `run-stage` が本家と非互換になる |
| F5 | `inputs` / `outputs` は文字列で記述用途のみ。機械可読な出力は `produces` | E1 | |
| F6 | `scope-grid.json` は 2 段構造で中間 `"stages"` キーを省略しない | E1+E2 | レガシー `mapping[scope].stages` 互換 |
| F7 | スコープ存在の権威は identity ファイル。グリッド列は権威ではない | **E1**（join の軸を型で固定）+E3 | 帰結が §4 の #5・#6 の非対称 |
| F8 | グリッド未収載の slug は 3 値（`Option<PlanAction>` の `None`）。`SKIP` に畳まない | **E1** | 畳み込みは呼び出し側 = 集約 `WorkflowExecution` の `effective_plan`（B1 / 設計監査 R2） |
| F9 | 未知スコープの非対称: `subgraph_for_scope` のみ逐語拒否、他 3 述語は `None` / 空 | E2+E3 | 戻り型が `Result` と `Option` に分かれること自体が装置 |
| F10 | `stage-graph.json` の欠損／不正 JSON は fatal（非ゼロ exit ＋ stderr 逐語、stdout は汚さない）。`scope-grid.json` の欠損は転置導出フォールバック | E2+E3 | 逐語はゴールデン採取後に文言カタログで固定（§10） |
| F11 | `mode: "agent-team"` は明示的に未実装として拒否し、既定経路へフォールスルーさせない | **E1**+E3 | enum に variant を持たせ、`match` の網羅性で漏れをビルドエラーにする |
| F12 | `initialization` の 3 ステージは全スコープ列で EXECUTE（転置の特例） | E2+E3 | 転置を実装する場合の述語は `phase == initialization \|\| scopes.contains(scope)` |
| F13 | 全 `requires_stage` エッジで `numeric_stage_order(dep) < numeric_stage_order(self)` | E2（compile 時）＋proptest | **読込時には検証しない**。ランタイムが topo ソートを省ける根拠であり、upstream も「今日のランタイム反復をゲートしない」と明記 |
| F14 | コンポーザが書いた `scope-grid.json` の列は逐語で読み、読取後に再コンパイルしない | E5+E3 | upstream の明文の運用契約。孤児列の扱い（`preserveNames`）は compile 側（スライス 2） |

## 9. 実装順序（D10 × domain-model-first）

1. **ドメイン例をユビキタス言語のテストとして書く**: 「identity ファイルがありグリッド列が無いスコープは unknown ではない」「グリッドに無い slug は SKIP ではない」「`subgraph_for_scope` は拒否し `stages_in_scope` は空を返す」「`initialization` はどのスコープでも EXECUTE」。テスト名は 01 の正準用語を使う。
2. **Domain Primitive → 集約 `WorkflowDefinition` を TDD で実装**（`StageGraph` は内包の成果物値であって集約ルートではない — §2.1）。proptest は `StageNumber` の全順序性、`StageSlug` の parse 往復、転置の冪等性、`grid().action()` の 3 値性、F13（生成したグラフでエッジ局所順序が破れないこと）に適用する。
3. **in-memory Gateway** で 3 入力を固定バイト列として与え、述語面（§2.3 のクエリメソッド 6 つ ＋ `grid().action()`）のユースケーステストを回す。ここまでファイル I/O は登場しない。
4. **実 Gateway とゴールデン**: ピン留め `dist/claude/` の実 JSON（Issue #7 項目 0 の採取物）を読ませ、33 ノード・11 スコープ列・EXECUTE 数のパリティと、`stages_in_scope` の `.aidlc-plan.json` バイト同値を検証する。Repository 実装が付与する `WorkflowDefinitionId` / `DefinitionRevision`（ADR-008）の付与規則もここで固定する。逐語文言の期待値固定も同時に行う。

## 10. 実装ノート — 仕様と実装の分離（00-policy §2 の判定原則）

**serde による構造的パースは「ロード時無検証」からの逸脱ではない**。upstream はロード時に `JSON.parse` の結果を信頼境界 1 回のキャストで通すが、その理由はデータがフレームワーク自身の生成物だからであり、upstream 自身が *"Phase E will replace this trust boundary with an `isStageEntryArray()` type guard"* と将来の厳格化を予告している。**dist の正規データに対しては観測差が生じない**ため、Rust 側の構造的パースはロード時検証の補強として扱い、逸脱台帳には載せない。ただし観測差が出うる 3 点は明示的に決める:

| # | 論点 | 本仕様の規範 |
| --- | --- | --- |
| 1 | 未知フィールド | **許容**（`deny_unknown_fields` 禁止 — F1） |
| 2 | 欠損 optional | `Option` ないし空 default（配列・マップには `#[serde(default)]`） |
| 3 | 未知の列挙値 | **全列挙（`phase` / `execution` / `review_class` / `mode`）を厳密 enum とし、未知値は load 時に落とす**（2026-08-22 オーナー裁定）。`mode` は `agent-team` を variant として保持し、使用時拒否は F11 が担う。ドメイン型に `Unknown` variant を持たせないことで Always Valid を維持する。upstream（load は通り使用時に壊れる）との観測差は**手編集グラフの未知値に限られ**、dist の正規データでは生じない — fail-loud 側に倒す。**2026-08-22 のゴールデン採取で正規データ 33 ノードの全数 load を確認済み**（`tests/golden/upstream-3c3146cf/` ＋ `golden_parity_test.rs`） |

**グリッドのセル単位の異常**（文法外 slug・`EXECUTE`/`SKIP` 以外の値）は**そのセルだけ落とす**。結果は 3 値契約の「未収載」（§4 #7）になり、upstream の「列に slug が無い」と同じ観測へ収束する。1 セルの異常でグリッド全体を転置導出へ倒さない（列全体の破棄は §4 #3 のファイル単位の失敗のみ）。

その上で、次の内部機構は仕様を守る限り自由に選んでよい（＝実装であって仕様ではない）:

| upstream の機構 | 自由にしてよい理由 |
| --- | --- |
| モジュールレベル可変シングルトン＋`_reset*ForTests()` | 観測不能。Rust では `OnceCell` / 注入 / 呼び出しごとの明示ロードのいずれでもよい。Gateway に閉じ込めるほうが 10 §3 のポート設計と整合する |
| `loadScopeGrid` → `loadGraph` の遅延循環依存 | TS の import 循環回避の産物。Rust には不要 |
| `loadScopeMapping` がレガシー `ScopeDefinition` 形へ join する構造 | 旧 `scope-mapping.json` 互換のための中間表現。grid と metadata を別型で持ち、必要な述語だけ供給してよい |
| 線形スキャンと毎回の再ソート | 33 ノード規模の前提。slug 索引化・メモ化は自由 |
| `topoSort` / `findCycles`（ランタイム未使用） | upstream 自身が「今日のランタイム反復をゲートしない」と明記。実装しない選択も可 |
| プロセスごとのワンショット読込（mtime 監視なし） | 長寿命プロセス化する場合の再読込戦略は自由。ただし**同一呼び出し内でグラフが変わらない**ことは continue_token の route hash 束縛が前提にしている |
| 例外 throw によるエラー生成 | Rust は `Result` でよい。契約は逐語文言・exit code・stdout を汚さないことだけ |
| computed field の旧名 `display_order`（upstream 04） | 実フィールドは `number`。旧名に追随しない |

**書き側（compile）の前提**: `FIELD_ORDER` 28 のキー順と `undefined` 落とし、`contract-pretty` の体裁（2 スペース＋末尾改行）は `compile --check` のバイト比較の前提であり、ADR 0001 決定 3 が「28 フィールド順は struct 宣言で符号化」と規定している。~~並び順の実バイトはピン留め配布物からの採取待ち~~ → **2026-08-22 に採取完了**（`research/golden-3c3146cf-graph-dist.md` §1・§2）。28 エントリの順・`undefined` 落としの実装（`if (v === undefined) continue;` の 1 行のみで、`null` / `[]` / `""` / `false` は落とさない）・`JSON.stringify(x, null, 2)` ＋末尾改行 1 個の体裁がいずれも確定し、dist 実バイトとのラウンドトリップがバイト完全一致した。スライス 2 の着手条件はこれで満たされている（読み専用の本スライスでは不要）。

**逐語文言の採取状態（2026-08-22 更新）**: 本書が §4 で規範化した読込失敗文言のうち、graph 読込 3 形・`Unknown scope` 1 形・scope frontmatter 2 形は、抽出文書では [F]/[G] 由来（★）だったが、**Issue #7 項目 0 のゴールデン採取でピン留め `3c3146cf` に対し 6/6 バイト一致**した。文言カタログ（ADR 0002）側でも `SpecQuotedOnly` として残っていた 4 件（`state::field_not_found` / `state::file_not_found` / `lock::acquire_failed` / `bolt::invalid_mode`）が 4/4 一致で `Captured` へ昇格済みである。**ゴールデンテストの期待値固定も完了**しており、配布実バイトは `tests/golden/upstream-3c3146cf/` に置き、`modules/core/interface-adapter/tests/golden_parity_test.rs` が 33 ノード全数 load・文書順 = 数値順・11 列の EXECUTE 数・reviewer 13（adversarial 5 / advisory 8）・`enabled` キー 0 を検証する。残る不一致は scope `.md` の `name:` 重複拒否文言 1 形のみで、§11 に裁定待ちとして立ててある。

## 11. 未決事項

- ~~**serde の厳格度**（§10 の表 #3）の確定~~ → **2026-08-22 裁定済み**: 全列挙を load 時厳格とする（§10 表 #3 に記載）。ゴールデン採取後の再確認のみ残る。
- **文書順保持（F2）の確定**。本書は暫定規範として文書順保持を採ったが、「読込時に数値順へ正規化する」を選ぶ場合は手編集グラフに対する挙動が本家と分岐するため逸脱台帳マターになる。
- ~~**`scope-grid.json` 欠損時の転置導出フォールバック**（§4 #3）をスライス 1 でも実装するか~~ → **2026-08-22 裁定済み**: upstream 忠実にフォールバックを実装する（*"callers never see a hard ENOENT for a derivable artifact"*）。fatal 化は診断改善として将来 doctor 側で扱う。
- **`AIDLC_SCOPE_GRID` / `AIDLC_SCOPES_DIR` / `AIDLC_SCOPE_MAPPING`** が upstream の環境変数一覧（03 §2.3）に載っていない（テストシーム専用）。D6 の互換対象に含めるか、含めるなら同名で用意するか。**実在は確定**（`aidlc-graph.ts:383-394` / `:428` / `:380`）だが、含めるかどうかはオーナー裁定。
- ~~**`enabled` の意味論**（プラグイン選択でノードが削除されるのか `enabled: false` が立つのか、有効時にキーが出力されるのか）~~ → **2026-08-22 ゴールデン採取で確定**（`docs/specs/research/golden-3c3146cf-graph-dist.md` §4）。**ノードは削除されない**（`applyPluginSelection` は配列長を変えず、`canonicalStageGraphJson` が無効ノードも全件 emit する）。**有効時はキーが出力されない**（毎回 `delete` してから無効時のみ `= false` を立てるため `undefined` 落ちする。dist 実測 **0/33**）。型は `enabled?: false` で `true` は表現不可能、判定は一貫して `s.enabled !== false`。→ §3.1 の「欠損は有効とみなす」は正しい。**新たに判明した非対称**: グリッド側は無効ノードを行ごと落とす（`:1958`）ので「graph には出るが grid 行には無い」= 3 値契約の未収載（§4 #7）として観測される。
- ~~**`summary_confirmation` の値域**~~ → **2026-08-22 ゴールデン採取で確定**（同 §5(d)）。`"required" | "if-present"` の**2 値列挙**（`aidlc-graph.ts:200`、検証は `aidlc-stage-schema.ts:326-333`）。boolean 相当ではない。dist 実測は `required` 27 件・`if-present` 0 件。
- ~~**逐語文言 6 形のピン留め確認**（§10）~~ → **2026-08-22 ゴールデン採取で 6/6 確定**。graph 読込 3 形は `golden-3c3146cf-lib.md` §8.2（`aidlc-lib.ts:8558-8585`）、scope frontmatter 2 形は同 §8.3（`:8661` / `:8663`）、`Unknown scope` 1 形は `golden-3c3146cf-graph-dist.md` §3.1（`aidlc-graph.ts:997` と `:1052` の **2 箇所**で throw する）。いずれも既存の逐語とバイト一致した。**残件は 1 つ**: 未知スコープに `None` / 空を返す 3 述語側の実装逐語（`aidlc-lib.ts` の追加採取待ち）。`name:` 重複拒否文言は **2026-08-22 裁定: D6 の既定どおり upstream 逐語へ一致させた**（実装修正済み — 逸脱ではない）。
- ~~**`FIELD_ORDER` 28 の並び順**。compile（スライス 2）の着手条件~~ → **2026-08-22 ゴールデン採取で確定**（`golden-3c3146cf-graph-dist.md` §1、`aidlc-graph.ts:449-478`）。dist 33 ノード全件のキー列がこの順の部分列であること（違反 0・圏外キー 0）も機械検証済みで、ADR 0001 決定 3 の struct 宣言順を確定できる。
- ~~**§4 #8（`initialization` 全列 EXECUTE）の適用範囲**~~ → **2026-08-22 裁定済み**: 本仕様の §4 #3 フォールバックが写すのは upstream の**実行時フォールバック** `loadScopeGrid → transposeScopeGrid`（`aidlc-graph.ts:415-445 → :1400-1405` — **特例あり**）であり、現実装（`ScopeGrid::derive_from_graph` の特例あり転置）は正しい。特例を持たない `transposeScopeGridForMapping`（`aidlc-lib.ts:8618-8632`）は legacy scope-mapping 経路のもので、本仕様の実装対象外。F12 は「compile 転置と実行時フォールバック転置の両方」に適用され、mapping 経路には適用されない。
- upstream 04 が computed field を旧名 `display_order` で記述している件（実フィールドは `number`）が意図的な別名か doc drift かの確認。実装への影響は低い。
- ~~**文書番号の採番**~~ → **2026-08-22 裁定済み**: 本書が 12 号を確定使用し、knowledge コンテキスト仕様は 13 号とする（`11-workspace.md` §10 の予告を修正済み）。
- スライス 2 の範囲確定: `compileStageGraph`（B6）、stage frontmatter スキーマ、エージェントペルソナ、3 ダイヤル、キーワード推論（アルファベット順 first-match の決定論）、composed scope のマージ（`preserveNames`）。
