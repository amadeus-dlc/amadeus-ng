> 抽出元: upstream as-built 仕様 (awslabs/aidlc-workflows v2 @ 3c3146cf, v2.6.40)。2026-08-22 の 3 エージェント精密抽出。11-workspace.md と audit_lock.qnt の執筆材料。

以下、`03-state-audit-runtime.md`(全 1268 行、以下「03」)全文と `07-hooks.md`(以下「07」)の精読に基づく完全列挙です。典拠は「03 §節 L 行番号」(as-built 仕様の節・行) と、仕様が引く upstream コードサイト (`file.ts:line`) の両方を付します。逐語契約は原文のまま引用します。

# タスク 2: 監査台帳の契約の完全列挙

対象ファイル: `/Users/j5ik2o/orca/workspaces/amadeus-ng/docs/docs/upstream/specs/03-state-audit-runtime.md` (主典拠)、`/Users/j5ik2o/orca/workspaces/amadeus-ng/docs/docs/upstream/specs/07-hooks.md` (フック発行イベント)

---

## 1. Audit shard — パス形式・ブロック文法・追記専用・書込ガード

### 1.1 パス形式

| 項目 | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| shard パス | `<record>/audit/<host>-<clone-id>.md` | 03 §6.3 L700-702 |
| 書込先解決 | `auditFilePath(projectDir, intent?, space?)`。intent 未解決時は `<space>/intents/audit/<shard>` にフォールバック | 03 §6.3 L704-706 / `aidlc-lib.ts:3668` |
| shard 名合成 | `` `${host}-${cloneId(projectDir)}.md` ``。`host` = `os.hostname()` を小文字化、`[a-z0-9-]` 以外の連続を `-` に潰し、trim、**48 文字上限**、空なら `"host"` | 03 §3.3 L204-206 / `aidlc-lib.ts:4499` |
| shard 列挙 | `auditShards(projectDir, intent?, space?)`: (a) `undefined intent + 明示 space` 形式は **space レベル shard を先頭に前置** (DocumentKB provenance / doctor 用)、(b) 解決済み intent の shard は末尾、(c) intent が全く解決しないときは space shard *が* 台帳 (pre-birth の read/write parity — 省略時に 10 fixture suite が壊れた旨のコメント付き)。`*.md` のみ返し、各 shard dir はシンボリックリンク連鎖検査済み | 03 §6.3 L708-716 / `aidlc-lib.ts:4530`, `:4523-4528` |
| shard dir | `auditShardDir` は intent 未解決で `null` → bare space の列挙子は `[]` | 03 §6.3 L707-708 / `aidlc-lib.ts:4512` |
| 全 shard 読取 | `readAllAuditShards`: shard 内容を `\n` で連結。各 shard は `readAppendOnlyFileNoFollowOrThrow` 経由。消失/拒否 shard はスキップ。*"growth during the read is explicitly not a failure"* (読取中の成長は失敗ではない — 生きている台帳をマージから落とさない) | 03 §6.3 L714-717 / `aidlc-lib.ts:4568`, `:7521` |
| space レベル shard の義務 | 3 つの `DOCUMENT_*` イベントは、文書が intent スコープでも **space レベル shard** (`spaces/<space>/intents/audit/`) が必須の置き場。理由: 文書は単一 intent より長命で `associate`/`dissociate` がスコープを移せる | 03 §6.3 L719-722 / `audit-format.md:160,168-173`; `aidlc-audit.ts:117-120` |
| 例外的パス合成 | `appendAuditEntryAtPathUnlocked` は DocumentKB がその shard パスを自前合成するためだけに存在 — 通常解決ではそのパスを*要求できない* | 03 §6.3 L722-724 / `aidlc-audit.ts:751`, `:581-594` |

### 1.2 Markdown ブロック文法 (JSONL ではない)

- **前提の訂正 (仕様冒頭)**: *"The audit log is not JSONL. It is a Markdown block stream (`## Heading` / `**Field**: value` / `---`)"* (03 §1 L25-26)。
- 空ファイルへの初回書込はヘッダ `# AI-DLC Audit Log\n` を出力 (03 §6.1 L642-643 / `aidlc-audit.ts:693`)。
- 各イベントは `renderAuditBlock` (`aidlc-audit.ts:485`) が描画するブロックとして追記 (03 §6.1 L643-652):

```text
\n## <Heading>\n
**Timestamp**: <ISO 8601, second precision>\n
**Event**: <EVENT_TYPE>\n
**<Key>**: <value>\n      (repeated)
\n---\n
```

- 具体例 (03 §6.1 L656-664、逐語):

```text
## Stage Completion
**Timestamp**: 2026-08-21T09:14:07Z
**Event**: STAGE_COMPLETED
**Stage**: requirements-analysis
**Details**: Stage Requirements Analysis completed

---
```

- Heading は `EVENT_HEADINGS` (`aidlc-audit.ts:192`) から、無ければ生イベント名にフォールバック。読み手は `\n---\n` で分割 (`findAllEvents`, `aidlc-lib.ts:7767`) (03 §6.1 L666-668)。
- 構造化 emitter 以外に、`audit-format.md` は `### Error Format` (`:301`) と `### Recovery Format` (`:313`) の 2 つの自由prose ブロック形を文書化 — `append-raw` CLI 経由でのみ到達 (03 §6.1 L670-672)。

### 1.3 フィールド検証と行終端エスケープ

`validateAuditEntry` (`aidlc-audit.ts:463`) の 3 条件 (03 §6.2 L675-684):

| # | 検証 | エラー/根拠 (逐語) | 典拠 |
| --- | --- | --- | --- |
| 1 | イベント型が `VALID_EVENT_TYPES` に含まれる | `Invalid event type: <x>. Must be one of: <full list>` | 03 §6.2 L677-678 |
| 2 | フィールドキーが `RESERVED_FIELD_KEYS = {"Event"}` に含まれない | 呼出側供給の `Event` は第二の `**Event**:` 行を描画し *"forge a second matching line and spoof multiline event queries"* (第二の一致行を偽造し複数行イベントクエリを詐称する) | 03 §6.2 L679-681 / `aidlc-audit.ts:452`, `:472-473` |
| 3 | 各キーが `AUDIT_FIELD_KEY_PATTERN = /^[A-Za-z][A-Za-z0-9 ._()/-]*$/` に一致 | *"remain[s] one Markdown label on one physical line"* (1 物理行上の 1 Markdown ラベルに留まる) | 03 §6.2 L682-684 / `aidlc-audit.ts:461` |

- `EMITTER_OWNED_FIELD_KEYS = {"Timestamp","Event"}` (`aidlc-audit.ts:460`) は描画時にスキップ。非対称は意図的 (`:444-451`): `Timestamp` は互換のため公開 CLI が*受理*するが値は捨てられる — emitter 自身の `**Timestamp**:` 行が先に書かれ、全パーサは最初の一致を取るため。`audit-format.md:16-23` は同契約を述べ、旧版由来の重複 timestamp フィールドが歴史的 shard に残りうると警告 (03 §6.2 L686-690)。
- **行終端エスケープ** (逐語、03 §6.2 L692-696): `const safeValue = String(value).replace(/\r\n?|\n|\u2028|\u2029/g, "\\n");` (`aidlc-audit.ts:499`) — *"so a malicious or malformed input cannot forge a second audit field or event line."* クラスが `\u2028`/`\u2029` を含むのは、両者が (多くの Markdown 読取器では通常文字でも) JS の行終端子だから。

### 1.4 追記経路のガード (`appendAuditBlockAtPath`)

`appendAuditBlockAtPath` (`aidlc-audit.ts:615`) は台帳へ**追記する唯一の関数** (03 §6.7 L843-846)。防御手順 6 段 (03 §6.7 L847-859):

| # | ガード | 契約 (逐語) | コードサイト |
| --- | --- | --- | --- |
| 1 | 封じ込め検査 | shard のプロジェクト相対パスが `""`、`".."`、`../` 開始、絶対パスのいずれでもないこと。違反時 `Refusing audit shard outside project: <p>` | `aidlc-audit.ts:625-627` |
| 2 | シンボリックリンク連鎖拒否 | `assertNoSymlinkInChainOrThrow` を親の `mkdir -p` の**前後**で実行 | `:628-630` |
| 3 | open フラグ | `O_RDWR \| O_APPEND \| O_CREAT \| O_NOFOLLOW \| O_NONBLOCK`、mode `0o666` | `:634-642` |
| 4 | 正規ファイル検査 | `fstat` が regular file を報告しなければ `Refusing non-regular audit shard: <p>` | (同節) |
| 5 | 記述子同一性再検証 | `verifyPathStillNamesDescriptor()` — シンボリックリンク不在の再表明、再 `realpath`、封じ込め再検査、`dev`/`ino` が open 済み記述子と一致し続けること。書込の**前後両方**で実行 — 書込中 rename は、もはや発見不能な行を報告するのではなく、囲んでいる audit-first トランザクションを失敗させる | `:677-690` |
| 6 | 部分書込ループ | `writeAll` は partial write でループし、0 バイト書込で `Audit append made no write progress` を throw | `:599` |

**nlink の非対称** (03 §6.7 L860-866):
- 通常追記経路では `nlink != 1` を**拒否しない**。理由 (`:645-652`): `rsync --link-dest` / `cp -al` スナップショットが生きた shard を `nlink 2` にし、拒否は *"bricked every later gate/hook append framework-wide"* (以後の全 gate/hook 追記をフレームワーク全体で文鎮化した)。ハードリンクは検査済みパス内の同一 inode の別名でありリダイレクトを与えない。
- 明示的 fork/merge 経路は厳格のまま: `readAuditSnapshot` (`:705`) は多重リンク shard を拒否 (`:719-721`)、`verifyExpectedPrefix` (`:657`) はマージ追記中に `nlink` と期待プレフィクスの SHA-256 を再検査。

### 1.5 追記専用の規律と例外 (03 §6.10 L990-1017)

| 規則 | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| フォーマット標準 | *"Append-only — NEVER modify or delete existing entries."* さらに ISO-8601 timestamp、credentials/PII 禁止、*"Human decisions recorded verbatim — NEVER summarize"* | `audit-format.md:284`, `:286` / 03 L992-994 |
| 構造的保証 | **main** intent shard を書き換えるコードパスは存在しない。全ての in-place 台帳書込は `appendAuditBlockAtPath` 経由で、`O_APPEND` open のみ・append のみ (`writeAll` → `writeSync`, `aidlc-audit.ts:603`) | 03 L995-997 |
| **例外: worktree ミラー shard** | `audit-fork` がこれをファイル全体の `writeBufferAtomic` tmp+rename で*確立*する (`aidlc-audit.ts:1252`; helper `aidlc-lib.ts:7260-7281` — `openSync(tmp, "wx")` → `writeFileSync` → `renameSync`)。create-if-absent ではなく既存 worktree shard を*置換*しうる (§6.9 参照)。fork 境界書込後はその shard への以後の書込も再び append、main への merge back は delta のみ append | 03 L998-1003 |
| バイト書込の完全集合 | `aidlc-audit.ts` 内のバイト書込は 3 呼出サイトが全て: `:603` (`writeSync`、append 経路)、`:1239` (`writeBufferAtomic`、clone-id トークン — 台帳ではない)、`:1252` (`writeBufferAtomic`、worktree shard) — M15 | 03 L1004-1006, L1259-1265 |
| 読取ガード | `readAppendOnlyFileNoFollowOrThrow` (`aidlc-lib.ts:7521`) は symlink (`<what> is a symlink, which is not followed: <p>`)、非正規ファイル、パス→記述子同一性不一致 (`<what> changed while opening: <p>`) を拒否 — ただし成長は許容 (生きた台帳は読取下で成長するのが期待) | 03 L1007-1010 |
| MEMORY_EMPTY 再発行 | runtime graph 再コンパイルは `MEMORY_EMPTY` 行を重複排除せず再発行。doctor はレート計算時に `(Stage, ISO-second)` で重複排除 | 03 L1011-1013 / `aidlc-runtime.ts:20-23` |
| git マージ戦略の否定的決定 | *"there is intentionally NO .gitattributes merge=union, which was proven to corrupt the multi-line audit blocks"* (意図的に merge=union なし — 複数行 audit ブロックを破損させると実証済み) | 03 §3.4 L253-255 / `dot-gitignore:70-71` |

---

## 2. Clone id

| 項目 | 契約 | 典拠 |
| --- | --- | --- |
| 定数 | `export const CLONE_ID_FILE = ".aidlc-clone-id";` | 03 §3.3 L194-196 / `aidlc-lib.ts:3681` |
| パス | `cloneIdPath(projectDir)` = `aidlc/.aidlc-clone-id` | 03 L198 / `aidlc-lib.ts:3683` |
| 形式 | 検証 regex `/^[a-z0-9]{1,32}$/`。欠如時は `randomUUID()` から **12 hex 文字**を鋳造・永続化、その後 **re-read** して並行初回鋳造が単一のオンディスクトークンに収束。値はプロセスごとに memoise。書込不能 workspace はインメモリトークンに劣化 | 03 L198-202 / `aidlc-lib.ts:3700` |
| gitignore | `aidlc/.aidlc-clone-id` は ignore glob の 1 つ (11 glob 中) | 03 §3.4 L219-231 / `dot-gitignore:34-63` |
| **machine-local が本質である理由** (逐語) | コメント `aidlc-lib.ts:3675-3680`: トークンは gitignore され *"so it never travels in a commit — that is what makes the token DISTINCT across clones (a fresh checkout has no token file and mints its own)"*。これが並行 audit append 上の git マージ衝突を除去する | 03 L208-211 |
| gitignore 側の理由 (逐語) | *"it MUST stay machine-local (gitignored) or every clone from a commit would share a shard and git-conflict"* | 03 §3.4 L243 / `dot-gitignore:38-39` |
| **fork/merge でのスレッディング** | `worktreeAuditFilePath` は **main の** `projectDir` を取り、shard 名に main clone のトークンを埋め込む (worktree が自分で鋳造したものではない)。逐語: *"the fork and merge subprocesses are both spawned from the main checkout, so threading the main clone-id makes them resolve the SAME worktree shard across the two PIDs"* (`aidlc-lib.ts:6198-6203`)。さらに `audit-fork` は clone-id トークンファイルを worktree にコピーし (`aidlc-audit.ts:1232-1239`)、worktree ローカルのツールが merge の消費する shard へ追記するようにする | 03 §4.5 L426-432 |

---

## 3. イベント語彙 — 86 イベント / 22 カテゴリ

### 3.1 構造と検証

| 項目 | 契約 | 典拠 |
| --- | --- | --- |
| 権威集合 | `VALID_EVENT_TYPES` (`aidlc-audit.ts:39-189`) = **86** イベント名。`EVENT_HEADINGS` (`:192-279`) は 86 全てに heading を持ち集合差は双方向とも空 (M2/M3) | 03 §6.5 L762-764 |
| ドキュメント同期 | `core/knowledge/aidlc-shared/audit-format.md` は同じ 86 を **22 カテゴリ見出し**で文書化しコードと厳密一致 (M4/M5/M9)。ドリフトガードは `tests/unit/t28-audit-event-sync.test.ts` (出荷バイトから両集合を抽出し、語彙を再宣言せずに関係を assert) | 03 L764-767 |
| 命名規約 (逐語) | `SUBJECT_PAST_VERB` — *"every event answers 'what happened?'"* | 03 L769-770 / `audit-format.md:14` |
| 新イベント発明禁止 (逐語) | `audit-format.md:3`: *"Event names MUST match this table exactly. Do not invent new event types. For stage completions, ALWAYS use `STAGE_COMPLETED` — do not substitute stage-specific names like \"Requirements Analysis Complete\" or \"Code Generated\"."* | 03 L801-803 |

### 3.2 全 22 カテゴリ・86 イベント (03 §6.5 L772-795 の表を完全転記)

| カテゴリ | n | イベント |
| --- | ---: | --- |
| Workflow Lifecycle | 4 | `WORKFLOW_STARTED` `WORKFLOW_COMPLETED` `WORKFLOW_PARKED` `WORKFLOW_UNPARKED` |
| Phase Lifecycle | 4 | `PHASE_STARTED` `PHASE_COMPLETED` `PHASE_VERIFIED` `PHASE_SKIPPED` |
| Stage Lifecycle | 6 | `STAGE_STARTED` `STAGE_AWAITING_APPROVAL` `STAGE_REVISING` `STAGE_COMPLETED` `STAGE_JUMPED` `STAGE_SKIPPED` |
| Session (hook-owned) | 5 | `SESSION_STARTED` `SESSION_RESUMED` `SESSION_COMPACTED` `SESSION_ENDED` `HUMAN_TURN` |
| Initialization | 3 | `WORKSPACE_SCAFFOLDED` `WORKSPACE_SCANNED` `WORKSPACE_INITIALISED` |
| Navigation | 7 | `SCOPE_CHANGED` `PLUGIN_SELECTION_CHANGED` `DEPTH_CHANGED` `TEST_STRATEGY_CHANGED` `REVIEW_CLASS_CHANGED` `SCOPE_DETECTED` `RECOMPOSED` |
| Interaction | 8 | `DECISION_RECORDED` `GATE_APPROVED` `GATE_REJECTED` `QUESTION_ANSWERED` `SUMMARY_CONFIRMATION_RECORDED` `REVIEW_REQUESTED` `REVIEW_COMPLETED` `PIPELINE_LINK_COMPLETED` |
| Unit Lifecycle | 4 | `UNIT_STARTED` `UNIT_PAUSED` `UNIT_RESUMED` `UNIT_COMPLETED` |
| Artifact | 3 | `ARTIFACT_CREATED` `ARTIFACT_UPDATED` `ARTIFACT_REUSED` |
| Subagent | 1 | `SUBAGENT_COMPLETED` |
| Reviewer Enforcement | 2 | `REVIEWER_SCOPE_BLOCKED` `REVIEW_FREEZE_BLOCKED` |
| Plan Approval | 1 | `PLAN_APPROVAL_BLOCKED` |
| Documents | 3 | `DOCUMENT_INDEXED` `DOCUMENT_UPDATED` `DOCUMENT_REMOVED` |
| Utility | 1 | `HEALTH_CHECKED` |
| Error/Recovery | 2 | `ERROR_LOGGED` `RECOVERY_COMPLETED` |
| Construction Bolt | 4 | `BOLT_STARTED` `BOLT_COMPLETED` `BOLT_FAILED` `AUTONOMY_MODE_SET` |
| Worktree | 7 | `WORKTREE_CREATED` `WORKTREE_MERGED` `WORKTREE_DISCARDED` `STATE_FORKED` `STATE_MERGED` `AUDIT_FORKED` `AUDIT_MERGED` |
| Practices | 4 | `PRACTICES_DISCOVERED` `PRACTICES_AFFIRMED` `PRACTICES_OVERRIDE` `PRACTICES_SECTION_EMPTY` |
| Merge Dispatch | 3 | `MERGE_DISPATCH_INVOKED` `MERGE_DISPATCH_RETURNED` `MERGE_DISPATCH_FALLBACK` |
| Sensor | 5 | `SENSOR_FIRED` `SENSOR_PASSED` `SENSOR_FAILED` `SENSOR_BUDGET_OVERRIDE` `GUARDRAIL_LOADED` |
| Learning Loop | 3 | `MEMORY_EMPTY` `RULE_LEARNED` `SENSOR_PROPOSED` |
| Swarm | 6 | `SWARM_STARTED` `SWARM_UNIT_CONVERGED` `SWARM_UNIT_FAILED` `SWARM_BATON_RETURNED` `SWARM_COMPLETED` `SWARM_DEGRADED` |

検算: 4+4+6+5+3+7+8+4+3+1+2+1+3+1+2+4+7+4+3+5+3+6 = **86**、カテゴリ数 = **22**。

### 3.3 MANDATORY 8 (03 L797-799, M6)

レジストリで `✓` 印の 8 イベント: `WORKFLOW_STARTED`, `WORKFLOW_COMPLETED`, `WORKFLOW_PARKED`, `WORKFLOW_UNPARKED`, `PHASE_STARTED`, `PHASE_COMPLETED`, `STAGE_STARTED`, `STAGE_COMPLETED`。

### 3.4 呼出側 Event キー供給禁止の理由

§1.3 の検証 2 のとおり: 呼出側が `Event` フィールドを供給すると第二の `**Event**:` 行が描画され、*"forge a second matching line and spoof multiline event queries"* — 複数行イベントクエリの詐称が可能になるため `RESERVED_FIELD_KEYS = {"Event"}` で拒否 (03 §6.2 L679-681 / `aidlc-audit.ts:452`, `:472-473`)。`Timestamp` は `EMITTER_OWNED_FIELD_KEYS` として受理はするが値は無視 (§1.3 参照)。

### 3.5 フック発行イベントの対応 (07 参照。仕様 03 §8 L1162 が 07 に委譲する部分)

| イベント | 発行フック | 典拠 (07) |
| --- | --- | --- |
| `SESSION_STARTED` / `SESSION_RESUMED` | `aidlc-session-start.ts` (SessionStart)。写像: `startup → SESSION_STARTED`, `clear → SESSION_STARTED`, `resume → SESSION_RESUMED`, `malformed → SESSION_STARTED`, `compact`/`unknown` → **発行なし** ("firing it twice would pollute the audit trail") | 07 §4.1 L117 |
| `SESSION_ENDED` | `aidlc-session-end.ts` (SessionEnd)。`Reason` フィールド付き、帰属は fail-closed | 07 §4.4 L145 |
| `SESSION_COMPACTED` | `aidlc-validate-state.ts` (PreCompact)。`Current Stage`・`State Validity` (`valid`/`invalid`) 付き | 07 §4.3 L141 |
| `HUMAN_TURN` | `aidlc-record-human-turn.ts` (UserPromptSubmit と PostToolUse `AskUserQuestion` の 2 seam)。`appendAuditEntry("HUMAN_TURN", {}, projectDir)` — payload なし。presence-only ("the prompt text is irrelevant, so stdin is not read") | 07 §6 L231-238 |
| `ARTIFACT_CREATED` / `ARTIFACT_UPDATED` | `aidlc-write-audit-log.ts` (PostToolUse `Write\|Edit`)。`Edit` は常に UPDATED; `Write` は `\|mtimeMs − birthtimeMs\| < 10` で CREATED、それ以外 UPDATED、stat 失敗は CREATED | 07 §4.2 L128 |
| `SUBAGENT_COMPLETED` | `aidlc-log-subagent.ts` (SubagentStop)。`Agent Type`、任意 `Agent ID`、200 字切詰め `Message` | 07 §4.2 L132 |
| `REVIEW_FREEZE_BLOCKED` | `aidlc-review-freeze.ts` — `Tool`, `Target`, `Stage`, 任意 `Unit` | 07 §5.3 L205 |
| `REVIEWER_SCOPE_BLOCKED` | `aidlc-reviewer-scope.ts` | 07 §5.4 L223 |
| `PLAN_APPROVAL_BLOCKED` | `aidlc-plan-approval-guard.ts` — `Tool`, `Target`, `Stage`, `Unit` (無ければリテラル `(missing marker)`) | 07 §5.2 L185 |
| ガード類の発行方式 | blocking ガードは拒否行を `appendAuditEntryUnlocked` + `acquireAuditLock(projectDir, 5, 50)` (5 回 × 50 ms — 標準予算より遥かに小さい) で発行。*"a dropped advisory row is preferable to a slow block"* | 07 §3 L85 |

---

## 4. Authority classes — 3 つの deny-list (03 §6.6 L805-839)

| 集合 | n | 意味 | 拒否点 | サイト |
| --- | ---: | --- | --- | --- |
| `CLI_RESERVED_EVENT_TYPES` | 8 | `main` 内での**パース前拒否** — どの emit 経路よりも前 | 公開 audit CLI 全体 | `aidlc-audit.ts:292` / 03 L811 |
| `CLI_PROTECTED_EVENT_TYPES` | 18 | `handleAppend` が拒否。**バイパス env: `AIDLC_ALLOW_DIRECT_AUDIT_EVENTS=1`** | `append` サブコマンド | `aidlc-audit.ts:348` / 03 L812; env は 03 §2.3 L111 / `aidlc-audit.ts:432` |
| `MERGE_PROTECTED_EVENT_TYPES` | 26 (+ 全 `DOCUMENT_*` を prefix で) | worktree delta で運ばれてはならない | `audit-merge` の `validateMergeDelta` | `aidlc-audit.ts:395`, prefix 規則 `:426-429` / 03 L813 |

**CLI_PROTECTED 18 の内訳** (03 L815-819、prose 列挙で 18 個が完結):

| 群 | イベント |
| --- | --- |
| human authority (5) | `HUMAN_TURN` `GATE_APPROVED` `GATE_REJECTED` `QUESTION_ANSWERED` `AUTONOMY_MODE_SET` |
| reviewer/pipeline receipts (4) | `REVIEW_REQUESTED` `REVIEW_COMPLETED` `PIPELINE_LINK_COMPLETED` `ARTIFACT_REUSED` |
| swarm attempt/convergence (2) | `SWARM_STARTED` `SWARM_UNIT_CONVERGED` |
| unit receipts (4) | `UNIT_STARTED` `UNIT_PAUSED` `UNIT_RESUMED` `UNIT_COMPLETED` |
| documents (3) | `DOCUMENT_INDEXED` `DOCUMENT_UPDATED` `DOCUMENT_REMOVED` |

**拒否メッセージ (逐語)**:

- CLI_PROTECTED (03 L820-821):
  > `Direct emission of <E> is blocked: it is an authority-bearing receipt owned by its emitting tool or hook (gate resolutions and approvals come from aidlc-orchestrate.ts report, interview answers and reviews from aidlc-log.ts, human presence from the prompt-submit hook). The audit CLI appends diagnostic events only.`
- CLI_RESERVED (03 L823-825):
  > `<E> is reserved for its owning hook/tool and cannot be appended through the public audit CLI.`
- MERGE_PROTECTED (merge 時、03 §6.9 L974): `worktree audit delta contains protected authority event <E>`

**MERGE_PROTECTED の設計根拠** (03 L827-834): 意図的に prefix ファミリでなく明示列挙。理由コメント (`aidlc-audit.ts:377-394`): Bolt/swarm worktree は `STAGE_*`, `SENSOR_*`, reviewer receipts, `ARTIFACT_*` を正当な作業成果物として発行し、*"the referee's defence against a lying conductor is artifact re-verification at finalize, not delta filtering."* これらファミリの prefix ブラックリストは `bolt complete --merge` を決定論的に回復不能にした。**ブロックされるもの**: human authority、unit-lifecycle receipts、referee bookkeeping (fork/merge/swarm/bolt/worktree 行 — main 側発行)、`DOCUMENT_*` (prefix)。

**注意 (仕様執筆時の欠落データ)**: 03 は `CLI_RESERVED_EVENT_TYPES` (8) と `MERGE_PROTECTED_EVENT_TYPES` (26) の**明示的メンバー列挙を掲載していない** (M4 は基数のみ検証)。逐語リストが必要なら upstream `aidlc-audit.ts:292` / `:395` の実測が必要。CLI_PROTECTED (18) のみ上記 prose で完結する。

**モデルの限界の自認** (逐語、03 L836-839): `audit-format.md:66-73` — `HUMAN_TURN` は *"chronological presence evidence, not authenticated decision content"*、`--user-input` / `--feedback` / `--details` は呼出側供給 prose、*"Audit shards are operational evidence, not a tamper-proof human-authorship boundary."*

---

## 5. appendAuditEntries・順序保証・audit-first・ロック

### 5.1 `appendAuditEntries` の契約 (03 §6.10 L1014-1017)

audit-only トランザクションプリミティブ `appendAuditEntries` (`aidlc-audit.ts:770`):
- 全エントリをディスクに触る**前に**検証し、その後**1 ロック内で全ブロックを 1 write** で書く。
- 逐語 (`:765-769`): *"a malformed later entry cannot leave an earlier entry committed, and no concurrent emitter can interleave between the blocks"* (不正な後続エントリが先行エントリをコミット済みのまま残すことはなく、並行 emitter がブロック間に割り込むこともない)。
- `AIDLC_METRICS_ENDPOINT` 設定時、全ての構造化 append が metrics module を叩く (03 §2.3 L116 / `aidlc-audit.ts:514`)。

### 5.2 順序保証 — シーケンス番号は存在しない (03 §1 L27-31, §6.4 L726-758)

| 層 | 契約 | 典拠 |
| --- | --- | --- |
| 前提 | 行は秒精度 ISO timestamp を持ち序数的なものは他にない | 03 §6.4 L727-728 |
| 単一 shard 内 | append 順 = バッファ順で保存される | 03 L730 |
| shard 横断 | バッファ位置は情報を持たない — `readAllAuditShards` は**ファイル名順**に連結。`findAllEvents` (`aidlc-lib.ts:7761`) が `**Timestamp**` で時系列ソートし、タイはバッファ位置で破る (`:7799-7801`)。理由 (`:7791-7798`): 素朴な `[len-1]` 最新読みは *"could otherwise pick an OLDER event from a lexically-later shard"* | 03 L731-737 |
| **authority-bearing 比較は cross-shard タイで fail-closed** | `humanActedSinceGate` (`aidlc-lib.ts:3774`) は連結バッファも共有 reader (`readAuditShardEvents`) も使わず、`auditShards(projectDir)` (`:3780`) で shard を自前列挙し、各々を `readAppendOnlyFileNoFollowOrThrow` (`:3786`) で読み、自前の `{ ts, shard, pos, human }` レコード (`:3811-3816`) を構築 — shard index と shard 内 append 位置を全イベントに保持。最新 `HUMAN_TURN` 候補が**別 shard** の最新 gate resolution と 1 秒を共有するとき: *"execution order is unknowable and the check fails CLOSED (require a fresh turn) rather than let shard-filename order pick a winner"* (`:3752-3754`)。強制述語 (`:3838-3853`): 最新 turn が勝つのは**全ての**最新 resolution が `resolution.shard === human.shard && resolution.pos < human.pos` を満たすときのみ | 03 L738-748; §1 L28-31 |
| unit lifecycle の Run floor | カウンタなしで厳密トークンにより強い境界を達成: `Run floor` = `<event>:<timestamp>#<ordinal>`。異 shard の等時刻境界は決定論的な `AMBIGUOUS:<timestamp>#<digest>` floor に劣化し、過去の receipt は一致できない | 03 L750-753 / `audit-format.md:114-119` |
| Sensor 行の相関 | 全 `SENSOR_*` 行が 8-hex の `Fire id` を持つ。`audit-format.md:248` は強調: *"Pair by `Fire id`, not by audit-row index"* — 1 tool call が 4 並列 sensor fire に fan out し terminal 行が duration で交錯するため | 03 L755-758 |

### 5.3 audit-first 不変条件の正確な機構 (03 §5.7 L596-599)

- 全ての read-modify-write ハンドラは `withAuditLock(pd, …)` 内で走り、**read → decide → audit → write が 1 クリティカルセクション**。
- 不変条件は *audit-first*: **audit 行はロック内で発行され state 書込がそれに続く。throw された audit エラーは state 書込をスキップする** (`aidlc-state.ts:128-130`、例 `:2255-2286`)。
- §1.4 のガード 5 と接続: 書込中 rename は「囲んでいる audit-first トランザクション」を失敗させる (03 §6.7 L855-856)。

### 5.4 ロック (クラッシュ込み Quint モデル `audit_lock.qnt` の直接材料、03 §6.8 L868-938)

| 要素 | 契約 | 典拠 |
| --- | --- | --- |
| 実体 | **`os.tmpdir()` 内の `mkdir`-EEXIST ディレクトリによるクロスプロセス mutex** | 03 L870-871 / `aidlc-lib.ts:6753-6755` |
| Identity | `auditLockIdentity(projectDir, intent?, space?)` = `<realpath(projectDir)>\x00<space>\x00<intent>`、intent 省略時は `<realpath(projectDir)>\x00__workspace__` (`WORKSPACE_LOCK_SENTINEL`, `:6777`) | 03 L873-876 / `aidlc-lib.ts:6799` |
| Keying 不変条件 (2 つ、`:6757-6768`) | (1) intent 省略は予約 sentinel をハッシュし `activeIntent()` を**決して**解決しない — birth 時 active intent は無く、解決すると並行初回 2 実行が別バケットをキーし両方 birth する。全 `intents.json` 変異はこのバケットを取る。(2) 複合 identity はロック dir と in-process depth/handler map の両方をキーする — さもなくば map が intent 間で衝突 | 03 L876-882 |
| 位置 | `auditLockDir` = `join(tmpdir(), `.aidlc-audit-${md5(identity).slice(0,8)}.lock`)` | 03 L884 / `aidlc-lib.ts:6814` |
| Acquire | `acquireAuditLock(projectDir, maxRetries=50, retryMs=100, intent?, space?, reapLiveOwnerAfterStale=true)`: `mkdirSync(lockDir)` → `writeOwnerStamp` ループ。`EEXIST` で `reapStaleLock` を試み、成功時 mkdir を即再試行、失敗時 `retryMs` sleep。予算超過で `false` 返却; 呼出側は `Failed to acquire audit lock after retries` に翻訳 (`aidlc-audit.ts:543`) | 03 L886-890 / `aidlc-lib.ts:7138` |
| Owner stamp | ロック dir 内 `owner.json` = `{ pid, startedAtMs, reapLiveOwnerAfterStale, token? }` | 03 L892-893 / `aidlc-lib.ts:6824-6826` |
| Reap 条件 | `process.kill(pid, 0)` が `ESRCH` (owner 消滅)、または stamp 年齢 > `lockStaleMs()` — デフォルト `DEFAULT_LOCK_STALE_MS = 10 * 60 * 1000` (`:6784`)、`AIDLC_LOCK_STALE_MS` で上書き可。生存中かつ閾値以下の holder は決して奪わない (`:6771-6774`)。**unstamped** dir (mkdir 完了・`owner.json` 未書込) は `unstampedGraceMs()` — デフォルト 5000 ms、`AIDLC_LOCK_UNSTAMPED_GRACE_MS` (`:6925-6932`) — で保護され、acquire 途中の生存プロセスから盗まれない | 03 L895-900 |
| **Steal は CAS** | `reapStaleLock` (`:7023`) はロック dir を reaper 私有の `<lockDir>.dead.<pid>-<counter>` へ rename して退避し、移動済み dir に `stampMatches` (`:6960`) を呼んで判定したのと同じロックを掴んだか確認。不一致なら rename で戻す。残余レース (`:6993-7014`): 復元は第三プロセスが隙間で再 `mkdir` していれば `EEXIST` で失敗しうる — その場合は生きたロックが既に存在するので私有 dir は単に破棄 | 03 L902-907 |
| 再入 | `withAuditLock` (`:7570`) は identity ごとの depth カウンタを保持 — 保持中セクション内のネスト呼出は再取得せず早期解放もしない。初回取得時に `process.on("exit")` ハンドラを設置し lock dir を `rm -rf` — *"if the body calls process.exit (Bun skips `finally` in that case) … so the project isn't poisoned for ~5s on the next invocation"* (`:7601-7609`) | 03 L909-914 |
| 自己デッドロック回避 | `holdsAuditLock` (`:7637`) は複合 identity 下でその exit ハンドラの存在を probe し、`emitAudit` (`aidlc-state.ts:141`) と `emitError` (`aidlc-lib.ts:9977`) の両方がそれで分岐して `appendAuditEntryUnlocked` を選ぶ | 03 L914-915 |
| 解放 | depth が 0 に戻るときのみ `rm -rf lockDir` + exit ハンドラ除去 (mermaid 図 + text fallback) | 03 L917-938 |
| audit-merge の拡大予算 | デフォルト `200 × 100 ms = 20 s` (並列 Bolt 競合向け)、`AIDLC_AUDIT_LOCK_RETRIES` / `_RETRY_MS` で調整 | 03 §6.9 L975; §2.3 L115 / `aidlc-audit.ts:1363-1371` |
| フックの縮小予算 | blocking ガードは `acquireAuditLock(projectDir, 5, 50)` | 07 §3 L85 |

### 5.5 fork/merge の追加契約 (R6 の受け皿として関連、03 §6.9 L940-988)

`aidlc-audit.ts` の 5 サブコマンド (M8): `append`, `append-batch`, `append-raw`, `audit-fork`, `audit-merge`。

**`audit-fork --slug <s> [--intent <i>] [--space <sp>]`** (`:1123`):
1. pre-emit ガードは clean fail — `main audit not found at <p>; start a workflow first …` / `worktree directory not found at <p>; run aidlc-worktree create first`;
2. per-intent ロック下で main をスナップショット; `boundary = bytes.length`, `sourceHash = sha256(bytes)`;
3. `AUDIT_FORKED` を `Bolt slug`, `Source Audit Hash`, `Fork Boundary` 付きで発行 — `expectedIdentity` prefix 検査でピン留めし、スナップショットと emit の間に並行 append が滑り込めない;
4. clone-id トークンを worktree にコピーし、shard をファイル全体 tmp+rename で書く (`writeBufferAtomic(wtAuditPath, mainAfterFork)`, `:1252`)。

再 fork の 3 逐語拒否 (`:1164-1182`): *"…with unmerged work after AUDIT_FORKED; merge the delta with audit-merge, or discard the worktree"* / *"…its AUDIT_FORKED row does not match the authoritative main row"* / *"…its fork prefix differs from main"*。3 ガードと `alreadyCurrent` 短絡は全て `if (existingFork)` 内 (`:1161-1188`) — この slug の `AUDIT_FORKED` 行を持たない既存 worktree shard はどれにも一致せず、step-4 で丸ごと置換される (03 L959-966)。

**`audit-merge --slug <s>`** (`:1320`) は **delta のみ** (`wtContent.slice(fork.end)`) を追記:
- `validateMergeDelta` (`:974`): delta はブロック境界で終わる (`worktree audit delta ends with an incomplete block`)、各ブロックは `Event` と `Timestamp` を厳密 1 つ (または event なし timestamp 厳密 1 の完全な `append-raw` note)、イベントは `VALID_EVENT_TYPES` に含まれ (`worktree audit delta contains unknown event <E>`)、merge-protected でない (`worktree audit delta contains protected authority event <E>`);
- ロック内で main を再スナップショット; worktree スナップショットは pre-lock 読取とバイト・inode 一致必須 (`worktree audit changed while merge was preparing; retry the merge`);
- *authoritative* fork 行は書込可能な worktree コピーを信用せず **main** から回復 (`:1404-1411`)、全相関フィールド一致必須;
- main 先頭 `boundary` バイトの SHA-256 = 記録済み `Source Audit Hash` 必須。不一致: `main audit prefix-hash at byte <n> does not match recorded Source Audit Hash; refusing to merge (mid-Bolt tampering suspected)`; main が boundary より短い場合: `… (main-audit truncation suspected)`。

`AUDIT_MERGED` のフィールド: `Bolt slug`, `Entries Merged`, `Source Audit Hash`, `Fork Boundary`, `Fork Timestamp`。`audit-format.md:211`: per-Bolt エントリ順は保存、cross-Bolt 順は merge 完了順 (03 L986-988)。

---

## 6. runtime-graph.json — 決定性契約と rebuild トリガ (workspace 所有部分)

### 6.1 性格と決定性 (03 §7.1 L1031-1041)

| 項目 | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| 正体 | `<record>/runtime-graph.json` は**materialised, derived view** — 構造的 `stage-graph.json` のデータプレーン鏡像。gitignored (§3.4) かつ再導出可能 | 03 L1033-1037 |
| Pure observer | *"Pure observer — never mutates state.md, never asks the user, only reads the audit log + memory.md files and writes runtime-graph.json + emits MEMORY_EMPTY rows for zero-entry approved stages."* | `aidlc-runtime.ts:1-13` / 03 L1034-1036 |
| **決定性契約** | *"re-running compile against the same audit log produces a byte-equivalent runtime-graph.json."* (同一 audit log への compile 再実行はバイト等価な runtime-graph.json を生む) | `aidlc-runtime.ts:19-23` / 03 L1039-1040 |
| スキーマ固定 | `docs/reference/13-runtime-graph.md` にピン留め: *"Changing the shape requires bumping every consumer (Bolt fork/merge, gate ritual, lifecycle, doctor) in the same change."* | `aidlc-runtime.ts:15-17` / 03 §7.3 L1070-1072 |

### 6.2 compile の入力規則 (03 §7.2 L1044-1062)

| 規則 | 内容 | 典拠 |
| --- | --- | --- |
| skip | `aidlc-state.md` 不在時 `{skipped:"no-state"}` + stderr note | `aidlc-runtime.ts:316`, `:320-326` |
| 入力 | **全 shard** を `readAllAuditShards` で読む | `:328` |
| header | 最新 `WORKFLOW_STARTED`; `workflow_id` と `started_at` は共にその行の timestamp; `scope` は state ファイルの `Scope` を audit 行より優先 | `:239` / 03 L1048-1050 |
| pairing | `pairStartedCompleted`: 同一 slug の `STAGE_STARTED`@T1 と後続 `STAGE_COMPLETED` を対にする; 最新 `STAGE_STARTED` が勝つので re-jump は行をリセット | `:172`, `:138-147` |
| single-stage 除外 | `isSingleStageRow`: `--single` stage-runner 行を `/^\*\*Workflow\*\*:\s*single-stage:/m` で除外; main-workflow 行は `Workflow` フィールドを持たず、不在 = main | `:168`, `:166`, `:158-165` |
| memory | `readMemory`: §13 の 4 見出し下の diary エントリを数える; `memory.md` 不在で `{null, null}` (diary 出荷前完了 stage の backfill 規則)、存在するが空なら zero counts | `:271` |
| bolt_dag | `computeBoltDag`: units-generation の edge block をパース; 不在・不正・巡回はいずれも `bolt_dag` ノードを丸ごと省略 (wrong-but-valid DAG をエンコードしない)。不在は silent (`:301`)、不正/巡回は理由と詳細を stderr へ (`:304-309`) | `:299` |

### 6.3 rebuild トリガ (03 §7.2 L1064-1066; 詳細は 07 §8.1 L319-332)

- 03 側の記述: **compile は PostToolUse Bash hook (`aidlc-rebuild-stage-graph.ts`) により全ての transition-class audit emit で自動起動**; 手動起動は debug surface (`aidlc-runtime.ts:1312-1314`)。
- 07 §8.1 のパイプライン (workspace 仕様に必要な範囲):
  1. session binding (intent birth の session stamp / handoff receipt) — フィルタ前;
  2. コマンドフィルタ `classifyRuntimeCompileCommand`: compile ツール自身の呼出は `reject` (再帰ガード)。Kiro IDE は `tool_input.source = "ide-audit-sync"` でこの pre-filter をスキップ;
  3. active intent の**全 shard** を読む — *"the state tool that wrote the transition runs in a SEPARATE process"* のため自プロセス shard だけでは足りない (`:151-161`);
  4. **transition フィルタ**: 末尾 3 audit ブロック (approve は 1 Bash call で `GATE_APPROVED + STAGE_COMPLETED + STAGE_STARTED` を書きうる) を逐語 regex に照合 (`:192`):
     `/^\*\*Event\*\*:\s*(GATE_APPROVED|STAGE_STARTED|STAGE_AWAITING_APPROVAL|AUDIT_MERGED|WORKFLOW_COMPLETED)\s*$/m`
     — `WORKFLOW_COMPLETED` は terminal approve でも compile を発火させるため、`STAGE_AWAITING_APPROVAL` は gate ritual が `STAGE_STARTED` 時点スナップショットの memory-entry count を読まないために含まれる;
  5. **冪等ガード (IDE モードのみ)**: `runtime-graph.json` の mtime が最新 audit shard 以上なら skip (`:200-232`);
  6. dispatch: `bun run <harness>/tools/aidlc-runtime.ts compile` 同期、`cwd: projectDir`、**30 s timeout**; 非ゼロ exit は drop として記録され親 Bash call を決してブロックしない。
- 再帰ガードは二層: コマンドレベルの `aidlc-runtime.ts` reject **かつ** compile 自身が発行する `MEMORY_EMPTY` がイベント regex に不在 (07 §8.1 L332 / `:19-21`)。

### 6.4 fragment fork/merge (03 §7.5 L1142-1152)

- `fragment-fork --slug` は main の `runtime-graph.json` を Bolt worktree にバイトコピー; `fragment-merge --slug` は worktree fragment を除去 (冪等)。**どちらも audit イベントを発行しない**: *"the fork boundary is already triple-attested by BOLT_STARTED + STATE_FORKED + AUDIT_FORKED, the merge boundary by BOLT_COMPLETED + STATE_MERGED + AUDIT_MERGED"* (`aidlc-runtime.ts:1104-1107`)。
- `fragment-fork` は single-read プロトコル (一度読み、そのバッファから書き、同じバッファをハッシュ) で並行 compile とのコピー/ハッシュ競合を閉じる (`:1120-1122`)。main に graph が無ければ空 graph を fragment パスに書く (`writeEmptyGraph`, `:813`)。
- **戻りの content merge は意図的に存在しない**: main の graph は post-Bash hook により main audit から event-sourced に再構築され、content merge は compile と競合するため (`:1109-1112`)。
- 折り込み規則 (fold-in) の意味論はこの範囲外 (verification 所有 — B8 の注記どおり、03 に折り込み規則の記述はない。03 が workspace に与えるのは上記の決定性・トリガ・fragment 契約まで)。

---

## 7. 関連 env バイパス一覧 (audit/lock 関連のみ抜粋、03 §2.3 L99-116)

| 変数 | 効果 | サイト |
| --- | --- | --- |
| `AIDLC_ALLOW_DIRECT_AUDIT_EVENTS` | `=1` で audit CLI が authority-bearing イベントを発行可能 | `aidlc-audit.ts:432` |
| `AIDLC_STATE_TRANSITION_OWNER` | engine 所有 state verb には `orchestrate:<ppid>` と一致必須 | `aidlc-state.ts:540` |
| `AIDLC_ALLOW_DIRECT_STATE_TRANSITIONS` | `=1` で engine-ownership 検査をバイパス | `aidlc-state.ts:541` |
| `AIDLC_SKIP_HUMAN_PRESENCE_GUARD` | `=1` で `HUMAN_TURN` freshness gate 無効化 | `aidlc-lib.ts:6543` (`humanPresenceGuardDisabled`, 宣言 `:6542`) |
| `AIDLC_LOCK_STALE_MS` | stale-lock 年齢閾値 (デフォルト 600000) | `aidlc-lib.ts:6787`、デフォルト `:6784` |
| `AIDLC_LOCK_UNSTAMPED_GRACE_MS` | unstamped lock dir の猶予 (デフォルト 5000) | `aidlc-lib.ts:6925-6931` |
| `AIDLC_AUDIT_LOCK_RETRIES` / `_RETRY_MS` | `audit-merge` の acquire 予算 (デフォルト 200 × 100 ms) | `aidlc-audit.ts:1363-1371` |
| `AIDLC_METRICS_ENDPOINT` | 設定時、全構造化 append が metrics module を叩く | `aidlc-audit.ts:514` |

---

## 8. 監査関連の観測済み乖離 (03 §6.11 L1019-1025 — 仕様執筆時に転記すべき既知の食い違い)

| 乖離 | 証拠 |
| --- | --- |
| `audit-format.md:10` は mandatory events が `tests/feature/t48-audit-event-emitters.sh` で assert されると言うが、`tests/feature/` はリポジトリに存在しない (M13)。生きた同期ガードは `tests/unit/t28-audit-event-sync.test.ts` (`.sh` 前身から移行の旨をヘッダに記載) | docs vs tree |
| `worktree-info-schema.md:42` は `merge_held` を `<path>/aidlc-docs/aidlc-state.md` から読むと記すが、フラット `aidlc-docs/` は一度きりの移行元 (`FLAT_MIGRATION_ROOT`, `aidlc-lib.ts:1823`) としてのみ残存; 生きた worktree state パスは `worktreeStateFilePath` = `<wt>/<recordPrefix>/aidlc-state.md` (`aidlc-lib.ts:6193`)。同じ stale パスが `aidlc-state.ts:4071`, `aidlc-runtime.ts:1101`, `:1306` のコメントにも | docs/comments vs code |
| `audit-format.md:20-23` は旧 shard の重複 `Timestamp` を許容し whole-file reader に dedupe を求める。`findAllEvents` はブロックごと最初の一致 (`aidlc-lib.ts:7772`, 非 global `m` regex) でこれを満たすが、`validateMergeDelta` は timestamp ≠1 のブロックを*拒否* (`aidlc-audit.ts:987-989`) — legacy 二重 timestamp ブロックは worktree からマージ不能 | code vs code |

---

## 9. Quint モデル `audit_lock.qnt` への設計インプット要約 (§5.4 から抽出したモデル化対象の状態機械)

| モデル要素 | 仕様上の対応 |
| --- | --- |
| 状態: lock dir {absent, present-unstamped, present-stamped(pid, startedAtMs), renamed-aside(.dead.<pid>-<counter>)} | 03 §6.8 L886-907 |
| アクション: mkdir (成功/EEXIST)、writeOwnerStamp、reap 判定 (ESRCH ∨ age>staleMs、ただし unstamped は graceMs 保護)、CAS steal (rename aside → stampMatches → commit/rollback)、rollback の EEXIST 分岐 (第三者再 mkdir → 私有 dir 破棄)、release (rm -rf、depth=0 時のみ)、クラッシュ (exit ハンドラ実行 / 未実行の両方 — `process.exit` で Bun が `finally` をスキップする旨 L911-913) | 03 §6.8 L895-915 |
| 再入: identity ごと depth カウンタ; ネストは再取得も早期解放もしない | 03 L909-911 |
| 安全性の対象不変条件: 生存中かつ閾値以下の holder は決して奪われない (L897-898); audit-first (audit 発行 → state 書込、audit throw で state スキップ、L596-599); appendAuditEntries の非割込 1 write (L1014-1017) | — |
| 活性の対象: acquire 予算 50×100ms (merge 200×100ms) 後の fail (`Failed to acquire audit lock after retries`)、stale reap による回復、クラッシュ後 ~staleMs/graceMs での回復 | 03 L886-900 |