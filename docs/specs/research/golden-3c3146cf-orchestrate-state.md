> 採取元: **`awslabs/aidlc-workflows` 公開リポジトリからの直接採取** — ピン留めコミット `3c3146cf`（`3c3146cfd7cef33020d48e8d48d4e80d0f8c2820`、v2.6.40、branch `v2`）の `core/tools/aidlc-orchestrate.ts`（6,169 行 / SHA-256 `581519ed4a1d58254b0ffbafd3520e446dde741cae21c4f3539ff8f2db765a22`）・`core/tools/aidlc-state.ts`（4,278 行 / SHA-256 `6d4488b95f26813f23a2c7051fd631f4ff88689af9801562805be8bf25d45fd1`）・`core/tools/aidlc-lib.ts`（10,668 行 / SHA-256 `ba4e2259cab97393279cf9c1c63da24d8fbf035cb153e009b41d3e0c8a78d97f`）。既存 research 文書と違い、as-built 仕様（`docs/upstream/specs/`）の二次引用ではなく **upstream ソースの実バイトを `curl` で取得して読解した一次採取**である。採取日 **2026-08-22**（Issue #7 項目 0）。10-orchestration.md と `formal/engine_loop.qnt` の裏取り材料。
>
> **検証 grep の要約**（as-built 02 Measurement notes の再実行はすべて一致）: L516 行数 6,169 ✅ ／ L519 engine が構築する directive kind 8 種と件数（error 15 / done 7 / load-steering 2 / invoke-swarm 2 / ask 2 / run-stage 1 / print 1 / parked 1）✅ ／ L520 `present-gate` / `dispatch-subagent` はコメント 1 件のみで never constructed ✅ ／ L522 engine サブコマンド 4 種 ✅ ／ L523 `handleNext` の 21 ラベル分岐（集合一致。ただし**出現順は `4c` が `4a`/`4b` より先**で Measurement notes の列挙順とは異なる）✅ ／ L524 `report --result` の 10 語 ✅ ／ L531 `DIRECTIVE_MAX_BYTES = 28 KiB` ✅ ／ L532 gate suppression 4 箇所（`:4139` / `:4198` / `:4356` / `:4486`）✅ ／ L533 conductor-persona decision コメント 1 件 ✅。加えて B10 の決定的事実として **review freshness / `produces[]` 無効化の経路に `mtime` 参照が 0 件**（`aidlc-state.ts` 0 件 / `aidlc-orchestrate.ts` 1 件はコメント / `aidlc-lib.ts` 31 件は全件目視して別用途と確認）を実測した。
>
> 本書は採取レポートの**原文**であり、逐語ブロック・upstream 行番号を採取時のまま保持する。本文が記録する `/private/tmp/…/scratchpad/…` は採取セッションの作業ディレクトリであり、既に存在しない。

---

> 採取元: ピン留めコミット `3c3146cf` (awslabs/aidlc-workflows v2.6.40)。ダウンロード先と SHA-256:
> - `/private/tmp/claude-501/-Users-j5ik2o-orca-workspaces-amadeus-ng-docs/351513f3-85bf-44e8-92ca-bea27cc446f6/scratchpad/upstream-src/core/tools/aidlc-orchestrate.ts` — 6169 行 / `581519ed4a1d58254b0ffbafd3520e446dde741cae21c4f3539ff8f2db765a22`
> - `/private/tmp/claude-501/-Users-j5ik2o-orca-workspaces-amadeus-ng-docs/351513f3-85bf-44e8-92ca-bea27cc446f6/scratchpad/upstream-src/core/tools/aidlc-state.ts` — 4278 行 / `6d4488b95f26813f23a2c7051fd631f4ff88689af9801562805be8bf25d45fd1`
> - `/private/tmp/claude-501/-Users-j5ik2o-orca-workspaces-amadeus-ng-docs/351513f3-85bf-44e8-92ca-bea27cc446f6/scratchpad/upstream-src/core/tools/aidlc-lib.ts` — 10668 行 / `ba4e2259cab97393279cf9c1c63da24d8fbf035cb153e009b41d3e0c8a78d97f`
>   (担当外だが、採取項目 1 の「floor 第 3 要素の出所」を確定するには不可欠だったため追加取得。読み取りのみ)
>
> リポジトリのファイルは一切変更していない。逐語はすべて一字一句そのまま、行番号併記。

---

# 1. `verifyReviewerPrecondition` の完全ロジック (aidlc-state.ts)

## 1.1 位置と呼び出し元

| 対象 | 行 |
|---|---|
| 設計コメント (§12a / RFC Track 1) | `aidlc-state.ts:1753-1774` |
| 本体 `verifyReviewerPrecondition` | `aidlc-state.ts:1775-1944` |
| `staleSourcePreconditionError` | `:1946-1967` |
| `staleReviewPreconditionError` | `:1999-2024` |
| `reviewerPreconditionError` | `:2026-2037` |
| `reviewRecoverySpentInCurrentAttempt` | `:2039-2062` |

呼び出し 4 箇所 (**engine の report ではなく state 側の 4 完了ハンドラに置く**理由は `aidlc-orchestrate.ts:5878-5883` の逐語コメント「a report-only guard is bypassable, issue #366」):

| 行 | ハンドラ | 第 4 引数 `requireReceiptExistence` |
|---|---|---|
| `aidlc-state.ts:2202-2207` | `handleAdvance` (`:2064`) | `!alreadyMarkedCompleted` |
| `:2326-2331` | `handleFinalize` (`:2304`) | `!alreadyMarkedCompleted` |
| `:2452-2457` | `handleCompleteWorkflow` (`:2406`) | `!alreadyMarkedCompleted` |
| `:2783` | `handleApprove` (`:2622`) | 省略 → default `true` |

## 1.2 ★ floor 第 3 要素「latest relevant produces[] write」の出所 — **監査行 (ARTIFACT_CREATED / ARTIFACT_UPDATED) 由来。ファイル mtime は一切使っていない**

`verifyReviewerPrecondition` は floor スキャンを自前実装せず、`aidlc-lib.ts` の `freshReviewReceipts` に完全委譲する (`aidlc-state.ts:1816-1821`):

```
  // The fresh-receipt scan lives in aidlc-lib.ts (freshReviewReceipts) so the
  // review-freeze PreToolUse hook and this precondition read the SAME window:
  // event interleave (timestamp, buffer-position tiebreak), the stage-agnostic
  // WORKFLOW_STARTED/STAGE_JUMPED floor, the unit-major STAGE_STARTED skip,
  // and per-unit write invalidation are all documented there.
  const receipts = freshReviewReceipts(pd, content, stage, { reviewClass });
```

`freshReviewReceipts` (`aidlc-lib.ts:5004-5323`) の構造は **2 段**であり、B10 実装上は「floor 3 要素」という研究文書の記述より正確な区別が要る。

**(A) floorIdx (真の floor) は 4 イベントのみ** — produces[] write は含まれない (`aidlc-lib.ts:5073-5087`):

```
  let floorIdx = -1;
  for (let i = 0; i < events.length; i++) {
    const e = events[i];
    if (e.event === "WORKFLOW_STARTED" || e.event === "STAGE_JUMPED") {
      floorIdx = i;
      continue;
    }
    if (auditBlockField(e.block, "Stage") !== stage.slug) continue;
    if (e.event === "STAGE_STARTED" && !unitMajor) {
      if (auditBlockField(e.block, "Workflow")?.startsWith("single-stage:")) continue;
      floorIdx = i;
    } else if (e.event === "GATE_REJECTED") {
      floorIdx = i;
    }
  }
```

- `WORKFLOW_STARTED` / `STAGE_JUMPED` は **stage 非依存** (どの stage の receipt も無効化する)。
- `STAGE_STARTED` は当該 stage のみ、かつ `unit-major` では**スキップ**、かつ `Workflow` が `single-stage:` 始まりの行は除外。
- `GATE_REJECTED` は当該 stage のみ。

**(B) produces[] write は「floor」ではなく、floor 以降の順走査中の *clear-on-write*** (`aidlc-lib.ts:5114-5154`)。イベントは `ARTIFACT_CREATED` / `ARTIFACT_UPDATED` の**監査ブロック**であり、その `File` フィールドを `producesArtifactUnit(stage, file, recordedRepos)` に通して当該 stage の produces[] かどうかを判定する:

```
  for (let i = floorIdx + 1; i < events.length; i++) {
    const e = events[i];
    if (e.event === "ARTIFACT_CREATED" || e.event === "ARTIFACT_UPDATED") {
      const file = auditBlockField(e.block, "File");
      if (!file) continue;
      const targetUnit = producesArtifactUnit(stage, file, recordedRepos);
      if (targetUnit === undefined) continue;
```

- 非 per-unit: 既収集の `stageVerdict` を破棄し `stageStale = true` にする (`:5121-5131`)。
- per-unit で `targetUnit === null` (パスからユニットを特定できない曖昧ケース): **fail-closed で全ユニット receipt を破棄** (`:5132-5142`)。
- per-unit で特定できた場合: そのユニットの receipt のみ破棄 (`:5143-5153`)。

イベントストリームのソートは (timestamp, buffer position) の二段 (`:5067`):

```
  events.sort((a, b) => (a.ts !== b.ts ? (a.ts < b.ts ? -1 : 1) : a.pos - b.pos));
```

理由の逐語 (`:4986-4992`): `a timestamp-only floor is unsafe because isoTimestamp() is second-precision, so a review and the reject that should invalidate it can share a timestamp and a `<` compare would keep the stale review. Ordering by (timestamp, buffer position) breaks that tie.`

**mtime 不使用の検証**: `grep -c 'mtime\|birthtime'` → `aidlc-state.ts:0` / `aidlc-orchestrate.ts:1` / `aidlc-lib.ts:31`。全ヒットを目視した結果、mtime の使用箇所は (a) ターンシェイプマーカー `.aidlc-human-turn` vs `.aidlc-engine-touch` の比較 (`aidlc-lib.ts:6084`)、(b) audit lock ディレクトリの経年判定 (`:6939`, `:6952-6964`)、(c) active-directive / hook マーカーの鮮度窓 (`:6103`, `:6121`) のみ。**review freshness 経路に mtime は 1 箇所も存在しない**。

なお第 2 の鮮度軸「artifact fingerprint」も mtime ではなく**内容 sha256** である (`reviewArtifactFingerprint`, `aidlc-lib.ts:4946-4984`): `statSync` は `isFile()` 判定にのみ使い (`:4971-4976`)、値は `createHash("sha256").update(readFileSync(entry.path))` (`:4977`)。マニフェストは論理パス昇順で `[logicalPath, "sha256:<hex>" | "missing" | "not-file"]` を並べ、その JSON をさらに sha256 する (`:4963-4983`)。

→ **B10 実装への含意**: 「latest relevant produces[] write」は監査台帳の走査で再現できる。ファイルシステム時刻に一切依存しないため、Quint モデル上も監査行列 (audit trace) だけで完全に決定的にモデル化できる。

## 1.3 早期リターンとガード順序 (`:1794-1848`)

1. `if (!stage.reviewer) return;` — `// stage declares no reviewer — nothing to enforce` (`:1794`)
2. review class 解決 (`:1802-1812`)。`autonomousSwarm` なら `stage.review_class ?? "adversarial"` をそのまま、そうでなければ `resolveReviewClass(...)`。`reviewClass === "none"` なら return。逐語コメント (`:1796-1801`):
```
  // Interactive directives omit the reviewer when the effective class resolves
  // to `none`; their completion path must use that same resolution or it asks
  // for a receipt the conductor was explicitly told not to create. Autonomous
  // swarm stages are the exception: their declared reviewer is the only
  // pre-merge verification inside each Bolt, so caps/overrides do not silence
  // the receipt requirement there.
```
3. source-freshness 判定 (`:1830-1844`)。`stage.workspace_requires === true && !AIDLC_SKIP_SOURCE_FRESHNESS && !settledSwarm && receipts.sourceStale` → `staleSourcePreconditionError`。
4. `if (!requireReceiptExistence) return;` (`:1848`) — 逐語コメント (`:1846-1847`): `// Already-[x] recovery skips only existence/cardinality. A modern binding was still compared above, so crash-window recovery cannot ship changed source.`
5. 非 per-unit: `sawStageReview` が false なら、`receipts.stageStale` で `staleReviewPreconditionError`、そうでなければ `reviewerPreconditionError` (`:1853-1865`)。
6. per-unit: `resolveBoltDag(pd)` → `malformed` / `none|0 units` / 通常 の 3 経路 (`:1867-1943`)。kind 剪定で applicable produces が 0 のユニットは review 対象から除外 (`:1880-1892`, 逐語コメント `// A kind-pruned unit with no applicable produces[] never receives a stage directive, so it cannot owe a review.`)。

## 1.4 ★ 拒否文言 — **完全逐語** (実際は 6 種。研究文書の 4 種 + 2)

### (1) receipt なし — `reviewerPreconditionError` (`aidlc-state.ts:2026-2037`)

```
function reviewerPreconditionError(slug: string, reviewer: string): never {
  error(
    `Refusing to complete "${slug}": it declares a reviewer (${reviewer}) but no ` +
      `fresh REVIEW_COMPLETED is recorded for it. Invoke the reviewer ` +
      `(stage-protocol-reviewer.md §12a) and record the verdict with \`aidlc-log.ts review --stage ` +
      `${slug} --reviewer ${reviewer} --verdict <READY|NOT-READY>\` before completing. ` +
      `Terminal ordering: apply any fixes FIRST, then run the reviewer, record the ` +
      `receipt, and stop editing produces[] artifacts - a later write to one ` +
      `invalidates the receipt and re-opens this refusal. Do not apply suggestions ` +
      `riding on a READY verdict; surface them at the gate instead.`
  );
}
```

### (2) receipt 無効化 (recovery 未消費) — `staleReviewPreconditionError` else 腕 (`:2013-2023`)

```
  error(
    `Refusing to complete "${slug}": its terminal review receipt from ${reviewer} ` +
      `was invalidated by a later write to a declared produces[] artifact. Run ` +
      `one recovery review pass with \`aidlc-log.ts review --stage ${slug} ` +
      `--reviewer ${reviewer} --iteration <next ordinal>\`, then record the verdict ` +
      `with the same command plus \`--verdict <READY|NOT-READY>\`. After that ` +
      `receipt, stop editing produces[] artifacts. If the recovery pass was already ` +
      `spent, present the situation to the human at the approval gate; a human ` +
      `Request Changes decision resets the review attempt. Do not record a rejection ` +
      `on the human's behalf.`
  );
```

### (3) receipt 無効化 (recovery 消費済み) — `staleReviewPreconditionError` if 腕 (`:2004-2011`)

```
    error(
      `Refusing to complete "${slug}": its stale-receipt recovery review from ` +
        `${reviewer} was invalidated by another later write to a declared ` +
        `produces[] artifact. Present the situation to the human at the approval ` +
        `gate. Only a human Request Changes decision resets the review attempt; ` +
        `do not record it on the human's behalf.`
    );
```

### (4) source-fingerprint 不一致 (recovery 未消費) — `staleSourcePreconditionError` else 腕 (`:1959-1966`)

```
  error(
    `Refusing to complete "${slug}": the workspace source no longer matches the ` +
      `state of the most recent recorded review (source-fingerprint mismatch). ` +
      `Re-invoke ${reviewer} against the current source, record the one bounded ` +
      `stale-receipt recovery REVIEW_REQUESTED/REVIEW_COMPLETED pair, or revert ` +
      `the source edit. The recovery pass remains available after the normal ` +
      `review iteration budget is exhausted.`,
  );
```

### (5) source-fingerprint 不一致 (recovery 消費済み) — 同 if 腕 (`:1951-1958`)

```
  if (recoverySpent) {
    error(
      `Refusing to complete "${slug}": the workspace source no longer matches ` +
        `the stale-receipt recovery review (source-fingerprint mismatch). ` +
        `Present this at the approval gate. Only a human Request Changes decision ` +
        `resets the review attempt; do not record it on the human's behalf.`,
    );
  }
```

### (6) per-unit — 欠落ユニット集計 + 3 種ガイダンス (`:1894-1943`)

拒否本文 (`:1935-1942`):
```
    error(
      `Refusing to complete "${stage.slug}": it declares a reviewer (${reviewer}) but ` +
        `${missing.length} of ${reviewUnits.length} applicable units have no fresh recorded ` +
        `review (${missing.join(", ")}). Invalidated receipts: ` +
        `${stale.length > 0 ? stale.join(", ") : "none"}. Never reviewed: ` +
        `${neverReviewed.length > 0 ? neverReviewed.join(", ") : "none"}. ` +
        guidance.join(" ")
    );
```

ガイダンス 3 種 — recovery available (`:1905-1913`):
```
      guidance.push(
        `For invalidated units with recovery available (${recoveryAvailable.join(", ")}), ` +
          `run \`aidlc-log.ts review --stage ${stage.slug} --unit <unit> --reviewer ` +
          `${reviewer} --iteration <next ordinal>\`, then record the verdict with ` +
          `the same command plus \`--verdict <READY|NOT-READY>\` and stop editing ` +
          `produces[] artifacts.`,
      );
```
recovery spent (autonomousSwarm 分岐あり、`:1914-1927`):
```
      guidance.push(
        autonomousSwarm
          ? `For autonomous units whose recovery was already spent (${recoverySpent.join(", ")}), ` +
            `do not put them in --claimed or finalize/merge them. Halt and ask the ` +
            `human whether to restart each Bolt; on approval abort/discard the old ` +
            `Bolt and rerun the current swarm prepare step so a fresh BOLT_STARTED ` +
            `boundary resets review accounting.`
          : `For units whose recovery was already spent (${recoverySpent.join(", ")}), ` +
            `present the situation to the human at the approval gate. Only a human ` +
            `Request Changes decision resets the review attempt; do not record it ` +
            `on the human's behalf.`,
      );
```
never reviewed (`:1928-1934`):
```
      guidance.push(
        `For never-reviewed units (${neverReviewed.join(", ")}), run the normal ` +
          `\`aidlc-log.ts review --stage ${stage.slug} --unit <unit> --reviewer ` +
          `${reviewer} --iteration <next ordinal>\` request and record its verdict.`,
      );
```

### 付随 (bolt DAG 解決不能, `:1869-1873`)
```
    error(
      `Refusing to complete "${stage.slug}": its per-unit review set cannot be ` +
        `resolved because unit-of-work-dependency.md is ${resolution.reason} ` +
        `(${resolution.detail}). Fix the fenced units block before completing.`,
    );
```

## 1.5 ★ recovery 1 回制限の状態追跡 — 完全な機構

**recovery は state フィールドでもカウンタでもなく、監査行のフィールド `Recovery: stale-receipt` によって表現される。1 回制限は「現在の attempt 窓 (floorIdx 以降) に recovery 付き REVIEW_REQUESTED が 1 件でもあれば消費済み」という導出述語。**

1. **記録**: `REVIEW_REQUESTED` 行が `Recovery` フィールドを持ち、値が `stale-receipt` のとき recovery pass とみなす (`aidlc-lib.ts:5170-5181`):
```
    if (e.event === "REVIEW_REQUESTED") {
      const previous = pendingRequests.get(requestKey);
      const recovery =
        previous?.recovery === true ||
        auditBlockField(e.block, "Recovery") === "stale-receipt";
      if (recovery) sourceRecoverySpent = true;
      pendingRequests.set(requestKey, {
        unit,
        iteration,
        recovery,
      });
      continue;
    }
```
   `requestKey` は `` `${unit ?? ""} ${iterationField}` `` (`:5169`)。同一キーの再 REQUEST は `previous?.recovery === true` で **recovery フラグが粘着**する。

2. **cap 免除**: recovery review の verdict は iteration 予算を無視して terminal 扱い (`:5201-5208`):
```
    const terminalVerdict = request.recovery
      ? verdict
      : terminalReviewVerdict(
          verdict,
          iterationField,
          reviewClass,
          maxIterations,
        );
```

3. **伝播**: 受理された receipt に `stageReceiptRecovery` / `unitReceiptRecovery.set(unit, request.recovery)` として保持 (`:5259`, `:5266`)。source binding 側は `newestSourceProgress = { nextIteration: iteration + 1, recoverySpent: request.recovery }` (`:5218-5221`)。

4. **無効化時の持ち越し**: 後続の produces[] write が receipt を消すとき、`recoverySpent` を `StaleReviewProgress` に載せて引き継ぐ (`:5124-5127`, `:5135-5138`, `:5146-5149`)。fingerprint 不一致経路も同様 (`:5241-5253`)。

5. **読み出し**: `verifyReviewerPrecondition` は `receipts.sourceStaleProgress?.recoverySpent === true` (`aidlc-state.ts:1842`)、`receipts.stageStaleProgress?.recoverySpent === true` (`:1859`)、`receipts.unitStaleProgress.get(unit)?.recoverySpent !== true` (`:1899`) で分岐し、上記 (3)(5)(6-spent) の文言を選ぶ。

6. **リセット条件**: 走査は `floorIdx + 1` から始まるので、**新しい `WORKFLOW_STARTED` / `STAGE_JUMPED` / (非 unit-major の) `STAGE_STARTED` / `GATE_REJECTED` が入ると窓ごと消え、recovery は再び利用可能になる**。これが全拒否文の「Only a human Request Changes decision resets the review attempt」(= `GATE_REJECTED` を人間が起こす) の実装的裏付け。

7. **reject 側の追加ガード** — autonomous モードでの `GATE_REJECTED` 濫用防止。`reviewRecoverySpentInCurrentAttempt` (`aidlc-state.ts:2039-2062`) が `sourceRecoverySpent` / 各 `recoverySpent` のいずれかを見て true を返すと、`handleReject` は human presence を強制する (`:2914-2934`):
```
  const autonomousMode = isAutonomousMode(content);
  const recoveryResetNeedsHuman =
    autonomousMode && reviewRecoverySpentInCurrentAttempt(pd, content, stage);
  if (
    (!autonomousDecision || recoveryResetNeedsHuman) &&
    !humanPresenceGuardDisabled() &&
    !humanActedSinceGate(pd)
  ) {
    if (recoveryResetNeedsHuman) {
      error(
        `Refusing to reject "${slug}": the stale-receipt recovery review was already spent ` +
          "in this stage attempt, so GATE_REJECTED may reset review accounting only after " +
          "a real human has acted. Present the escalation to the human and wait for a typed " +
          "Request Changes decision before retrying.",
      );
    }
    error(
      `Refusing to reject "${slug}": a real human has not acted at this gate since it opened. ` +
        "Requesting changes requires a typed human turn before it can commit.",
    );
  }
```

---

# 2. report 段 11 / 段 12 の全拒否文言

## 2.1 段 11 `checkStageCompletionEvidence` (`aidlc-orchestrate.ts:5128-5230`)

呼び出しは 2 箇所: gate-lifecycle アームの `awaiting-approval` / `revised` 時 (`:5681-5696`) と、forward verdict 経路で checkbox ≠ completed のとき (`:5754-5766`)。設計コメント (`:5125-5127`):
```
// The evidence required before a gated stage may either enter [?] or resolve
// approval. Sharing this check prevents gate-start, revised, and approved from
// disagreeing about whether per-unit work and collaborator dispatch completed.
```

判定順は **pipeline → per-unit (paused → uncovered) → ensemble** の 4 グループ、計 5 種の拒否文言。

### (11-a) pipeline link receipt 欠落 (`:5155-5163`)
```
      return {
        ok: false,
        message:
          `Stage "${slug}" is mode: pipeline and cannot enter or complete approval until every ` +
          `declared link has a current-attempt PIPELINE_LINK_COMPLETED receipt. Missing: ${missing.join(", ")}. ` +
          `After each link returns, run \`bun ${harnessDir()}/tools/aidlc-log.ts link --stage ${slug} ` +
          `--link <agent>${evidence.repos.length > 0 ? " --repo <repo>" : ""}\`. ` +
          `Set AIDLC_DISABLE_ENSEMBLE_EVIDENCE=1 only to recover a legitimately-run in-flight pipeline.`,
      };
```
発火条件: `node.mode === "pipeline" && process.env.AIDLC_DISABLE_ENSEMBLE_EVIDENCE !== "1"` (`:5146-5149`)。

### (11-b) per-unit の unit list 解決不能 (`:5170-5176`)
```
      return {
        ok: false,
        message:
          `Stage "${slug}" is per-unit (for_each: unit-of-work) but the unit list cannot be resolved: ` +
          `inception/units-generation/unit-of-work-dependency.md is ${resolution.reason} ` +
          `(${resolution.detail}). Fix the fenced units block before entering approval.`,
      };
```

### (11-c) ★ paused-unit 拒否 (`:5184-5193`)
```
      if (ledger.checkpoint?.state === "paused") {
        const cp = ledger.checkpoint;
        return {
          ok: false,
          message:
            `Stage "${slug}" cannot enter approval: unit "${cp.unit}" is paused` +
            `${cp.reason ? ` (reason: ${cp.reason})` : ""}. Resume and complete it first ` +
            `(bun ${harnessDir()}/tools/aidlc-state.ts unit resume --stage ${slug} --unit ${cp.unit}).`,
        };
      }
```
逐語コメント (`:5182-5183`): `// A paused unit blocks approval outright: its work is not done and the pause carries an explicit next action a gate must not paper over.`

### (11-d) per-unit カバレッジ不足 (`:5207-5215`)
```
      if (pick !== null) {
        return {
          ok: false,
          message:
            `Stage "${slug}" is per-unit (for_each: unit-of-work) and ${pick.uncovered.length} of ` +
            `${units.length} units are not yet complete (${pick.uncovered.join(", ")}). ` +
            "Run `next` to complete the remaining units before entering approval.",
        };
      }
```
なお `pick` が `{error}` 形なら、その文字列がそのまま中継される (`:5204-5206`)。

### (11-e) ensemble contribution evidence — `checkEnsembleEvidence` (`:5034-5123`, 拒否は `:5114-5122`)
```
  return {
    ok: false,
    message:
      `Stage "${slug}" is mode: ${node.mode} - its ensemble must convene before approval, and the ` +
      `contribution files are the evidence. Missing or malformed: ${missing.join("; ")}. ` +
      `Dispatch each support agent to write ${contributionPath} ` +
      `(first line: **Collaborator:** <agent-slug>) per stage-protocol-ensemble.md §5, then re-report. ` +
      `Set AIDLC_DISABLE_ENSEMBLE_EVIDENCE=1 only to recover a legitimately-run stage whose files were lost.`,
  };
```
`missing` の要素は `` `${subject} (no contribution file)` `` (`:5101`) と `` `${subject} (missing identity-marker first line)` `` (`:5105`) の 2 形。`subject` は `unit === null ? agent : ` + `` `${agent} for unit "${unit}"` `` (`:5096`)。判定される 1 行目は厳密一致 `` `**Collaborator:** ${agent}` `` (`:5104`)。
`contributionPath` は 2 形 (`:5111-5113`):
```
  const contributionPath = usesUnitDirs
    ? `${prefix}/construction/<unit>/${slug}/contributions/<agent-slug>.md`
    : `${prefix}/${node.phase}/${slug}/contributions/<agent-slug>.md`;
```
早期 ok 条件 (`:5046-5054`): `!isGated || !requiresEnsembleEvidence(node) || options.settledSwarm === true || AIDLC_DISABLE_ENSEMBLE_EVIDENCE === "1"`。`requiresEnsembleEvidence` (`:5025-5028`) は `node.mode === "mob" || (node.mode === "subagent" && support_agents.length > 0)`。

## 2.2 段 12 practices-discovery promotion receipt (`aidlc-orchestrate.ts:5772-5784`)

```
  if (
    slug === "practices-discovery" &&
    stageCheckbox.state !== "completed" &&
    !hasFreshPracticesAffirmationReceipt(pd, stateContent)
  ) {
    emit(errorDirective(
      'Cannot approve "practices-discovery" before practices-promote succeeds. ' +
        "Run aidlc-state.ts practices-promote after the human approves; it records " +
        "Practices Affirmed Timestamp and a fresh PRACTICES_AFFIRMED receipt for " +
        "this stage attempt, then report --result approved --user-input \"<exact choice>\".",
    ));
    return;
  }
```

解決文字列 (1 本):
```
Cannot approve "practices-discovery" before practices-promote succeeds. Run aidlc-state.ts practices-promote after the human approves; it records Practices Affirmed Timestamp and a fresh PRACTICES_AFFIRMED receipt for this stage attempt, then report --result approved --user-input "<exact choice>".
```

判定関数 `hasFreshPracticesAffirmationReceipt` (`:4761-4800+`) の floor は **3 イベント** (`:4772-4776`):
```
  const FLOOR_EVENTS = new Set([
    "STAGE_STARTED",
    "GATE_REJECTED",
    "STAGE_REVISING",
  ]);
```
ソートは `(timestampMs, position)` (`:4791-4796`)。`Practices Affirmed Timestamp` は `isConcreteIsoInstant` (`:4747-4752`) で厳密検証:
```
  const isoInstant =
    /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,3})?(?:Z|[+-]\d{2}:\d{2})$/;
  return isoInstant.test(value) && !Number.isNaN(Date.parse(value));
```

---

# 3. ★ stale re-report の moved-on 述語 (`aidlc-orchestrate.ts:5841-5859`)

**インデックス比較でも `nextInScopeStage` 走査でもない。「`--stage` で名指しされた slug が `Current Stage` と異なり、かつ `Current Stage` の checkbox 行が存在し、かつその状態が `pending` でない」という *checkbox 状態* 述語である。**

```
    } else {
      // Stale re-report guard. If the workflow has already moved on — Current
      // Stage points at a DIFFERENT slug whose checkbox has left pending — a
      // re-report of the completed stage is a replay, not a recovery. Spawning
      // advance here would demote a gate-held `[?]`/`[R]` current stage back to
      // `[-]` and re-emit STAGE_STARTED. The legitimate recovery (approve
      // landed but advance crashed: slug === currentSlug, next still pending)
      // falls through to advance below.
      const currentCb =
        slug === currentSlug ? undefined : checkboxForSlug(stateContent, currentSlug);
      if (currentCb && currentCb.state !== "pending") {
        emit({
          kind: "done",
          reason:
            `Stage "${slug}" is already completed and the workflow has moved on to ` +
            `"${currentSlug}" (scope: ${scope}); idempotent re-report, no transition needed.`,
        });
        return;
      }
      sequence.push(["advance", slug]);
    }
```

**Quint モデル近似の検証結果**:

| 観点 | 実装 |
|---|---|
| 到達条件 | `stageCheckbox.state === "completed"` かつ `!isFinal` (`:5828`, `:5841`) |
| moved-on 述語 | `slug !== currentSlug` ∧ `checkboxForSlug(state, currentSlug) ≠ null` ∧ `state ≠ "pending"` |
| グラフ順序の参照 | **なし** — stage index 比較も後続 stage 走査も行わない |
| 正当な recovery の通し方 | `slug === currentSlug` → `currentCb === undefined` → advance へ。または `currentSlug` の checkbox が `pending` (= advance がクラッシュして次段が未開始) → advance へ |
| checkbox 行が state ファイルに無い場合 | `currentCb` が falsy → **advance へフォールスルー** (fail-open) |
| 出力 | `done` (冪等)。state 変更ゼロ |

対比: final 側の冪等 no-op は別分岐 (`:5829-5837`) で、こちらは `Status` フィールド一致で判定する:
```
      if (status === "Completed") {
        emit({
          kind: "done",
          reason:
            `Workflow is already completed at "${slug}" (scope: ${scope}); no transition was needed.${NEW_WORK_HINT}`,
        });
        return;
      }
```
(`status` は `getField(stateContent, "Status") ?? ""`, `:5803`。`NEW_WORK_HINT` は `:853` 定義)

→ **Quint モデルへの含意**: 遷移関数の引数に「グラフ上の位置関係」は不要。`(checkbox[slug], gated, final, checkbox[currentStage], slug==currentStage, explicitStage, status)` で完全に決定的。研究文書 §9 の 5 引数モデルは `checkbox[currentStage]` と `slug==currentStage` を分離すれば正確になる。

---

# 4. forward 表 #6 (in-progress + gated + 明示 `--stage` なし) と `gate-start --recovered`

## 4.1 拒否文言の完全逐語 (`aidlc-orchestrate.ts:5862-5877`)

```
  } else if (isGated) {
    if (stageCheckbox.state === "in-progress") {
      if (!explicitStage) {
        emit({
          kind: "error",
          message:
            `Stage "${slug}" is still in-progress. To approve a gated stage that has not entered ` +
            `awaiting-approval, report the acted directive explicitly with --stage "${slug}" so ` +
            "the engine cannot mistake a freshly advanced Current Stage for the completed one.",
        });
        return;
      }
      // Backfilled gate — tag the row Recovered=true so audit consumers can
      // tell the engine-opened gate from an organic gate-start.
      sequence.push(["gate-start", slug, "--recovered"]);
    }
```

解決文字列 (1 本):
```
Stage "<slug>" is still in-progress. To approve a gated stage that has not entered awaiting-approval, report the acted directive explicitly with --stage "<slug>" so the engine cannot mistake a freshly advanced Current Stage for the completed one.
```

その直後、必ず `approve` が積まれる (`:5884`, 手前に reviewer precondition を engine 側に置かない理由の逐語コメント `:5878-5883`):
```
    // Reviewer precondition (§12a / RFC Track 1) is NOT enforced here. Like the
    // artifact, human-presence, and revision guards, it lives in
    // aidlc-state.ts handleApprove — the ONE seam every approve passes through
    // (report shells out to `state.ts approve`, but agents also call it directly
    // on recovery, so a report-only guard is bypassable, issue #366). See
    // verifyReviewerPrecondition in aidlc-state.ts.
    sequence.push(approveArgs(slug, flags));
```
`approveArgs` (`:5370-5374`) は `["approve", slug]` に `flags.userInput` があれば `--user-input <text>` を足すだけ。

→ 従って #6 の実シーケンスは `gate-start <slug> --recovered` + `approve <slug> [--user-input …]`、成功時 done の `committed.join(" + ")` は `gate-start + approve`。

## 4.2 `--recovered` の監査行での現れ方 (`aidlc-state.ts:2569-2611`)

書き込み側 (`handleGateStart`, `:2600-2607`):
```
  try {
    const fields: Record<string, string> = { Stage: slug };
    if (artifacts) fields.Artifacts = artifacts;
    if (recovered) fields.Recovered = "true";
    emitAudit(pd, "STAGE_AWAITING_APPROVAL", fields);
  } catch (e) {
    error(`Audit emission failed: ${errorMessage(e)}`);
  }
```
→ **イベント種別は通常の `STAGE_AWAITING_APPROVAL` のまま。区別は `Recovered: true` という追加フィールド 1 個だけ。** フラグ検出は `const recovered = args.includes("--recovered");` (`:2581`)、usage は `Usage: aidlc-state.ts gate-start <slug> [--artifacts <csv>] [--recovered]` (`:2574`)。ヘッダコメント (`:2569-2572`):
```
// gate-start <slug> — transition [-] → [?], emit STAGE_AWAITING_APPROVAL.
// --recovered marks a BACKFILLED gate row (the engine opening a gate the
// conductor skipped, e.g. report's explicit-stage recovery) with
// Recovered=true so audit consumers can tell backfills from organic opens.
```
順序: audit emit が `writeStateFile` より先 (`:2604` → `:2609`)。emit 失敗は `error()` で state write を止める (audit-first)。

読み出し側 (`unrecordedRevisionSinceGateOpen`, `:354-...`) — **`Recovered: true` の gate 行はアンカーになれない**:
```
    recovered: auditField(blocks[i], "Recovered") === "true",   // :387
```
```
    if (events[i].event === "STAGE_AWAITING_APPROVAL" && !events[i].recovered) {   // :405
      anchor = i;
      anchorIsGateOpen = true;
    } else if (events[i].event === "STAGE_STARTED") {                              // :408
      anchor = i;
```
理由の逐語 (`:314-320`):
```
//   1. an anchor exists: the LAST ORGANIC (non-Recovered) STAGE_AWAITING_APPROVAL
//      for this slug, or, when the stage was (re)started after it / it never
//      happened, the LAST STAGE_STARTED for this slug (the current stage run's
//      boundary). Recovered=true gate rows are NEVER the anchor: report
//      synthesizes one right before approve when the conductor skipped
//      gate-start, so its timestamp postdates the human turns and revision
//      writes the predicate needs inside the window, AND
```
`Recovered: "true"` を書く他の箇所: `aidlc-state.ts:2755`, `:2763`, `:2767` (approve 側の GATE_REJECTED + STAGE_REVISING 補填ペア)、`:2978`。

---

# 5. next ラダーの未抽出逐語

## 5.1 分岐 0 — Kiro roll-forward ラッチ (`:2635-2681`)

発火ガード (`:2648-2651`) — **17 個のフラグがすべて未設定である「真に裸の next」**:
```
  if (!flags.readOnly && !flags.workspaceCommand && !flags.pluginCommand && !flags.knowledgeCommand && !flags.stage && !flags.phase &&
      !flags.scope && !flags.positionalScope && !flags.intent && !flags.resume &&
      !flags.depth && !flags.testStrategy && !flags.review &&
      !flags.single && !flags.compose && !flags.newScope && !flags.report) {
```
ラッチ読み取り (`:2653-2672`): `aidlc/.aidlc-readonly-latch` (JSON `{turn?, flag?, source?}`) と `aidlc/.aidlc-turn-counter` (整数)。ラベル整形 (`:2666-2671`):
```
        if (typeof lr.flag === "string") {
          // Read-only flags render with `--`; noun commands render as typed.
          const nounCommand = lr.source === "workspace-verb" || lr.source === "plugin-verb" ||
            lr.source === "knowledge-verb";
          label = nounCommand ? `\`${lr.flag}\`` : `--${lr.flag}`;
        }
```
既定ラベルは `let label = "the read-only command";` (`:2658`)。判定と発出 (`:2673-2680`):
```
      if (counter >= 0 && latchTurn === counter) {
        emit({
          kind: "done",
          reason: `The read-only/navigation command (${label}) already ran this turn and its output was shown above. This was a read-only utility or a workspace switch, not workflow work — there is nothing to advance. The workflow is unchanged; if one is active it remains paused where it was. STOP.`,
        });
        return;
      }
    } catch { /* advisory: guard is best-effort, never blocks a real next */ }
```
初期値は `counter = -1`, `latchTurn = -2` (`:2656-2657`) なので、両ファイル欠如時は `counter >= 0` が false で必ずフォールスルー。

## 5.2 分岐 1b / 1c / 1d — 名詞トークン (`:2711-2775`)

3 分岐とも同一形: `command.kind === "error"` なら `errorDirective(command.message)`、そうでなければ argv を組んで `printDirective`。

**1b workspace** (`:2721-2738`) — 唯一 `Invalid workspace command.` の argv 変換失敗経路を持つ:
```
    const argv = workspaceCommandUtilityArgv(command);
    if (argv === null) {
      emit(errorDirective("Invalid workspace command."));
      return;
    }
    const [verb, ...tail] = argv;
    const suffix = tail.length > 0 ? ` ${tail.map(shellArg).join(" ")}` : "";
    emit(printDirective(
      `Run \`bun ${harnessDir()}/tools/aidlc-utility.ts ${verb}${suffix}\`, print its output verbatim, then stop.`,
    ));
```
→ **1b だけ「terminal utility, NOT workflow work」の追伸が付かない**(スペース切替は workflow を止めない)。

**1c plugin** (`:2743-2755`):
```
    const argv = command.kind === "help" ? ["help"] : command.argv;
    const [verb, ...tail] = argv;
    const suffix = tail.length > 0 ? ` ${tail.map(shellArg).join(" ")}` : "";
    emit(printDirective(
      `Run \`bun ${harnessDir()}/tools/aidlc-utility.ts ${verb}${suffix}\`, print its output verbatim, then stop. This is a terminal utility, NOT workflow work: do NOT run \`next\` and do NOT advance, resume, or run any workflow stage.`,
    ));
```

**1d knowledge** (`:2762-2774`) — 1c と同文だが**ツール名が `aidlc-knowledge.ts`**:
```
    emit(printDirective(
      `Run \`bun ${harnessDir()}/tools/aidlc-knowledge.ts ${verb}${suffix}\`, print its output verbatim, then stop. This is a terminal utility, NOT workflow work: do NOT run \`next\` and do NOT advance, resume, or run any workflow stage.`,
    ));
```
逐語コメント (`:2758-2761`): `// Branch 1d - DocumentKB verbs are terminal commands, never freeform intent text. Same shape as 1c, but the directive names aidlc-knowledge.ts: this is the first public noun whose verbs live in their own tool rather than in aidlc-utility.ts, so the tool name is part of what each site must agree on.`

参考 — **分岐 1** の逐語 (`:2705-2707`):
```
    emit(printDirective(
      `Run \`bun ${harnessDir()}/tools/aidlc-utility.ts ${sub}${extra}\`, print its output verbatim, then stop. This is a read-only utility, NOT workflow work: do NOT run \`next\` and do NOT advance, resume, or run any workflow stage.`,
    ));
```

## 5.3 分岐 4c — compose print 本文 (`composeDispatchDirective`, `:930-980`)

分岐側 (`:2940-2949`):
```
  if (flags.compose || flags.newScope || flags.report) {
    if (flags.stage || flags.phase) {
      emit(errorDirective(
        "Cannot combine compose with --stage/--phase. Compose re-shapes the plan; jump moves the cursor. Run them separately.",
      ));
      return;
    }
    emit(composeDispatchDirective(flags, stateContent !== null));
    return;
  }
```

### in-flight アーム (`:936-945`) — 4 パーツ
```
    parts.push(
      `Dispatch the composer agent (${hd}/agents/aidlc-composer-agent.md) as a subagent to propose re-shaping the RUNNING workflow's pending stages` +
        (flags.intent ? ` for: "${flags.intent}".` : "."),
      "The composer reads the live state file's Stage Progress, re-estimates the entropy components from what completed stages resolved, validates the flipped grid with --strict, and proposes SKIP/un-SKIP flips for PENDING, ahead-of-cursor stages only (completed [x], in-progress [-], and skipped [S] stages are frozen; an ADD whose required producer is skipped or behind the cursor is rejected, not proposed).",
      "This is mode in-flight, not matched/custom routing: preserve the current scope, depth, frozen actions, and full effective grid; stock-distance rankings are advisory only and MUST NOT trigger stock-grid adoption. Return the exact approved command delta as changes.skip and changes.add arrays.",
      "BEFORE presenting the gate, write the pending-proposal marker `aidlc/.aidlc-compose-pending` (any content) so the turn can end at the gate; on approve run `bun " +
        hd +
        "/tools/aidlc-utility.ts recompose --skip <changes.skip> --add <changes.add>` (comma-separated) and DELETE the marker; on reject/edit-then-resolve delete the marker too. Never write scope registry files for an in-flight proposal.",
    );
```

### front アーム (`:947-959`)
```
    parts.push(
      `Dispatch the composer agent (${hd}/agents/aidlc-composer-agent.md) as a subagent to propose the workflow plan for: "${flags.intent ?? ""}".`,
    );
    if (flags.report) {
      parts.push(
        `First have it read and triage the scan report at "${flags.report}" (auto-fixable vs human-decision findings), then compose a compact fix-and-ship grid - this often routes to the stock bugfix or security-patch scope rather than minting a new one.`,
      );
    }
    if (flags.newScope) {
      parts.push(
        "--new-scope was passed: the composer must SYNTHESIZE a custom scope even if a stock scope matches.",
      );
    }
```

### 共通末尾 (`:961-970`)
```
  const proposalShape = inFlight
    ? "mode in-flight, the current scopeName, an ars block (the five component scores with method codekb|fallback), an arsRationale, the preserved full effective grid, exact changes.skip and changes.add arrays, a per-change rationale, a summary the strict validator computed, and two pre-rendered markdown tables (ARS scores with bands; per-stage decisions with reasoning)"
    : "mode matched|custom, scopeName, an ars block (the five component scores with method codekb|fallback), an arsRationale, the per-stage EXECUTE/SKIP grid, a per-SKIP rationale, a summary the validator computed, and two pre-rendered markdown tables (ARS scores with bands; per-stage decisions with reasoning)";
  const modeContract = inFlight
    ? "the composer's mode is IN-FLIGHT and FINAL for the returned delta: nearest_stock is advisory, the running scope and frozen actions stay unchanged, and approval uses only changes.skip/changes.add through recompose; neither presentation nor comparison with stock grids may alter that delta"
    : "the composer's mode is FINAL for the grid it returned: it routed matched-vs-custom solely on the final proposal validator's nearest_stock distance, a matched proposal already carries the revalidated stock grid verbatim, and neither presentation nor your own comparison of grids ever changes the verdict - never re-derive it, and a MATCHED proposal writes no scope file; if the human edits that stock grid, re-dispatch the composer, which must convert it to CUSTOM and revalidate before re-presenting";
  parts.push(
    `The composer runs \`bun ${hd}/tools/aidlc-utility.ts detect --json\` (read-only scan + scope-registry paths), estimates the five entropy components (intent ambiguity, structural uncertainty, verification entropy, risk, unresolved assumptions) per its persona, and returns a structured proposal: ${proposalShape}.`,
    `Render the proposal to the human as THREE blocks before the approve/edit/reject gate (see the composer block in SKILL.md), leading with plain language rather than the scores: (1) a two-or-three-sentence recommendation in your own words - what kind of change this looks like, how much process you suggest, and the steps in plain terms - followed by the validator's summary line formatted "<execute> stages EXECUTE / <skip> SKIP, <gates> approval gates" plus scopeName and mode (${modeContract}); (2) the composer's stage-decision table verbatim, with any fold advisories beneath it; (3) under a "Scoring detail (advisory)" heading, the composer's ARS score table verbatim with its method line and arsRationale. Relay the composer's tables and numbers as returned - never recompute, collapse into prose, or drop them. Do NOT write any file and do NOT advance any stage before an explicit approval.`,
  );
  const directive = printDirective(parts.join(" "));
```

### narration (`:976-978`)
```
  directive.narration = inFlight
    ? "Looking at what is left to do and working out which of the remaining steps still earn their place. I will show you the change before anything moves."
    : "Working out which steps of the development process this piece of work actually needs, based on what you have asked for and what is already in the codebase. I will show you the plan before anything runs.";
```

## 5.4 分岐 4a — `--new-intent` (`:2966-2982`)

```
  if (flags.newIntent) {
    const description = flags.intent?.trim();
    if (!description) {
      emit(errorDirective(
        "`next --new-intent` requires a nonblank new-work description after the confirmed scope.",
      ));
      return;
    }
    …
    emit(createPrintDirective(flags.scope ?? scope, flags, description));
    return;
  }
```
scope 選択の逐語コメント (`:2974-2979`): `// Use the EXPLICIT --scope, not the precedence-ladder `scope` (which lets the ACTIVE intent's state scope win — wrong for a brand-new intent: the offer confirmed a scope for the NEW work, independent of what's in flight). Fall back to the resolved scope only when no flag was passed.`

`createPrintDirective` の new-intent 腕 (`:899-908`):
```
  const directive = flags.newIntent
    ? printDirective(
      `${runCmd} to start the new intent${cost}.${labelHint} Then STOP, do NOT re-run \`next\` in this session. ` +
        `This is a NEW, unrelated intent, and the current session still carries the previous intent's context. ` +
        `Tell the user to start a fresh session using this harness's reset or restart flow, then invoke its AI-DLC entry skill to begin the new intent with a clean slate. ` +
        `Nothing is lost: the intent is saved on disk and resumes on the next \`next\`.`,
      )
    : printDirective(
      `${runCmd} to start the workflow${cost}, then re-run \`next\` to continue.${labelHint}`,
    );
```
`runCmd` = `` `Run \`bun ${harnessDir()}/tools/aidlc-utility.ts ${cmd.join(" ")}\`` `` (`:898`)。`labelHint` (`:887-888`):
```
    labelHint =
      ` Replace \`--label\` with a 2-3 word kebab essence of the description (e.g. "simple calc"), which becomes the readable folder name for this piece of work.`;
```
narration (`:912-914`):
```
  directive.narration = clause
    ? `Setting up a ${scope} workflow for this: ${clause}.`
    : `Setting up a ${scope} workflow for this.`;
```
cost 句 `costClause` (`:679-686`):
```
  return `${c.execute} of ${c.total} stages, ${c.gates} approval gates${perUnit}`;
```
その `perUnit` は (`:682-684`):
```
  const perUnit = c.perUnitStages > 0
    ? `, ${c.perUnitStages} ${c.perUnitStages === 1 ? "stage repeats" : "stages repeat"} per unit of work in Construction`
    : "";
```

## 5.5 分岐 5 — scope-change / config-change (`:3028-3065`)

scope-change (`:3035-3048`):
```
    if (
      flags.scope &&
      validScopes().has(flags.scope) &&
      flags.scope !== currentStateScope
    ) {
      const parts = [`scope-change --scope ${flags.scope}`];
      if (flags.depth) parts.push(`--depth ${flags.depth}`);
      if (flags.testStrategy) parts.push(`--test-strategy ${flags.testStrategy}`);
      if (flags.review) parts.push(`--review ${flags.review}`);
      emit(printDirective(
        `Run \`bun ${harnessDir()}/tools/aidlc-utility.ts ${parts.join(" ")}\` to change scope, then print its output verbatim and stop.`,
      ));
      return;
    }
```
config-change (`:3052-3064`):
```
    if (
      (!flags.scope || flags.scope === currentStateScope) &&
      (flags.depth || flags.testStrategy || flags.review)
    ) {
      const parts = ["config-change"];
      if (flags.depth) parts.push(`--depth ${flags.depth}`);
      if (flags.testStrategy) parts.push(`--test-strategy ${flags.testStrategy}`);
      if (flags.review) parts.push(`--review ${flags.review}`);
      emit(printDirective(
        `Run \`bun ${harnessDir()}/tools/aidlc-utility.ts ${parts.join(" ")}\` to update the configuration, then print its output verbatim and stop.`,
      ));
      return;
    }
```
外側ガードは `if (stateContent && !flags.stage && !flags.phase)` (`:3028`)。同値 `--scope` を config-only として扱う理由の逐語 (`:3049-3051`): `// A depth / test-strategy / review modifier with no scope change is a config-change. A same-as-current --scope is also config-only: dropping it here would silently discard the modifiers and run the current stage.`

## 5.6 分岐 7b — 位置引数 scope、state なし (`:3111-3127`)

```
  if (
    !stateContent &&
    flags.positionalScope &&
    !flags.scope &&
    !flags.resume
  ) {
    // Don't birth a duplicate over a multi-intent workspace whose cursor is
    // unset (fresh clone) — prompt the human to pick an existing intent. null →
    // zero intents → birth as before.
    const pick = intentPickPromptIfRecordsExist(pd);
    if (pick) {
      emit(pick);
      return;
    }
    emit(createPrintDirective(flags.positionalScope, flags, flags.intent));
    return;
  }
```

`intentPickPromptIfRecordsExist` の ask 逐語 (`:1014-1019`):
```
  return askDirective(
    `This project already has ${intents.length} piece${intents.length === 1 ? "" : "s"} of work in progress${spaceLabel}, and none is currently selected ` +
      `(which one you are on is tracked per-person and does not travel with the repo). ` +
      `Pick the one to work on with \`/aidlc intent <slug>\`: ${list}. ` +
      "That selects it; re-run `next` afterward to carry on where it left off.",
  );
```
補助変数 (`:1011-1013`): `const list = slugs.map((s) => `\`${s}\``).join(", ");` / `const spaceLabel = space === "default" ? "" : ` in space "${space}"`;`。null 返却条件 (`:1006-1007`): intents 0 件、または `intents.some((i) => i.active)`。

## 5.7 分岐 8 — 自由記述 prose、state なし (`:3148-3183`)

keyword hit アーム (`:3155-3166`):
```
    const inferred = inferScopeFromText(flags.intent);
    if (inferred.source === "keyword") {
      // Preview the ceremony the user is confirming: stage/gate counts from the
      // compiled grid (never estimates). Drop the clause if the scope does not
      // resolve (a fixture tree without it) rather than emit a broken preview.
      const clause = costClause(inferred.scope);
      const cost = clause ? ` - ${clause}` : "";
      emit(askDirective(
        `This looks like "${inferred.scope}" work, so I'd run the "${inferred.scope}" plan for: "${flags.intent}"${cost}. ` +
          "Say go ahead, name a different plan, or say \"compose\" and I'll tailor one to this task.",
      ));
      return;
    }
```

compose offer アーム (`:3167-3182`):
```
    const express = scopeCostSummary("express");
    const classic = scopeCostSummary("classic");
    const feat = scopeCostSummary("feature");
    const fallbackExamples = [...validScopes()].slice(0, 3).join(", ") || "an explicit scope";
    const examples = express && classic && feat
      ? `express = ${express.execute} of ${express.total} stages, classic = ${classic.execute}, feature = all ${feat.execute}`
      : fallbackExamples;
    emit(askDirective(
      `None of the ready-made plans is an obvious fit for: "${flags.intent}". ` +
        "I can work out a plan tailored to this task (recommended: reply \"compose\"), " +
        `or you can pick one directly (e.g. ${examples}; see /aidlc --help for the full list).`,
    ));
    return;
```

## 5.8 分岐 9c — ワークフロー稼働中の自由記述 prose (`:3241-3261`)

```
  if (flags.intent && !flags.scope && !flags.positionalScope && !flags.resume) {
    const activeLabel =
      (getField(stateContent, "Project") ?? "").trim() ||
      (getField(stateContent, "Current Stage") ?? "").trim() ||
      "the active workflow";
    …
    const inferred = inferScopeFromText(flags.intent);
    emit(newWorkRoutingAskDirective(
      `Work is already in progress on: "${activeLabel}". You said: "${flags.intent}". ` +
        `Is this (1) part of that work - continue it; (2) a separate new piece of work - ` +
        `Yes, set it up alongside the current one as "${inferred.scope}" work without changing it; ` +
        "or (3) a change to how the remaining plan is shaped?",
      flags.intent,
      inferred.scope,
    ));
    return;
  }
```
`newWorkRoutingAskDirective` (`:618-631`) が付ける追加フィールド:
```
  return {
    kind: "ask",
    ask_type: "new-work-routing",
    response_route: "next",
    question,
    new_work_description: description,
    proposed_scope: proposedScope,
  };
```

参考: 分岐 6 (resume ask) の逐語 (`:3084-3087`):
```
    emit(askDirective(
      `An existing workflow was found${where}. How would you like to proceed? ` +
        "Resume from last checkpoint, redo the current stage, jump to a stage, or start fresh.",
    ));
```
(`where` = `currentSlug.length > 0 ? ` (currently at "${currentSlug}")` : ""`, `:3076`)

---

# 6. `handleResumeReport` の 4 択列挙文言 (`aidlc-orchestrate.ts:5383-5457`)

## 6.1 前置ガード 3 種
```
:5387-5392   "A resume-choice report is not a stage transition; omit --stage."
:5393-5398   "report --result resumed requires --user-input with the human's resume choice."
:5401-5406   "No active intent workflow state found (aidlc-state.md is absent) - nothing to resume."
:5408-5413   "State file has no Current Stage field - cannot resume from the last checkpoint."
```
(注: この 2 本のダッシュは **ASCII ハイフン `-`**。段 6 の同種文言 `:5551` は **em dash `—`** — 混在しているので実装時は要注意)

## 6.2 数字メニュー正規化 (`:5417-5424`)
```
  const numericChoices: Readonly<Record<string, string>> = {
    "1": "resume from last checkpoint",
    "2": "redo the current stage",
    "3": "jump to a stage",
    "4": "start fresh",
  };
  const rawChoice = flags.userInput.trim().toLowerCase();
  const choice = numericChoices[rawChoice] ?? rawChoice;
```

## 6.3 4 ルートの print 逐語 (判定は `choice.includes(...)` の順次判定)

| 順 | 述語 | 行 |
|---|---|---|
| 1 | `choice.includes("redo")` | `:5425-5431` |
| 2 | `choice.includes("jump")` | `:5432-5437` |
| 3 | `choice.includes("fresh") \|\| choice.includes("start over")` | `:5438-5443` |
| 4 | `choice.includes("resume") \|\| choice.includes("checkpoint") \|\| choice.includes("continue")` | `:5444-5453` |

```
      `Redo accepted at "${slug}". Run \`bun ${harnessDir()}/tools/aidlc-jump.ts execute --target ${slug} --direction redo --scope ${scope}\` to reset the current stage, then re-run \`next\` to start it over.`
```
```
      `Jump accepted. Ask the human which stage to jump to, then re-run \`next --stage <slug>\`; the direction and the target are worked out and checked for you.`
```
```
      "Start-fresh accepted. Confirm the new work's scope and description with the human, then run `next --new-intent --scope <scope> \"<description>\"` — the existing workflow stays in place and the new intent starts alongside it."
```
```
      `Resume choice accepted at "${slug}". Re-run \`next\` to continue from the last checkpoint.`
```

## 6.4 ★ 非該当時の 4 択列挙 error (`:5454-5456`)
```
  emit(errorDirective(
    `Unrecognized resume choice "${flags.userInput}". Accepted choices: 1/resume from last checkpoint, 2/redo the current stage, 3/jump to a stage, or 4/start fresh.`,
  ));
```
解決文字列:
```
Unrecognized resume choice "<user-input>". Accepted choices: 1/resume from last checkpoint, 2/redo the current stage, 3/jump to a stage, or 4/start fresh.
```

**注意点 (Quint/実装向け)**: `redo` の判定が `jump` より先なので、"redo by jumping" のような入力は redo に落ちる。また `--result` は `resume`/`resumed` どちらでもここへ来るが、エラー文言は `report --result resumed requires ...` と `resumed` に固定。

---

# 7. `classifyStateVersion` を report / next が呼ぶ位置と中継形式

## 7.1 中継アダプタ (`aidlc-orchestrate.ts:641-652`)

```
// State-schema-version guard. The classifier (aidlc-lib.ts
// `classifyStateVersion`) is the single source of truth for parsing and
// classifying `- **State Version**: N` lines; runtime (next/report) and doctor
// call it the same way so they can never disagree on whether a state is
// unparseable / past / future / ok. staleStateVersionError() is the runtime
// adapter: it returns the classifier's message on any incompatible verdict and
// null on `ok`, so next/report can emit the message as an errorDirective
// before any workflow-cursor read/advance.
function staleStateVersionError(stateContent: string): string | null {
  const verdict = classifyStateVersion(stateContent);
  return verdict.kind === "ok" ? null : verdict.message;
}
```
import は `aidlc-orchestrate.ts:165`。**中継形式は「verdict の kind を捨て、message 文字列だけを `errorDirective` に載せる」**。kind (`past`/`future`/`unparseable`) は directive に出ない。

## 7.2 `handleNext` 側の呼び出し位置 (`:2789-2803`)

```
  const pd = resolveProjectDir(projectDir);
  const stateContent = loadStateFileIfPresent(pd);
  // Runtime state-version guard (see staleStateVersionError): refuse to advance
  // a pre-v8 state up front rather than silently routing until it hits the
  // renamed/missing Inception rows. Fires after the workspace/plugin/compose
  // branches above (those are version-independent) and before any branch that
  // reads or advances the workflow cursor.
  // `!== null` (not truthiness): a PRESENT but zero-byte aidlc-state.md returns
  // "" and must still be refused (an empty version → missing/unparseable branch),
  // not skipped as if the file were absent.
  if (stateContent !== null) {
    const stale = staleStateVersionError(stateContent);
    if (stale) {
      emit(errorDirective(stale));
      return;
    }
  }
```
**位置**: 分岐 2 (`--stage`+`--phase`, `:2777-2784`) の直後、`recordPrefix` 解決 (`:2809`) と分岐 2.5 (`:2830`) の直前。すなわち **`resolveProjectDir` + `loadStateFileIfPresent` の直後、カーソルを読む最初の分岐の前**。分岐 0/1/1b/1c/1d/2 はこのガードより前に return するのでバージョン非依存。

## 7.3 `handleReport` 側の呼び出し位置 (`:5472-5488`)

```
  // Runtime state-version guard (see staleStateVersionError): `report` commits a
  // lifecycle transition, so a pre-v8 state must be refused here too — before any
  // report sub-branch mutates it. Covers every report path (result, skeleton
  // stance, single) via one early check.
  {
    const pd = resolveProjectDir(projectDir);
    const sc = loadStateFileIfPresent(pd);
    // `!== null` (not truthiness): a present but zero-byte state file returns ""
    // and must still be refused, not treated as an absent file.
    if (sc !== null) {
      const stale = staleStateVersionError(sc);
      if (stale) {
        emit(errorDirective(stale));
        return;
      }
    }
  }
```
**位置**: `parseReportFlags` (`:5465`) と `touchEngineMarker` (`:5470`) の直後、`--single` 分岐 (`:5496`) より**前**。ブロックスコープ `{ … }` で pd/sc を局所化しており、後段 (`:5545-5546`) で改めて `pd` / `stateContent` を取り直す (二重読み)。

→ **重要**: report では `--single` と `--skeleton-stance` より前に版ガードが走るため、**全 report 経路が版ガードを通る**。next では逆に read-only / workspace / plugin / knowledge 系はガードを通らない (state を読まないため)。B10 実装ではこの非対称性を保持すること。

## 7.4 分類器本体 (`aidlc-lib.ts:10604-10668`) — 逐語

```
/** The current state-graph schema version. Bump when the graph adds/renames/removes rows. */
export const CURRENT_STATE_VERSION = "8";

export type StateVersionClassification =
  | { kind: "ok" }
  | { kind: "unparseable"; message: string }
  | { kind: "past"; version: string; message: string }
  | { kind: "future"; version: string; message: string };
```
パース (`:10638-10641`):
```
  const versionMatch = stateContent.match(/^- \*\*State Version\*\*:[ \t]*(\S+)[ \t]*$/m);
  if (versionMatch === null) return { kind: "unparseable", message: unparseableMessage };
  const v = versionMatch[1];
  if (!/^\d+$/.test(v)) return { kind: "unparseable", message: unparseableMessage };
  if (v === CURRENT_STATE_VERSION) return { kind: "ok" };
```

3 メッセージ逐語:

**unparseable** (`:10628-10634`):
```
  const unparseableMessage =
    "Incompatible workflow state: the State Version field is missing, empty, " +
    "or unparseable in aidlc-state.md, so this state cannot be matched to the " +
    `current v${CURRENT_STATE_VERSION} stage graph and cannot be advanced safely. ` +
    "Archive your workspace ('mv aidlc aidlc.archive') and start a fresh " +
    "workflow (describe what to build), or finish this workflow on the prior " +
    "shell. Run `/aidlc --doctor` for the full diagnosis.";
```

**future** (`:10643-10653`):
```
  if (Number(v) > Number(CURRENT_STATE_VERSION)) {
    return {
      kind: "future",
      version: v,
      message:
        `Incompatible workflow state: State Version ${v} is newer than the ` +
        `current v${CURRENT_STATE_VERSION} stage graph this build understands, so ` +
        "it cannot be advanced safely. Upgrade the framework to a build that ships " +
        `state schema v${v} (or newer), or finish this workflow on the shell that ` +
        "produced it. Run `/aidlc --doctor` for the full diagnosis.",
    };
  }
```

**past** (`:10655-10667`):
```
  return {
    kind: "past",
    version: v,
    message:
      `Incompatible workflow state: State Version ${v} predates the current ` +
      `v${CURRENT_STATE_VERSION} stage graph. v8 renamed the Inception ` +
      "`application-design` stage to `domain-design` and inserted " +
      "`contract-design`, so this state's stage rows no longer match the graph " +
      "and cannot be advanced safely. Archive your workspace " +
      `('mv aidlc aidlc.v${v}-archive') and start a fresh workflow (describe what ` +
      "to build), or finish this workflow on the prior shell. Run `/aidlc --doctor` " +
      "for the full diagnosis.",
  };
```
→ **archive パスが unparseable と past で異なる**: `mv aidlc aidlc.archive` vs `mv aidlc aidlc.v<N>-archive`。研究文書 (report-guards §1 段 1) は unparseable だけを挙げていたが、past も archive を案内する。

---

# 付録 A: 研究文書との相違点 (訂正事項)

| # | 研究文書の記述 | ピン留めソース上の実際 |
|---|---|---|
| A1 | next-ladder §1「21 ラベルの正確な集合: `… 3b, 4, 4a, 4b, 4c, 5 …`」 | ソース上の**出現順は `… 3b, 4, 4c, 4a, 4b, 5 …`** (4c が 4a/4b より先)。集合としては一致するが順序は誤り。§5 の表側 (行 30-32) は正しい順序 |
| A2 | report-guards §6-5「floor 付きスキャン: 最新 `STAGE_STARTED`・それ以降の `GATE_REJECTED`・最新の関連 `produces[]` write より後の行のみ有効」 | 前 2 者は真の floorIdx (`aidlc-lib.ts:5073-5087`) だが、**produces[] write は floor ではなく順走査中の clear-on-write** (`:5114-5154`)。加えて floor には `WORKFLOW_STARTED` と `STAGE_JUMPED` (**stage 非依存**) も含まれ、`unit-major` では `STAGE_STARTED` がスキップされる。floor イベントは計 4 種 |
| A3 | report-guards §6「`verifyReviewerPrecondition` の拒否文は 4 種」 | **6 種** (§1.4 参照)。per-unit 集計文と、bolt DAG 解決不能文が追加。また 4 種のうち 3 種は recovery 消費有無で 2 腕に分かれる |
| A4 | report-guards §3.2「`in-progress` + gated → 明示 `--stage` 必須 (flowchart 上の文言 "error: report the acted directive explicitly")」 | 実文言は `:5868-5870` の 3 行連結 (§4.1 参照)。flowchart 表現ではなく実在の error message |
| A5 | report-guards §8「非該当回答は 4 択を列挙する `error`」 | 実文言は `Unrecognized resume choice "…". Accepted choices: 1/resume from last checkpoint, 2/redo the current stage, 3/jump to a stage, or 4/start fresh.` (§6.4) |
| A6 | report-guards §1 段 1「`unparseable` は archive (`mv aidlc aidlc.archive`) を案内」 | `past` も archive を案内するが**パスが異なる** (`mv aidlc aidlc.v<N>-archive`) |
| A7 | report-guards §9「forward verdict の遷移関数は (checkbox, gated, final, moved-on, explicit-stage) の 5 引数で決定的」 | moved-on は単独の状態ではなく `(slug == currentStage, checkbox[currentStage])` の合成。`final` 側の no-op は `Status` フィールド由来なので、正確には 7 引数 (§3 の表参照) |

# 付録 B: B10 (reviewer precondition) 実装への直接的含意

1. **監査台帳だけで完結する**。ファイル mtime は不要。`ARTIFACT_CREATED` / `ARTIFACT_UPDATED` の `File` フィールドと produces[] マッチングだけで invalidation を再現できる。
2. **イベント順序は (Timestamp 文字列比較, シャード連結上のブロック位置) の辞書順**。`isoTimestamp()` は秒精度なので、同秒はブロック位置で決着させる必要がある (`aidlc-lib.ts:5067`)。
3. **fingerprint は内容 sha256 の二段ハッシュ** (`reviewArtifactFingerprint`)。欠落は `"missing"`、非ファイルは `"not-file"` という**明示的マニフェストエントリ**として参加する — 「レビュー後にアーティファクトを新規作成する」ケースも receipt を無効化するため (`aidlc-lib.ts:4940-4944` の doc comment)。
4. **recovery は監査行フィールド `Recovery: stale-receipt` の 1 ビット**。カウンタも state フィールドも持たない。窓 (floorIdx 以降) が更新されればリセットされる。
5. **per-unit の曖昧パスは fail-closed** (`targetUnit === null` → 全ユニット receipt 破棄)。
6. **precondition は state 側 4 ハンドラに置く**。engine の report 側に置くと直接呼び出しで迂回される (`aidlc-orchestrate.ts:5878-5883`)。


## RESOLVED OPEN QUESTIONS
- ★ [B10 の要] verifyReviewerPrecondition の floor 第 3 要素「latest relevant produces[] write」の出所 → **監査行 (ARTIFACT_CREATED / ARTIFACT_UPDATED) 由来。ファイル mtime は一切使用していない**。aidlc-lib.ts:5116-5119 で監査ブロックの `File` フィールドを `producesArtifactUnit()` に通して判定する。grep 検証: `mtime|birthtime` は aidlc-state.ts:0 件、aidlc-orchestrate.ts:1 件 (ターンマーカーのコメント)、aidlc-lib.ts:31 件 (全てターンシェイプマーカー・audit lock 経年・hook マーカー鮮度窓であり review 経路とは無関係)。第 2 の鮮度軸である artifact fingerprint も mtime ではなく内容 sha256 の二段ハッシュ (aidlc-lib.ts:4946-4984)。
- ★ [構造訂正] 「floor 3 要素」というモデルは不正確。真の floorIdx は 4 イベント (WORKFLOW_STARTED / STAGE_JUMPED = stage 非依存、STAGE_STARTED = 当該 stage のみかつ unit-major ではスキップかつ single-stage: 除外、GATE_REJECTED = 当該 stage のみ / aidlc-lib.ts:5073-5087)。produces[] write は floor ではなく floorIdx+1 からの順走査中の clear-on-write (aidlc-lib.ts:5114-5154)。
- ★ recovery 1 回制限の状態追跡機構が確定 → state フィールドでもカウンタでもなく、REVIEW_REQUESTED 監査行の `Recovery: stale-receipt` フィールド 1 個 (aidlc-lib.ts:5174)。同一 requestKey (`${unit} ${iteration}`) の再 REQUEST でフラグが粘着 (:5172-5174)。recovery 付き review は iteration cap を免除される (:5201-5208)。receipt 無効化時に StaleReviewProgress.recoverySpent として持ち越される。窓 (floorIdx) が更新されるとリセット = 「Only a human Request Changes decision resets the review attempt」の実装的裏付け。reject 側では reviewRecoverySpentInCurrentAttempt (aidlc-state.ts:2039-2062) が autonomous モードでの human presence 強制に使われる。
- ★ verifyReviewerPrecondition の拒否文言は 4 種ではなく **6 種** → (1) reviewerPreconditionError (:2026-2037) (2) staleReviewPreconditionError else 腕 (:2013-2023) (3) 同 if 腕 = recovery 消費済み (:2004-2011) (4) staleSourcePreconditionError else 腕 (:1959-1966) (5) 同 if 腕 = recovery 消費済み (:1951-1958) (6) per-unit 欠落集計 + 3 種ガイダンス (:1935-1942 + :1905-1934)。付随して bolt DAG malformed (:1869-1873)。全て完全逐語を採取。
- ★ report 段 11 checkStageCompletionEvidence の全拒否文言 (研究文書で「詳細メッセージは仕様に逐語なし」だった項目) → **5 種**を完全採取: (11-a) pipeline link receipt 欠落 (:5155-5163) (11-b) per-unit unit list 解決不能 (:5170-5176) (11-c) paused-unit (:5184-5193) (11-d) per-unit カバレッジ不足 (:5207-5215) (11-e) ensemble contribution evidence (:5114-5122、missing 要素 2 形 :5101/:5105、contributionPath 2 形 :5111-5113)。
- ★ report 段 12 practices-discovery promotion receipt の拒否文言 (研究文書で「拒否文は仕様に逐語未収載」だった項目) → aidlc-orchestrate.ts:5777-5782 を完全採取。判定関数 hasFreshPracticesAffirmationReceipt (:4761-) の FLOOR_EVENTS は STAGE_STARTED / GATE_REJECTED / STAGE_REVISING の 3 種、ソートは (timestampMs, position)、タイムスタンプ検証は isConcreteIsoInstant の厳密 ISO regex (:4747-4752)。
- ★ stale re-report の moved-on 述語 (Quint モデル近似の検証) → **インデックス比較でも nextInScopeStage 走査でもない**。aidlc-orchestrate.ts:5849-5851 の checkbox 状態述語: `slug !== currentSlug` ∧ `checkboxForSlug(state, currentSlug)` が非 null ∧ `state !== "pending"`。グラフ順序を一切参照しない。checkbox 行が存在しなければ advance へ fail-open。final 側の冪等 no-op は別分岐 (:5829-5837) で `Status === "Completed"` 判定。
- ★ forward 表 #6 (in-progress + gated + 明示 --stage なし) の実文言 → aidlc-orchestrate.ts:5866-5870 の error。実シーケンスは明示 --stage ありなら `gate-start <slug> --recovered` + `approve <slug> [--user-input]` の 2 段 (:5876, :5884)。
- ★ gate-start --recovered の監査行での現れ方 → イベント種別は通常の STAGE_AWAITING_APPROVAL のまま、`Recovered: "true"` フィールドが 1 個追加されるだけ (aidlc-state.ts:2600-2604)。消費側の unrecordedRevisionSinceGateOpen では Recovered=true 行は決してアンカーにならない (:387, :405-407、理由コメント :314-320)。
- ★ next ラダー分岐 0 (Kiro ラッチ) 逐語 → 発火は 17 フラグ全未設定の「真に裸の next」(:2648-2651)。ラッチは aidlc/.aidlc-readonly-latch (JSON {turn,flag,source}) と aidlc/.aidlc-turn-counter。done reason 全文と label 整形規則 (noun command は バッククォート、read-only flag は `--` 前置、既定 "the read-only command") を採取 (:2656-2680)。
- ★ next ラダー分岐 1b/1c/1d の名詞トークン逐語 → 3 分岐とも同形だが、1b (workspace) のみ「terminal utility, NOT workflow work」の追伸が付かず、1b のみ argv 変換失敗の `Invalid workspace command.` 経路を持つ。1d は 1c と同文だがツール名が aidlc-knowledge.ts (:2711-2775)。
- ★ next ラダー分岐 4c compose print 本文 → composeDispatchDirective (:930-980) の in-flight 4 パーツ / front 1-3 パーツ / 共通末尾 2 パーツ (proposalShape, modeContract の 2x2 分岐含む) / narration 2 種を完全採取。
- ★ next ラダー分岐 4a (--new-intent) 逐語 → 空 description の error (:2969-2971)、createPrintDirective の newIntent 腕 (:899-905)、labelHint (:887-888)、narration (:912-914)、costClause (:679-686) を採取。
- ★ next ラダー分岐 5 (scope-change / config-change) 逐語 → 2 本の printDirective 全文と、同値 --scope を config-only 扱いする条件 (:3028-3064)。
- ★ next ラダー分岐 7b 逐語 → 発火 4 条件と intentPickPromptIfRecordsExist の ask 全文 (:1014-1019)、null 返却条件 (:1006-1007)。
- ★ next ラダー分岐 8 逐語 → keyword hit アームの ask 全文 (:3161-3164) と compose offer アームの ask 全文 + examples 生成規則 (:3170-3181)。
- ★ next ラダー分岐 9c 逐語 → ask 全文 (:3253-3256) と newWorkRoutingAskDirective の追加フィールド ask_type:"new-work-routing" / response_route:"next" / new_work_description / proposed_scope (:618-631)。
- ★ handleResumeReport の 4 択列挙文言 → 非該当時 error 全文 `Unrecognized resume choice "…". Accepted choices: 1/resume from last checkpoint, 2/redo the current stage, 3/jump to a stage, or 4/start fresh.` (:5454-5456)。加えて numericChoices マップ (:5417-5422)、4 ルートの print 全文、前置ガード 4 種を採取。判定順は redo → jump → fresh/start over → resume/checkpoint/continue。
- ★ classifyStateVersion を report/next が呼ぶ位置と中継形式 → 中継アダプタ staleStateVersionError (:649-652) が verdict.kind を捨て message のみを errorDirective に載せる。next: :2797-2803 (resolveProjectDir + loadStateFileIfPresent 直後、分岐 2.5 の直前)。report: :5476-5488 (touchEngineMarker 直後、--single 分岐より前)。**非対称性**: report は全経路が版ガードを通るが、next は read-only/workspace/plugin/knowledge/分岐 2 がガード前に return する。分類器本体 (aidlc-lib.ts:10627-10668) の 3 メッセージ全文と regex `/^- \*\*State Version\*\*:[ \t]*(\S+)[ \t]*$/m` を採取。CURRENT_STATE_VERSION = "8"。
- [副産物] archive パスは unparseable (`mv aidlc aidlc.archive`) と past (`mv aidlc aidlc.v<N>-archive`) で異なる。研究文書は unparseable のみを記載していた。
- [副産物] next-ladder 研究文書 §1 の「21 ラベルの正確な集合」の列挙順が誤り。ソース出現順は `0 1 1b 1c 1d 2 2.5 2.6 3b 4 4c 4a 4b 5 6 7 7b 8 9 9c 10` で、4c が 4a/4b より先。同文書 §5 の表側は正しい順序で記載されている。
- [副産物] verifyReviewerPrecondition の呼び出し 4 箇所を特定: handleAdvance (:2202)、handleFinalize (:2326)、handleCompleteWorkflow (:2452)、handleApprove (:2783)。前 3 者は第 4 引数 requireReceiptExistence に `!alreadyMarkedCompleted` を渡し、handleApprove は省略 (default true)。

## VERIFIED COUNTS
- [一致] 02 Measurement notes L523「21 labelled branches in handleNext」: `sed -n '2587,3357p' core/tools/aidlc-orchestrate.ts | grep -cE '^  // Branch [0-9]+(\.[0-9]+)?[a-z]? [—-]'` → **21** (期待値 21)。ラベル集合も一致: 0, 1, 1b, 1c, 1d, 2, 2.5, 2.6, 3b, 4, 4c, 4a, 4b, 5, 6, 7, 7b, 8, 9, 9c, 10。**ただし出現順は 4c が 4a/4b より先**で、Measurement notes の列挙順 (4, 4a, 4b, 4c) とは異なる (集合は同一)。
- [一致] 02 Measurement notes L532「4 per-unit / per-batch gate suppressions」: `grep -n 'directive.gate = false' core/tools/aidlc-orchestrate.ts` → **4139, 4198, 4356, 4486** (期待値 4139/4198/4356/4486、完全一致)。
- [一致] 02 Measurement notes L522「4 engine subcommands」: `sed -n '6125p;6149p'` → `const commandKind = (["next", "continue", "report", "park"] as const)...` / `` `Unknown subcommand: ${subcommand ?? "(none)"}. Valid: next, continue, report, park` `` (期待値と完全一致)。
- [一致] 02 Measurement notes L531「DIRECTIVE_MAX_BYTES = 28 KiB」: `sed -n '1140,1143p'` → `28 * 1024`, `20 * 1024`, `6 * 1024`, `8 * 1024` (期待値と完全一致)。
- [一致] 02 Measurement notes L533「The conductor-persona decision comment is unique」: `grep -c 'Decision D-E' core/tools/aidlc-orchestrate.ts` → **1** (期待値 1、行 2132)。
- [一致] 02 Measurement notes L520「present-gate / dispatch-subagent are never constructed」: `grep -c 'present-gate|dispatch-subagent' core/tools/aidlc-orchestrate.ts` → **1** (期待値: 1 hit、行 1032、コメント内)。
- [一致] 02 Measurement notes L519「8 kinds constructed by the engine」: `grep -o 'kind: "[a-z-]*"' | sort | uniq -c | sort -rn` → error 15, done 7, load-steering 2, invoke-swarm 2, ask 2, run-stage 1, print 1, parked 1, not-plugin 1, not-knowledge 1 → directive kind は 8 種 (期待値と完全一致。件数もすべて一致)。
- [一致] 02 Measurement notes L524「10 accepted report --result outcomes」: REPORT_RESULTS = FORWARD ∪ GATE ∪ RESUME ∪ {skipped} を定義 (aidlc-orchestrate.ts:4736-4745) から再構成した文字列 → `report requires --result <outcome>. Accepted: approved, completed, complete, done, awaiting-approval, rejected, revised, resume, resumed, skipped (the verdict for the stage just acted on).` (期待値の逐語と完全一致、順序も一致)。
- [一致] 02 Measurement notes L516「Line counts in §1.1」: `wc -l core/tools/aidlc-orchestrate.ts` → **6169** (期待値 6169)。
- [新規検証・一致] aidlc-state.ts 行数 = **4278**。SHA-256 = 6d4488b95f26813f23a2c7051fd631f4ff88689af9801562805be8bf25d45fd1。aidlc-orchestrate.ts SHA-256 = 581519ed4a1d58254b0ffbafd3520e446dde741cae21c4f3539ff8f2db765a22。
- [新規検証・B10 決定的] mtime 不使用の確認: `grep -c 'mtime|mtimeMs|birthtime'` → aidlc-state.ts **0 件**、aidlc-orchestrate.ts **1 件** (行 586、ターンシェイプマーカーのコメント)、aidlc-lib.ts **31 件**。lib の全ヒットを目視し、(a) .aidlc-human-turn vs .aidlc-engine-touch 比較 (:6084)、(b) audit lock ディレクトリ経年 (:6939, :6952-6964)、(c) active-directive / hook マーカー鮮度窓 (:6103, :6121) のみと確認。**review freshness / produces[] invalidation 経路に mtime は 0 件**。
- [新規検証] verifyReviewerPrecondition の呼び出し数: `grep -n 'verifyReviewerPrecondition' core/tools/aidlc-state.ts` → 定義 1 (:1775) + 呼び出し 4 (:2202, :2326, :2452, :2783) = **5 ヒット** (04 §5.5 の「completion ハンドラ 4 種すべてが実装」と一致)。
- [新規検証] freshReviewReceipts の RELEVANT イベント集合 = 8 種 (WORKFLOW_STARTED, STAGE_STARTED, STAGE_JUMPED, GATE_REJECTED, ARTIFACT_CREATED, ARTIFACT_UPDATED, REVIEW_REQUESTED, REVIEW_COMPLETED) — aidlc-lib.ts:5050-5059。うち floorIdx を動かすのは **4 種** (WORKFLOW_STARTED, STAGE_JUMPED, STAGE_STARTED, GATE_REJECTED / :5073-5087)。研究文書の「3 要素 floor」は不正確。
- [新規検証] hasFreshPracticesAffirmationReceipt の FLOOR_EVENTS = **3 種** (STAGE_STARTED, GATE_REJECTED, STAGE_REVISING) — aidlc-orchestrate.ts:4772-4776。研究文書 §1 段 12 の記載「最新の STAGE_STARTED/GATE_REJECTED/STAGE_REVISING floor イベント」と一致。
- [新規検証] `Recovery` フィールド値 `stale-receipt` の参照は aidlc-lib.ts:5174 の **1 箇所のみ** (grep 'stale-receipt' 全ヒット確認: aidlc-lib.ts 1 件、aidlc-state.ts 6 件は全てエラーメッセージ文言またはコメント)。
- [新規検証] `Recovered` フィールドの書き込み箇所: aidlc-state.ts:2603 (handleGateStart --recovered)、:2755, :2763, :2767 (approve 側 GATE_REJECTED+STAGE_REVISING 補填)、:2978。読み出し: :387 (unrecordedRevisionSinceGateOpen)、:405 (アンカー除外条件)。
- [新規検証] 仕様が引用する行番号の実ソース照合 — すべて一致: checkStageCompletionEvidence = :5128-5230 (完全一致)、practices 拒否 = :5772-5784 (完全一致)、stale re-report guard = :5842-5859 (コメント :5842 開始、実コード :5849-5859、完全一致)、handleResumeReport = :5383-5457 (完全一致)、Branch 0 = :2635-2681 (完全一致)、Branch 1b/1c/1d = :2711-2775 (完全一致)、verifyReviewerPrecondition 本体 = :1775-1944 (タスク指示の :1763-2030 付近と整合: コメント :1753 開始、ヘルパー :2037 終了)。
- [未検証・スコープ外] 02 Measurement notes L529 の「33 compiled stages / 30 non-initialization / 5 per-unit」、L530 の「30 generated stage-runner skills」、L516 の他ファイル行数、L517/L518/L521/L525-528 の実行系プローブは dist/ ツリーおよび stage-graph.json / aidlc-directive.ts を要するため本タスクの担当範囲外。再実行していない。
