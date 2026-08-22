> 採取元: **`awslabs/aidlc-workflows` 公開リポジトリからの直接採取** — ピン留めコミット `3c3146cf`（`3c3146cfd7cef33020d48e8d48d4e80d0f8c2820`、v2.6.40、branch `v2`）の `core/tools/aidlc-audit.ts`（1,589 行 / 61,642 B）と `core/knowledge/aidlc-shared/audit-format.md`（322 行 / 34,762 B）。既存 research 文書と違い、as-built 仕様（`docs/upstream/specs/`）の二次引用ではなく **upstream ソースの実バイトを `curl` で取得して読解した一次採取**である。採取日 **2026-08-22**（Issue #7 項目 0）。11-workspace.md と `modules/shared/audit-events` の裏取り材料。
>
> **検証 grep の要約**（ピン留めソース上で再実行、全 6 項目一致）: M1 副次 `aidlc-audit.ts` 1,589 行 / 61,642 B ✅ ／ M2 `VALID_EVENT_TYPES` = 86（distinct 86）✅ ／ M3 `EVENT_HEADINGS` = 86 かつ `VALID_EVENT_TYPES` との双方向差分が空 ✅ ／ M4 authority 3 集合 = `CLI_RESERVED` 8 / `CLI_PROTECTED` 18 / `MERGE_PROTECTED` 26 ✅ ／ M6 MANDATORY（`✓`）= 8 ✅ ／ M15 バイト書込サイト 5 行（import 2 + 呼出 3、行番号まで一致）✅。加えて amadeus-ng 突合 2 件（86 語のワイヤ綴り集合は完全一致、`CLI_PROTECTED` は集合一致・**宣言順のみ不一致**）と、エラーメッセージの list 連結長 1,550 バイトを実測した。
>
> 本書は採取レポートの**原文**であり、逐語ブロック・upstream 行番号・食い違い表（D-1 / D-2）を採取時のまま保持する。本文が記録する `/private/tmp/…/scratchpad/…` は採取セッションの作業ディレクトリであり、既に存在しない。

---

> **注意**: 「7 件」という指示は research §5.5 の省略記号付き引用を指すが、実測すると省略記号 (`…`) は **5 箇所**であった (`docs/specs/research/workspace-audit-ledger.md` L261, L266×3, L272)。本報告では推測せず、audit-fork / audit-merge / validateMergeDelta の**全 26 件**の拒否文言を逐語で掲載し、うち research が省略した／一切引用しなかったものを明示する。

# ゴールデン採取: `core/tools/aidlc-audit.ts` @ awslabs/aidlc-workflows `3c3146cf` (v2.6.40)

## 0. 取得と同一性

| 項目 | 値 |
| --- | --- |
| 取得元 | `https://raw.githubusercontent.com/awslabs/aidlc-workflows/3c3146cf/core/tools/aidlc-audit.ts` |
| 保存先 | `/private/tmp/claude-501/-Users-j5ik2o-orca-workspaces-amadeus-ng-docs/351513f3-85bf-44e8-92ca-bea27cc446f6/scratchpad/upstream-src/core/tools/aidlc-audit.ts` |
| サイズ | **61,642 バイト / 1,589 行** (指示の期待値と一致、M1 の `1589` とも一致) |
| 併せて取得 | `core/knowledge/aidlc-shared/audit-format.md` (34,762 B / 322 行、M6 検証用) |
| 機械可読ゴールデン | `…/scratchpad/golden/aidlc-audit-golden.json` (12,837 B) |

リポジトリのファイルは一切変更していない。

---

## 1. `EVENT_HEADINGS` 全 86 エントリ (`:192-279`) — 逐語

宣言 (`:192`) と閉じ (`:279`):

```ts
const EVENT_HEADINGS: Record<string, string> = {
```
```ts
};
```

| 行 | イベント名 | heading 文字列 (逐語) |
| ---: | --- | --- |
| 193 | `STAGE_STARTED` | `Stage Start` |
| 194 | `STAGE_AWAITING_APPROVAL` | `Stage Awaiting Approval` |
| 195 | `STAGE_REVISING` | `Stage Revising` |
| 196 | `STAGE_COMPLETED` | `Stage Completion` |
| 197 | `STAGE_JUMPED` | `Stage Jump` |
| 198 | `STAGE_SKIPPED` | `Stage Skip` |
| 199 | `PHASE_STARTED` | `Phase Start` |
| 200 | `PHASE_COMPLETED` | `Phase Completion` |
| 201 | `PHASE_VERIFIED` | `Phase Verification` |
| 202 | `PHASE_SKIPPED` | `Phase Skip` |
| 203 | `WORKFLOW_STARTED` | `Workflow Start` |
| 204 | `WORKFLOW_COMPLETED` | `Workflow Completion` |
| 205 | `WORKFLOW_PARKED` | `Workflow Parked` |
| 206 | `WORKFLOW_UNPARKED` | `Workflow Unparked` |
| 207 | `SESSION_STARTED` | `Session Start` |
| 208 | `SESSION_RESUMED` | `Session Resume` |
| 209 | `SESSION_COMPACTED` | `Session Compacted` |
| 210 | `SESSION_ENDED` | `Session End` |
| 211 | `HUMAN_TURN` | `Human Turn` |
| 212 | `WORKSPACE_SCAFFOLDED` | `Workspace Scaffolded` |
| 213 | `WORKSPACE_SCANNED` | `Workspace Scanned` |
| 214 | `WORKSPACE_INITIALISED` | `Workspace Initialised` |
| 215 | `DECISION_RECORDED` | `Decision Recorded` |
| 216 | `GATE_APPROVED` | `Gate Approved` |
| 217 | `GATE_REJECTED` | `Gate Rejected` |
| 218 | `QUESTION_ANSWERED` | `Question Answered` |
| 219 | `SUMMARY_CONFIRMATION_RECORDED` | `Summary Confirmation Recorded` |
| 220 | `REVIEW_REQUESTED` | `Review Requested` |
| 221 | `REVIEW_COMPLETED` | `Review Completed` |
| 222 | `PIPELINE_LINK_COMPLETED` | `Pipeline Link Completed` |
| 223 | `UNIT_STARTED` | `Unit Started` |
| 224 | `UNIT_PAUSED` | `Unit Paused` |
| 225 | `UNIT_RESUMED` | `Unit Resumed` |
| 226 | `UNIT_COMPLETED` | `Unit Completed` |
| 227 | `ARTIFACT_CREATED` | `Artifact Created` |
| 228 | `ARTIFACT_UPDATED` | `Artifact Updated` |
| 229 | `ARTIFACT_REUSED` | `Artifact Reused` |
| 230 | `SUBAGENT_COMPLETED` | `Subagent Completed` |
| 231 | `REVIEWER_SCOPE_BLOCKED` | `Reviewer Scope Blocked` |
| 232 | `REVIEW_FREEZE_BLOCKED` | `Review Freeze Blocked` |
| 233 | `PLAN_APPROVAL_BLOCKED` | `Plan Approval Blocked` |
| 234 | `DOCUMENT_INDEXED` | `Document Indexed` |
| 235 | `DOCUMENT_UPDATED` | `Document Updated` |
| 236 | `DOCUMENT_REMOVED` | `Document Removed` |
| 237 | `HEALTH_CHECKED` | `Health Check` |
| 238 | `SCOPE_DETECTED` | `Scope Detection` |
| 239 | `SCOPE_CHANGED` | `Scope Change` |
| 240 | `PLUGIN_SELECTION_CHANGED` | `Plugin Selection Change` |
| 241 | `DEPTH_CHANGED` | `Depth Change` |
| 242 | `TEST_STRATEGY_CHANGED` | `Test Strategy Change` |
| 243 | `REVIEW_CLASS_CHANGED` | `Review Class Change` |
| 244 | `RECOMPOSED` | `Plan Recomposed` |
| 245 | `ERROR_LOGGED` | `Error Logged` |
| 246 | `RECOVERY_COMPLETED` | `Recovery Completed` |
| 247 | `BOLT_STARTED` | `Bolt Started` |
| 248 | `BOLT_COMPLETED` | `Bolt Completed` |
| 249 | `BOLT_FAILED` | `Bolt Failed` |
| 250 | `AUTONOMY_MODE_SET` | `Autonomy Mode Set` |
| 251 | `WORKTREE_CREATED` | `Worktree Created` |
| 252 | `WORKTREE_MERGED` | `Worktree Merged` |
| 253 | `WORKTREE_DISCARDED` | `Worktree Discarded` |
| 254 | `STATE_FORKED` | `State Forked` |
| 255 | `STATE_MERGED` | `State Merged` |
| 256 | `AUDIT_FORKED` | `Audit Forked` |
| 257 | `AUDIT_MERGED` | `Audit Merged` |
| 258 | `PRACTICES_DISCOVERED` | `Practices Discovered` |
| 259 | `PRACTICES_AFFIRMED` | `Practices Affirmed` |
| 260 | `PRACTICES_OVERRIDE` | `Practices Override` |
| 261 | `PRACTICES_SECTION_EMPTY` | `Practices Section Empty` |
| 262 | `MERGE_DISPATCH_INVOKED` | `Merge Dispatch Invoked` |
| 263 | `MERGE_DISPATCH_RETURNED` | `Merge Dispatch Returned` |
| 264 | `MERGE_DISPATCH_FALLBACK` | `Merge Dispatch Fallback` |
| 265 | `SENSOR_FIRED` | `Sensor Fired` |
| 266 | `SENSOR_PASSED` | `Sensor Passed` |
| 267 | `SENSOR_FAILED` | `Sensor Failed` |
| 268 | `SENSOR_BUDGET_OVERRIDE` | `Sensor Budget Override` |
| 269 | `GUARDRAIL_LOADED` | `Guardrail Loaded` |
| 270 | `MEMORY_EMPTY` | `Memory Empty` |
| 271 | `RULE_LEARNED` | `Rule Learned` |
| 272 | `SENSOR_PROPOSED` | `Sensor Proposed` |
| 273 | `SWARM_STARTED` | `Swarm Started` |
| 274 | `SWARM_UNIT_CONVERGED` | `Swarm Unit Converged` |
| 275 | `SWARM_UNIT_FAILED` | `Swarm Unit Failed` |
| 276 | `SWARM_BATON_RETURNED` | `Swarm Baton Returned` |
| 277 | `SWARM_COMPLETED` | `Swarm Completed` |
| 278 | `SWARM_DEGRADED` | `Swarm Degraded` |

**性質 (実測)**
- 86 行、キー重複なし、**heading 文字列も 86 個すべて相異** (重複 heading 0 件) → heading からイベント名への逆写像も全単射。
- `heading === eventName` になるエントリは **0 件** — すべて別文字列。したがって `EVENT_HEADINGS[x] || x` のフォールバック (`:489`) が発動するのは非 taxonomy 名のときだけ。
- 語形の非一様性に注意 (Rust 側で機械生成すると必ず外す箇所):
  - `STAGE_COMPLETED → Stage Completion` / `PHASE_COMPLETED → Phase Completion` / `WORKFLOW_COMPLETED → Workflow Completion` は **`Completion`** (名詞化)。一方 `UNIT_COMPLETED → Unit Completed`、`REVIEW_COMPLETED → Review Completed`、`BOLT_COMPLETED → Bolt Completed`、`SUBAGENT_COMPLETED → Subagent Completed`、`RECOVERY_COMPLETED → Recovery Completed`、`SWARM_COMPLETED → Swarm Completed`、`PIPELINE_LINK_COMPLETED → Pipeline Link Completed` は **`Completed`** (過去分詞のまま)。
  - `_STARTED` も同様に割れる: `STAGE_STARTED → Stage Start` / `PHASE_STARTED → Phase Start` / `WORKFLOW_STARTED → Workflow Start` / `SESSION_STARTED → Session Start` は **`Start`**。`UNIT_STARTED` / `BOLT_STARTED` / `SWARM_STARTED` は **`Started`**。
  - `SESSION_RESUMED → Session Resume` (`Resumed` ではない) だが `UNIT_RESUMED → Unit Resumed`。`SESSION_ENDED → Session End`。
  - `_CHANGED` 系は名詞化: `SCOPE_CHANGED → Scope Change`、`DEPTH_CHANGED → Depth Change`、`TEST_STRATEGY_CHANGED → Test Strategy Change`、`REVIEW_CLASS_CHANGED → Review Class Change`、`PLUGIN_SELECTION_CHANGED → Plugin Selection Change`。`SCOPE_DETECTED → Scope Detection`、`PHASE_VERIFIED → Phase Verification`、`HEALTH_CHECKED → Health Check`、`STAGE_JUMPED → Stage Jump`、`STAGE_SKIPPED → Stage Skip`、`PHASE_SKIPPED → Phase Skip`。
  - `RECOMPOSED → Plan Recomposed` — **語幹に無い "Plan" が付く唯一の例**。機械変換では絶対に出ない。
  - `WORKSPACE_INITIALISED → Workspace Initialised` — 英式綴り (`-ised`) をそのまま維持。

---

## 2. Authority deny-list 3 集合の全メンバー逐語

### 2.1 `CLI_RESERVED_EVENT_TYPES` (8) — `:292-301`

```ts
const CLI_RESERVED_EVENT_TYPES = new Set([
  "HUMAN_TURN",
  "SUMMARY_CONFIRMATION_RECORDED",
  "ARTIFACT_CREATED",
  "ARTIFACT_UPDATED",
  "ARTIFACT_REUSED",
  "REVIEW_REQUESTED",
  "REVIEW_COMPLETED",
  "PIPELINE_LINK_COMPLETED",
]);
```

拒否関数 (`:303-309`) と拒否文言:

```ts
function refuseReservedCliEvent(eventType: string): void {
  if (CLI_RESERVED_EVENT_TYPES.has(eventType)) {
    jsonError(
      `${eventType} is reserved for its owning hook/tool and cannot be appended through the public audit CLI.`,
    );
  }
}
```

**強制点 (重要)**: `refuseReservedCliEvent` は **`main()` 内だけ**から呼ばれる — `:1546` (`append` ケース、`parseFieldArgs` より前 = パース前拒否) と、`refuseReservedCliBatch` (`:311-328`) 経由で `:1557` (`append-batch` ケース)。`handleAppend` / `appendAuditEntry` には入っていないので、**ライブラリ import 経路は CLI_RESERVED を通過しない**。また **env バイパスは存在しない** (`AIDLC_ALLOW_DIRECT_AUDIT_EVENTS` は CLI_PROTECTED 専用)。

`refuseReservedCliBatch` は JSON をパースし配列でなければ黙って return、各要素の `eventType` が string のときだけ `refuseReservedCliEvent` を呼ぶ。パース例外は握り潰す — コメント逐語:

```ts
    // The normal append-batch parser owns malformed-JSON diagnostics.
```

### 2.2 `CLI_PROTECTED_EVENT_TYPES` (18) — `:348-376` (`export` 付き)

```ts
export const CLI_PROTECTED_EVENT_TYPES = new Set([
  "HUMAN_TURN",
  "GATE_APPROVED",
  "GATE_REJECTED",
  "QUESTION_ANSWERED",
  "REVIEW_REQUESTED",
  "REVIEW_COMPLETED",
  "PIPELINE_LINK_COMPLETED",
  "ARTIFACT_REUSED",
  "SWARM_STARTED",
  "SWARM_UNIT_CONVERGED",
  "AUTONOMY_MODE_SET",
  // Unit lifecycle receipts: routing trusts UNIT_COMPLETED as the completion
  // signal (unitSettled) and UNIT_PAUSED as the hard-stop checkpoint, and the
  // owning verb verifies artifacts before committing — a CLI-forged receipt
  // would skip that verification. Owned by `aidlc-state.ts unit`.
  "UNIT_STARTED",
  "UNIT_PAUSED",
  "UNIT_RESUMED",
  "UNIT_COMPLETED",
  // DocumentKB provenance: the knowledge tool emits these through the library
  // inside its catalog transaction. A CLI-forged DOCUMENT_INDEXED whose
  // Digest+Source match a real row would make the tool's idempotent
  // audit-repair pass treat provenance as already recorded and SUPPRESS the
  // genuine row, so the CLI must not mint them.
  "DOCUMENT_INDEXED",
  "DOCUMENT_UPDATED",
  "DOCUMENT_REMOVED",
]);
```

バイパスと拒否 (`:431-442`):

```ts
function directAuditEventsAllowed(): boolean {
  return process.env.AIDLC_ALLOW_DIRECT_AUDIT_EVENTS === "1";
}

function refuseProtectedEvent(eventType: string): never {
  jsonError(
    `Direct emission of ${eventType} is blocked: it is an authority-bearing receipt owned by its ` +
      "emitting tool or hook (gate resolutions and approvals come from aidlc-orchestrate.ts report, " +
      "interview answers and reviews from aidlc-log.ts, human presence from the prompt-submit hook). " +
      "The audit CLI appends diagnostic events only."
  );
}
```

強制点: `handleAppend` (`:812-814`) と `handleAppendBatch` (`:856-860`)。両者とも `&& !directAuditEventsAllowed()` 付き。

### 2.3 `MERGE_PROTECTED_EVENT_TYPES` (26) — `:395-425` + prefix 規則 `:426-429`

```ts
const MERGE_PROTECTED_EVENT_TYPES = new Set([
  // Human authority (GATE_RESOLUTION_EVENTS + presence + autonomy).
  "HUMAN_TURN",
  "GATE_APPROVED",
  "GATE_REJECTED",
  "QUESTION_ANSWERED",
  "SUMMARY_CONFIRMATION_RECORDED",
  "AUTONOMY_MODE_SET",
  // Routing-trusted unit lifecycle receipts.
  "UNIT_STARTED",
  "UNIT_PAUSED",
  "UNIT_RESUMED",
  "UNIT_COMPLETED",
  // Referee/conductor bookkeeping, emitted against main only.
  "AUDIT_FORKED",
  "AUDIT_MERGED",
  "STATE_FORKED",
  "STATE_MERGED",
  "SWARM_STARTED",
  "SWARM_COMPLETED",
  "SWARM_DEGRADED",
  "SWARM_BATON_RETURNED",
  "SWARM_UNIT_CONVERGED",
  "SWARM_UNIT_FAILED",
  "BOLT_STARTED",
  "BOLT_COMPLETED",
  "BOLT_FAILED",
  "WORKTREE_CREATED",
  "WORKTREE_DISCARDED",
  "WORKTREE_MERGED",
]);
```

**DOCUMENT_* prefix 規則 (`:426-429`、逐語)**:

```ts
function mergeEventIsProtected(eventType: string): boolean {
  if (MERGE_PROTECTED_EVENT_TYPES.has(eventType)) return true;
  return eventType.startsWith("DOCUMENT_");
}
```

つまり判定は「列挙集合 26 に含まれる **または** イベント名が `DOCUMENT_` で始まる」。現行 86 語で prefix 側に落ちるのは `DOCUMENT_INDEXED` / `DOCUMENT_UPDATED` / `DOCUMENT_REMOVED` の 3 語だが、`validateMergeDelta` は prefix 判定より前に `VALID_EVENT_TYPES` 検査 (`:991-993`) を通すため、未知の `DOCUMENT_*` は "unknown event" で先に落ちる。prefix 規則は**将来 86 語に `DOCUMENT_*` が追加されたときの前方互換**として機能する。

### 2.4 3 集合の関係 (実測)

| 関係 | 結果 |
| --- | --- |
| `CLI_RESERVED \ CLI_PROTECTED` (3) | `ARTIFACT_CREATED`, `ARTIFACT_UPDATED`, `SUMMARY_CONFIRMATION_RECORDED` |
| `CLI_PROTECTED \ CLI_RESERVED` (13) | `GATE_APPROVED`, `GATE_REJECTED`, `QUESTION_ANSWERED`, `AUTONOMY_MODE_SET`, `SWARM_STARTED`, `SWARM_UNIT_CONVERGED`, `UNIT_STARTED`, `UNIT_PAUSED`, `UNIT_RESUMED`, `UNIT_COMPLETED`, `DOCUMENT_INDEXED`, `DOCUMENT_UPDATED`, `DOCUMENT_REMOVED` |
| `CLI_RESERVED ∪ CLI_PROTECTED` | **21** (CLI 全体で塞がるイベント数) |
| `CLI_PROTECTED \ MERGE_PROTECTED` (7) | `ARTIFACT_REUSED`, `REVIEW_REQUESTED`, `REVIEW_COMPLETED`, `PIPELINE_LINK_COMPLETED`, `DOCUMENT_INDEXED`, `DOCUMENT_UPDATED`, `DOCUMENT_REMOVED` (後 3 者は prefix 規則で実質 merge も塞がる) |
| `MERGE_PROTECTED \ CLI_PROTECTED` (15) | `SUMMARY_CONFIRMATION_RECORDED`, `AUDIT_FORKED`, `AUDIT_MERGED`, `STATE_FORKED`, `STATE_MERGED`, `SWARM_COMPLETED`, `SWARM_DEGRADED`, `SWARM_BATON_RETURNED`, `SWARM_UNIT_FAILED`, `BOLT_STARTED`, `BOLT_COMPLETED`, `BOLT_FAILED`, `WORKTREE_CREATED`, `WORKTREE_DISCARDED`, `WORKTREE_MERGED` |

**3 集合は互いに包含関係にない。** REVIEW_REQUESTED / REVIEW_COMPLETED が merge では許され CLI では禁止される点が非対称の核心 (`:377-394` の設計コメント通り: worktree の per-unit reviewer receipt は正当な作業成果物)。

---

## 3. `validateAuditEntry` (`:463-483`) — 全検査と list 連結形式

```ts
function validateAuditEntry(entry: AuditEntryInput): void {
  if (!VALID_EVENT_TYPES.has(entry.eventType)) {
    throw new Error(
      `Invalid event type: ${entry.eventType}. Must be one of: ${[...VALID_EVENT_TYPES].join(", ")}`
    );
  }
  for (const key of Object.keys(entry.fields)) {
    if (RESERVED_FIELD_KEYS.has(key)) {
      throw new Error(
        `Reserved field key: ${key}. The emitter writes **${key}**: itself; a caller-supplied ` +
          "value would forge a second matching line and spoof multiline event queries."
      );
    }
    if (!AUDIT_FIELD_KEY_PATTERN.test(key)) {
      throw new Error(
        `Invalid audit field key: ${JSON.stringify(key)}. Field keys must match ` +
          `${AUDIT_FIELD_KEY_PATTERN} so they remain one Markdown label on one physical line.`
      );
    }
  }
}
```

**検査は 3 つ、順序は固定** — (1) イベント型、(2) 予約フィールドキー、(3) キー正規表現。(2)(3) は `Object.keys(entry.fields)` の**挿入順**でキーごとに (2)→(3) の順に判定するため、`Event` と不正キーが同時にあるときは `Event` が先に来ていれば予約エラーが勝つ。値に対する検査は**一切ない** (`validateAuditEntry` は値を見ない。値の無害化は `renderAuditBlock` の行終端エスケープだけ)。

### 3.1 `Must be one of:` の list 連結形式

- 区切り: **`", "` (カンマ + 半角スペース 1 個)**。末尾区切りなし。
- 順序: `[...VALID_EVENT_TYPES]` = **Set の挿入順** = `aidlc-audit.ts:39-189` の**ソース宣言順**。`audit-format.md` のカテゴリ掲載順とは**異なる** (下記 §3.2)。
- 連結文字列は **1,550 文字 / 1,550 バイト (UTF-8)**。メッセージ全体は `Invalid event type: ` + 入力 + `. Must be one of: ` + 1550 文字。

連結される list の完全形 (逐語、1 行):

```
STAGE_STARTED, STAGE_AWAITING_APPROVAL, STAGE_REVISING, STAGE_COMPLETED, STAGE_JUMPED, STAGE_SKIPPED, PHASE_STARTED, PHASE_COMPLETED, PHASE_VERIFIED, PHASE_SKIPPED, WORKFLOW_STARTED, WORKFLOW_COMPLETED, WORKFLOW_PARKED, WORKFLOW_UNPARKED, SESSION_STARTED, SESSION_RESUMED, SESSION_COMPACTED, SESSION_ENDED, HUMAN_TURN, WORKSPACE_SCAFFOLDED, WORKSPACE_SCANNED, WORKSPACE_INITIALISED, DECISION_RECORDED, GATE_APPROVED, GATE_REJECTED, QUESTION_ANSWERED, SUMMARY_CONFIRMATION_RECORDED, REVIEW_REQUESTED, REVIEW_COMPLETED, PIPELINE_LINK_COMPLETED, UNIT_STARTED, UNIT_PAUSED, UNIT_RESUMED, UNIT_COMPLETED, ARTIFACT_CREATED, ARTIFACT_UPDATED, ARTIFACT_REUSED, SUBAGENT_COMPLETED, REVIEWER_SCOPE_BLOCKED, REVIEW_FREEZE_BLOCKED, PLAN_APPROVAL_BLOCKED, DOCUMENT_INDEXED, DOCUMENT_UPDATED, DOCUMENT_REMOVED, HEALTH_CHECKED, SCOPE_DETECTED, SCOPE_CHANGED, PLUGIN_SELECTION_CHANGED, DEPTH_CHANGED, TEST_STRATEGY_CHANGED, REVIEW_CLASS_CHANGED, RECOMPOSED, ERROR_LOGGED, RECOVERY_COMPLETED, BOLT_STARTED, BOLT_COMPLETED, BOLT_FAILED, AUTONOMY_MODE_SET, WORKTREE_CREATED, WORKTREE_MERGED, WORKTREE_DISCARDED, STATE_FORKED, STATE_MERGED, AUDIT_FORKED, AUDIT_MERGED, PRACTICES_DISCOVERED, PRACTICES_AFFIRMED, PRACTICES_OVERRIDE, PRACTICES_SECTION_EMPTY, MERGE_DISPATCH_INVOKED, MERGE_DISPATCH_RETURNED, MERGE_DISPATCH_FALLBACK, SENSOR_FIRED, SENSOR_PASSED, SENSOR_FAILED, SENSOR_BUDGET_OVERRIDE, GUARDRAIL_LOADED, MEMORY_EMPTY, RULE_LEARNED, SENSOR_PROPOSED, SWARM_STARTED, SWARM_UNIT_CONVERGED, SWARM_UNIT_FAILED, SWARM_BATON_RETURNED, SWARM_COMPLETED, SWARM_DEGRADED
```

### 3.2 宣言順 vs レジストリ順の食い違い (Rust 移植の落とし穴)

`aidlc-audit.ts` の宣言順は **Stage → Phase → Workflow → Session → Initialization → Interaction → Reviewer → Pipeline → Unit → Artifact → Subagent → …** で始まり、`audit-format.md` (および research §3.2 / Rust `EventType::ALL`) の **Workflow → Phase → Stage → …** とは冒頭から違う。エラーメッセージのバイト再現には**宣言順が正**。

### 3.3 フィールドキー関連 3 定数 (`:452`, `:460`, `:461`) — 逐語

```ts
const RESERVED_FIELD_KEYS = new Set(["Event"]);
```
```ts
const EMITTER_OWNED_FIELD_KEYS = new Set(["Timestamp", "Event"]);
const AUDIT_FIELD_KEY_PATTERN = /^[A-Za-z][A-Za-z0-9 ._()/-]*$/;
```

`AUDIT_FIELD_KEY_PATTERN` の文字クラス内訳: 先頭 1 文字は `[A-Za-z]`、2 文字目以降は `[A-Za-z0-9 ._()/-]` の 0 回以上。許容記号は **半角スペース・`.`・`_`・`(`・`)`・`/`・`-`** の 7 種 (`-` はクラス末尾なので範囲ではなくリテラル、`/` はクラス内なので無エスケープ)。

**バイト再現の実測**: `${AUDIT_FIELD_KEY_PATTERN}` のテンプレート補間は `String(RegExp)` → `/^[A-Za-z][A-Za-z0-9 ._()/-]*$/` を返す。upstream 実行系の `bun -e` と `node -e` の双方で `/` は**エスケープされない**ことを確認済み。よって `Invalid audit field key:` メッセージの中の正規表現部分はソースの字面と一字一句同じ。キー名は `JSON.stringify(key)` なので**ダブルクォートで囲まれ**、内部の制御文字は JSON エスケープされる。

`:444-451` と `:454-459` の設計コメント (逐語、Timestamp が予約でない理由):

```ts
// Field keys that can spoof event queries. A caller-supplied `Event` field
// lands as a SECOND `**Event**:` line, and the multiline regex in
// findAllEvents matches ANY line of a block — so a smuggled `--field
// Event=HUMAN_TURN` on a harmless event type would register as a forged event
// in every query. `Timestamp` is deliberately NOT reserved: the public `append`
// CLI accepts it, and it cannot spoof — the emitter's own `**Timestamp**:` line
// is written first and every parser takes the first match. renderAuditBlock
// drops it instead, so it can never render a second line.
```

---

## 4. `renderAuditBlock` (`:485-503`) — 正確なバイト構成

```ts
function renderAuditBlock(
  entry: AuditEntryInput,
  timestamp: string,
): string {
  const heading = EVENT_HEADINGS[entry.eventType] || entry.eventType;
  let block = `\n## ${heading}\n`;
  block += `**Timestamp**: ${timestamp}\n`;
  block += `**Event**: ${entry.eventType}\n`;
  for (const [key, value] of Object.entries(entry.fields)) {
    // The emitter already wrote these above; re-rendering one would put a
    // second identically-marked line in the block (issue #715).
    if (EMITTER_OWNED_FIELD_KEYS.has(key)) continue;
    // Escape every JavaScript line terminator in values so a malicious or
    // malformed input cannot forge a second audit field or event line.
    const safeValue = String(value).replace(/\r\n?|\n|\u2028|\u2029/g, "\\n");
    block += `**${key}**: ${safeValue}\n`;
  }
  return `${block}\n---\n`;
}
```

### 4.1 バイト列 (LF のみ、CR は出力されない)

```
\n ## <SP> <heading> \n
** T i m e s t a m p ** : <SP> <timestamp> \n
** E v e n t ** : <SP> <EVENT_TYPE> \n
[ ** <key> ** : <SP> <safeValue> \n ] *
\n - - - \n
```

- **先頭 `\n`** — ブロックは必ず改行で始まる (直前ブロックの `\n---\n` と合わせ、`---` 行の後に空行が入る形になる)。
- `## ` の後は heading のみ。heading は `EVENT_HEADINGS[eventType] || eventType`。空文字 heading は 86 語中に存在しないため `||` の falsy 分岐は非 taxonomy 名でのみ発生 (`appendAuditEntryAtPathUnlocked` 等はいずれも `validateAuditEntry` を通すので、実際には到達不能な防御)。
- **フィールド順 = `Object.entries(entry.fields)` の列挙順** = JS オブジェクトのプロパティ順。`AUDIT_FIELD_KEY_PATTERN` が先頭 `[A-Za-z]` を強制するため配列インデックス様キーは作れず、**常に挿入順**になる。Rust では `IndexMap` 相当の挿入順保持マップが必須 (`BTreeMap`/`HashMap` では不可)。
- **`Timestamp` / `Event` はスキップ** (`EMITTER_OWNED_FIELD_KEYS`)。`Event` は `validateAuditEntry` で既に throw 済みなので belt-and-braces、`Timestamp` は**受理して黙って捨てる**のがここ。
- **行終端エスケープ**: `String(value).replace(/\r\n?|\n|\u2028|\u2029/g, "\\n")`。正規表現の交替順が重要で、`\r\n` を先に食べるため CRLF は **`\n` 2 個ではなく 1 個**のリテラル `\n` になる。単独 `\r` は `\r\n?` の `?` で拾う。`\u2028` (LINE SEPARATOR) / `\u2029` (PARAGRAPH SEPARATOR) も対象。置換後の文字列はリテラル 2 文字 `\` + `n`。**タブ・NUL・その他制御文字は無処理で素通し**。
- **空値の扱い**: 値が空文字列でも行はスキップされず、`**<key>**: \n` (コロン + 半角スペース 1 個の後すぐ LF、**行末に半角スペースが残る**) を出力する。`fields` が空オブジェクトならフィールド行は 0 行。
- **末尾**: `${block}\n---\n` — 最後のフィールド行の LF に続けてもう 1 つ LF (= 空行)、`---`、LF。フィールドが 0 個なら `**Event**: …\n` の直後に空行 + `---`。
- `String(value)` による強制変換があるので、型を外れた値 (null / number) も文字列化して通る (TypeScript 型は `Record<string, string>` だが実行時保証はない)。

### 4.2 ヘッダ `# AI-DLC Audit Log`

`renderAuditBlock` は書かない。**空ファイル (size === 0) への最初の書込時のみ** `appendAuditBlockAtPath` が書く (`:693`):

```ts
    if (opened.size === 0) writeAll(fd, "# AI-DLC Audit Log\n");
```

`#` + 半角スペース + `AI-DLC Audit Log` + LF = **19 バイト**。その直後にブロックの先頭 `\n` が来るので、ヘッダ行と最初の `## …` の間に空行が 1 行入る。

### 4.3 `append-raw` の別ブロック形 (`:900-905`) — 参考

```ts
    let block = `\n## ${heading}\n`;
    block += `**Timestamp**: ${ts}\n`;
    block += `${expandedBody}\n`;
    block += `\n---\n`;
```

`**Event**:` 行を**持たない**。body は `body.replace(/\\n/g, "\n")` (`:880`) でリテラル `\n` を実改行に展開したものを**そのまま逐語**で書く (エスケープなし)。事前に body の各行を走査し、`- ` 接頭辞を剥がしてから `**Event**:` で始まる行の値が `VALID_EVENT_TYPES` に含まれれば拒否 (`:881-892`)。heading 側は `hasUnsafeSingleLineCharacter(heading)` で 1 物理行制約 (`:871-873`)。

---

## 5. `appendAuditBlockAtPath` (`:615-703`) の 6 段ガード — 逐語

宣言:

```ts
function appendAuditBlockAtPath(
  projectDir: string,
  shardPath: string,
  block: string,
  expectedIdentity?: AuditAppendExpectation,
): void {
```

### G1 封じ込め検査 (`:621-627`)

```ts
  const dir = dirname(shardPath);
  const projectAbs = resolve(projectDir);
  const projectReal = realpathSync(projectAbs);
  const rel = relative(projectAbs, resolve(shardPath));
  if (rel === "" || rel === ".." || rel.startsWith(`..${sep}`) || isAbsolute(rel)) {
    throw new Error(`Refusing audit shard outside project: ${shardPath}`);
  }
```

`realpath(projectDir)` ではなく **`resolve(projectDir)` を基準に相対化**している点に注意 (`projectReal` は symlink 検査の基準にだけ使う)。

### G2 symlink 連鎖拒否 — `mkdir` の前後 2 回 (`:628-630`)

```ts
  assertNoSymlinkInChainOrThrow(projectReal, rel);
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
  assertNoSymlinkInChainOrThrow(projectReal, rel);
```

### G3 open フラグ (`:631-642`)

```ts
  const noFollow = typeof fsConstants.O_NOFOLLOW === "number" ? fsConstants.O_NOFOLLOW : 0;
  let fd: number | undefined;
  try {
    fd = openSync(
      shardPath,
      fsConstants.O_RDWR |
        fsConstants.O_APPEND |
        fsConstants.O_CREAT |
        noFollow |
        fsConstants.O_NONBLOCK,
      0o666,
    );
```

`O_NOFOLLOW` が数値定数でない環境では **0 に劣化** (= 無効化) する。

### G4 非 regular 拒否 (`:643-644`)

```ts
    const opened = fstatSync(fd);
    if (!opened.isFile()) throw new Error(`Refusing non-regular audit shard: ${shardPath}`);
```

nlink 非対称の理由コメント (`:645-652`、逐語):

```ts
    // No nlink refusal on the ORDINARY append path: rsync --link-dest and
    // cp -al backup snapshots leave a live shard at nlink 2, and refusing it
    // here bricked every later gate/hook append framework-wide. A hardlink
    // aliases the same inode inside an already containment- and
    // symlink-chain-checked path, so it grants no redirect. The explicit
    // fork/merge path stays strict: readAuditSnapshot refuses a
    // multiply-linked main shard, and verifyExpectedPrefix below re-checks
    // during a merge append.
```

### G5 記述子同一性再検証 — 書込の前後 2 回 (`:677-699`)

`expectedIdentity` が渡された場合の追加検査 2 種:

```ts
    if (expectedIdentity &&
        (opened.dev !== expectedIdentity.dev || opened.ino !== expectedIdentity.ino)) {
      throw new Error(`Audit shard changed after validation: ${shardPath}`);
    }
    const verifyExpectedPrefix = (): void => {
      if (!expectedIdentity) return;
      const current = fstatSync(fd as number);
      if (current.nlink !== 1) throw new Error(`Audit shard became multiply linked: ${shardPath}`);
      const prefix = Buffer.alloc(expectedIdentity.prefixLength);
      let offset = 0;
      while (offset < prefix.length) {
        const count = readSync(fd as number, prefix, offset, prefix.length - offset, offset);
        if (count === 0) break;
        offset += count;
      }
      const hash = createHash("sha256").update(prefix.subarray(0, offset)).digest("hex");
      if (offset !== expectedIdentity.prefixLength || hash !== expectedIdentity.prefixHash) {
        throw new Error(`Audit shard prefix changed after validation: ${shardPath}`);
      }
    };
```

パス→記述子同一性 (逐語、コメント込み):

```ts
    // O_NOFOLLOW is not available on every platform and protects only the leaf.
    // Re-resolve after opening, require containment, and prove the pathname still
    // names the descriptor's inode before writing through the pinned descriptor.
    const verifyPathStillNamesDescriptor = (): void => {
      assertNoSymlinkInChainOrThrow(projectReal, rel);
      if (lstatSync(shardPath).isSymbolicLink()) {
        throw new Error(`Refusing symlinked audit shard: ${shardPath}`);
      }
      const currentReal = realpathSync(shardPath);
      if (currentReal !== projectReal && !currentReal.startsWith(`${projectReal}${sep}`)) {
        throw new Error(`Refusing audit shard outside project: ${shardPath}`);
      }
      const current = statSync(currentReal);
      if (current.dev !== opened.dev || current.ino !== opened.ino) {
        throw new Error(`Audit shard changed while opening: ${shardPath}`);
      }
    };
    verifyPathStillNamesDescriptor();
    verifyExpectedPrefix();
    if (opened.size === 0) writeAll(fd, "# AI-DLC Audit Log\n");
    writeAll(fd, block);
    // If an attacker renamed the leaf/parent during the descriptor write, fail
    // the enclosing audit-first transaction instead of reporting a ledger row
    // that is no longer discoverable at the canonical path.
    verifyPathStillNamesDescriptor();
    verifyExpectedPrefix();
  } finally {
    if (fd !== undefined) closeSync(fd);
  }
```

**呼出順の厳密形**: `verifyPathStillNamesDescriptor()` → `verifyExpectedPrefix()` → (ヘッダ) → `writeAll(block)` → `verifyPathStillNamesDescriptor()` → `verifyExpectedPrefix()`。**書込後の検証が throw しても、バイトは既にディスクに載っている** (`:1383-1387` の audit-merge コメントが明示するとおり、doctor が相関タグで回収する設計)。

### G6 no progress (`:599-607`)

```ts
function writeAll(fd: number, content: string): void {
  const bytes = Buffer.from(content, "utf-8");
  let offset = 0;
  while (offset < bytes.length) {
    const written = writeSync(fd, bytes, offset, bytes.length - offset);
    if (written <= 0) throw new Error("Audit append made no write progress");
    offset += written;
  }
}
```

`content === ""` のときはループに入らず何もしない (空 block は 0 write)。

### 5.1 `appendAuditEntries` (`:770-801`) は「物理的に 1 write」か → **厳密には No**

```ts
export function appendAuditEntries(
  entries: AuditEntryInput[],
  projectDir: string,
  intent?: string,
  space?: string,
): { appended: true; events: string[]; timestamps: string[] } {
  if (entries.length === 0) {
    throw new Error("appendAuditEntries requires at least one entry");
  }
  for (const entry of entries) validateAuditEntry(entry);

  if (!acquireAuditLock(projectDir, 50, 100, intent, space)) {
    throw new Error("Failed to acquire audit lock after retries");
  }
  try {
    const timestamps = entries.map(() => isoTimestamp());
    const payload = entries
      .map((entry, index) => renderAuditBlock(entry, timestamps[index]))
      .join("");
    appendAuditBlockAtPath(projectDir, auditFilePath(projectDir, intent, space), payload);
```

保証されるのは:
- 全エントリを**ディスクに触る前に**検証 (`for (const entry of entries) validateAuditEntry(entry);` がロック取得より前)。
- 全ブロックを `join("")` で **1 本の payload** に連結し、**`appendAuditBlockAtPath` 呼出は 1 回**、`writeAll` 呼出も 1 回。
- ロックは 1 回取得・1 回解放。`O_APPEND` オープンなので他プロセスの割り込みは原子的追記の粒度で排除。

保証されない (research §5.1 の「1 write」を精密化すべき点):
- `writeAll` は **partial write でループするため `writeSync` システムコールは複数回になりうる** (`:602-606`)。
- **shard が空 (size===0) のときはヘッダ用の `writeAll` が先に 1 回走る** (`:693`) ので、`writeAll` 呼出は 2 回・`writeSync` はさらに増える。
- タイムスタンプは `entries.map(() => isoTimestamp())` で**エントリごとに個別取得** (`:785`) するため、秒境界をまたぐと同一バッチ内で異なる秒になりうる。

`appendAuditEntries` の設計意図コメント (`:765-769`、逐語):

```ts
// Validate a related event set before touching disk, then append every block
// under one lock with one write. This is the audit-only transaction primitive
// for lifecycle pairs such as a synthetic single-stage STARTED/COMPLETED pair:
// a malformed later entry cannot leave an earlier entry committed, and no
// concurrent emitter can interleave between the blocks.
```

### 5.2 4 つの append 入口の比較

| 関数 | 行 | ロック | 権限検査 | 備考 |
| --- | ---: | --- | --- | --- |
| `appendAuditEntry` | 531 | `acquireAuditLock(projectDir, 50, 100, intent, space)` | なし | ライブラリ標準入口 |
| `appendAuditEntryUnlocked` | 560 | **取らない** (呼出側が保持) | なし | fork/merge/state の広域トランザクション用 |
| `appendAuditEntryAtPathUnlocked` | 751 | 取らない | なし | DocumentKB 専用 (shard パス自前合成) |
| `appendAuditEntries` | 770 | 取る | なし | バッチ (上記) |

権限検査 (CLI_RESERVED / CLI_PROTECTED) は `main()` と `handleAppend` / `handleAppendBatch` にしか無く、**上記 4 関数を import する経路はすべて素通し**。

---

## 6. audit-fork / audit-merge の拒否文言 — 全 26 件の完全形

凡例: **[省略]** = research §5.5 が `…` で省略していたもの (5 件)、**[未引用]** = research が一切引用していなかったもの (18 件)、**[既出]** = research に完全形で載っていたもの (3 件)。

### 6.1 `handleAuditFork` (`:1123-1292`)

| # | 行 | 逐語 | 状態 |
| ---: | ---: | --- | --- |
| F1 | 1141 | `main audit not found at ${mainAuditPath}; start a workflow first (describe what to build, e.g. /aidlc "build the auth service")` | **[省略]** |
| F2 | 1145 | `worktree directory not found at ${wtPath}; run aidlc-worktree create first` | [既出] |
| F3 | 1150 | `Failed to acquire audit lock after retries` | **[未引用]** (§5.4 に一般形のみ) |
| F4 | 1165-1166 | `worktree audit already exists at ${wtAuditPath} with unmerged work after AUDIT_FORKED; merge the delta with audit-merge, or discard the worktree` | **[省略]** |
| F5 | 1173-1174 | `worktree audit already exists at ${wtAuditPath}, but its AUDIT_FORKED row does not match the authoritative main row; discard the worktree before re-forking` | **[省略]** |
| F6 | 1179-1180 | `worktree audit already exists at ${wtAuditPath}, but its fork prefix differs from main; discard the worktree before re-forking` | **[省略]** |
| F7 | 1217 | `worktree path is outside project: ${wtPath}` | **[未引用]** |
| F8 | 1224 | `worktree path changed during audit-fork: ${wtPath}` | **[未引用]** |
| F9 | 1244 | `main audit changed identity during audit-fork` | **[未引用]** |

F4-F6 のソース逐語 (改行位置と文字列結合の形まで):

```ts
      if (existingFork) {
        if (existingContent.slice(existingFork.end) !== "") {
          throw new Error(
            `worktree audit already exists at ${wtAuditPath} with unmerged work after ` +
              `AUDIT_FORKED; merge the delta with audit-merge, or discard the worktree`,
          );
        }
        const mainContent = before.bytes.toString("utf-8");
        const mainFork = latestAuditFork(mainContent, slug);
        if (!mainFork || !forksCorrelate(existingFork, mainFork)) {
          throw new Error(
            `worktree audit already exists at ${wtAuditPath}, but its AUDIT_FORKED row ` +
              `does not match the authoritative main row; discard the worktree before re-forking`,
          );
        }
        if (!existing.bytes.equals(before.bytes.subarray(0, mainFork.end))) {
          throw new Error(
            `worktree audit already exists at ${wtAuditPath}, but its fork prefix differs ` +
              `from main; discard the worktree before re-forking`,
          );
        }
```

**F4 の重要な非自明点**: 文字列結合の切れ目が `after ` + `AUDIT_FORKED` なので、連結後は `…with unmerged work after AUDIT_FORKED; merge…` (スペース 1 個)。F5 は `AUDIT_FORKED row ` + `does not match…`、F6 は `fork prefix differs ` + `from main;…`。

`ERROR_LOGGED` の相関タグ (`:1259-1269`、逐語):

```ts
        appendAuditEntryUnlocked(
          "ERROR_LOGGED",
          {
            Tool: "aidlc-audit",
            Command: "audit-fork",
            Error: `[slug=${slug}] [fork-emitted:${auditTs}] ${message}`,
          },
```

`AUDIT_FORKED` のフィールド (`:1192-1199`、この順序で render される):

```ts
      const forkEntry = {
        eventType: "AUDIT_FORKED",
        fields: {
          "Bolt slug": slug,
          "Source Audit Hash": sourceHash,
          "Fork Boundary": String(boundary),
        },
      };
```

### 6.2 `validateMergeDelta` (`:974-1004`)

| # | 行 | 逐語 | 状態 |
| ---: | ---: | --- | --- |
| D1 | 976 | `worktree audit delta ends with an incomplete block` | [既出] |
| D2 | 983 | `worktree audit delta has malformed note block` | **[未引用]** |
| D3 | 986 | `worktree audit delta has duplicate Event fields` | **[未引用]** |
| D4 | 988 | `worktree audit delta must contain exactly one Timestamp field` | **[未引用]** |
| D5 | 992 | `worktree audit delta contains unknown event ${eventType}` | [既出] |
| D6 | 995 | `worktree audit delta contains protected authority event ${eventType}` | [既出] |

判定ロジック逐語 (パーサ再現に必要な正規表現込み):

```ts
function validateMergeDelta(delta: string): void {
  if (delta !== "" && !delta.endsWith("\n---\n")) {
    throw new Error("worktree audit delta ends with an incomplete block");
  }
  for (const block of delta.split(/\n---\n/).filter((part) => part.trim() !== "")) {
    const eventMatches = [...block.matchAll(/^(?:-\s*)?\*\*Event\*\*:\s*(.+)$/gm)];
    const timestampMatches = [...block.matchAll(/^(?:-\s*)?\*\*Timestamp\*\*:\s*(.+)$/gm)];
    if (eventMatches.length === 0) {
      const timestamps = block.match(/^(?:-\s*)?\*\*Timestamp\*\*:/gm) ?? [];
      if (timestamps.length !== 1) throw new Error("worktree audit delta has malformed note block");
      continue; // complete append-raw diagnostic note
    }
    if (eventMatches.length !== 1) throw new Error("worktree audit delta has duplicate Event fields");
    if (timestampMatches.length !== 1) {
      throw new Error("worktree audit delta must contain exactly one Timestamp field");
    }
    const eventType = eventMatches[0][1].trim();
    if (!VALID_EVENT_TYPES.has(eventType)) {
      throw new Error(`worktree audit delta contains unknown event ${eventType}`);
    }
    if (mergeEventIsProtected(eventType)) {
      throw new Error(`worktree audit delta contains protected authority event ${eventType}`);
    }
    const fields: Record<string, string> = {};
    for (const match of block.matchAll(/^(?:-\s*)?\*\*([^*]+)\*\*:\s*(.*)$/gm)) {
      const key = match[1].trim();
      if (key !== "Event" && key !== "Timestamp") fields[key] = match[2];
    }
    validateAuditEntry({ eventType, fields });
  }
}
```

注記: 抽出されたフィールド値は `match[2]` を **trim せず**そのまま入れる (キーだけ trim)。`validateAuditEntry` は値を見ないので影響は無いが、Rust 実装で trim すると差分が出る箇所ではない (再 render しないため)。`(?:-\s*)?` 接頭辞許容は旧 `- **Key**:` 形式の shard への互換。

### 6.3 `handleAuditMerge` (`:1320-1514`)

| # | 行 | 逐語 | 状態 |
| ---: | ---: | --- | --- |
| M1 | 1333 | `worktree audit not found at ${wtAuditPath}; nothing to merge` | **[未引用]** |
| M2 | 1336 | `main audit not found at ${mainAuditPath}; start a workflow first (describe what to build, e.g. /aidlc "build the auth service")` | **[未引用]** (F1 と同文) |
| M3 | 1344 | `worktree audit missing AUDIT_FORKED entry for slug ${slug}` | **[未引用]** |
| M4 | 1358 | `refusing malformed or unauthorized worktree audit delta: ${errorMessage(error)}` | **[未引用]** (D1-D6 のラッパ) |
| M5 | 1375 | `Failed to acquire audit lock after ${lockRetries} × ${lockRetryMs}ms = ${(lockRetries * lockRetryMs / 1000).toFixed(1)}s retries; another merge in flight?` | **[未引用]** |
| M6 | 1401 | `worktree audit changed while merge was preparing; retry the merge` | [既出] |
| M7 | 1409 | `main audit is missing AUDIT_FORKED for slug ${slug}` | **[未引用]** |
| M8 | 1411 | `worktree AUDIT_FORKED metadata does not match the authoritative main row` | **[未引用]** |
| M9 | 1414 | `invalid Fork Boundary ${boundary} for ${mainBuf.length}-byte main audit` | **[未引用]** |
| M10 | 1423-1426 | `main audit prefix-hash does not match recorded Source Audit Hash (expected at least ${boundary} bytes, got ${mainBuf.length}); refusing to merge (main-audit truncation suspected)` | **[省略]** |
| M11 | 1429-1430 | `main audit prefix-hash at byte ${boundary} does not match recorded Source Audit Hash; refusing to merge (mid-Bolt tampering suspected)` | [既出] |

**M5 の非 ASCII 注意**: 区切りは ASCII の `x` ではなく **U+00D7 MULTIPLICATION SIGN `×`**。`(lockRetries * lockRetryMs / 1000).toFixed(1)` なので既定値では `20.0` (小数 1 桁固定)。既定メッセージ全文は `Failed to acquire audit lock after 200 × 100ms = 20.0s retries; another merge in flight?`。

**M10 / M11 は排他ではなく順序依存** — 逐語:

```ts
    if (prefixHash !== sourceHash) {
      if (mainBuf.length < boundary) {
        throw new Error(
          `main audit prefix-hash does not match recorded Source Audit Hash ` +
            `(expected at least ${boundary} bytes, got ${mainBuf.length}); ` +
            `refusing to merge (main-audit truncation suspected)`,
        );
      }
      throw new Error(
        `main audit prefix-hash at byte ${boundary} does not match recorded Source Audit Hash; ` +
          `refusing to merge (mid-Bolt tampering suspected)`,
      );
    }
```

なお M9 (`:1413`) が `boundary > mainBuf.length` を先に弾くため、M10 の `mainBuf.length < boundary` 分岐は**現行制御フローでは到達不能**な冗長防御。research §5.5 が「main が boundary より短い場合」と説明していた条件は、実際には M9 で先に落ちる。

`AUDIT_MERGED` のフィールド (`:1449-1458`、この順序):

```ts
      const mergedEntry = {
        eventType: "AUDIT_MERGED",
        fields: {
          "Bolt slug": slug,
          "Entries Merged": String(entriesMerged),
          "Source Audit Hash": sourceHash,
          "Fork Boundary": String(boundary),
          "Fork Timestamp": forkTs,
        },
      };
```

delta と receipt の合成 append (`:1463-1472`) — **1 回の `appendAuditBlockAtPath` で `delta + renderAuditBlock(mergedEntry, mergedTimestamp)`**:

```ts
      appendAuditBlockAtPath(
        projectDir,
        mainAuditPath,
        delta + renderAuditBlock(mergedEntry, mergedTimestamp),
        {
          ...mainSnapshot.identity,
          prefixLength: boundary,
          prefixHash: sourceHash,
        },
      );
```

`entriesMerged` の数え方 (`:1433-1434`):

```ts
    const trimmed = delta.trim();
    if (trimmed !== "") entriesMerged = delta.split(/\n---\n/).filter((b) => b.trim()).length;
```

---

## 7. 検証 grep (ピン留めソース上で再実行)

| ID | 予測 | 実測 | 判定 |
| --- | --- | --- | --- |
| M2 | `VALID_EVENT_TYPES` = 86 | 86 (distinct 86) | 一致 |
| M3 | `EVENT_HEADINGS` = 86、双方向差分空 | 86 (distinct 86)、`valid−headings = []`、`headings−valid = []` | 一致 |
| M4 | RESERVED 8 / PROTECTED 18 / MERGE 26 | 8 / 18 / 26 (いずれも distinct 同数) | 一致 |
| M6 | MANDATORY (`✓`) = 8 | 8。`WORKFLOW_STARTED` `WORKFLOW_COMPLETED` `WORKFLOW_PARKED` `WORKFLOW_UNPARKED` `PHASE_STARTED` `PHASE_COMPLETED` `STAGE_STARTED` `STAGE_COMPLETED` | 一致 (research §3.3 と同一) |
| M15 | バイト書込サイト 3 (import 2 + 呼出 3 = 5 行) | 5 行: `:14` `writeSync` (import), `:33` `writeBufferAtomic` (import), `:603` `writeSync`, `:1239` `writeBufferAtomic`, `:1252` `writeBufferAtomic` | 一致 |
| M1 (副次) | `aidlc-audit.ts` = 1589 行 | 1589 行 / 61,642 B | 一致 |

---

## 8. amadeus-ng `modules/shared/audit-events/src/lib.rs` との突合

対象: `/Users/j5ik2o/orca/workspaces/amadeus-ng/docs/modules/shared/audit-events/src/lib.rs` (291 行)

| 項目 | 結果 |
| --- | --- |
| 86 語のワイヤ綴り集合 | **完全一致** (`upstream − rust = []`、`rust − upstream = []`) |
| 22 カテゴリ・カテゴリ別基数 | upstream `audit-format.md` レジストリと一致 (テスト `category_sizes_match_the_upstream_registry` の期待値が M5/M9 と整合) |
| MANDATORY 8 | **完全一致** (集合・順序とも) |
| CLI_PROTECTED 18 | **集合は一致。順序が不一致** (下記 D-1) |
| CLI_RESERVED 8 | **未定義** (lib.rs:5-6 のコメント通り、ゴールデン採取待ち) → 本報告 §2.1 で確定 |
| MERGE_PROTECTED 26 + `DOCUMENT_*` prefix | **未定義** (同上) → 本報告 §2.3 で確定 |
| EVENT_HEADINGS 86 | **未実装** (`heading` / `Heading` の識別子がファイル内に 0 件) → 本報告 §1 で確定 |

### 不一致 D-1: `CLI_PROTECTED` のメンバー順

- upstream (`:348-376`): `HUMAN_TURN, GATE_APPROVED, GATE_REJECTED, QUESTION_ANSWERED, REVIEW_REQUESTED, REVIEW_COMPLETED, PIPELINE_LINK_COMPLETED, ARTIFACT_REUSED, SWARM_STARTED, SWARM_UNIT_CONVERGED, AUTONOMY_MODE_SET, UNIT_STARTED, …`
- Rust (`lib.rs:161-180`): `HumanTurn, GateApproved, GateRejected, QuestionAnswered, **AutonomyModeSet**, ReviewRequested, ReviewCompleted, PipelineLinkCompleted, ArtifactReused, SwarmStarted, SwarmUnitConverged, UnitStarted, …`

`AUTONOMY_MODE_SET` の位置が upstream 11 番目に対し Rust は 5 番目。**集合としては同一** (`is_cli_protected` の判定は等価) で、upstream はこの集合を文字列連結して出力する箇所を持たないため**現時点で観測可能な影響はない**。ただし research §4 の表が「human authority (5)」に `AUTONOMY_MODE_SET` を含めた分類順を採用したことに由来する差なので、逐語性を重んじるなら upstream の宣言順へ揃えるのが安全。

### 不一致 D-2: `EventType::ALL` の順序はエラーメッセージ再現に使えない

`EventType::ALL` はレジストリ (カテゴリ) 順、`Invalid event type: … Must be one of: …` は `aidlc-audit.ts:39-189` の**宣言順**。両者は 1 番目から違う (`WORKFLOW_STARTED` vs `STAGE_STARTED`)。Rust 側でこのメッセージをバイト再現するには **§3.1 の宣言順の別リスト (または `ALL` とは独立の `VALID_DECLARATION_ORDER`)** が必要。

### 差分ではないが要追加

- Rust 側に heading 写像・`renderAuditBlock` 相当・`AUDIT_FIELD_KEY_PATTERN` / `RESERVED_FIELD_KEYS` / `EMITTER_OWNED_FIELD_KEYS` は未実装。オンディスクバイト互換にはいずれも必須。
- `fields` を保持するコンテナは**挿入順を保つ型**でなければならない (§4.1)。

---

## 9. research 文書への訂正候補 (2 件)

1. **§5.1「1 ロック内で全ブロックを 1 write」** — 「1 回の追記呼出」は正しいが、`writeAll` の partial-write ループと空 shard 時のヘッダ書込により **`writeSync` システムコールが 1 回とは限らない**。原子性の根拠は `O_APPEND` + audit ロックであり、単一 syscall ではない (§5.1)。
2. **§5.5「main が boundary より短い場合: `… (main-audit truncation suspected)`」** — `boundary > mainBuf.length` は直前の `invalid Fork Boundary <n> for <m>-byte main audit` (`:1413-1415`) で先に弾かれるため、truncation 分岐は**現行フローでは到達不能**な冗長防御 (§6.3)。

## RESOLVED OPEN QUESTIONS
- docs/specs/research/workspace-audit-ledger.md §4 の「注意 (仕様執筆時の欠落データ)」: 『03 は CLI_RESERVED_EVENT_TYPES (8) と MERGE_PROTECTED_EVENT_TYPES (26) の明示的メンバー列挙を掲載していない (M4 は基数のみ検証)。逐語リストが必要なら upstream aidlc-audit.ts:292 / :395 の実測が必要』→ 確定。CLI_RESERVED_EVENT_TYPES (aidlc-audit.ts:292-301) = HUMAN_TURN, SUMMARY_CONFIRMATION_RECORDED, ARTIFACT_CREATED, ARTIFACT_UPDATED, ARTIFACT_REUSED, REVIEW_REQUESTED, REVIEW_COMPLETED, PIPELINE_LINK_COMPLETED。MERGE_PROTECTED_EVENT_TYPES (:395-425) = HUMAN_TURN, GATE_APPROVED, GATE_REJECTED, QUESTION_ANSWERED, SUMMARY_CONFIRMATION_RECORDED, AUTONOMY_MODE_SET, UNIT_STARTED, UNIT_PAUSED, UNIT_RESUMED, UNIT_COMPLETED, AUDIT_FORKED, AUDIT_MERGED, STATE_FORKED, STATE_MERGED, SWARM_STARTED, SWARM_COMPLETED, SWARM_DEGRADED, SWARM_BATON_RETURNED, SWARM_UNIT_CONVERGED, SWARM_UNIT_FAILED, BOLT_STARTED, BOLT_COMPLETED, BOLT_FAILED, WORKTREE_CREATED, WORKTREE_DISCARDED, WORKTREE_MERGED (宣言順)。
- modules/shared/audit-events/src/lib.rs:5-6 の未定義宣言『CLI_RESERVED (8) と MERGE_PROTECTED (26+DOCUMENT_*) は as-built 仕様に全列挙が無く、upstream ソース読解 (stage-0 ゴールデン採取) 待ち — 誤推測は audit-merge 互換を壊すため未定義』→ 解消。両集合の全メンバーに加え、DOCUMENT_* prefix 規則の逐語 (mergeEventIsProtected, :426-429: `if (MERGE_PROTECTED_EVENT_TYPES.has(eventType)) return true; return eventType.startsWith("DOCUMENT_");`) を確定。判定は「列挙 26 に含まれる OR 名が DOCUMENT_ で始まる」の論理和。
- research §1.2『Heading は EVENT_HEADINGS (aidlc-audit.ts:192) から、無ければ生イベント名にフォールバック』のみで 86 対応表が未採取だった件 → 確定。:193-278 の 86 行を逐語採取。全 86 の heading 文字列が相異、heading == イベント名 のエントリは 0 件。語形は機械変換不能 (STAGE/PHASE/WORKFLOW/SESSION の _STARTED は 'Start'、UNIT/BOLT/SWARM の _STARTED は 'Started'; STAGE/PHASE/WORKFLOW の _COMPLETED は 'Completion'、他 7 語は 'Completed'; RECOMPOSED → 'Plan Recomposed' は語幹に無い 'Plan' が付く唯一例; WORKSPACE_INITIALISED → 'Workspace Initialised' は英式綴り維持)。
- research §1.3 表の検査 1『Invalid event type: <x>. Must be one of: <full list>』の <full list> 連結形式が未確定だった件 → 確定。`${[...VALID_EVENT_TYPES].join(", ")}` (aidlc-audit.ts:466)。区切りは半角カンマ+半角スペース 1 個、末尾区切りなし、順序は Set 挿入順 = aidlc-audit.ts:39-189 のソース宣言順 (audit-format.md のカテゴリ掲載順および Rust EventType::ALL の順とは異なる)。連結後の長さは 1,550 文字 / 1,550 バイト (UTF-8)。
- research §1.2 のブロック文法スケッチにあった『空値の扱い』が未確定だった件 → 確定。renderAuditBlock (:493-501) は値が空文字列でも行をスキップせず `**<key>**: ` + LF を出力する (コロンの後の半角スペースが行末に残る)。fields が空なら **Event**: 行の直後に空行 + `---` が続く。値は String(value) で強制変換され、行終端エスケープは /\r\n?|\n| | /g → リテラル 2 文字 \n (CRLF は \n 1 個に畳まれる)。タブ・NUL 等の制御文字は無処理で素通し。
- research §5.5 が省略記号で伏せていた audit-fork 再 fork 3 拒否の完全形 → 確定。(1) `worktree audit already exists at <p> with unmerged work after AUDIT_FORKED; merge the delta with audit-merge, or discard the worktree` (:1165-1166) (2) `worktree audit already exists at <p>, but its AUDIT_FORKED row does not match the authoritative main row; discard the worktree before re-forking` (:1173-1174) (3) `worktree audit already exists at <p>, but its fork prefix differs from main; discard the worktree before re-forking` (:1179-1180)。
- research §5.5 の pre-emit ガード `main audit not found at <p>; start a workflow first …` の省略部 → 確定。完全形は `main audit not found at <p>; start a workflow first (describe what to build, e.g. /aidlc "build the auth service")` (audit-fork :1141 / audit-merge :1336 で同文)。
- research §5.5 の `… (main-audit truncation suspected)` の省略部 → 確定。完全形は `main audit prefix-hash does not match recorded Source Audit Hash (expected at least <boundary> bytes, got <len>); refusing to merge (main-audit truncation suspected)` (:1423-1426)。加えて、この分岐の前提 `mainBuf.length < boundary` は直前の :1413-1415 の `invalid Fork Boundary <n> for <m>-byte main audit` で先に弾かれるため、現行制御フローでは到達不能な冗長防御であることが判明 (research §5.5 の説明への訂正候補)。
- research §5.1『全ブロックを 1 ロック内で 1 write』の物理性が未検証だった件 → 精密化して確定。appendAuditEntries (:770-801) は join("") で 1 本の payload を作り appendAuditBlockAtPath を 1 回だけ呼ぶ (writeAll も 1 回) が、writeAll (:599-607) は partial write でループするため writeSync システムコールは複数回になりうる。さらに shard が空 (size===0) のときは :693 でヘッダ `# AI-DLC Audit Log\n` の writeAll が先に走るため writeAll 呼出自体が 2 回になる。原子性の根拠は O_APPEND + audit ロックであり単一 syscall ではない。タイムスタンプも entries.map(() => isoTimestamp()) でエントリごとに個別取得 (:785) される。
- research §1.4 が 6 段ガードとして要約していた appendAuditBlockAtPath の実装形 → 逐語で確定 (:615-703)。封じ込め検査は realpath ではなく resolve(projectDir) 基準で相対化 (:624)、symlink 連鎖検査は mkdir の前後 2 回 (:628/:630)、O_NOFOLLOW は定数が数値でなければ 0 に劣化 (:631)、記述子同一性再検証と prefix 検証は書込の前後で verifyPathStillNamesDescriptor() → verifyExpectedPrefix() の順に各 2 回 (:691-692, :698-699)。書込後の検証が throw してもバイトは既にディスクに載っている。
- amadeus-ng の audit-events クレートで CLI_PROTECTED 18 の宣言順が upstream と異なる点が未検出だった → 検出・確定。upstream :348-376 では AUTONOMY_MODE_SET が 11 番目 (SWARM_UNIT_CONVERGED の直後)、Rust lib.rs:161-180 では 5 番目 (QuestionAnswered の直後)。集合は同一で is_cli_protected の判定は等価、upstream にこの集合を文字列連結する箇所は無いため現時点で観測可能な影響はないが、逐語性の観点では upstream 宣言順に揃えるのが安全。

## VERIFIED COUNTS
- M2 (VALID_EVENT_TYPES 基数): 期待 86 → 実測 86 (distinct 86)。一致。手法: aidlc-audit.ts を `const VALID_EVENT_TYPES = new Set([` から次の `]);` までスライスし /^\s*"([A-Z_]+)",$/gm を抽出。
- M3 (EVENT_HEADINGS 基数 + 双方向差分): 期待 86 かつ対称差空 → 実測 86 (distinct 86)、VALID−HEADINGS = []、HEADINGS−VALID = []。一致。手法: `const EVENT_HEADINGS` から次の `};` までスライスし /^\s*([A-Z_]+):/gm を抽出、M2 集合と両方向差分。副次確認: heading 文字列の重複 0 件、heading == イベント名 のエントリ 0 件。
- M4 (authority 3 集合の基数): 期待 CLI_RESERVED=8 / CLI_PROTECTED=18 / MERGE_PROTECTED=26 → 実測 8 / 18 / 26 (いずれも distinct 同数)。一致。手法: 各定数名から `new Set([` ～ `]);` をスライスし /^\s*"([A-Z_]+)",$/gm を抽出。MERGE_PROTECTED は :426-429 の startsWith("DOCUMENT_") prefix 規則を列挙メンバーに含めない点も M4 の注記どおり確認。
- M6 (MANDATORY ✓ イベント数): 期待 8 → 実測 8。一致。手法: `grep -cE '^\| ✓ `[A-Z_]+` \|' core/knowledge/aidlc-shared/audit-format.md` (ピン留め 3c3146cf から取得、34,762 B / 322 行)。列挙結果は WORKFLOW_STARTED, WORKFLOW_COMPLETED, WORKFLOW_PARKED, WORKFLOW_UNPARKED, PHASE_STARTED, PHASE_COMPLETED, STAGE_STARTED, STAGE_COMPLETED で research §3.3 および Rust EventType::MANDATORY と完全一致。
- M15 (バイト書込サイト): 期待 5 行 = import 2 + 呼出 3 → 実測 5 行。一致。手法: `grep -nE "writeSync|writeFileSync|appendFileSync|writeBufferAtomic|copyFileSync|createWriteStream|truncateSync|ftruncateSync" core/tools/aidlc-audit.ts`。結果: :14 writeSync (import), :33 writeBufferAtomic (import), :603 writeSync (writeAll 内、appendAuditBlockAtPath からのみ到達), :1239 writeBufferAtomic (clone-id トークン), :1252 writeBufferAtomic (worktree audit shard)。M15 の行番号と完全一致。
- M1 副次 (aidlc-audit.ts 行数/サイズ): 期待 1589 行 / 61,642 バイト → 実測 1589 行 / 61,642 バイト。一致。手法: `wc -c -l`。ピン留めコミット 3c3146cf からの取得物の同一性確認。
- 追加検証 (amadeus-ng 突合 1): 86 語のワイヤ綴り集合が upstream と Rust クレートで一致するか → upstream−rust = []、rust−upstream = []。一致 (86 / distinct 86)。手法: lib.rs の event_types! マクロ本体から /=\s*"([A-Z_]+)"/ を抽出し upstream VALID_EVENT_TYPES と両方向差分。
- 追加検証 (amadeus-ng 突合 2): CLI_PROTECTED 18 の集合一致と順序一致 → 集合一致 True、順序一致 False。不一致箇所は AUTONOMY_MODE_SET の位置 (upstream 11 番目 / Rust 5 番目)。基数はいずれも 18 で M4 と整合。
- 追加検証 (エラーメッセージ list 連結): [...VALID_EVENT_TYPES].join(", ") の実測長 = 1,550 文字 / 1,550 バイト (UTF-8)、要素 86、区切り ", "、末尾区切りなし。順序は Set 挿入順 = ソース宣言順で、Rust EventType::ALL の順 (レジストリ カテゴリ順) と不一致 (1 番目から相違: STAGE_STARTED vs WORKFLOW_STARTED)。
- 追加検証 (AUDIT_FIELD_KEY_PATTERN の文字列化): `${AUDIT_FIELD_KEY_PATTERN}` が Invalid audit field key メッセージ中でどう出るか → bun および node の双方で `/^[A-Za-z][A-Za-z0-9 ._()/-]*$/` を出力 (文字クラス内の / はエスケープされない)。ソース :461 の字面と一字一句一致することを確認。
