> 抽出元: upstream as-built 仕様 (awslabs/aidlc-workflows v2 @ 3c3146cf, v2.6.40)。2026-08-22 の精密抽出 (Issue #7 項目 3)。12-workflow-definition.md スライス 1 (グラフリーダ契約) の執筆材料。

以下、コンパイル済みグラフ成果物 (`stage-graph.json` / `scope-grid.json`) と scope identity ファイルの契約の完全列挙。ピン留めコミット `3c3146cf` の as-built 仕様 (`docs/upstream/specs/`) を一次典拠とし、`docs/specs/` の既存蒸留と手元の 2 つの実ツリーを突き合わせた。**他の research 文書と違い、本書には一次典拠で裏の取れていない記述が含まれる** — §0 の証拠格付けを先に読むこと。逐語契約はすべて原文 (英語) のまま保存する。典拠表記は `[S] <ファイル>:<行>` または `[S] <仕様書> §x.y`、および仕様が引用する upstream コードサイト (`file.ts:line`) の併記。

# Issue #7 項目 3 抽出結果: compiled graph 成果物の契約

## 0. 証拠の格付け (重要)

| 記号 | 出所 | 位置づけ |
| --- | --- | --- |
| **[S]** | `docs/upstream/specs/*.md` | **一次典拠**。ピン留めコミット `3c3146cf` (v2.6.40) の as-built 仕様。行番号は各ファイルの行 |
| **[F]** | `awslabs/aidlc-workflows` 実ツリー (HEAD = `eae912e`, **v2.2.0**) | 参考。upstream 本家だが**ピン留めより古い**。逐語文言・関数構造の裏取りに使用 |
| **[G]** | downstream fork `amadeus` の実ツリー (`bug-1845`) | 参考。**別系統に分岐済み** (32 ステージ・スコープ名も語彙も別)。JSON の**形**の実物確認にのみ使用。値・フィールド集合は upstream と一致しない |

**[F]/[G] から引いた事実はすべて「ピン留めコミットでの再確認が必要」**として §8 の未確定事項に上げてある。§1〜§6 の規範記述は [S] を正とし、[F]/[G] は補強としてのみ併記する。★ 印の付いた逐語文言は [F]/[G] 由来で、ピン留めでの採取 (Issue #7 項目 0) を待っている。

---

## 1. dist ツリー内の配置場所

### 1.1 成果物パス

```text
dist/claude/
└── .claude/                       ← harnessDir (ハーネスごとに .kiro / .codex / .cursor / .aidlc)
    ├── tools/
    │   ├── aidlc-orchestrate.ts   … エンジン (全ハーネスでバイト同一)
    │   └── data/
    │       ├── stage-graph.json   ★ 本件
    │       ├── scope-grid.json    ★ 本件
    │       ├── harness.json       … name / harnessDir / rulesSubdir / (plugins)
    │       ├── ars-priors.json
    │       ├── model-rates.json
    │       ├── memory-seed/
    │       └── templates/
    ├── scopes/aidlc-<name>.md     … スコープ identity ファイル (グリッド列と対で 1 スコープ)
    └── aidlc-common/stages/<phase>/<slug>.md   … ステージ本体 (レガシー fallback: skills/aidlc/stages)
```

典拠: `COMPILED_DATA = ["tools/data/stage-graph.json", "tools/data/scope-grid.json"]` (`scripts/package.ts:377`) — [S] `10-distribution-harnesses.md:223`。スコープ identity は `core/scopes/aidlc-<name>.md` → dist では `<harness>/scopes/` ([S] `01-workflow-model.md:329-331`、[S] `05-agents.md:656` の Kiro 側 `allowedPaths` `".kiro/scopes/**"`, `".kiro/tools/data/scope-grid.json"` が対応する dist 形)。

### 1.2 「コンパイル済みデータは dist にしか存在しない」

- ソースツリー `core/tools/data/` には `ars-priors.json` / `model-rates.json` / `templates/` のみ。`stage-graph.json` と `scope-grid.json` は**存在しない** — [S] `01-workflow-model.md:816-820` が `scripts/package.ts:18` を逐語引用: *"stage-graph.json + scope-grid.json — compiled data lives only in dist"*。計測は [S] `01-workflow-model.md:1147`。
- したがって **amadeus-ng が読むのは `dist/claude/.claude/tools/data/` に配布された 2 ファイル**であり、これは「配布物 = Published Language」という [S] `01-domain-model.md:62` (PL 3 本のうち第 1) の位置づけと一致する。

### 1.3 ハーネス差

| 成果物 | 7 ハーネス間の差 |
| --- | --- |
| `scope-grid.json` | **バイト同一** (ハッシュ 1 個、`60fb4547…`) |
| `stage-graph.json` | ハッシュ **5 種**。差分は**すべて `sensors_applicable[].path` のハーネス相対プレフィクス** (`.claude/sensors/…` vs `.codex/sensors/…`) のみ。`rules_in_context[].path` はワークスペース相対 (`aidlc/spaces/default/memory/…`) なので差が出ない |

典拠: [S] `01-workflow-model.md:822-827`、計測 [S] `01-workflow-model.md:1133-1134`、[S] `00-overview.md:218,445` (MD5 実測)。

### 1.4 パス解決とテストシーム

- `stageGraphPath()` = `AIDLC_STAGE_GRAPH ?? join(DATA_DIR, "stage-graph.json")`、`scopeGridPath()` = `AIDLC_SCOPE_GRID ?? join(DATA_DIR, "scope-grid.json")`、`DATA_DIR` は**ツールモジュール自身の隣** (`dirname(import.meta.url)/data`)。[F] `core/tools/aidlc-lib.ts:2797,2806,2813`、[G] `packages/framework/core/tools/amadeus-lib.ts:7257` で同一構造を確認。★
- `AIDLC_STAGE_GRAPH` は [S] にも登場する (プラグイン compose が spawn 環境に固定注入: [S] `11-plugin-system.md:486`)。`AIDLC_SCOPE_GRID` / `AIDLC_SCOPES_DIR` / `AIDLC_SCOPE_MAPPING` は [S] の環境変数一覧 ([S] `03-state-audit-runtime.md:97-119`) に**載っていない** → §8 の未確定事項。
- 「エンジンがどこにいるか」の解決は `aidlc-runtime-paths.ts` 側 (`AIDLC_RUNTIME_HARNESS_ROOT` → モジュール自身のハーネスルート → プロジェクトの `<projectDir>/<harnessDir>` → パッケージ済みランタイム)。**Mutation は project-owned** (読取 fallback を書込先にしてはならない) — [S] `03-state-audit-runtime.md:63-96`。

---

## 2. `stage-graph.json` の完全な JSON 形状

### 2.1 トップレベル

**ルートは配列** (`GraphStage[]`)。オブジェクトではない。

- エミッタ: `canonicalStageGraphJson` (`aidlc-graph.ts:1349-1362`) が**唯一の writer**。ピン留めされた **28 エントリの `FIELD_ORDER`** (`:449-478`) を走査し `undefined` を落とすため、キー順は構築順に依存しない — [S] `01-workflow-model.md:903-909`。逐語: *"formatter drift is impossible when there's exactly one writer"* (`aidlc-graph.ts:1345-1348`)。
- 体裁: `JSON.stringify(ordered, null, 2)` + 末尾改行 (2 スペースインデント)。[F] `core/tools/aidlc-graph.ts:1185-1198` で確認 ★、[S] は `10-distribution-harnesses.md:380` で `JSON.stringify(parsed, null, 2)` + trailing newline の規律を別成果物について明記。
- 要素数: v2.6.40 の Claude 配布で **33** (`number` は `0.1`…`4.7`) — 計測 [S] `01-workflow-model.md:1123`。
- 配列順序 = `numericStageOrder` (phase prefix → index の 2 段整数比較) による昇順。compile が `:1855` でソートしてから emit する — [S] `01-workflow-model.md:44-49, 858`。

### 2.2 ノードのフィールド (28 = `FIELD_ORDER`)

`FIELD_ORDER` の**メンバー集合**は [S] から次のように確定できる (28 という数も [S] `01-workflow-model.md:906` が明記)。ただし**並び順そのものは [S] に列挙がなく、§8 で未確定**。

| # | フィールド | 型 | 意味論 | 典拠 |
| --- | --- | --- | --- | --- |
| 1 | `slug` | `string` | `^[a-z][a-z0-9-]*$`。ファイル名 stem と一致必須。一意 | [S] 01 §3.2, §8.4 (#2,#3) |
| 2 | `number` | `string` | `"<phaseIndex>.<seq>"`。**エンジン割当**、著者は書けない (書いても順序ヒント扱いで絶対値は不使用)。`initialization=0 … operation=4` | [S] 01 §3.2, §8.2 |
| 3 | `name` | `string` | 表示名。著者 `name:` → 無ければ `titleCaseSlug(slug)` | [S] 01 §8.2-5 |
| 4 | `phase` | `string` | `initialization\|ideation\|inception\|construction\|operation` の閉集合 | [S] 01 §2.1 |
| 5 | `execution` | `"ALWAYS"\|"CONDITIONAL"` | **ステージ著者側の適用可否**。プラン所属 (EXECUTE/SKIP) とも gate 軸とも**直交** | [S] 01 §3.3 |
| 6 | `condition` | `string` | 自由記述の適用ルール (人間・LLM 向け)。必須フィールド | [S] 01 §3.2 |
| 7 | `lead_agent` | `string` | エージェント slug。予約疑似エージェント `orchestrator` を許す | [S] 01 §3.2 |
| 8 | `support_agents` | `string[]` | `mode ∈ {pipeline, mob}` のとき非空必須 | [S] 01 §3.2 |
| 9 | `mode` | `"inline"\|"subagent"\|"pipeline"\|"mob"\|"agent-team"` | トポロジ。`agent-team` は**予約**で出荷グラフには現れない。読み手は明示的に扱い、既定経路に落とさないこと (逐語: *"orchestrator code reading the `mode` field must handle `agent-team` explicitly. At minimum, throw \"mode agent-team not yet implemented\". Do not fall through to a default execution path."*) | [S] 01 §3.2, [S] `02-orchestration-engine.md:154`, [S] `04-stage-protocol.md:98` |
| 10 | `for_each` | `"unit-of-work"?` | 付いていれば Unit of Work ごとに反復。出荷グラフでは 5 ステージ | [S] 01 §4.1, 計測 [S] `02-orchestration-engine.md:529` |
| 11 | `workspace_requires` | `boolean?` | ワークスペース実体への書込を要求。出荷グラフでは `code-generation` のみ | [S] 01 §4.1 |
| 12 | `produces` | `string[]` | 成果物**語彙名** (パスではない)。`artifactFilename` が `<name>.md` に写像、例外 1 件のみ `traceability → traceability.json` | [S] 01 §4 末尾 |
| 13 | `optional_produces` | `string[]?` | 条件付き成果物。directive の `produces` 解決に**含まれる** | [S] `02-orchestration-engine.md:159` |
| 14 | `produces_kinds` | `Record<artifact, unitKind[]>?` | 成果物 → 適用 unit kind (`service`/`ui`/`packaging`/`library` 等)。**マップに無い成果物は全 kind に適用**。キーは `produces ∪ optional_produces` に含まれること | [S] 01 §3.2, §4.1 |
| 15 | `consumes` | `{artifact: string, required: boolean, conditional_on?: "brownfield"\|"greenfield"}[]` | 入力宣言。`required:false` は欠損しても無言で落ちる | [S] 01 §3.2, [S] `04-stage-protocol.md` §2.5 |
| 16 | `requires_stage` | `string[]` | 既知 slug のみ。**build 時に dedup 済み**。エッジ局所不変条件 `numericOrder(dep) < numericOrder(self)` を満たす | [S] 01 §3.2, §8.4 (#6,#7) |
| 17 | `sensors` | `string[]` | 著者が pull import したセンサー id | [S] `06-sensors.md` §3.1 |
| 18 | `scopes` | `string[]` | このステージを EXECUTE にするスコープ名の列挙。**scope-grid の転置元** | [S] 01 §5.1 |
| 19 | `reviewer` | `string?` | レビュアーエージェント slug。出荷グラフで 13 ステージ | [S] 01 §4, §4.2 |
| 20 | `reviewer_max_iterations` | `number?` | 正整数。`reviewer` 必須 | [S] 01 §3.2 |
| 21 | `review_class` | `"adversarial"\|"advisory"?` | `reviewer` 必須。出荷グラフで adversarial 5 / advisory 8 | [S] 01 §3.2、計測 [S] `05-agents.md:727` |
| 22 | `summary_confirmation` | `"required"?` (観測値) | 出荷グラフで 27 ステージが宣言 | 計測 [S] `01-workflow-model.md:1124` |
| 23 | `plugin` | `string?` | 所有プラグイン名。`aidlc` および `aidlc-` 始まりは不可。slug は `<plugin>-` 前置必須。frontmatter から**逐語コピー** | [S] `11-plugin-system.md:718-723`, [S] 01 §8.4 (#5) |
| 24 | `enabled` | `false?` | `applyPluginSelection()` が選択に応じてノードを削除、または `enabled: false` を立てる | [S] `11-plugin-system.md:466-468, 723` |
| 25 | `inputs` | `string` | **自由記述の散文** (配列ではない)。必須フィールド | [S] 01 §3.2、実物 [G] |
| 26 | `outputs` | `string` | 同上。**記述用途のみで機械可読ではない** (機械可読は `produces`) — [S] 01 §10 の相違 #7 が明言 | [S] 01 §3.2, §10-7 |
| 27 | `rules_in_context` | `{path: string, scope: "org"\|"team"\|"project"\|"phase"}[]` | compile 時に確定。長さは **3** (org+team+project) または **4** (+ 該当 phase)。出荷グラフでは 30 件が 4、initialization 3 件が 3。**`path` は `default` スペースに固定ピン**され、実行時に再解決しない | [S] `08-memory-rules-learnings.md:110-119, 618, 638` |
| 28 | `sensors_applicable` | `{id: string, path: string, matches?: string}[]` | compile 時に manifest の capability glob を**逐語スナップショット**。フック側は fire 時に manifest を再オープンしない | [S] `06-sensors.md:955-976` (`SensorResolution` at `aidlc-graph.ts:128-132`) |

**著者が書けないコンパイル専用フィールド**は 2 つ: `rules_in_context` と `sensors_applicable` (frontmatter に書くと拒否) — [S] `01-workflow-model.md:182-184` (`aidlc-graph.ts:174-184`)。

**`FIELD_ORDER` に「入らない」もの** (読み手が期待してはいけない):

| 除外されるキー | 理由 | 典拠 |
| --- | --- | --- |
| `when` (予約述語 `producer-in-plan`) | スキーマは受理するが `buildGraphStage()` が**ノードにコピーしない**。compile も runtime も読まない。逐語: *"`when:` is declared but not evaluated."* — ステージは `scopes:` リストだけでゲートされる | [S] `11-plugin-system.md:744-751` |
| `required_sections` | frontmatter は受理し、プラグイン compose はコア stage ソースにマージするが、**`FIELD_ORDER` に無いので `stage-graph.json` に到達しない**。センサー/ディスパッチャも読まない | [S] `06-sensors.md:965-974` |
| `on_failure` / `blocks_on` / `timeout` / `retry` | 予約 4 キー。frontmatter 段階で `` `${key} is reserved (${reason}); not active yet` `` として拒否 | [S] `01-workflow-model.md:157-160` (`aidlc-stage-schema.ts:148-153`) |

### 2.3 ノード由来だが **directive では形が変わる** 2 フィールド (読み手の要注意点)

| フィールド | グラフノード上の形 | `run-stage` directive 上の形 |
| --- | --- | --- |
| `sensors_applicable` | `{id, path, matches?}[]` | **`string[]` (id のみ)** |
| `rules_in_context` | `{path, scope}[]` | **`string[]` (配送済みパスの順序リスト)**。transport 時に active space 基準へ**上書き**される |

典拠: [S] `02-orchestration-engine.md:157, 162` (directive 側は `string[]`)、[S] `08-memory-rules-learnings.md:193, 618` (transport 時上書き)、[G] `amadeus-orchestrate.ts:2970` で `(node.sensors_applicable ?? []).map(s => s.id)` の射影を実確認 ★。**ここを取り違えると `run-stage` が本家と非互換になる。**

### 2.4 ノードに**無い**が directive にある派生フィールド

`stage_file` は `stageFileFor(node.phase, node.slug)` による**導出**であり、グラフには格納されていない ([G] `amadeus-orchestrate.ts:2971` ★)。[S] `02-orchestration-engine.md:152` は routing 群をまとめて *"Read straight off the compiled graph node"* と書いているが、`stage_file` だけは導出である。

---

## 3. `scope-grid.json` の完全な JSON 形状

### 3.1 形

```jsonc
{
  "<scope-name>": {
    "stages": {
      "<stage-slug>": "EXECUTE" | "SKIP"
      // …全コンパイル済みステージ分、stage-graph の数値順で並ぶ
    }
  }
  // …スコープ名はアルファベット昇順
}
```

TypeScript 型 ([F] `core/tools/aidlc-graph.ts:1207-1209` ★。コメント含めて [S] の記述と一致):

```ts
export interface ScopeGrid {
  [scope: string]: { stages: Record<string, "EXECUTE" | "SKIP"> };
}
```

- **純粋な転置**。グラフ閉包も述語適用も行わない。`{ <scope>: { stages: {...} } }` という 2 段構造は「レガシー `scope-mapping.json` の `.stages` 半分と完全に同形」であり、`mapping[scope].stages` を読む既存消費側をバイト単位で無変更に保つためのもの — [F] `aidlc-graph.ts:1212-1219` ★ ([S] `01-workflow-model.md:336-339` が同じ転置規則を記述)。
- **深さ・キーワード・説明はここに入らない**。それらは `<harness>/scopes/aidlc-<name>.md` の frontmatter 側 — [S] `01-workflow-model.md:325-345`。
- エミッタ: `canonicalScopeGridJson` (`aidlc-graph.ts:1416-1418`)。スコープ名の整列は転置側が済ませており、per-scope のステージキーはステージ数値順に従う — [S] `01-workflow-model.md:907-909`。体裁は stage-graph と同じ 2 スペース + 末尾改行。
- v2.6.40 Claude 配布での実測: **11 スコープ列**、EXECUTE 数は bugfix 7 / classic 26 / enterprise 33 / express 10 / feature 33 / infra 13 / mvp 23 / poc 8 / refactor 8 / security-patch 10 / workshop 26 — 計測 [S] `01-workflow-model.md:1125`。完全な 33×11 セル表は [S] `01-workflow-model.md` §5.3。

### 3.2 初期化ステージの特例

転置の述語は `s.phase === "initialization" || (s.scopes ?? []).includes(scope)` (`aidlc-graph.ts:1402`)。**initialization の 3 ステージは frontmatter に関係なく全列で EXECUTE** になる — [S] `01-workflow-model.md:341-345`。

### 3.3 composed scope (コンポーザ書込み列)

- コンポーザは承認時に **2 つだけ**書く: `scopesDir` に identity ファイル `aidlc-<name>.md` (frontmatter は `name` / `depth` / `keywords: []`) と、`scopeGridPath` に `"<name>": { "stages": { ... } }` エントリ。
- 逐語: *"**NEVER run `aidlc-graph.ts compile` after the write.** The runtime reads the JSON verbatim."* — [S] `01-workflow-model.md:1052-1057`。
- 再コンパイル時に消えないよう `mergeComposedScopes` (`aidlc-graph.ts:1432-1459`) が転置に無いオンディスク列を折り返す。ただし `preserveNames` ガードにより**対応する `.md` が無い孤児列は落とす**。落とさないと逐語 *"the name stays 'valid' and resolves as all-SKIP, an emptied plan with no diagnostic"* — [S] `01-workflow-model.md:548-559`。
- composed scope は `nearestStockScopes` の候補から意図的に除外される (stock 判定は「いずれかのステージが宣言している名前」に限る) — [S] `01-workflow-model.md:561-563`。

---

## 4. scope identity ファイル — 3 つ目の入力

Issue #7 項目 3 の記述は 2 ファイルしか挙げていないが、`validScopes()` の権威が `.md` の存在である以上、リーダには 3 つ目の入力がある。

### 4.1 「スコープは 1 ファイルではなく 2 ファイル」

[S] `01-workflow-model.md:325-345` (§5.1 *"A scope is two files, not one"*) が正:

1. **identity** — `core/scopes/aidlc-<name>.md` (dist では `<harness>/scopes/`)。YAML frontmatter が `name` / `depth` / `description` / `keywords`、任意で `plugin` / `testStrategy` / `runner` / `skeleton` / `review_cap` / `freeform_default` を供給。パーサは `loadScopeMetadataAll` (`core/tools/aidlc-lib.ts:8643-8722`)。
2. **グリッド列** — 全ステージの `scopes:` frontmatter リストの転置 (`transposeScopeGrid`, `core/tools/aidlc-graph.ts:1384-1409`)。

権威関係は逐語で明言されている: *"Scope validity is the .md-presence authority (validScopes), not the grid"* (`core/tools/aidlc-graph.ts:991-992`)。`loadScopeMapping` は両者をレガシーの `ScopeDefinition` 形 `{depth, stages, keywords, description, testStrategy?, plugin?, runner?, skeleton}` に再 join する (`core/tools/aidlc-lib.ts:8828-8852`)。

### 4.2 frontmatter の検証 (loud errors)

[S] `01-workflow-model.md:351-364`:

| 規則 | 逐語 / 典拠 |
| --- | --- |
| `skeleton` は `on` か `off` | `` `Scope file ${filePath} has invalid skeleton value "${skeleton}". Expected "on" or "off".` `` (`aidlc-lib.ts:8697-8700`) |
| `review_cap` は `adversarial` \| `advisory` \| `none` | `aidlc-lib.ts:8706-8716` |
| 2 ファイル間の `name:` 重複は致命 | `aidlc-lib.ts:8664-8670` |
| `plugin:` が `aidlc-` 始まりは拒否 (コアランナーのパスを潰すため) | `aidlc-lib.ts:8684-8687` |
| `freeform_default: true` は**有効な**スコープ中 1 つまで | `aidlc-lib.ts:8785-8790` |
| `description:` は「そのスコープが何のためにあるか」の 1 行で、エンジン自身が読む宣言された intent | [S] `01-workflow-model.md:366-372` (`aidlc-lib.ts:8674`, `:8842`) |

frontmatter 欠落・`name` 欠落の逐語文言は [F]/[G] のみで確認: `Scope file missing frontmatter: ${path}` / `Scope file ${path} missing required frontmatter: name` ★。

---

## 5. upstream エンジンの読込方法

### 5.1 読込の依存グラフ (実行時)

```text
loadGraph()            ← aidlc-graph.ts。プロセス内キャッシュ 1 個
  └─ loadStageGraph()  ← aidlc-lib.ts。プロセス内キャッシュ 1 個。stage-graph.json を読む
loadScopeGrid()        ← aidlc-graph.ts。キャッシュ 1 個。scope-grid.json を読む
loadScopeMetadata()    ← aidlc-lib.ts。キャッシュ 1 個。<harness>/scopes/*.md の frontmatter
loadScopeMapping()     ← aidlc-lib.ts。キャッシュ 1 個。metadata のキーを軸に grid を join
validScopes()          ← loadScopeMapping() のキー集合をソート
subgraphForScope(s)    ← validScopes で検証 → loadScopeGrid()[s] の EXECUTE 抽出 → loadGraph() を numericStageOrder でソート
```

典拠: [F] `aidlc-graph.ts:329-352, 704-713`、[F] `aidlc-lib.ts:2837-2864, 7393-7481` ([G] で同一構造を再確認: `amadeus-lib.ts:7281-7308, 7393, 7475`) ★。[S] 側の裏付けは `02-orchestration-engine.md:58` (エンジンが使うライブラリ read の一覧に `loadGraph` / `nextInScopeStage` / `firstInScopeStageOfPhase` / `validScopes`)、`01-workflow-model.md:919` (*"the runtime resolves stages from the compiled graph only (loadGraph)"*)。

### 5.2 キャッシュ

- `stage-graph.json` / `scope-grid.json` / scope metadata / scope mapping / `validScopes` は**それぞれモジュールレベルのシングルトンで 1 プロセス 1 回だけ読む**。`_reset*ForTests()` がテスト用の唯一の無効化口。★
- upstream の CLI はワンショットプロセスなので「1 呼び出し = 1 回読む」で十分。**mtime 監視も再読込もしない**。★
- `loadGraph()` は「返した配列を呼び出し側が mutate してはならない」と明記 ([F] `aidlc-graph.ts:700-701`) ★。

### 5.3 検証 (= ロード時にはほぼ何もしない)

**upstream はロード時にスキーマ検証を行わない。** `JSON.parse` の結果を**信頼境界 1 回のキャスト**で `StageEntry[]` として通す。コメントが理由を明記:

> "JSON.parse returns `any`; we trust the on-disk schema (project-controlled data file written by the framework, not user input). Phase E will replace this trust boundary with an `isStageEntryArray()` type guard."
> — [F] `core/tools/aidlc-lib.ts:2852-2855` ★ ([G] `amadeus-lib.ts:7297-7300` に同一)

`loadGraph()` 側も同様に *"Single trust-boundary cast: stage-graph.json was emitted by canonicalStageGraphJson, which writes only fields declared on GraphStage. The narrowing happens at compile, not at load."* ([F] `aidlc-graph.ts:706-709`) ★。

**構造検証はすべて compile 時** ([S] `01-workflow-model.md` §8.4 の 10 不変条件) と **`validateGrid`** ([S] `01-workflow-model.md` §5.4) に前倒しされている。`validateGrid` の代表的エラー:

- 未知 slug、`EXECUTE`/`SKIP` 以外の値 → エラー (strict/非 strict 両方)
- グリッドがコンパイル済みステージを 1 つでも欠く → `"Every compiled stage must be explicitly EXECUTE or SKIP."` (`aidlc-graph.ts:1134-1155`)

ただし**これらは `validate-grid` / `validate-scope` / `recompose` の検査であって、ランタイムの読込経路では走らない。**

### 5.4 欠損・不整合時の読込側の実挙動

| 事象 | 実挙動 |
| --- | --- |
| `stage-graph.json` が読めない | **throw** (§5.5 の逐語文言) |
| `stage-graph.json` が不正 JSON | **throw** (§5.5 の逐語文言) |
| `scope-grid.json` が読めない／不正 | **throw しない**。`loadGraph()` の `scopes[]` から**その場で転置して導出** (*"callers never see a hard ENOENT for a derivable artifact"*) — [F] `aidlc-graph.ts:322-352` ★、[G] `amadeus-lib.ts` `loadScopeGridForMapping` |
| グリッドにあるが `.md` が無いスコープ | **ランタイムから見えない**。`loadScopeMapping` は metadata のキーを軸に回すので列ごと落ちる |
| `.md` はあるがグリッド列が無いスコープ | **zero-EXECUTE な正当スコープ** (unknown ではない)。`grid[name]?.stages ?? {}` → `subgraphForScope` は `[]` を返す — [S] `01-workflow-model.md:339-341` (逐語: *"A scope file present with no stage naming it is a legal zero-EXECUTE scope, not an unknown one"*)、[F] `aidlc-graph.ts:1040-1046` |
| グリッド列に slug が無い | `effectivePlanAction` が **`undefined`** を返す (`SKIP` ではない = 「このグリッドがコンパイルしていないステージ」)。**2 値への畳み込みは呼び出し側の責務**と明記 — [G] `amadeus-lib.ts:8276-8290` ★ |
| ステージファイルがディスクに無いのにグラフにある | `stageGraphDrift().missingFiles` → doctor が**hard fail** |
| ステージファイルはあるがグラフに無い | `uncompiledStages` → **advisory のみ** (*"the file is silently never executed until `aidlc-graph compile` regenerates the graph. Advisory"*) — [S] `01-workflow-model.md:911-921`, [S] `07-hooks.md:119` |

### 5.5 逐語エラー文言 (★ = ピン留め未確認、§8 参照)

**グラフ読込** ([F]/[G] 双方で完全一致 — ピン留めでの再確認は必要):

```text
Stage graph not readable at ${p}: ${errorMessage(err)}. Reinstall the framework or re-run setup to restore the data file.
Stage graph not readable at ${p}: ${errorMessage(err)}. AIDLC_STAGE_GRAPH points to ${p}; unset it to use the default.
Stage graph at ${p} is not valid JSON: ${errorMessage(err)}
```

(env が設定されているときだけ hint 節が後者に切り替わる。[F] `aidlc-lib.ts:2841-2860`) ★

**スコープ解決**:

```text
Unknown scope: "${scope}". Valid scopes: ${[...validScopes()].join(", ")}
```

`subgraphForScope` のみ **throw**。`nextInScopeStage` / `firstInScopeStageOfPhase` / `stagesInScope` は同じ未知スコープに対して **`null` / `[]` を返す**。この非対称は観測可能な契約 — [F] `aidlc-graph.ts:1046-1050`, [G] `amadeus-lib.ts:8214, 8296, 8317` ★。

**スコープ metadata**:

```text
Scope file missing frontmatter: ${path}
Scope file ${path} missing required frontmatter: name
Scope file ${filePath} has invalid skeleton value "${skeleton}". Expected "on" or "off".
```

3 行目は [S] `01-workflow-model.md:355-357` に逐語あり。1〜2 行目は [F]/[G] 由来 ★。その他 [S] 明記のスコープ検証は §4.2 の表。

**グラフを読む上位の失敗態度**:

- エンジン: 未捕捉の read エラー (グラフ欠損・state 破損) は非ゼロ exit ＋ stderr にメッセージ。**"never a half-emitted directive on stdout"** — [S] `02-orchestration-engine.md:76` (`aidlc-orchestrate.ts:6163-6168`)。
- センサーディスパッチャ: emit の**前に**グラフを解決する (orphan-FIRED 防止)。`loadGraph()` が throw したらプロセスは exit 1 し、`SENSOR_FIRED` 行を 1 つも出さない — [S] `06-sensors.md:268`。
- セッション開始フック: `stageGraphDrift()` は「malformed なグラフが起動をブロックしないよう」ラップされている — [S] `07-hooks.md:119`。
- プラグイン compose: `<harness>/tools/data/stage-graph.json` を再読込し、全プラグインステージ slug が存在し `enabled: false` でないことを確認。**読めないグラフは「欠損」と同じ扱い** (自己修復の再コンパイルを起動) — [S] `11-plugin-system.md:466-468`。

### 5.6 グラフ／グリッドを読む主要な述語 (Rust で再現が要るもの)

| 関数 | 意味論 | 典拠 |
| --- | --- | --- |
| `subgraphForScope(scope)` | 未知スコープなら throw。`grid[scope].stages` の EXECUTE 集合でグラフを filter し、`numericStageOrder` でソートして返す。**ランタイムでは topo ソートしない** (compile の edge-local 不変条件により数値順が有効な topo 順であることが保証されているため) | [F] `aidlc-graph.ts:1032-1060` ★, [S] `01-workflow-model.md:880-887` |
| `nextInScopeStage(afterSlug, scope, stateContent?)` | **グラフ配列順**に `afterSlug` の次から前進走査。checkbox が completed/skipped なら読み飛ばし、`effectivePlanAction` が EXECUTE の最初のノードを返す。無ければ `null` | [G] `amadeus-lib.ts:8209-8248` ★, [S] `02-orchestration-engine.md:232` |
| `effectivePlanAction(suffixes, scopeStages, slug)` | `suffixes?.get(slug) ?? scopeStages[slug]` — **state ファイルの per-stage EXECUTE/SKIP サフィックス (recompose 由来) が静的グリッドに勝つ**。両方に無ければ `undefined` | [S] `01-workflow-model.md:196-199`, [S] `02-orchestration-engine.md:234`, [G] `amadeus-lib.ts:8283-8290` |
| `firstInScopeStageOfPhase(phase, scope)` | `subgraphForScope` の並びから最初の該当 phase ノード。walking-skeleton ゲートアンカーの**導出元** (ハードコードではない) | [S] `01-workflow-model.md:603-615`, [S] `02-orchestration-engine.md:254` |
| `stagesInScope(scope)` | 全ステージについて `{slug, phase, action}` を返す。`aidlc-graph resolve` が出す `.aidlc-plan.json` と**バイト同一**であることが全 11 スコープでパリティテスト済み | [S] `01-workflow-model.md:935-937` |
| `numericStageOrder(a, b)` | `"P.I"` を `parseInt` 2 個に割り、P → I の順で数値比較 (`"1.10" > "1.9"`) | [G] `amadeus-graph.ts` ★ |
| `validScopes()` | `loadScopeMapping()` のキーをソートした集合。**実体は scope `.md` の存在**が権威 (グリッドではない) | [S] `01-workflow-model.md:333-335` |
| route hash | `sha256(JSON.stringify({node, scopeStages: subgraphForScope(scope).map(s => s.slug)}))` — **グラフノード全体がハッシュ入力に入る** (continue_token の束縛) | [S] `02-orchestration-engine.md:181` |
| `stageGraphDrift()` | slug 集合の差分だけ (session-start hot path 用の軽量チェック)。`missingFiles` = graph→disk (hard fail)、`uncompiledStages` = disk→graph (advisory) | [S] `01-workflow-model.md:911-921` |

---

## 6. Rust リーダが保存すべき「仕様」と、最適化してよい「実装」

判定原則は `docs/specs/00-policy.md` §2 の 2026-08-22 オーナー裁定に従う — **仕様 = 観測可能な契約 (オンディスク形式・監査行・逐語文言・CLI 面での振る舞い・原子性) は踏襲必須。実装 = 内部機構は仕様を守る限り自由で、本家実装の模倣は要件ではない。**

### 6.1 保存すべき観測可能契約 (= 仕様。逸脱台帳行き)

**A. オンディスク形式 (読み)**

1. `stage-graph.json` の**ルートは配列**。要素は §2.2 の 28 フィールド集合 (未知フィールドは**将来追加を許容**して無視すること — §6.3 参照)。
2. `scope-grid.json` は `{ <scope>: { stages: { <slug>: "EXECUTE"|"SKIP" } } }`。**中間の `"stages"` キーを省略しない** (レガシー `mapping[scope].stages` 互換のための構造)。
3. `rules_in_context` は**オブジェクト配列** `{path, scope}`、`sensors_applicable` も**オブジェクト配列** `{id, path, matches?}`。文字列配列に潰さない。
4. `inputs` / `outputs` は**文字列** (配列にしない)。`outputs` は記述用途のみで、機械可読な出力は `produces`。
5. `number` は**文字列** `"P.I"`。数値化・正規化しない (`"3.10"` を `3.1` にしてはならない)。
6. **配列の文書順を保持する** (`Vec`)。`nextInScopeStage` は文書順で走査し `stageIndex` は文書順インデックスであるのに対し、`subgraphForScope` は `numericStageOrder` でソートする。この 2 経路の**使い分けを潰さない**。
7. パス配置と env シーム: `<harnessRoot>/tools/data/{stage-graph,scope-grid}.json`、`AIDLC_STAGE_GRAPH` / `AIDLC_SCOPE_GRID` のオーバライド。

**B. 読込の失敗態度**

8. `stage-graph.json` の**欠損／不正 JSON は throw (非ゼロ exit ＋ stderr)**。§5.5 の逐語 3 文言 (hint 節の env 分岐込み)。
9. `scope-grid.json` の欠損は**エラーにせず、グラフの `scopes[]` から転置で導出**する。ここを厳格化すると新規 fixture ツリーが壊れる。
10. **未知スコープの非対称**: `subgraphForScope` は `Unknown scope: "<s>". Valid scopes: <csv>` で throw、`nextInScopeStage` / `firstInScopeStageOfPhase` / `stagesInScope` は `None` / 空を返す。
11. **`.md` あり × グリッド列なし = zero-EXECUTE な正当スコープ** (エラーにしない)。**グリッド列あり × `.md` なし = ランタイムから不可視** (エラーにしない)。
12. **グリッドに slug が無い = `undefined`**。`SKIP` に畳まない (3 値を返し、畳み込みは呼び出し側に委ねる)。
13. `initialization` の 3 ステージは全スコープ列で EXECUTE (転置の特例)。
14. `mode: "agent-team"` は**明示的に未実装として拒否**し、既定経路にフォールスルーさせない。
15. **half-emitted directive を出さない** (読込失敗は stdout に何も書かず exit)。

**C. 派生・射影の形**

16. directive 側の `sensors_applicable` は **id の `string[]`**、`rules_in_context` は**パスの `string[]`** (transport 時に active space 基準へ上書き)。
17. `stage_file` はグラフに無く `phase` + `slug` から導出。
18. `rules_in_context[].path` は `default` スペースに**コンパイル時ピン**されており、実行時に再解決してはならない (active space への rebase は `rulesContentEntries` が別途行う)。
19. route hash の入力に**グラフノード全体**が入る (`JSON.stringify(node)` の非正準・挿入順シリアライズ)。ADR 0001 の `contract-compact` プロファイルの対象。
20. **コンポーザが書いた `scope-grid.json` は逐語で読む** (読んだ後に再コンパイルしない)。

**D. 書き (compile を実装する場合のみ)**

21. `FIELD_ORDER` 28 の**キー順**と `undefined` 落とし、`JSON.stringify(x, null, 2)` + 末尾改行。`compile --check` のバイト比較が成立するために必須 (ADR 0001 の `contract-pretty`)。
22. scope-grid はスコープ名アルファベット昇順、per-scope はステージ数値順。
23. `mergeComposedScopes` の `preserveNames` ガード (`.md` 無き孤児列は落とす)。

### 6.2 最適化してよい内部機構 (= 実装。仕様書の「実装ノート」行き)

| 本家の機構 | amadeus-ng で自由にしてよい理由 |
| --- | --- |
| **モジュールレベル可変シングルトン + `_reset*ForTests()`** | 観測不能。Rust では `OnceCell`／リポジトリ trait への `&self` 注入／呼び出しごとの明示ロードのいずれでもよい。むしろ Gateway に閉じ込めるのが `10-orchestration.md` §3 の `StageGraphReader` ポート設計と整合 |
| **`loadScopeGrid` → `loadGraph` の遅延循環依存** (`require()` による circular import 回避) | TS の import 循環回避の産物。Rust には不要 |
| **`loadScopeMapping` が `ScopeDefinition` へ join する形** | レガシー `scope-mapping.json` 互換のための中間表現。Rust では grid と metadata を別型で持ち、必要な述語だけ提供してよい |
| **「ロード時に検証しない」というトラストキャスト** | upstream 自身が *"Phase E will replace this trust boundary with an `isStageEntryArray()` type guard"* と将来の厳格化を予告している。Rust の serde による**構造的パースは強化であって逸脱ではない** — ただし §6.3 の条件付き |
| **線形スキャン** (`findIndex` / `filter` / 33 件のプレーンソート) | 33 ノード規模の前提。`HashMap<StageSlug, usize>` のインデックス化は自由 |
| **`subgraphForScope` の毎回再ソート** | メモ化自由 |
| **`topoSort` / `findCycles` を持つが runtime では使わない** | [S] `01-workflow-model.md:887` が *"do not gate runtime iteration today"* と明記。実装しないという選択も可 |
| **プロセスごとのワンショット読込 (mtime 監視なし)** | 長寿命プロセスにする場合の再読込戦略は自由。ただし**同一呼び出し内でグラフが変わらない**ことは continue_token の route hash 束縛が前提にしている |
| **エラー生成が例外 throw** | Rust は `Result` でよい。**逐語文言と exit code と stdout を汚染しないことだけ**が契約 |
| **`display_order` という名前** | [S] `04-stage-protocol.md:107` が `stage-definition.md` から引く旧名。**実フィールドは `number`** ([S] `01-workflow-model.md` §3.2/§8.2 が正)。旧名に追随しないこと |

### 6.3 Rust で serde を使うときの明示的な設計判断 (要オーナー裁定)

upstream が「ロード時無検証」であるのに対し、serde はパース時に構造を強制する。**この差は観測可能になりうる**ので、次を明示的に決める必要がある:

1. **未知フィールドを許容する** (`deny_unknown_fields` を付けない)。プラグインや将来版が `FIELD_ORDER` を増やしても、既存 amadeus-ng が読めなくなってはならない (実際 [G] fork は `plugin_source` / `bundle` / `category` を独自追加している)。
2. **欠損 optional は `Option` / 空 default**。`#[serde(default)]` を配列・マップに付ける。
3. **どこまでを parse エラーにするか**: 本家は「`mode` が未知文字列」でも load は通り、使用時に初めて壊れる。Rust で `mode` を enum にすると load 時に落ちる。**「グラフ全体が読めない」と「1 ノードが使えない」の観測差**が出るため、`mode` / `execution` / `phase` / `review_class` を厳格 enum にするか、`Unknown(String)` を持つ寛容 enum にするかを決める必要がある (推奨: `phase` / `execution` / `review_class` は厳格、`mode` は `agent-team` を含む閉集合＋未知は使用時エラー、で §6.1-14 を満たす)。
4. **数値の型**: `reviewer_max_iterations` は整数型に固定 (ADR 0001 決定 4「整数は整数型で持つ」)。
5. **`enabled` の欠損 = true** (`Option<bool>` の `None` を有効とみなす)。ただし「削除される」のか「`false` が立つ」のかは §8 の未確定事項。

### 6.4 本抽出の落とし先 (執筆時点の状況)

抽出開始時点の `docs/specs/` にはグラフリーダの契約が無く、断片が次の 5 か所に散っていた:

- `docs/specs/01-domain-model.md:41,62` — 「コンパイル済み `stage-graph.json` / `scope-grid.json`」を **Published Language の 1 本目**として位置づけ (workflow-definition → orchestration / verification)。`:75` で `StageGraph` を workflow-definition の成果物集約として列挙。`:195` で `StageGraph` / `RuntimeGraph` / `UnitDag` の語彙分離を規定 (無修飾の「グラフ」は禁止)。
- `docs/specs/10-orchestration.md:16` (裁定 B1) — **scope grid は workflow-definition の不変の成果物として読むだけ**。recompose の flip は orchestration のコマンドで、`effectivePlanAction` は orchestration が所有する read model。永続化は workspace。
- `docs/specs/10-orchestration.md:80` — ポート `StageGraphReader` の**1 行のみ**。
- `docs/specs/10-orchestration.md:176` — in-memory Gateway 一式 (StateFile / AuditLedger / **StageGraph** / Lock) でユースケーステストを回す方針。
- `docs/adr/0001-canonical-json-serializer.md` — **「stage-graph の 28 フィールド順は struct 宣言で符号化」** (決定 3)。ディスク成果物は `contract-pretty` (2 スペース + 末尾改行)、route hash 等は `contract-compact`。

→ 本抽出の規範化先は [`12-workflow-definition.md`](../12-workflow-definition.md) (スライス 1 = グラフリーダ契約) で、`10-orchestration.md` §3 の `StageGraphReader` 行からも参照を張ってある。

---

## 7. 実装への含意 (優先順)

1. `StageGraphReader` ポートは**2 つの成果物を 1 つの Gateway で**扱う (compile が両者を lockstep で出すため、片方だけ新しい状態は本家でも想定外)。ただし**失敗態度は非対称** (graph は fatal、grid は導出フォールバック)。
2. ドメイン型は `StageGraph` (`Vec<StageNode>` + slug インデックス) と `ScopeGrid` (`BTreeMap<ScopeName, BTreeMap<StageSlug, PlanAction>>` — ただし**内部順序と emit 順序は別問題**、読みだけなら BTreeMap でよい)。
3. `validScopes()` の権威が **scope `.md` の存在**である以上、項目 3 のリーダは `<harness>/scopes/aidlc-*.md` の frontmatter パーサも**同時に**必要になる (`depth` / `keywords` / `description` / `testStrategy` / `runner` / `skeleton` / `review_cap` / `freeform_default` / `plugin`)。Issue の記述は 2 ファイルだけを挙げているが、実際は**3 つ目の入力**がある (§4)。
4. Next ラダー分岐 5・finality 判定・skeleton アンカーがすべて `subgraphForScope` / `nextInScopeStage` / `firstInScopeStageOfPhase` に依存するので、これら 3 述語＋`effectivePlanAction` (の grid 参照半分) が項目 3 の最小面。
5. B10 (レビュアーレシート述語) に必要なのはノードの `reviewer` / `review_class` / `reviewer_max_iterations` の 3 フィールドのみ。feature 相当スコープに reviewer 宣言ステージが存在することは [S] `01-workflow-model.md` §4 の 13 ステージ表で確認できる。

---

## 8. 未確定事項

- `FIELD_ORDER` 28 エントリの**並び順**が [S] に列挙されていない。メンバー集合は [S] から確定できる (12 必須 + 15 任意 − `when` − `required_sections` + `rules_in_context` + `sensors_applicable` + `enabled` = 28 で数が一致) が、順序は未確認。v2.2.0 [F] は 22 エントリ、downstream fork [G] は別メンバーの 28 エントリで、いずれもピン留め `3c3146cf` の順序を証明しない。ADR 0001 決定 3 が「28 フィールド順は struct 宣言で符号化」と規定している以上、compile (書き) を実装する時点で `dist/claude/.claude/tools/data/stage-graph.json` の実バイトから確定させる必要がある。**読み専用のスライス 1 では不要**。
- `enabled` フィールドの正確な意味論。[S] `11-plugin-system.md:723` の *"applyPluginSelection() then deletes or sets `enabled: false` on each node per the selection"* は「ノードを削除する」とも「`enabled` キーを削除する」とも読める。[S] `11-plugin-system.md:466-468` の compose 側は「slug が present かつ `enabled: false` でないこと」を検査しており両方の存在を示唆。`enabled` が真のとき出力にキーが出るのか出ないのかを、ピン留めの実 JSON で確認する必要がある。
- `summary_confirmation` の値域。[S] は `summary_confirmation: required` の観測値と「27 ステージが宣言」しか示さない。列挙型なのか (`required` / `optional` / …)、boolean 相当なのかが未確定。
- §5.5 の逐語エラー文言 3 種 (`Stage graph not readable at …` / `… is not valid JSON: …` / `Unknown scope: "…". Valid scopes: …`) と scope frontmatter の 2 種は [F] v2.2.0 と [G] fork で完全一致していたが、**ピン留め `3c3146cf` での逐語確認が未了**。D6 で「LLM エージェントの分岐条件になっている逐語文言」は互換必須なので、文言カタログ (ADR 0002) に載せる前に確定が必要。
- `AIDLC_SCOPE_GRID` / `AIDLC_SCOPES_DIR` / `AIDLC_SCOPE_MAPPING` が [S] `03-state-audit-runtime.md` §2.3 の環境変数一覧に載っていない。テストシーム専用なので D6 の互換対象に含めるかどうか (含めるなら Rust 側にも同名で用意するか) がオーナー裁定事項。
- serde による構造的パースをどこまで厳格にするか (§6.3-3)。upstream は「ロード時無検証」なので、`mode` に未知値が入ったグラフは load を通り使用時に壊れる。Rust で enum 化すると load 時に落ち、「グラフ全体が読めない」対「1 ノードだけ使えない」という観測差が生じる。推奨は `phase` / `execution` / `review_class` を厳格 enum、`mode` は `agent-team` を含む閉集合＋未知は使用時エラー、だが要裁定。
- `scope-grid.json` 欠損時の「グラフから転置して導出」フォールバックを Rust でも実装するか。dist 配布では必ず両方揃うので実質デッドパスだが、`compile` を実装しないスライス 1 では「グリッドが無い」= 設定ミスとして fatal にしたほうが診断が良い可能性がある。逸脱台帳行きかどうかの判断が要る。
- 項目 3 のスコープに **`<harness>/scopes/aidlc-*.md` の frontmatter パーサ**が含まれるか。`validScopes()` の権威が `.md` 存在である以上、grid だけでは `Unknown scope` 判定も zero-EXECUTE 判定も成立しない。Issue #7 項目 3 の記述は 2 ファイルしか挙げていないため、スコープの明示的な拡張が必要 (本書 §4 は 3 入力前提で書いてある)。
- `stage-graph.json` の配列順 (文書順) と `numericStageOrder` ソート順が乖離しうるケースの扱い。本家では compile が数値順にソートして emit するので実質一致するが、`nextInScopeStage` は文書順、`subgraphForScope` は再ソートという 2 経路が残っている。amadeus-ng が「読み込み時に必ず数値順に正規化する」ことを選ぶと、手編集グラフに対する挙動が本家と分岐する。仕様として文書順保持を義務づけるか、正規化を許すかの裁定 (`12-workflow-definition.md` §8 F2 は**文書順保持を暫定規範**として採用済み)。
- [S] `04-stage-protocol.md:61,107` が旧名 `display_order` で computed field を記述している (実フィールドは `number`)。[S] `01-workflow-model.md` §10 の相違表にこの行が入っていないため、04 側の記述が意図的な別名なのか単なる upstream 側 doc drift の転写なのかを確認したい (実装は `number` に従えばよいので影響は低い)。
- 本調査で参照した 2 つの実ツリー ([F] awslabs v2.2.0 / [G] amadeus fork) は **stage-0 の正式なゴールデン採取源ではない**。Issue #7 項目 0 (オーナー担当のゴールデン採取) でピン留め `3c3146cf` の `dist/claude/` が導入され次第、本書の ★ 印および [F]/[G] 由来の記述をすべて再検証する必要がある。
