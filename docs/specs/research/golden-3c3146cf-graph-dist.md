> 採取元: **`awslabs/aidlc-workflows` 公開リポジトリからの直接採取** — ピン留めコミット `3c3146cf`（`3c3146cfd7cef33020d48e8d48d4e80d0f8c2820`、v2.6.40、branch `v2`）の `core/tools/aidlc-graph.ts`（120,939 B）・`core/tools/aidlc-stage-schema.ts`（28,582 B）と、配布物 `dist/claude/.claude/tools/data/{stage-graph.json,scope-grid.json}`（81,850 B / 13,509 B）。既存 research 文書と違い、as-built 仕様（`docs/upstream/specs/`）の二次引用ではなく **upstream ソースと配布実バイトを `curl` で取得して読解・機械照合した一次採取**である。採取日 **2026-08-22**（Issue #7 項目 0）。12-workflow-definition.md スライス 1 の未決事項を裁定するための材料。
>
> **検証 grep の要約**（全項目一致）: `FIELD_ORDER` 28 エントリ（`aidlc-graph.ts:449-478`）✅ ／ dist 33 ノード全件のキー列が `FIELD_ORDER` の部分列・違反 0・圏外キー 0 ✅ ／ スコープ列 11・`33 × 11 = 363` セル全数が as-built `01-workflow-model.md` §5.3 の表と一致 ✅ ／ 列ごとの EXECUTE 数 11/11 一致 ✅ ／ `enabled` キー出現 **0/33**・`plugin` キー **0/33** ✅ ／ `summary_confirmation` は `required` 27 件・`if-present` 0 件 ✅ ／ `stage-graph.json` md5 `3ee59d7a177bd55d2e8392fb9028561d` が as-built M18 と一致、`scope-grid.json` sha1 `60fb4547307a925456bafbcfabf2ffd408552f1d` が as-built `01:1133` と一致（**as-built の実測は md5 ではなく `shasum` = SHA-1**）✅ ／ emit 体裁（`JSON.stringify(x, null, 2)` + 末尾改行 1 個）のラウンドトリップが両ファイルとも byte 完全一致 ✅ ／ 配列順が `numericStageOrder` と完全一致（`0.1`〜`4.7` の 33 件）✅。
>
> **永続化された成果物**: 本書が実測対象とした dist 実バイト 2 本は [`tests/golden/upstream-3c3146cf/`](../../../tests/golden/upstream-3c3146cf/) にバイト無変更で置いてあり、パリティテスト `modules/core/interface-adapter/tests/golden_parity_test.rs` が本書の実測値を期待値として読む。
>
> 本書は採取レポートの**原文**であり、逐語ブロック・upstream 行番号・実測表を採取時のまま保持する。本文が記録する `/private/tmp/…/scratchpad/…` は採取セッションの作業ディレクトリであり、既に存在しない（dist 2 本の永続先は上記のとおり）。

---

> **注**: 本レポートは調査結果のみ。リポジトリのファイルは一切変更していない (`git status` は開始時と同一)。

---

## 0. 採取物とピン留め同一性

| 保存先 (絶対パス) | bytes | 同一性検証 |
| --- | ---: | --- |
| `/private/tmp/claude-501/-Users-j5ik2o-orca-workspaces-amadeus-ng-docs/351513f3-85bf-44e8-92ca-bea27cc446f6/scratchpad/upstream-src/aidlc-graph.ts` | 120,939 | 指示された想定サイズと**一致** |
| `.../scratchpad/upstream-src/aidlc-stage-schema.ts` | 28,582 | (補助採取。`summary_confirmation` 値域確定のため) |
| `.../scratchpad/upstream-dist/stage-graph.json` | 81,850 | md5 `3ee59d7a177bd55d2e8392fb9028561d` = as-built `00-overview.md:445` (M18) と**一致** |
| `.../scratchpad/upstream-dist/scope-grid.json` | 13,509 | sha1 `60fb4547307a925456bafbcfabf2ffd408552f1d` = as-built `01-workflow-model.md:1133` の `60fb4547…` と**一致** |

`curl` 一発で全件取得成功 (`gh api` フォールバック不要)。参考値: stage-graph sha256 `c7afda6e0c57a7a248cb6322878d3ed3c58b14d7b483269e03add20d436bab8c` / scope-grid sha256 `326deb8be9e027f832adf21f37e89c3fa86e531840233852d7be5d9bc5ff67aa`、scope-grid md5 `ef5c35ef6e6a31ffb636383d673dd31f`。

> **重要な訂正**: as-built `01:1133` の `60fb4547…` は **md5 ではなく shasum (SHA-1)**。測定コマンドが `shasum` であることと一致する。タスク指示の「md5 が as-built の実測 (scope-grid 60fb4547…) と一致するか」は、**ハッシュ種別が md5 ではなく sha1** という点だけ読み替えれば **一致** である。

---

## 1. `FIELD_ORDER` 28 エントリの正確な並び順 (:449-478) — **確定**

`aidlc-graph.ts:447-478` 逐語:

```ts
// --- Field-order pin for canonical JSON emission ---

const FIELD_ORDER = [
  "slug",
  "number",
  "name",
  "plugin",
  "enabled",
  "phase",
  "execution",
  "condition",
  "lead_agent",
  "support_agents",
  "mode",
  "for_each",
  "workspace_requires",
  "produces",
  "optional_produces",
  "produces_kinds",
  "consumes",
  "requires_stage",
  "sensors",
  "scopes",
  "reviewer",
  "reviewer_max_iterations",
  "review_class",
  "summary_confirmation",
  "inputs",
  "outputs",
  "rules_in_context",
  "sensors_applicable",
] as const;
```

行番号対応 (ADR 0001 決定 3「28 フィールド順は struct 宣言で符号化」用の確定表):

| # | 行 | key | | # | 行 | key |
| ---: | ---: | --- | --- | ---: | ---: | --- |
| 1 | 450 | `slug` | | 15 | 464 | `optional_produces` |
| 2 | 451 | `number` | | 16 | 465 | `produces_kinds` |
| 3 | 452 | `name` | | 17 | 466 | `consumes` |
| 4 | 453 | `plugin` | | 18 | 467 | `requires_stage` |
| 5 | 454 | `enabled` | | 19 | 468 | `sensors` |
| 6 | 455 | `phase` | | 20 | 469 | `scopes` |
| 7 | 456 | `execution` | | 21 | 470 | `reviewer` |
| 8 | 457 | `condition` | | 22 | 471 | `reviewer_max_iterations` |
| 9 | 458 | `lead_agent` | | 23 | 472 | `review_class` |
| 10 | 459 | `support_agents` | | 24 | 473 | `summary_confirmation` |
| 11 | 460 | `mode` | | 25 | 474 | `inputs` |
| 12 | 461 | `for_each` | | 26 | 475 | `outputs` |
| 13 | 462 | `workspace_requires` | | 27 | 476 | `rules_in_context` |
| 14 | 463 | `produces` | | 28 | 477 | `sensors_applicable` |

### 1.1 research §8 のメンバー算術も裏取り済み

`aidlc-stage-schema.ts:161-176` 逐語:

```ts
const REQUIRED_FIELDS = [
  "slug",
  "phase",
  "execution",
  "condition",
  "lead_agent",
  "support_agents",
  "mode",
  "produces",
  "consumes",
  "requires_stage",
  "inputs",
  "outputs",
] as const;

const OPTIONAL_FIELDS = ["number", "name", "plugin", "for_each", "workspace_requires", "optional_produces", "produces_kinds", "sensors", "scopes", "reviewer", "reviewer_max_iterations", "review_class", "summary_confirmation", "when", "required_sections"] as const;
```

12 (必須) + 15 (任意) − `when` − `required_sections` + `rules_in_context` + `sensors_applicable` + `enabled` = **28**。research §8 の推定算術は**完全に一致**した。なお `enabled` は `OPTIONAL_FIELDS` に**無い** = ステージ YAML では宣言不可能で、compile が注入するフィールドである (§4 参照)。

### 1.2 ネストしたオブジェクトのキー順も構築順で pin されている (FIELD_ORDER には現れない)

compile 側 (スライス 2) のバイトパリティに必須:

- `Consume`: `aidlc-graph.ts:1988-1997`
  ```ts
  const consumes: Consume[] = consumesRaw.map((c) => {
    const out: Consume = {
      artifact: c.artifact,
      required: c.required,
    };
    if (c.conditional_on !== undefined) {
      out.conditional_on = c.conditional_on;
    }
    return out;
  });
  ```
  → `artifact` → `required` → (任意) `conditional_on`。dist 実バイトでも 139/139 件がこの順。
- `SensorResolution`: `aidlc-graph.ts:783-786`
  ```ts
    const entry: SensorResolution = { id: sensor.id, path: sensor.path };
    if (sensor.manifest.matches !== undefined) {
      entry.matches = sensor.manifest.matches;
    }
  ```
  → `id` → `path` → (任意) `matches`。dist 実バイトでも 81/81 件がこの順。
- `RuleResolution`: `aidlc-graph.ts:683,685` はいずれも `out.push({ path: r.path, scope: r.scope });` → `path` → `scope`。dist 実バイトでも 129/129 件がこの順。

---

## 2. canonical emit 実装 (インデント・末尾改行・undefined 落とし)

### 2.1 `canonicalStageGraphJson` (`:1345-1362`) 逐語

```ts
/** Canonical JSON emitter. The ONLY place that writes stage-graph.json
 *  bytes. Pinning the emitter in one function makes `compile --check`
 *  byte-compare robust — formatter drift is impossible when there's
 *  exactly one writer. */
export function canonicalStageGraphJson(stages: GraphStage[]): string {
  // Build each object with pinned key order so JSON.stringify emits
  // keys in the canonical order regardless of construction order.
  const ordered = stages.map((s) => {
    const out: Record<string, unknown> = {};
    for (const key of FIELD_ORDER) {
      const v: unknown = s[key as keyof GraphStage];
      if (v === undefined) continue;
      out[key] = v;
    }
    return out;
  });
  return `${JSON.stringify(ordered, null, 2)}\n`;
}
```

観測可能契約 3 点が確定:
1. **undefined 落とし**は `if (v === undefined) continue;` の 1 行のみ。`null` / `[]` / `""` / `false` は**落とさない** (`workspace_requires: false` は emit されうる — ただし後述の通り dist には現れない)。
2. **インデント 2 スペース** (`JSON.stringify(ordered, null, 2)`)。
3. **末尾改行 1 個**をテンプレートリテラルで付与。

### 2.2 `canonicalScopeGridJson` (`:1411-1418`) 逐語

```ts
/** Canonical JSON emitter for the scope grid. The ONLY place that writes
 *  scope-grid.json bytes — same sole-writer discipline as
 *  canonicalStageGraphJson, so `compile --check` byte-compares are robust.
 *  Scopes are emitted in sorted order (transposeScopeGrid already sorts);
 *  per-scope stage keys follow the stages array's numeric order. */
export function canonicalScopeGridJson(grid: ScopeGrid): string {
  return `${JSON.stringify(grid, null, 2)}\n`;
}
```

グリッド側にはフィールド順 pin が**無い**。列順は `transposeScopeGrid` (`:1398` の `[...scopeNames].sort()`) と `mergeComposedScopes` (`:1456-1457`) の再ソートに委ねられ、**行 (slug) 順は `stages` 配列の順 = 文書順**をそのまま踏襲する。

### 2.3 実バイトでの体裁検証 (再現実行)

`json.dumps(d, indent=2, ensure_ascii=False) + "\n"` でラウンドトリップした結果、**両ファイルともバイト完全一致** (81,850 / 13,509)。CRLF なし。`\u` エスケープ **0 件**、非 ASCII は em-dash `—` の生 UTF-8 が 11 個 (33 バイト、`condition` 内) — つまり `JSON.stringify` は非 ASCII をエスケープしないので、Rust 側も `serde_json::to_string_pretty` の既定 (非エスケープ) で一致する。

### 2.4 `transposeScopeGrid` (`:1379-1409`) — 初期化ステージ特例の逐語

```ts
  const grid: ScopeGrid = {};
  for (const scope of [...scopeNames].sort()) {
    const stagesMap: Record<string, "EXECUTE" | "SKIP"> = {};
    for (const s of stages) {
      stagesMap[s.slug] =
        s.phase === "initialization" || (s.scopes ?? []).includes(scope)
          ? "EXECUTE"
          : "SKIP";
    }
    grid[scope] = { stages: stagesMap };
  }
```

`initialization` フェーズは `scopes:` 宣言に関わらず全列 EXECUTE — research §3.2 の記述を逐語確認。

---

## 3. `subgraphForScope` / `validateGrid` / `mergeComposedScopes`

### 3.1 `Unknown scope: …` throw 逐語 — **確定 (★ 解消)**

`aidlc-graph.ts:994-999`:

```ts
export function subgraphForScope(scope: string): GraphStage[] {
  if (!validScopes().has(scope)) {
    throw new Error(
      `Unknown scope: "${scope}". Valid scopes: ${[...validScopes()].join(", ")}`
    );
  }
```

**同一文言がもう 1 箇所**、`resolvePlanForScope` (`:1050-1054`) にも存在する (research 未記載):

```ts
  if (!validScopes().has(scope)) {
    throw new Error(
      `Unknown scope: "${scope}". Valid scopes: ${[...validScopes()].join(", ")}`
    );
  }
```

`grep -n "Unknown scope"` は `997` と `1052` の **2 件のみ**。

### 3.2 非対称の裏取り

`nextInScopeStage` / `firstInScopeStageOfPhase` / `stagesInScope` は **`aidlc-graph.ts` に実装が無い** (定義は `aidlc-lib.ts`)。本ファイルでの言及はヘッダコメント `:1-5` のみ:

```ts
// Stage-graph library + CLI. Exports the 8-function API consumed by
// the doctor handler (see aidlc-utility.ts handleDoctor) and the
// runtime resolution layer (lib.ts's nextInScopeStage,
// firstInScopeStageOfPhase, stagesInScope delegate here via lazy
// require).
```

したがって **「throw するのは `subgraphForScope`／`resolvePlanForScope` の 2 つだけで、`nextInScopeStage` 等は null/[] を返す」という非対称の *throw 側* はピン留めで確定**したが、***null 返し側* は本担当ファイル外 (`aidlc-lib.ts`) なのでピン留め未確認のまま**。research §5.6 / 12 §4 の該当行は依然 [G] 由来である (別途 `aidlc-lib.ts` の採取が要る)。

なお **未知スコープの定義そのもの**は逐語で確定した (`:988-993`):

```ts
 *  Throws on unknown scope. Returns [] when scope has zero EXECUTE
 *  entries — a legitimate edge case, e.g. a freshly-dropped
 *  .claude/scopes/aidlc-x.md that no stage names yet (valid scope, empty
 *  grid column). Scope validity is the .md-presence authority (validScopes),
 *  not the grid: a scope present as a file but absent from the grid is a
 *  zero-EXECUTE scope, not an unknown one. */
```

→ 12 §「identity ファイルがありグリッド列が無いスコープは unknown ではない」は **[S] だけでなく実装逐語でも裏取り完了**。

また **ランタイム非トポソート**も逐語確認 (`:980-986`, `:1006-1008`):

```ts
/** The scope's sub-DAG as a linear array, sorted by numeric order.
 *  Filter to scope-mapping's EXECUTE slice, then sort by number.
 *  No topological sort at runtime — numeric order is a valid topo-
 *  order of the full graph (proven by t65, protected by compile's
 *  invariant) and therefore of any node subset. The future worktree
 *  scheduler will consume the sub-DAG structure directly for
 *  parallelism.
```

```ts
  return loadGraph()
    .filter((s) => executeSlugs.has(s.slug))
    .sort((a, b) => numericStageOrder(a.number, b.number));
```

### 3.3 `validateGrid` 逐語 (`:1118-1218`) — エラー/アドバイザリ 5 形

```ts
      errors.push(
        `Grid names unknown stage "${slug}" - not in the compiled stage graph.`
      );
```
(`:1136-1138`)

```ts
      errors.push(
        `Grid entry "${slug}" has invalid action "${action}" (expected EXECUTE or SKIP).`
      );
```
(`:1142-1144`)

```ts
    errors.push(
      `Grid is missing ${missingSlugs.length} compiled stage entr${missingSlugs.length === 1 ? "y" : "ies"}: ` +
        `${missingSlugs.join(", ")}. Every compiled stage must be explicitly EXECUTE or SKIP.`,
    );
```
(`:1151-1154` — 単複の `entry`/`entries` 切替に注意)

```ts
        errors.push(
          `Stage "${stage.slug}" requires artifact "${consume.artifact}" ` +
            `but no stage in the graph produces it.`
        );
```
(`:1180-1183` — TRUE orphan、両モード共通の hard error)

```ts
        const message =
          `Stage "${stage.slug}" requires artifact "${consume.artifact}" ` +
          `whose producer(s) [${producers.map((p) => p.slug).join(", ")}] ` +
          `are not on the "${label}" path.`;
        if (opts?.strict) {
          errors.push(
            `${message} Strict (recompose) mode rejects a starved required input.`
          );
        } else {
          advisories.push(`${message} Ensure existing artifact is current.`);
        }
```
(`:1188-1198` — 同一 message に strict/lenient で異なる**後置節**が付く。`label` の既定は `:1126` の `opts?.label ?? "proposed grid"`、`validateScope` からは `:1096` で `label: scope` が渡る)

`validateScope` は grid 全体を組んでから委譲する (`:1092-1096`):

```ts
  const subgraph = subgraphForScope(scope); // throws on unknown scope (unchanged)
  const grid: Record<string, "EXECUTE" | "SKIP"> = {};
  for (const s of loadGraph()) grid[s.slug] = "SKIP";
  for (const s of subgraph) grid[s.slug] = "EXECUTE";
  return validateGrid(grid, { ...opts, label: scope });
```

`required: false` の沈黙は `:1169` の `if (!consume.required) continue;`、`conditional_on` フィルタは `:1171-1177`。

### 3.4 `mergeComposedScopes` の `preserveNames` (`:1420-1459`) 逐語

```ts
/** Fold COMPOSED-scope entries from the on-disk grid into a freshly
 *  transposed one. The transpose derives only the stock scopes (those a
 *  stage's `scopes:` frontmatter names); a composed scope's grid entry is
 *  appended at approval time by the composer and has no frontmatter
 *  producer, so a bare re-transpose would silently drop it — and with the
 *  scope's `.md` still present the name stays "valid" and resolves as
 *  all-SKIP, an emptied plan with no diagnostic. Any on-disk entry whose
 *  scope name the transpose does not produce survives the recompile; keys
 *  re-sort so the canonical emitter stays deterministic. Unparseable or
 *  malformed on-disk grids contribute nothing (fresh wins). When
 *  `preserveNames` is supplied, an orphan grid column with no matching scope
 *  identity file is dropped rather than mistaken for a composed scope. */
export function mergeComposedScopes(
  fresh: ScopeGrid,
  onDiskJson: string | null,
  preserveNames?: ReadonlySet<string>,
): ScopeGrid {
  if (!onDiskJson) return fresh;
  let onDisk: unknown;
  try {
    onDisk = JSON.parse(onDiskJson);
  } catch {
    return fresh;
  }
  if (typeof onDisk !== "object" || onDisk === null || Array.isArray(onDisk)) return fresh;
  const merged: ScopeGrid = { ...fresh };
  for (const [name, entry] of Object.entries(onDisk as Record<string, unknown>)) {
    if (name in merged) continue;
    if (preserveNames !== undefined && !preserveNames.has(name)) continue;
    if (
      typeof entry === "object" && entry !== null && !Array.isArray(entry) &&
      typeof (entry as { stages?: unknown }).stages === "object"
    ) {
      merged[name] = entry as ScopeGrid[string];
    }
  }
  const sorted: ScopeGrid = {};
  for (const k of Object.keys(merged).sort()) sorted[k] = merged[k];
  return sorted;
}
```

**`preserveNames` の意味論 (確定)**: `undefined` を渡すと「on-disk にあって fresh に無い列」を**すべて**保存する。集合を渡すと**その集合に含まれる名前だけ**保存し、identity ファイルの無い孤児列を落とす。呼び出し側 (`:1941-1966`) では `composedNames` = 「on-disk 由来で stock でない名前」∩「`loadScopeMetadataAll()` にある名前」として構成され、さらに `filterScopeGrid(..., selectedScopeNames, composedNames)` で plugin 選択のフィルタから composed 列だけを免除している。

---

## 4. `applyPluginSelection` の `enabled` 意味論 — **完全確定 (OPEN QUESTION 解消)**

`aidlc-graph.ts:1573-1578` 逐語 (関数全体が 6 行):

```ts
function applyPluginSelection(stages: GraphStage[]): void {
  for (const stage of stages) {
    delete stage.enabled;
    if (!stageEnabledBySelection(stage)) stage.enabled = false;
  }
}
```

型宣言 (`:141`): `  enabled?: false;` — **`true` は型として表現不可能**。

確定した 3 点:

1. **ノードは削除されない**。`applyPluginSelection` は配列長を変えず、`canonicalStageGraphJson(stages)` (`:1953`) は**無効ノードも含めて全件 emit** する。[S] `11-plugin-system.md:723` の *"deletes or sets `enabled: false`"* の "deletes" は **`delete stage.enabled` (キーの削除) を指す**のであって、ノード削除ではない。
2. **有効時はキーが出力されない**。毎回 `delete` してから、無効なときだけ `= false` を立てる。`canonicalStageGraphJson` の `if (v === undefined) continue;` により、有効ノードでは `enabled` キーが**JSON に現れない**。→ **`None` (キー不在) = 有効**という amadeus-ng の解釈 (`fs_stage_graph_reader.rs:219`) は正しい。
3. **グリッド側は無効ノードを行ごと落とす** (`:1958`): `transposeScopeGrid(stages.filter((s) => s.enabled !== false), seededScopeNames)`。つまり **graph には出るが grid の行には出ない**という非対称がある。読み手はこれを「グリッドが列にその slug を持たない」= 3 値契約の**未収載 (`None`)** として観測する。

判定は一貫して `s.enabled !== false` で行われる (`:1590`, `:1595`, `:1622`, `:1625`, `:1958`) — つまり **`undefined` も `true` も「有効」**。閉包検証の逐語 (`:1602-1606`):

```ts
      throw new Error(
        `Plugin selection closure failed: enabled stage "${stage.slug}" consumes required artifact "${consume.artifact}", ` +
          `but its only producer(s) are disabled: ${producerList}. ` +
          `Enable plugin(s) ${disabledPlugins.join(", ")} or disable the consuming stage.`
      );
```

順序エッジのアドバイザリ (`:1626`): `` dropped.push(`${stage.slug} requires ${dep} (${stagePluginOwner(depStage)}, disabled)`); ``、既定オーナーは `:1570` の `return stage.plugin ?? "aidlc";`。

---

## 5. dist 実バイトの実測

### (a) 先頭ノードの実キー順 = FIELD_ORDER 検証 — **一致**

`stage-graph.json` の `[0]` (`workspace-scaffold`) の実キー順:

```
 1 slug          7 lead_agent      13 sensors
 2 number        8 support_agents  14 scopes
 3 name          9 mode            15 inputs
 4 phase        10 produces        16 outputs
 5 execution    11 consumes        17 rules_in_context
 6 condition    12 requires_stage  18 sensors_applicable
```

**33 ノード全件**について「実キー列が `FIELD_ORDER` の部分列である」ことを機械検証 → **違反 0 件**。`FIELD_ORDER` に無いキーの出現 → **0 件**。

### (b) ノード数 33・スコープ列 11・列ごとの EXECUTE 数

- ノード数 = **33** ✅、`number` は `0.1`〜`4.7` で**配列順 = `numericStageOrder` 順** (F2 の「文書順保持」は数値順と一致するので、スライス 1 では観測差なし)。
- スコープ列 = **11**、挿入順 = 辞書順 (`bugfix, classic, enterprise, express, feature, infra, mvp, poc, refactor, security-patch, workshop`)。各列の top-level キーは `stages` のみ、行数は全列 **33**、行順は**グラフの文書順と完全一致** (辞書順ではない)。
- 列ごとの EXECUTE 数 (as-built `01-workflow-model.md:414-451` の Total 行との突合):

| scope | as-built | 実測 | | scope | as-built | 実測 |
| --- | ---: | ---: | --- | --- | ---: | ---: |
| enterprise | 33 | **33** ✅ | | security-patch | 10 | **10** ✅ |
| feature | 33 | **33** ✅ | | express | 10 | **10** ✅ |
| classic | 26 | **26** ✅ | | poc | 8 | **8** ✅ |
| workshop | 26 | **26** ✅ | | refactor | 8 | **8** ✅ |
| mvp | 23 | **23** ✅ | | bugfix | 7 | **7** ✅ |
| infra | 13 | **13** ✅ | | | | |

さらに **セル単位で 363 セル (33 行 × 11 列) 全数**を as-built §5.3 の表と突合 → **不一致 0**。表の行名 (`initialization (3)` を 3 slug へ展開) とグラフ slug の集合も過不足なし。セル値域は `EXECUTE` 197 / `SKIP` 166 の 2 値のみ。

### (c) `enabled` キーの出現統計 — **0/33**

`plugin` も **0/33**。dist claude 配布はプラグイン無選択状態で compile されているため、両キーとも一切現れない。§4 の意味論と整合 (「有効ならキーが出ない」)。

キー出現統計 (FIELD_ORDER 順、分母 33):

| key | n | key | n | key | n | key | n |
| --- | ---: | --- | ---: | --- | ---: | --- | ---: |
| slug | 33 | mode | 33 | requires_stage | 33 | inputs | 33 |
| number | 33 | for_each | **5** | sensors | 33 | outputs | 33 |
| name | 33 | workspace_requires | **1** | scopes | 33 | rules_in_context | 33 |
| plugin | **0** | produces | 33 | reviewer | **13** | sensors_applicable | 33 |
| enabled | **0** | optional_produces | **1** | reviewer_max_iterations | **13** | | |
| phase | 33 | produces_kinds | **4** | review_class | **13** | | |
| execution | 33 | consumes | 33 | summary_confirmation | **27** | | |
| condition | 33 | | | | | | |

`workspace_requires` が 1 件しか出ないのは、**値 `false` のステージは YAML に書かれず `undefined` として落ちる**ため (`optional_produces` / `produces_kinds` / `for_each` も同様に「宣言したステージだけ」)。→ **Rust 側 `#[serde(default)]` + `bool` は正しい**。

### (d) `summary_confirmation` の実値域 — **`required` のみ 27 件**

型宣言 (`aidlc-graph.ts:199-200`):

```ts
  // Deterministic pre-generation consolidated-summary checkpoint policy.
  summary_confirmation?: "required" | "if-present";
```

スキーマ側の強制 (`aidlc-stage-schema.ts:326-333`) 逐語:

```ts
    "summary_confirmation" in o &&
    o.summary_confirmation !== undefined &&
    o.summary_confirmation !== "required" &&
    o.summary_confirmation !== "if-present"
```
```ts
      `summary_confirmation must be one of required, if-present, got ${describe(o.summary_confirmation)}`,
```

→ **2 値列挙 (`required` / `if-present`)**。boolean 相当ではない。dist 実測は **`required` × 27、`if-present` × 0** で、as-built `04-stage-protocol.md:154,634` (M12) の「27 × `required`; no `if-present`」と**一致**。

### (e) `sensors_applicable` / `rules_in_context` の実形状

**`rules_in_context`** — 要素は例外なく `{path, scope}` の 2 キー (129/129)、キー順は `path` → `scope`。配列順は常に **org → team → project → phase** (30 ステージ) または **org → team → project** (initialization の 3 ステージ; phase ルールファイルが `initialization` について存在しないため)。`scope` 値域は 4 値のみ (org 33 / team 33 / project 33 / phase 30)。phase ルールの実パスは 4 種 (`aidlc/spaces/default/memory/phases/{ideation,inception,construction,operation}.md`)。実サンプル (`0.1 workspace-scaffold`):

```json
[{"path": "aidlc/spaces/default/memory/org.md", "scope": "org"}, {"path": "aidlc/spaces/default/memory/team.md", "scope": "team"}, {"path": "aidlc/spaces/default/memory/project.md", "scope": "project"}]
```

**`sensors_applicable`** — 配列長は 0(×3) / 2(×19) / 3(×5) / 4(×2) / 5(×4)。要素は **81 件すべてが `{id, path, matches}` の 3 キー** (キー順 `id`→`path`→`matches`)。実在する 6 種の全パターン:

```json
{"id": "claim-sources", "path": ".claude/sensors/aidlc-claim-sources.md", "matches": "**/{aidlc-docs,intents}/**"}
{"id": "required-sections", "path": ".claude/sensors/aidlc-required-sections.md", "matches": "**/{aidlc-docs,intents}/**"}
{"id": "upstream-coverage", "path": ".claude/sensors/aidlc-upstream-coverage.md", "matches": "**/{aidlc-docs,intents}/**"}
{"id": "traceability", "path": ".claude/sensors/aidlc-traceability.md", "matches": "**/traceability.json"}
{"id": "linter", "path": ".claude/sensors/aidlc-linter.md", "matches": "**/*.{ts,js}"}
{"id": "type-check", "path": ".claude/sensors/aidlc-type-check.md", "matches": "**/*.{ts,tsx}"}
```

出現数: `required-sections` 30 / `upstream-coverage` 29 / `traceability` 8 / `type-check` 7 / `linter` 6 / `claim-sources` 1。

> ⚠️ **ソースコメントと実データの乖離**: `aidlc-graph.ts:121-127` は *"matches is omitted when the manifest declares no path filter (e.g., required-sections, upstream-coverage)"* と書くが、**ピン留め dist ではその 2 つの sensor もちゃんと `matches` を持つ** (`**/{aidlc-docs,intents}/**`)。コメントが陳腐化している。ただし `:783-786` の実装は `matches` が任意である構造を保つので、**Rust 側は `Option<String>` のままが正しい** (実データは常に Some)。

`sensors` (生の import id 列) と `sensors_applicable` の長さは対応する (例: `code-generation` は `["required-sections","linter","type-check","traceability"]` → 4 件)。

### (f) ハッシュ一致 — **一致** (§0 参照、ただし種別は sha1)

---

## 6. `12-workflow-definition.md` §11 / research §8 の解消状況

| 未決事項 | 状態 | 根拠 |
| --- | --- | --- |
| `FIELD_ORDER` 28 の並び順 | **解消** | `aidlc-graph.ts:449-478` + dist 33 ノード全件検証 |
| `enabled` の意味論 (ノード削除か / 有効時のキー) | **解消** | `:141`, `:1573-1578`, `:1953`, `:1958` + dist 0/33 |
| `summary_confirmation` の値域 | **解消** | `:200` + schema `:326-333` + dist (`required` 27, `if-present` 0) |
| ★ `Unknown scope: "…". Valid scopes: …` | **解消** (かつ発生箇所は 2 つ) | `:997`, `:1052` |
| ★ `Stage graph not readable at …` | **未解消** | `aidlc-lib.ts` 側。本担当ファイルには存在しない (`grep` 0 件) |
| ★ `… is not valid JSON: …` | **未解消** | 同上 |
| ★ `Scope file missing frontmatter: …` | **未解消** | 同上 |
| ★ `Scope file … missing required frontmatter: name` | **未解消** | 同上 |
| ★ `nextInScopeStage` / `firstInScopeStageOfPhase` の null 返し | **半解消** | throw 側 2 関数は確定。null 返し側は `aidlc-lib.ts` 未採取 |
| ★ `loadGraph` の信頼境界コメント | **解消** | `:804-806` が [F] 引用と逐語一致 |
| ★ グリッド欠損時の転置導出フォールバック | **解消** | `:415-445` (`loadScopeGrid`)、doc に *"so callers never see a hard ENOENT for a derivable artifact"* (`:420`) |
| ★ 「呼び出し側は返り配列を mutate してはならない」 | **解消** | `:798` `Caller must NOT mutate the returned array.` |
| ★ 純粋転置・レガシー `.stages` 互換 | **解消** | `:1364-1373` のブロックコメント |
| ★ `AIDLC_SCOPE_GRID` テストシーム | **解消** | `:383-394` |
| F2 (文書順保持 vs 数値順正規化) | **観測差なし**を確認 | dist の配列順 = 数値順 (33/33)。手編集グラフでのみ差が出る |
| 12 §10 表 #3 (全列挙を load 時厳格) | **正規データ全数 load 可**を静的確認 | §7 |
| 04 の `display_order` 旧名 | **未解消** | 本ファイルに `display_order` は 0 件 (実フィールドは `number`) — 04 側 doc drift の可能性が高いが確証なし |
| `AIDLC_SCOPES_DIR` / `AIDLC_SCOPE_MAPPING` を D6 互換に含めるか | **オーナー裁定待ち** (事実としては `:428`, `:380` に実在) |

**残タスク (Issue #7 項目 0 の続き)**: 逐語 4 形 (`Stage graph not readable` / `is not valid JSON` / scope frontmatter 2 形) と `nextInScopeStage` 系の null 返しは `core/tools/aidlc-lib.ts` の採取が必要。**ADR 0002 文言カタログへの登録は、この 4 形が揃うまで保留すべき**。

---

## 7. `FsStageGraphReader` が dist 実バイトを読めるかの静的検討

対象: `/Users/j5ik2o/orca/workspaces/amadeus-ng/docs/modules/core/interface-adapter/src/orchestration/fs_stage_graph_reader.rs`

### 7.1 ワイヤ構造体 × dist 実データ 突合 — **全項目 PASS**

dist 全 33 ノード / 全 11 列に対し、`WireStageNode` / `WireConsume` / `WireRuleInContext` / `WireSensorRef` / `WireScopeColumn` の要求 (必須の有無・JSON 型・列挙値域・slug 文法・番号文法) を機械照合 → **違反 0 件**。

| 検査 | 結果 |
| --- | --- |
| default 無し必須 6 フィールド (`slug`/`number`/`name`/`phase`/`execution`/`mode`) が全ノードに文字列で存在 | 33/33 ✅ |
| `condition`/`lead_agent`/`inputs`/`outputs` が文字列 | 33/33 ✅ |
| `support_agents`/`produces`/`optional_produces`/`requires_stage`/`sensors`/`scopes` が `list<str>` | ✅ |
| `workspace_requires` が bool | 1/1 ✅ |
| `reviewer_max_iterations` が非負整数 (`u32` 適合) | 13/13 (全て `2`) ✅ |
| `produces_kinds` が `map<str, list<str>>` | 4/4 ✅ |
| `consumes[]` に `artifact` (str) + `required` (bool 139/139) | ✅ |
| `consumes[].conditional_on` 値域 | `brownfield` 14 件のみ (`greenfield` は dist に非出現) → `BrownfieldGreenfield::parse` 適合 ✅ |
| `rules_in_context[]` = `{path, scope}` のみ | 129/129 ✅ |
| `sensors_applicable[]` ⊆ `{id, path, matches}` かつ `id`/`path` 必須 | 81/81 ✅ |
| grid 各列 top-level キー = `{stages}` のみ | 11/11 ✅ |
| grid セル値 ∈ `{EXECUTE, SKIP}` (`PlanAction::parse`) | 363/363 ✅ |
| slug 文法 `^[a-z][a-z0-9-]*$` (`StageSlug::parse`) — ノード・`requires_stage`・grid 行 | 全件 ✅ |
| `number` 文法 `^\d+\.\d+$` (`StageNumber::parse`、ドット 1 個) | 33/33 ✅ |
| slug 一意性 (`StageGraph::new` の `DuplicateSlug`) | 重複 0 ✅ |
| `requires_stage` の参照先が全てグラフ内 | 全件 ✅ (リーダ側は未検証だが問題なし) |
| UTF-8 妥当性 (`fs::read_to_string`) | ✅ (非 ASCII は em-dash のみ) |

厳密 enum 4 種の値域も dist をカバーしている:

| enum | Rust が受理 | dist 実測 |
| --- | --- | --- |
| `PhaseId` (`phase.rs:53-57`) | initialization / ideation / inception / construction / operation | initialization 3, ideation 7, inception 9, construction 7, operation 7 → **全て受理** |
| `ExecutionKind` (`execution_kind.rs:43-44`) | ALWAYS / CONDITIONAL | ALWAYS 11, CONDITIONAL 22 → **受理** |
| `StageMode` (`stage_mode.rs:54-58`) | inline / subagent / pipeline / mob / agent-team | inline 29, subagent 2, pipeline 1, mob 1 → **受理** (`agent-team` は dist 非出現) |
| `ReviewClass` (`review_class.rs:48-49`) | adversarial / advisory | advisory 8, adversarial 5 → **受理** |
| `RuleScope` (`stage_node.rs:71-74`) | org / team / project / phase | 4 値のみ → **受理** |

**結論: `FsStageGraphReader::load()` はピン留め dist の実バイトを 33 ノード全数 load できる** (12 §10 表 #3 が要求した「ゴールデン採取で正規データ全数 load 確認」を静的に充足)。

### 7.2 スライス 1 (読み) では問題ないが記録すべき差分

1. **`name` / `number` が `#[serde(default)]` 無しの必須**。upstream スキーマ上この 2 つは `OPTIONAL_FIELDS` だが compile が必ず seed するため、**正規 dist では常に存在**する。手編集で欠落したグラフに対してのみ Rust は `InvalidJson` で落ち、upstream は通してしまう — 12 §10 表 #3 と同じ「fail-loud 側に倒す」枠内の差で、**逸脱台帳は不要**。
2. **grid の行順が `BTreeMap<String,String>` で辞書順に潰れる**。読みの述語 (slug 引き) には**観測差なし**。ただし `canonicalScopeGridJson` は「**per-scope stage keys follow the stages array's numeric order**」(`:1414-1415`) を要求するので、**スライス 2 (compile) ではこの `BTreeMap` を emit にそのまま流用できない**。emit 経路には「グラフ配列順で行を並べる」順序保持の別型が要る。列 (スコープ名) 側は upstream も辞書順ソートなので `BTreeMap` で一致する。
3. **`SensorRef.matches` は実データでは常に Some** (81/81)。`Option` のままで正しいが、ゴールデンテストの期待値は「Some」で固定してよい。
4. **`enabled` の非対称 (graph に出るが grid 行から消える)** は dist では発現しない (`enabled` 0/33) ため、スライス 1 のゴールデンでは検証できない。プラグイン選択済みの配布物が必要で、**ピン留め dist からは採取不能**。
5. `WireStageNode.enabled: Option<bool>` は upstream 型 (`enabled?: false`) より寛容 (`true` も受理)。判定を `!= Some(false)` 相当に保つ限り upstream の `s.enabled !== false` と一致する。

---

## 8. 付随所見 (research/spec の訂正候補)

- **`aidlc-graph.ts:8` と `:1632` のコメントが「31 stage definitions / 31 YAML stage files」と書くが、実グラフは 33 ノード**。ヘッダコメントの陳腐化 (initialization 3 + ideation 7 + inception 9 + construction 7 + operation 7 = 33)。as-built §5.3 の 33 が正しい。
- research §2.1 は emit を `aidlc-graph.ts:1185-1198` としているが、ピン留めでは **`:1349-1362`**。同様に `loadGraph` は `:704-713` → **`:797-811`**、`subgraphForScope` は `:1032-1060` → **`:994-1009`**、`ScopeGrid` 型は `:1207-1209` → **`:1375-1377`**、`loadScopeGrid` フォールバックは `:322-352` → **`:415-445`**、`validateGrid` は `:1166-1201` → **`:1118-1218`**、`validateScope` は `:1085-1097` → **`:1085-1097` (偶然一致)**。**行番号だけがずれており、逐語内容は一致**している (v2.2.0 → 2.6.40 でファイルが 22→28 フィールドに拡張された分)。
- as-built `01-workflow-model.md:1124` の測定ノート (modes 29/2/1/1、execution 11/22、`for_each` 5、`workspace_requires` 1、reviewer 13、`summary_confirmation` 27) は**ピン留めで全項目再現**。
- `resolvePlanForScope` (`:1047-1065`) が `Unknown scope` throw の 2 番目の発生源であることは research 未記載。`.aidlc-plan.json` を書く `aidlc-graph resolve` サブコマンドの実体であり、12 §9-4 の「`stages_in_scope` の `.aidlc-plan.json` バイト同値」検証の対向実装なので、**仕様書に追記する価値がある**。

## RESOLVED OPEN QUESTIONS
- FIELD_ORDER 28 エントリの並び順 (research §8 / 12 §11) — 確定。aidlc-graph.ts:449-478 の逐語で slug, number, name, plugin, enabled, phase, execution, condition, lead_agent, support_agents, mode, for_each, workspace_requires, produces, optional_produces, produces_kinds, consumes, requires_stage, sensors, scopes, reviewer, reviewer_max_iterations, review_class, summary_confirmation, inputs, outputs, rules_in_context, sensors_applicable。dist 33 ノード全件の実キー列がこの順の部分列であることを機械検証済 (違反 0、FIELD_ORDER 外のキー 0)。ADR 0001 決定 3 の struct 宣言順はこれで確定できる。
- FIELD_ORDER のメンバー算術 (research §8) — 確定。aidlc-stage-schema.ts:161-176 で REQUIRED_FIELDS 12 / OPTIONAL_FIELDS 15 を確認し、12+15-when-required_sections+rules_in_context+sensors_applicable+enabled=28 が成立。enabled は OPTIONAL_FIELDS に無く、ステージ YAML では宣言不可能な compile 注入フィールドであることも判明。
- enabled の意味論 (research §8 / 12 §11) — 完全確定。(1) ノードは削除されない: applyPluginSelection (:1573-1578) は配列長を変えず、canonicalStageGraphJson(stages) (:1953) が無効ノードも含め全件 emit する。[S] 11-plugin-system.md:723 の "deletes" は delete stage.enabled (キー削除) の意味。(2) 有効時はキーが出力されない: 毎回 delete してから無効時のみ = false を立てるので、有効ノードでは undefined 落としにより JSON にキーが現れない。dist 実測 0/33 が裏付け。amadeus-ng の 'None = キー不在 = 有効' 解釈は正しい。(3) 型は enabled?: false (:141) で true は表現不可。(4) 判定は一貫して s.enabled !== false。(5) グリッド側は無効ノードを行ごと落とす (:1958) ため graph には出るが grid 行には出ない非対称がある。
- summary_confirmation の値域 (research §8 / 12 §11) — 確定。aidlc-graph.ts:200 の summary_confirmation?: "required" | "if-present" と aidlc-stage-schema.ts:326-333 の検証 (逐語エラー: summary_confirmation must be one of required, if-present, got ${describe(...)}) により 2 値列挙。boolean 相当ではない。dist 実測は required 27 件のみ、if-present 0 件で as-built 04-stage-protocol.md:154,634 (M12) と一致。
- 逐語 ★ 1/5: `Unknown scope: "${scope}". Valid scopes: ${[...validScopes()].join(", ")}` — ピン留めで確定。さらに research 未記載の第 2 の発生源 resolvePlanForScope (:1052) を発見 (subgraphForScope :997 と合わせて 2 箇所、grep 全 2 件)。
- subgraphForScope の非対称のうち throw 側 — 確定。throw するのは subgraphForScope と resolvePlanForScope の 2 関数のみ。nextInScopeStage / firstInScopeStageOfPhase / stagesInScope は aidlc-graph.ts に実装が無く (ヘッダコメント :3-5 が lib.ts への委譲を明記)、null/[] 返し側の逐語確認は aidlc-lib.ts の追加採取が必要 (未解消として残る)。
- canonicalStageGraphJson / canonicalScopeGridJson の emit 体裁 — 確定。undefined 落としは `if (v === undefined) continue;` の 1 行のみ (null/[]/""/false は落とさない)、インデントは JSON.stringify(x, null, 2)、末尾改行 1 個をテンプレートリテラルで付与。dist 実バイトを indent=2 + 末尾改行でラウンドトリップし両ファイルとも byte 完全一致 (81,850 / 13,509) を確認。\u エスケープ 0 件・非 ASCII は生 UTF-8 (em-dash 11 個) なので Rust の serde_json 既定と一致する。
- FIELD_ORDER に現れないネストオブジェクトのキー順 — 新規確定 (compile スライス 2 のバイトパリティ前提)。Consume は artifact→required→(任意)conditional_on (:1988-1997)、SensorResolution は id→path→(任意)matches (:783-786)、RuleResolution は path→scope (:683,685)。dist 実バイトでもそれぞれ 139/139・81/81・129/129 件がこの順。
- 「identity ファイルがありグリッド列が無いスコープは unknown ではない」— 実装逐語でも裏取り完了。:988-993 の 'Scope validity is the .md-presence authority (validScopes), not the grid: a scope present as a file but absent from the grid is a zero-EXECUTE scope, not an unknown one.'
- ランタイム非トポソート (research §5.6) — 逐語確定。:980-986 の 'No topological sort at runtime — numeric order is a valid topo-order of the full graph (proven by t65, protected by compile's invariant) and therefore of any node subset.' と実装 :1006-1008 の filter → numericStageOrder sort。
- scope-grid.json 欠損時の転置導出フォールバック (12 §11 で 2026-08-22 裁定済) — 実装逐語で再確認。loadScopeGrid (:415-445) の doc コメントに 'so callers never see a hard ENOENT for a derivable artifact' (:420)、実装は try/catch で transposeScopeGrid(loadGraph()) に倒す (:438-443)。AIDLC_SCOPE_MAPPING フィクスチャ seam が優先される分岐 (:428-436) も確認。
- initialization フェーズの全列 EXECUTE 特例 (research §3.2) — 逐語確定。transposeScopeGrid :1400-1405 の s.phase === "initialization" || (s.scopes ?? []).includes(scope)。dist でも 3 initialization stage が 11 列すべてで EXECUTE。
- loadGraph の信頼境界コメント (research §5.3 ★) — ピン留めで逐語一致を確認 (:804-806)。'Caller must NOT mutate the returned array.' (:798) も一致。
- scope-grid.json の純粋転置・レガシー .stages 互換 (research §3.1 ★) — :1364-1373 のブロックコメントで逐語確定。'It is a PURE transpose — no graph-closure, no predicate.' および 'exactly the `.stages` half of the legacy scope-mapping.json so the runtime consumers that read `mapping[scope].stages` stay byte-for-byte unchanged.'
- AIDLC_SCOPE_GRID テストシーム (research §1.4 ★) — :383-394 で確定。'Evaluated at call time so tests that set/unset mid-process see it.' も逐語確認。
- 12 §10 表 #3 (全列挙を load 時厳格) の残タスク「ゴールデン採取での正規データ全数 load 確認」— 静的に充足。dist 33 ノード全件が PhaseId/ExecutionKind/StageMode/ReviewClass/RuleScope/BrownfieldGreenfield/PlanAction/StageSlug/StageNumber の全パースを通ることを機械照合で確認 (違反 0)。
- F2 (文書順保持 vs 数値順正規化) — dist では観測差なしを確認。stage-graph.json の配列順 (0.1〜4.7) は numericStageOrder 順と完全一致するため、正規データ上は両方針が同一挙動。差が出るのは手編集グラフのみ。
- mergeComposedScopes の preserveNames 意味論 — 確定。undefined なら on-disk 由来の未知列をすべて保存、集合を渡すとその集合に含まれる名前だけ保存し identity ファイルの無い孤児列を落とす (:1448)。呼び出し側 (:1941-1966) は composedNames = (on-disk 由来 ∧ 非 stock) ∩ loadScopeMetadataAll() のキー、として構成し filterScopeGrid の第 3 引数 (免除集合) にも同じ集合を渡す。

## VERIFIED COUNTS
- FIELD_ORDER エントリ数: 期待 28 / 実測 28 (aidlc-graph.ts:449-478) — 一致
- stage-graph.json ノード数: 期待 33 (as-built 01 §5.3) / 実測 33 — 一致
- FIELD_ORDER 部分列検証: 33 ノード全件のキー列が FIELD_ORDER の部分列 / 違反 0 件、FIELD_ORDER 外キーの出現 0 件 — 一致
- scope-grid.json スコープ列数: 期待 11 (as-built 01 §5.2/§5.3) / 実測 11 — 一致
- 列ごとの EXECUTE 数 (as-built 01 §5.3 Total 行): enterprise 33/33, feature 33/33, classic 26/26, workshop 26/26, mvp 23/23, infra 13/13, security-patch 10/10, express 10/10, poc 8/8, refactor 8/8, bugfix 7/7 — 11 列すべて一致
- as-built 01 §5.3 表のセル単位突合: 363 セル (33 行 × 11 列) 全数 / 不一致 0 件 — 一致 (表の行名集合とグラフ slug 集合も過不足なし)
- grid セル値域: 期待 {EXECUTE, SKIP} / 実測 EXECUTE 197 + SKIP 166 = 363、他の値 0 件 — 一致
- enabled キー出現: 期待 (プラグイン無選択なら 0) / 実測 0/33 — 一致。plugin キーも 0/33
- summary_confirmation: as-built 04 §M12 期待 'required 27 件、if-present 0 件' / 実測 required 27, if-present 0 — 一致
- reviewer 宣言ステージ数: as-built 01:259 および 01:1124 期待 13 / 実測 13 (reviewer_max_iterations 13, review_class 13 も同数) — 一致
- mode 分布: as-built 01:1124 期待 29 inline / 2 subagent / 1 pipeline / 1 mob / 実測 同一 — 一致
- execution 分布: as-built 01:1124 期待 11 ALWAYS / 22 CONDITIONAL / 実測 同一 — 一致
- for_each 宣言数: as-built 01:1124 期待 5 / 実測 5 (全て 'unit-of-work') — 一致
- workspace_requires 宣言数: as-built 01:1124 期待 1 / 実測 1 (値 true) — 一致
- stage-graph.json md5: as-built 00-overview §M18 期待 3ee59d7a177bd55d2e8392fb9028561d / 実測 3ee59d7a177bd55d2e8392fb9028561d — 一致
- scope-grid.json ハッシュ: as-built 01:1133 期待 60fb4547… (測定コマンドは shasum = SHA-1) / 実測 sha1 60fb4547307a925456bafbcfabf2ffd408552f1d — 一致 (※タスク指示は 'md5' と表現していたが as-built の実測は md5 ではなく sha1。参考: scope-grid の md5 は ef5c35ef6e6a31ffb636383d673dd31f)
- aidlc-graph.ts サイズ: 期待 120,939 bytes (タスク指示) / 実測 120,939 — 一致
- emit 体裁: JSON.stringify(x, null, 2) + 末尾改行 1 個 の再現 / stage-graph.json 81,850 bytes・scope-grid.json 13,509 bytes ともにバイト完全一致 — 一致 (CRLF 0、\u エスケープ 0、非 ASCII は em-dash 11 個の生 UTF-8)
- 配列順 vs numericStageOrder: 期待 一致 (compile が数値順で emit) / 実測 0.1〜4.7 の 33 件が完全に数値順 — 一致
- grid 行順: 期待 'per-scope stage keys follow the stages array's numeric order' (:1414-1415) / 実測 11 列すべてで行順 = グラフ文書順 (辞書順ではない) — 一致
- grid 列順: 期待 sorted (:1398, :1457) / 実測 挿入順 = 辞書順 — 一致
- rules_in_context 形状: 要素 129 件すべて {path, scope} の 2 キー・キー順 path→scope、scope 値域は org 33 / team 33 / project 33 / phase 30 の 4 値のみ、配列順は org→team→project(→phase) 固定 (3 キー版は initialization の 3 ステージのみ) — 12 §/research の記述と一致
- sensors_applicable 形状: 要素 81 件すべて {id, path, matches} の 3 キー・キー順 id→path→matches。id 別内訳 required-sections 30 / upstream-coverage 29 / traceability 8 / type-check 7 / linter 6 / claim-sources 1 — ★不一致あり: ソースコメント :121-127 は 'matches is omitted ... (e.g., required-sections, upstream-coverage)' と書くが実データでは両者とも matches を持つ (コメントの陳腐化)。実装 :783-786 の任意構造は維持されているので Option<String> のままが正しい
- consumes 形状: 要素 139 件、キー順 artifact→required(→conditional_on)。required は bool 139/139、conditional_on は brownfield 14 件のみ (greenfield は dist 非出現) — :1988-1997 の構築順と一致
- FsStageGraphReader ワイヤ構造体適合検査: 必須/型/列挙値域/slug 文法/番号文法/slug 一意性/UTF-8 の全項目を dist 33 ノード + 363 grid セルに対し機械照合 / 違反 0 件 — 全数 load 可能
- Unknown scope throw の出現箇所: grep -n 'Unknown scope' aidlc-graph.ts / 実測 2 件 (:997 subgraphForScope, :1052 resolvePlanForScope) — research は 1 件しか記載しておらず要追記
- aidlc-graph.ts 内の graph 読込失敗文言 (Stage graph not readable / is not valid JSON) および scope frontmatter 文言 (missing frontmatter / missing required frontmatter: name): grep 実測 0 件 — 本ファイルには存在せず aidlc-lib.ts の追加採取が必要 (★ 未解消)
