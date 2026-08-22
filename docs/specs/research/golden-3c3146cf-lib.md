> 採取元: **`awslabs/aidlc-workflows` 公開リポジトリからの直接採取** — ピン留めコミット `3c3146cf`（`3c3146cfd7cef33020d48e8d48d4e80d0f8c2820`、v2.6.40、branch `v2`）の `core/tools/aidlc-lib.ts`（10,668 行 / 450,663 B、SHA-256 `ba4e2259cab97393279cf9c1c63da24d8fbf035cb153e009b41d3e0c8a78d97f`）。補助として `aidlc-audit.ts`（1,589 行）・`aidlc-bolt.ts`（970 行）。既存 research 文書と違い、as-built 仕様（`docs/upstream/specs/`）の二次引用ではなく **upstream ソースの実バイトを `curl` で取得して読解した一次採取**である。採取日 **2026-08-22**（Issue #7 項目 0）。11-workspace.md / 12-workflow-definition.md / 文言カタログ（ADR 0002）の裏取り材料。
>
> **検証 grep の要約**: サイズ 450,663 B・10,668 行が指示の期待値および as-built 測定 M1 と一致 ✅ ／ M14（`^export function (hooksHealthDir|recoveryFilePath|planFilePath|runtimeGraphPath|sensorsDir)` = 5）✅ ／ M15（`aidlc-audit.ts` のバイト書込サイト 5 行、行番号一致）✅ ／ M12（`setOrInsertField` 非呼出 3 行・呼出 0、`AUTONOMY_MODE_FIELD` 2 行）✅ ／ **既存 research が引く `aidlc-lib.ts` 行番号 48 箇所を全数照合 → 48/48 一致**、ただし `workflow-definition-graph-reader.md:221,268` の `:2837-2864` / `:2841-2860` のみ **v2.2.0 由来のドリフトで不一致**（正位置は `loadStageGraph` `:8552` / `loadStageGraphAll` + エラー逐語 `:8558-8585`）／ **文言カタログの `SpecQuotedOnly` 4 件はピン留めで 4/4 バイト一致**（em dash U+2014・引用符・末尾ピリオドまで含む。出典行のみ `:6564`→`:6572`、`:6453`→`:6456` に訂正）。
>
> 本書は採取レポートの**原文**であり、逐語ブロック・upstream 行番号・食い違い表（A1〜A16 / B / C / D）を採取時のまま保持する。本文が記録する `/private/tmp/…/scratchpad/…` は採取セッションの作業ディレクトリであり、既に存在しない。

---

# ゴールデン採取結果 — `core/tools/aidlc-lib.ts` @ `3c3146cf` (v2.6.40)

**取得**: `curl -fsSL https://raw.githubusercontent.com/awslabs/aidlc-workflows/3c3146cf/core/tools/aidlc-lib.ts` → 成功。
**保存先**: `/private/tmp/claude-501/-Users-j5ik2o-orca-workspaces-amadeus-ng-docs/351513f3-85bf-44e8-92ca-bea27cc446f6/scratchpad/upstream-src/core/tools/aidlc-lib.ts`
**サイズ**: 450663 bytes / 10668 行 (指示の期待値・03 の M1 と一致)。
補助取得 (message-catalog 検証用): 同ディレクトリに `aidlc-audit.ts` (1589 行)、`aidlc-bolt.ts` (970 行)。

**リポジトリのファイルは一切変更していない。**

---

## 1. ロック — `reapStaleLock` / `stampMatches` / `writeOwnerStamp`

### 1.1 結論 (最重要 2 点)

- **`reapLiveOwnerAfterStale` は reap 判定で確かに読まれる。ただし「所有者が生存している」枝でのみ。** 所有者が死亡 (`ESRCH`) している場合はこのフラグは**完全に無視**され、年齢によらず即 reap される。未スタンプ dir の枝でも読まれない (owner.json が無いので読めない)。
- **`stampMatches` は 4 フィールドすべてを比較する** — `pid` / `startedAtMs` / `reapLiveOwnerAfterStale` / `token`。すなわち **`reapLiveOwnerAfterStale` は upstream では「同一性の一部」として扱われている**。

### 1.2 `reapStaleLock` の完全な判定ロジック (逐語)

```
7023	function reapStaleLock(lockDir: string, reapUnstamped = true): boolean {
7024	  const owner = readOwnerStamp(lockDir);
7025	  if (owner === null) {
7026	    // UNSTAMPED dir: a live holder mid-acquire (between mkdir and stamp) OR a
7027	    // process SIGKILL'd in that window. Distinguish by the dir's own age — only
7028	    // steal one OLDER than the grace window; a fresh unstamped dir is a live
7029	    // holder about to stamp and MUST NOT be robbed (the C2b concurrent-fork
7030	    // serialization depends on this).
7031	    if (!reapUnstamped) return false;
7032	    const mtime = lockDirMtimeMs(lockDir);
7033	    if (mtime === null) return false; // vanished — let the next mkdir try
7034	    if (lockAcquireEpochMs() - mtime <= unstampedGraceMs()) return false;
7035	    // else: an old unstamped dir → genuine leak, fall through to steal.
7036	  } else if (ownerAlive(owner)) {
7037	    if (!owner.reapLiveOwnerAfterStale) return false;
7038	    // Live owner: only reclaim if its stamp is over-age (a wedged-but-running
7039	    // holder). A fresh, live holder is never robbed.
7040	    if (lockAcquireEpochMs() - owner.startedAtMs <= lockStaleMs()) return false;
7041	  }
7042	  // STEP 1 — CAS swap: move the dir to a reaper-private nonce path. This is the
7043	  // atomic arbiter; only one process wins the rename of a given dir.
7044	  const dead = `${lockDir}.dead.${reapSuffix()}`;
7045	  try {
7046	    renameSync(lockDir, dead);
7047	  } catch {
7048	    return false; // another waiter already reclaimed (or the holder released)
7049	  }
7050	  // STEP 2 — verify the dir we just grabbed STILL carries the identity judged
7051	  // stale. stampMatches re-reads owner.json inside the now-private `dead` dir.
7052	  if (!stampMatches(dead, owner)) {
7053	    // STEP 3 — we grabbed a FRESH lock a competitor re-acquired in the window.
7054	    // Restore it so the live holder is not robbed.
7055	    try {
7056	      renameSync(dead, lockDir);
7057	    } catch {
7058	      // lockDir already re-created by yet another process → the live lock is
7059	      // back in place; discard our private snapshot.
7060	      try { rmSync(dead, { recursive: true, force: true }); } catch { /* harmless */ }
7061	    }
7062	    return false;
7063	  }
7064	  // Legitimate steal: dead owner, live-but-over-age, or old-unstamped — AND the
7065	  // identity we grabbed matches what we judged. Remove the private dir.
7066	  try {
7067	    rmSync(dead, { recursive: true, force: true });
7068	  } catch {
7069	    // leftover .dead dir is harmless (it never collides with the live lock name)
7070	  }
7071	  return true;
7072	}
```

判定の 3 枝 (`if/else if` — **`else` 枝は無い**):

| 枝 | 条件 | 挙動 |
| --- | --- | --- |
| 未スタンプ (`owner === null`) | `!reapUnstamped` → false / `mtime === null` → false / `now - mtime <= graceMs` → false | それ以外は steal へ落ちる |
| 所有者生存 (`ownerAlive(owner)`) | `!owner.reapLiveOwnerAfterStale` → **false** / `now - startedAtMs <= staleMs` → false | それ以外は steal へ落ちる |
| 所有者死亡 (暗黙) | ガード無し | **無条件に steal へ落ちる** (`reapLiveOwnerAfterStale` も年齢も見ない) |

`reapSuffix()` (`:6913-6916`) は `${process.pid}-${++_reapCounter}` (プロセス単調カウンタ。`Math.random`/`Date.now` 禁止規約による)。
`ownerAlive` → `isPidAlive` (`:6892-6901`): `pid` が非整数 or `<= 0` は **not alive**。`process.kill(pid,0)` 成功 = alive、`EPERM` = alive、それ以外 (ESRCH) = dead。

### 1.3 `stampMatches` が比較するフィールドの全列挙 (逐語)

```
6960	function stampMatches(dir: string, judged: LockOwner | null): boolean {
6961	  const now = readOwnerStamp(dir);
6962	  if (judged === null) {
6963	    // Old-unstamped leak. Still unstamped + still over grace (mtime preserved by
6964	    // rename). A re-created dir resets mtime → under grace → mismatch; a now-
6965	    // stamped dir → a live re-acquirer → mismatch.
6966	    if (now !== null) return false;
6967	    const mtime = lockDirMtimeMs(dir);
6968	    if (mtime === null) return false; // vanished — nothing to steal
6969	    return lockAcquireEpochMs() - mtime > unstampedGraceMs();
6970	  }
6971	  if (now === null) return false;
6972	  return (
6973	    now.pid === judged.pid &&
6974	    now.startedAtMs === judged.startedAtMs &&
6975	    now.reapLiveOwnerAfterStale === judged.reapLiveOwnerAfterStale &&
6976	    now.token === judged.token
6977	  );
6978	}
```

**比較フィールドは 4 つ**: `pid`, `startedAtMs`, `reapLiveOwnerAfterStale`, `token`。
`token` は `readOwnerStamp` (`:6867`) が `typeof parsed.token === "string" && parsed.token.length > 0` のときのみ設定するので、`acquireAuditLock` 経路では両辺とも `undefined` になり `undefined === undefined` で真。

**`judged === null` (未スタンプ判定) の場合は「今も未スタンプ」＋「今も grace 超過」の 2 条件を rename 後に再検査する** — 単なる「両方 null なら一致」ではない。

### 1.4 `writeOwnerStamp` の出力バイト (逐語)

型 (研究文書の `:6824-6826` と一致):

```
6824	interface LockOwner {
6825	  pid: number; startedAtMs: number; reapLiveOwnerAfterStale: boolean; token?: string;
6826	}
```

書込:

```
6832	function writeOwnerStamp(
6833	  lockDir: string,
6834	  reapLiveOwnerAfterStale = true,
6835	  token?: string,
6836	): LockOwner | null {
6837	  const owner: LockOwner = {
6838	    pid: process.pid,
6839	    startedAtMs: lockAcquireEpochMs(),
6840	    reapLiveOwnerAfterStale,
6841	    ...(token ? { token } : {}),
6842	  };
6843	  try {
6844	    writeFileSync(ownerStampPath(lockDir), JSON.stringify(owner), {
6845	      encoding: "utf-8",
6846	      mode: 0o600,
6847	      ...(token ? { flag: "wx" } : {}),
6848	    });
6849	    return owner;
6850	  } catch {
6851	    // Best-effort: a missing stamp degrades the reaper to age-only on the next
6852	    // waiter (it can't read a PID), never to incorrectness.
6853	    return null;
6854	  }
6855	}
```

確定事項:

- **キー順は `pid` → `startedAtMs` → `reapLiveOwnerAfterStale` → (`token`)**。オブジェクトリテラルの挿入順がそのまま `JSON.stringify` の出力順になる。
- **`token` は引数が truthy のときだけキーが出現する。`acquireAuditLock` は token を渡さないので audit ロックの owner.json に `token` は無い。** token を渡すのは `acquireActiveDirectiveLock` (`:7118`) のみ。
- **インデント無し** (`JSON.stringify(owner)` — `null, 2` ではない)、末尾改行無し。実バイト例: `{"pid":12345,"startedAtMs":1755800000000,"reapLiveOwnerAfterStale":true}`
- ファイルパーミッション **`0o600`**、`encoding: "utf-8"`。token 付きのときだけ `flag: "wx"` (排他生成)。
- **例外は握り潰して `null` を返す (best-effort)。書込失敗で acquire が失敗することはない。**
- `startedAtMs` は `lockAcquireEpochMs()` (`:6881-6883`) = `Math.floor(performance.timeOrigin + performance.now())`。**`Date.now()` ではない** (lint 禁止のため)。

---

## 2. `auditLockDir` / `lockStaleMs` / `unstampedGraceMs`

```
6777	export const WORKSPACE_LOCK_SENTINEL = "__workspace__";
```
```
6784	export const DEFAULT_LOCK_STALE_MS = 10 * 60 * 1000;
6785	
6786	function lockStaleMs(): number {
6787	  const raw = process.env.AIDLC_LOCK_STALE_MS;
6788	  if (raw) {
6789	    const n = Number(raw);
6790	    if (Number.isFinite(n) && n > 0) return n;
6791	  }
6792	  return DEFAULT_LOCK_STALE_MS;
6793	}
```
```
6799	export function auditLockIdentity(projectDir: string, intent?: string, space?: string): string {
6800	  let canonicalProjectDir = resolvePath(projectDir);
6801	  try {
6802	    canonicalProjectDir = realpathSync(canonicalProjectDir);
6803	  } catch {
6804	    // Birth and diagnostics can lock before the project exists. The absolute
6805	    // lexical path is stable until realpath can resolve filesystem aliases.
6806	  }
6807	  if (intent === undefined) {
6808	    return `${canonicalProjectDir}\x00${WORKSPACE_LOCK_SENTINEL}`;
6809	  }
6810	  const sp = space ?? activeSpace(projectDir);
6811	  return `${canonicalProjectDir}\x00${sp}\x00${intent}`;
6812	}
6813	
6814	export function auditLockDir(projectDir: string, intent?: string, space?: string): string {
6815	  const identity = auditLockIdentity(projectDir, intent, space);
6816	  const hash = createHash("md5").update(identity).digest("hex").slice(0, 8);
6817	  return join(tmpdir(), `.aidlc-audit-${hash}.lock`);
6818	}
```

**`realpathSync` が失敗したときは lexical な絶対パスにフォールバックする** (throw しない) — 研究文書に無かった詳細。

### 未スタンプ猶予の実装 — **`mtime` である (`birthtime` ではない)**

```
6918	// Grace window (ms) for an UNSTAMPED lock dir. acquireAuditLock mkdirs the lock
6919	// dir THEN writes owner.json, so there is a brief window where a live holder's
6920	// dir has no stamp yet. A waiter must NOT steal an unstamped dir younger than
6921	// this grace (it is a live process mid-acquire) — only an unstamped dir OLDER
6922	// than the grace is treated as a genuine leak (e.g. a SIGKILL between mkdir and
6923	// stamp). Generous relative to the mkdir→write gap, tiny relative to the stale
6924	// threshold. Tunable via AIDLC_LOCK_UNSTAMPED_GRACE_MS.
6925	function unstampedGraceMs(): number {
6926	  const raw = process.env.AIDLC_LOCK_UNSTAMPED_GRACE_MS;
6927	  if (raw) {
6928	    const n = Number(raw);
6929	    if (Number.isFinite(n) && n > 0) return n;
6930	  }
6931	  return 5000;
6932	}
6933	
6934	// The lock dir's own mtime epoch (ms), or null if it can't be stat'd. Used as the
6935	// age anchor for an UNSTAMPED dir (no owner.json yet / ever). statSync mtime is a
6936	// wall-clock ms, comparable to lockAcquireEpochMs()'s epoch family.
6937	function lockDirMtimeMs(lockDir: string): number | null {
6938	  try {
6939	    return statSync(lockDir).mtimeMs;
6940	  } catch {
6941	    return null;
6942	  }
6943	}
```

`stampMatches` の未スタンプ枝が成立する根拠も逐語で明記されている: *"renameSync preserves the inode's mtime, so a genuine old leak keeps its over-grace mtime through the move"* (`:6952-6954`)。

env 上書きの受理条件は 2 つとも同一: `Number.isFinite(n) && n > 0` — **0 や負値・NaN は既定値へ落ちる** (研究文書に無かった詳細)。

---

## 3. `readAllAuditShards` の連結順序 (OPEN QUESTION の確定) と `findAllEvents` のタイブレーク

### 3.1 連結順序 — **「ファイル名順」は不正確。正しくは「ディレクトリ群順 → 各群内でファイル名昇順」**

```
4530	export function auditShards(projectDir: string, intent?: string, space?: string): string[] {
4531	  const dirs: string[] = [];
4532	  if (intent === undefined && space !== undefined) {
4533	    dirs.push(join(spaceRecordRoot(projectDir, space), "audit"));
4534	  }
4535	  const intentDir = auditShardDir(projectDir, intent, space);
4536	  if (intentDir !== null && !dirs.includes(intentDir)) dirs.push(intentDir);
4537	  if (intentDir === null && dirs.length === 0) {
4538	    dirs.push(join(spaceRecordRoot(projectDir, space), "audit"));
4539	  }
4540	  const paths: string[] = [];
4541	  for (const shardDir of dirs) {
4542	    try {
4543	      assertNoSymlinkInChainOrThrow(projectDir, relative(projectDir, shardDir));
4544	    } catch {
4545	      continue;
4546	    }
4547	    let entries: string[];
4548	    try {
4549	      entries = readdirSync(shardDir);
4550	    } catch {
4551	      continue;
4552	    }
4553	    for (const file of entries.sort()) {
4554	      if (file.endsWith(".md")) paths.push(join(shardDir, file));
4555	    }
4556	  }
4557	  // Explicit-space aggregation keeps the resolved intent last for the few
4558	  // diagnostic paths that inspect the raw audit tail.
4558	  return paths;
4560	}
```

```
4568	export function readAllAuditShards(projectDir: string, intent?: string, space?: string): string {
4569	  const shards = auditShards(projectDir, intent, space);
4570	  if (shards.length === 0) return "";
4571	  const parts: string[] = [];
4572	  for (const path of shards) {
4573	    try {
4574	      const content = readAppendOnlyFileNoFollowOrThrow(path, "audit shard").toString("utf-8");
4575	      assertNoSymlinkInChainOrThrow(realpathSync(projectDir), relative(projectDir, path));
4576	      parts.push(content);
4577	    } catch {
4578	      // A vanished shard (ENOENT race) or a refused one (symlinked chain,
4579	      // wrong kind) — skip it. Growth during the read is NOT a failure here:
4580	      // the append-only reader tolerates it, so a live ledger being appended
4581	      // to no longer drops its whole shard from this merge.
4582	    }
4583	  }
4584	  return parts.join("\n");
4585	}
```

確定事項:

1. **`readAllAuditShards` は `auditShards()` の返り順をそのまま使う** (再ソートしない)。
2. **`auditShards()` はグローバルなファイル名ソートをしない。** ディレクトリを 2 群 (space レベル audit dir → intent audit dir) の順に走査し、**各ディレクトリ内でのみ `entries.sort()`** をかける。したがって「space 群のファイル名昇順」→「intent 群のファイル名昇順」という 2 段順序になる。`intent === undefined && space !== undefined` のときだけ space 群が前置される。
3. **連結子は `"\n"`** (`parts.join("\n")`) — `"\n---\n"` ではない。
4. 読取順の意味論は「シャード内は append 順、シャード横断は時刻順が**パーサの責務**」(`:4562-4567` のコメント)。
5. **各シャードに対して symlink 連鎖検査を 2 回行う** — `readAppendOnlyFileNoFollowOrThrow` の内部検査に加え、読取**後**に `assertNoSymlinkInChainOrThrow(realpathSync(projectDir), relative(projectDir, path))` を実行。`auditShards()` 側は projectDir を `realpathSync` せずに `assertNoSymlinkInChainOrThrow(projectDir, ...)` を呼んでおり、**2 箇所で第 1 引数の正規化が非対称**である。
6. **失敗シャードは黙ってスキップ** (catch 空)。読取中の成長は失敗としない。

なお `findAllEvents` 自身のコメント (`:7791-7792`) が *"readAllAuditShards concatenates per-clone shards in FILENAME order"* と書いているが、これは**単一ディレクトリ内に限れば真**という意味であり、2 群前置のケースを含めた全体順序の記述としては不正確 (upstream 側コメントの軽微な drift)。

### 3.2 `findAllEvents` のタイブレーク実装 (逐語)

```
7761	export function findAllEvents(
7762	  audit: string,
7763	  event: string,
7764	  slug?: string,
7765	): { timestamp: string; block: string }[] {
7766	  const results: { timestamp: string; block: string; pos: number }[] = [];
7767	  const blocks = audit.replace(/\r\n/g, "\n").split(/\n---\n/);
7768	  const eventRegex = new RegExp(`^\\*\\*Event\\*\\*:\\s*${escapeRegex(event)}\\s*$`, "m");
7769	  const slugRegex = slug
7770	    ? new RegExp(`^\\*\\*Bolt slug\\*\\*:\\s*${escapeRegex(slug)}\\s*$`, "m")
7771	    : null;
7772	  const tsRegex = /^\*\*Timestamp\*\*:\s*(\S+)/m;
7773	  let pos = 0;
7774	  for (const block of blocks) {
7775	    if (!eventRegex.test(block)) {
7776	      pos++;
7777	      continue;
7778	    }
7779	    if (slugRegex && !slugRegex.test(block)) {
7780	      pos++;
7781	      continue;
7782	    }
7783	    const tsMatch = block.match(tsRegex);
7784	    if (!tsMatch) {
7785	      pos++;
7786	      continue;
7787	    }
7788	    results.push({ timestamp: tsMatch[1], block, pos });
7789	    pos++;
7790	  }
```
```
7799	  results.sort((a, b) => {
7800	    if (a.timestamp !== b.timestamp) return a.timestamp < b.timestamp ? -1 : 1;
7801	    return a.pos - b.pos;
7802	  });
7803	  return results.map(({ timestamp, block }) => ({ timestamp, block }));
7804	}
```

確定事項:

- **`pos` は「連結バッファ全体のブロック索引」**。マッチしないブロックでもインクリメントされるので、結果配列の索引ではなくバッファ位置である。
- **時刻比較は文字列の辞書順** (`a.timestamp < b.timestamp`)。Date へのパースはしない。ISO-8601 の辞書順 = 時系列順という前提に依存。
- タイブレークは `a.pos - b.pos` (バッファ位置昇順)。
- `Timestamp` 抽出は非 global の `m` フラグ regex なので**ブロック内の最初の一致のみ**を取る (レガシー二重 Timestamp ブロックへの耐性)。
- CRLF は分割前に LF へ正規化する。
- `**Event**` / `**Bolt slug**` の値側は `\s*$` でアンカーされ、イベント名は `escapeRegex` される。

---

## 4. `readAppendOnlyFileNoFollowOrThrow` の全拒否文言

```
7521	export function readAppendOnlyFileNoFollowOrThrow(path: string, what: string): Buffer {
7522	  let fd: number;
7523	  try {
7524	    const noFollow = typeof fsConstants.O_NOFOLLOW === "number" ? fsConstants.O_NOFOLLOW : 0;
7525	    fd = openSync(path, fsConstants.O_RDONLY | noFollow | fsConstants.O_NONBLOCK);
7526	  } catch (e) {
7527	    const code = (e as NodeJS.ErrnoException).code;
7528	    if (code === "ELOOP") {
7529	      throw new Error(`${what} is a symlink, which is not followed: ${path}`);
7530	    }
7531	    const err = new Error(`${what} could not be opened: ${path} (${errorMessage(e)})`);
7532	    (err as NodeJS.ErrnoException).code = code;
7533	    throw err;
7534	  }
7535	  try {
7536	    const st = fstatSync(fd);
7537	    if (!st.isFile()) {
7538	      throw new Error(`${what} is not a regular file: ${path}`);
7539	    }
7540	    // Fallback for platforms without O_NOFOLLOW, and a pathname/descriptor
7541	    // identity check for races on every platform.
7542	    if (lstatSync(path).isSymbolicLink()) {
7543	      throw new Error(`${what} is a symlink, which is not followed: ${path}`);
7544	    }
7545	    const current = statSync(realpathSync(path));
7546	    if (current.dev !== st.dev || current.ino !== st.ino) {
7547	      throw new Error(`${what} changed while opening: ${path}`);
7548	    }
7549	    return readFileSync(fd);
7550	  } finally {
7551	    closeSync(fd);
7552	  }
7553	}
```

**拒否文言は 4 種** (symlink 文言は 2 経路で同一文字列):

| # | 文言テンプレート | 契機 |
| --- | --- | --- |
| 1 | `${what} is a symlink, which is not followed: ${path}` | open が `ELOOP` |
| 2 | `${what} could not be opened: ${path} (${errorMessage(e)})` | 上記以外の open 失敗。**`err.code` に元の errno コードを転記して throw する** |
| 3 | `${what} is not a regular file: ${path}` | `fstat` が regular file を報告しない (**「非 regular」の逐語はこれ**) |
| 4 | `${what} is a symlink, which is not followed: ${path}` | `lstat` フォールバック (O_NOFOLLOW 非対応プラットフォーム向け) |
| 5 | `${what} changed while opening: ${path}` | `realpath` 経由 `stat` の `dev`/`ino` が fd と不一致 |

**`<what>` の実引数語彙 (`aidlc-lib.ts` 内)**: **`"audit shard"` の 1 語のみ** (`:3786`, `:4574`, `:4610-4612` の 3 サイト)。`aidlc-audit.ts` / `aidlc-bolt.ts` には呼出サイトが 1 つも無い (grep 済み)。他ツール (`aidlc-state.ts` 等) は本タスクの担当外のため未確認。

*(注: `errorMessage(e)` は本ファイル内のヘルパ。第 2 文言のみ動的部分が 2 つある。)*

---

## 5. `slugify` / `isoTimestamp` の完全実装

### 5.1 `slugify` (逐語)

```
1713	// Deterministic free-text → SLUG_RE-valid kebab: lowercase; non-alphanumerics →
1714	// hyphens; collapse + trim hyphens; cap length; ensure a leading letter. Pure +
1715	// idempotent (slugify(slugify(x)) === slugify(x)). Falls back to "intent" when
1716	// the input reduces to empty.
1717	export function slugify(text: string, maxLength = 48): string {
1718	  let s = text
1719	    .toLowerCase()
1720	    .replace(/[^a-z0-9]+/g, "-")
1721	    .replace(/^-+|-+$/g, "")
1722	    .slice(0, maxLength)
1723	    .replace(/-+$/g, "");
1724	  // Ensure a leading LETTER (SLUG_RE = /^[a-z][a-z0-9-]*$/).
1725	  if (!/^[a-z]/.test(s)) s = `intent-${s}`.replace(/-+$/g, "");
1726	  if (s.length === 0) s = "intent";
1727	  return s;
1728	}
```

適用順序 (この順序が仕様): ① `toLowerCase` → ② `[^a-z0-9]+` を単一 `-` へ (**連続潰しはここで同時に起こる**) → ③ 先頭末尾の `-` を trim → ④ **`slice(0, maxLength)`** → ⑤ **切詰めで露出した末尾 `-` を再 trim** → ⑥ 先頭が `[a-z]` でなければ `intent-` を前置してから再度末尾 `-` trim → ⑦ 空なら `"intent"`。
既定 `maxLength = 48`。intent dir 名は `slugify(label, 24)` (`:1766`)。
**`toLowerCase` は Unicode 対応なので、非 ASCII は ② で `-` に潰れる。** 数字始まりの入力は ⑥ で `intent-` が付く (例: `slugify("42")` → `"intent-42"`)。

関連 (`:1754-1766`):

```
1754	export function dateStamp(date: Date = new Date()): string {
1755	  const yy = String(date.getUTCFullYear()).slice(-2);
1756	  const mm = String(date.getUTCMonth() + 1).padStart(2, "0");
1757	  const dd = String(date.getUTCDate()).padStart(2, "0");
1758	  return `${yy}${mm}${dd}`;
1759	}
```
```
1765	export function intentDirNameBase(label: string, date: Date = new Date()): string {
1766	  return `${dateStamp(date)}-${slugify(label, 24)}`;
1767	}
```

### 5.2 `isoTimestamp` (逐語)

```
9871	// --- Timestamp ---
9872	
9873	export function isoTimestamp(): string {
9874	  return new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
9875	}
```

- **タイムゾーンは常に UTC** (`Date.prototype.toISOString` の定義により `Z` 固定)。
- **パディングは `toISOString` に丸投げ** — 独自のゼロ埋めは無い。`YYYY-MM-DDTHH:mm:ssZ` の 20 文字。
- ミリ秒 3 桁 (`.\d{3}`) を除去して秒精度化。**正規表現は `\.\d{3}Z$` にアンカーされているので、年が 4 桁を超える拡張表記 (`+YYYYYY-...`) でも末尾は同じ形なので置換は成立する。**
- **秒精度であることが監査行の順序契約 (`findAllEvents` のタイブレーク、`humanActedSinceGate` の fail-closed) の前提。**

---

## 6. `classifyStateVersion` の完全実装

```
10604	/** The current state-graph schema version. Bump when the graph adds/renames/removes rows. */
10605	export const CURRENT_STATE_VERSION = "8";
10606	
10607	export type StateVersionClassification =
10608	  | { kind: "ok" }
10609	  | { kind: "unparseable"; message: string }
10610	  | { kind: "past"; version: string; message: string }
10611	  | { kind: "future"; version: string; message: string };
```

```
10627	export function classifyStateVersion(stateContent: string): StateVersionClassification {
10628	  const unparseableMessage =
10629	    "Incompatible workflow state: the State Version field is missing, empty, " +
10630	    "or unparseable in aidlc-state.md, so this state cannot be matched to the " +
10631	    `current v${CURRENT_STATE_VERSION} stage graph and cannot be advanced safely. ` +
10632	    "Archive your workspace ('mv aidlc aidlc.archive') and start a fresh " +
10633	    "workflow (describe what to build), or finish this workflow on the prior " +
10634	    "shell. Run `/aidlc --doctor` for the full diagnosis.";
10635	  // Anchor the tail with `[ \t]*$`: the schema token is a bare integer with
10636	  // no trailing content on the line, so `State Version: 8 garbage` fails to
10637	  // match and falls into the unparseable branch.
10638	  const versionMatch = stateContent.match(/^- \*\*State Version\*\*:[ \t]*(\S+)[ \t]*$/m);
10639	  if (versionMatch === null) return { kind: "unparseable", message: unparseableMessage };
10640	  const v = versionMatch[1];
10641	  if (!/^\d+$/.test(v)) return { kind: "unparseable", message: unparseableMessage };
10642	  if (v === CURRENT_STATE_VERSION) return { kind: "ok" };
10643	  if (Number(v) > Number(CURRENT_STATE_VERSION)) {
10644	    return {
10645	      kind: "future",
10646	      version: v,
10647	      message:
10648	        `Incompatible workflow state: State Version ${v} is newer than the ` +
10649	        `current v${CURRENT_STATE_VERSION} stage graph this build understands, so ` +
10650	        "it cannot be advanced safely. Upgrade the framework to a build that ships " +
10651	        `state schema v${v} (or newer), or finish this workflow on the shell that ` +
10652	        "produced it. Run `/aidlc --doctor` for the full diagnosis.",
10653	    };
10654	  }
10655	  return {
10656	    kind: "past",
10657	    version: v,
10658	    message:
10659	      `Incompatible workflow state: State Version ${v} predates the current ` +
10660	      `v${CURRENT_STATE_VERSION} stage graph. v8 renamed the Inception ` +
10661	      "`application-design` stage to `domain-design` and inserted " +
10662	      "`contract-design`, so this state's stage rows no longer match the graph " +
10663	      "and cannot be advanced safely. Archive your workspace " +
10664	      `('mv aidlc aidlc.v${v}-archive') and start a fresh workflow (describe what ` +
10665	      "to build), or finish this workflow on the prior shell. Run `/aidlc --doctor` " +
10666	      "for the full diagnosis.",
10667	  };
10668	}
```

### 非整数比較の意味論 (確定)

**「非整数の比較」は起こらない。** 順序比較 `Number(v) > Number(CURRENT_STATE_VERSION)` に到達する前に **`/^\d+$/.test(v)` で非負整数十進表記でないものは全て `unparseable`** に落ちる。したがって:

- `"08"` → regex 通過 → `"08" === "8"` は偽 → `Number("08") = 8 > 8` は偽 → **`past` に分類される** (`version: "08"`、メッセージ内も `"08"` で補間、archive 先は `aidlc.v08-archive`)。**これが唯一の「同値だが past 判定」ケース**。
- `"8.0"` / `"-1"` / `"+8"` / `"8 "` / `"０８"` (全角) → `unparseable`。
- `"9999999999999999999999"` → regex 通過 → `Number(...)` は精度落ちするが `> 8` は真 → `future`。
- 空値 (`- **State Version**:` の後が空) → `(\S+)` が一致しないので `versionMatch === null` → `unparseable`。
- `- **State Version**: 8 garbage` → 末尾 `[ \t]*$` アンカーにより一致せず → `unparseable`。

### 4 分類の逐語文言 (上記コードブロックが正本)。展開形:

- **unparseable**: `Incompatible workflow state: the State Version field is missing, empty, or unparseable in aidlc-state.md, so this state cannot be matched to the current v8 stage graph and cannot be advanced safely. Archive your workspace ('mv aidlc aidlc.archive') and start a fresh workflow (describe what to build), or finish this workflow on the prior shell. Run `/aidlc --doctor` for the full diagnosis.`
- **future**: `Incompatible workflow state: State Version <v> is newer than the current v8 stage graph this build understands, so it cannot be advanced safely. Upgrade the framework to a build that ships state schema v<v> (or newer), or finish this workflow on the shell that produced it. Run `/aidlc --doctor` for the full diagnosis.`
- **past**: `Incompatible workflow state: State Version <v> predates the current v8 stage graph. v8 renamed the Inception `application-design` stage to `domain-design` and inserted `contract-design`, so this state's stage rows no longer match the graph and cannot be advanced safely. Archive your workspace ('mv aidlc aidlc.v<v>-archive') and start a fresh workflow (describe what to build), or finish this workflow on the prior shell. Run `/aidlc --doctor` for the full diagnosis.`
- **ok**: メッセージ無し (`{ kind: "ok" }` のみ)。

`CURRENT_STATE_VERSION` は **文字列 `"8"`** (数値ではない)。

---

## 7. checkbox の suffix writer (`setStageSuffix`)

```
6727	// The suffix-setter twin of setCheckbox: flips ONE stage line's plan suffix
6728	// (the em-dash EXECUTE/SKIP tail the router's override channel reads)
6729	// in either direction, leaving the checkbox marker untouched. setCheckbox owns
6730	// the marker (run-state); this owns the suffix (the plan) - the two edit
6731	// disjoint fields of the same line, so recompose and jump compose cleanly.
6732	// Returns the content unchanged when the slug has no stage line.
6733	export function setStageSuffix(
6734	  content: string,
6735	  slug: string,
6736	  action: "EXECUTE" | "SKIP"
6737	): string {
6738	  const regex = new RegExp(
6739	    `^(- \\[[ xSR?-]\\] ${escapeRegex(slug)}\\s*—\\s*)(EXECUTE|SKIP)\\b`,
6740	    "m"
6741	  );
6742	  return content.replace(regex, `$1${action}`);
6743	}
```

対になる `setCheckbox` と `parseCheckboxes`:

```
6678	export function parseCheckboxes(content: string): CheckboxLine[] {
6679	  const results: CheckboxLine[] = [];
6680	  const regex = /^- \[([ xSR?-])\] (\S+)\s*—\s*(.*)$/gm;
```
```
6713	export function setCheckbox(
6714	  content: string,
6715	  slug: string,
6716	  newState: CheckboxState
6717	): string {
6718	  const marker = CHECKBOX_MAP[newState];
6719	  // Match any checkbox state for this slug
6720	  const regex = new RegExp(
6721	    `^(- )\\[[ xSR?-]\\]( ${escapeRegex(slug)} —)`,
6722	    "m"
6723	  );
6724	  return content.replace(regex, `$1${marker}$2`);
6725	}
```

直列化の確定事項:

- **`(EXECUTE|SKIP)\b` にのみ一致し、`\b` 直後 (例 `SKIP: reason` の `:` 以降) は保存される。** すなわち `SKIP: not in scope` → `EXECUTE` に flip すると `EXECUTE: not in scope` が残る。**理由節を消さない**のが実装挙動。
- 置換は `"m"` フラグ (非 global) なので **最初の一致行 1 行のみ**。
- **一致しなければ content をそのまま返す (silent no-op)。**
- **区切りは em dash `—` (U+2014)**。`setStageSuffix` は `\s*—\s*`、`parseCheckboxes` も `\s*—\s*` だが、**`setCheckbox` だけは正確に `" <slug> —"` (半角空白 + em dash) を要求する** — 3 者で許容度が非対称。
- マーカークラスは 3 箇所とも `[ xSR?-]` (6 値: 空白 / `x` / `S` / `R` / `?` / `-`)。
- `CHECKBOX_MAP` (`:112-119`): `pending:"[ ]"`, `"in-progress":"[-]"`, `"awaiting-approval":"[?]"`, `revising:"[R]"`, `completed:"[x]"`, `skipped:"[S]"`。
- `escapeRegex` (`:10038-10040`): `str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")` — **`-` はエスケープ対象外** (文字クラス外なので問題なし)。

---

## 8. `loadStageGraph` のエラー逐語 / scope metadata loader の逐語

### 8.1 行番号の訂正

**研究文書 `workflow-definition-graph-reader.md:221, 268` が挙げる `aidlc-lib.ts:2837-2864` / `:2841-2860` は v2.2.0 の行番号であり、ピン留め `3c3146cf` では別物 (active-directive ロックトランザクションと `freshActiveDirectiveMarker`)。** 正しい位置は `loadStageGraph` = `:8552`、`loadStageGraphAll` = `:8558`、エラー逐語 = `:8565-8581`。

### 8.2 グラフ読込のエラー逐語 (逐語)

```
8552	export function loadStageGraph(): StageEntry[] {
8553	  if (_stageGraph !== null) return _stageGraph;
8554	  _stageGraph = loadStageGraphAll().filter((s) => s.enabled !== false);
8555	  return _stageGraph;
8556	}
8557	
8558	export function loadStageGraphAll(): StageEntry[] {
8559	  if (_stageGraphAll !== null) return _stageGraphAll;
8560	  const p = stageGraphPath();
8561	  let raw: string;
8562	  try {
8563	    raw = readFileSync(p, "utf-8");
8564	  } catch (err) {
8565	    const hint = process.env.AIDLC_STAGE_GRAPH
8566	      ? `AIDLC_STAGE_GRAPH points to ${p}; unset it to use the default.`
8567	      : "Reinstall the framework or re-run setup to restore the data file.";
8568	    throw new Error(
8569	      `Stage graph not readable at ${p}: ${errorMessage(err)}. ${hint}`
8570	    );
8571	  }
8572	  let parsed: StageEntry[];
8573	  try {
8574	    // JSON.parse returns `any`; we trust the on-disk schema (project-controlled
8575	    // data file written by the framework, not user input). Phase E will
8576	    // replace this trust boundary with an isStageEntryArray() type guard.
8577	    parsed = JSON.parse(raw);
8578	  } catch (err) {
8579	    throw new Error(
8580	      `Stage graph at ${p} is not valid JSON: ${errorMessage(err)}`
8581	    );
8582	  }
8583	  _stageGraphAll = parsed;
8584	  return parsed;
8585	}
```

**エラー逐語は 3 形**(hint 分岐込み) — 研究文書 §5.5 の 3 行と**バイト一致で確認**:

```text
Stage graph not readable at ${p}: ${errorMessage(err)}. Reinstall the framework or re-run setup to restore the data file.
Stage graph not readable at ${p}: ${errorMessage(err)}. AIDLC_STAGE_GRAPH points to ${p}; unset it to use the default.
Stage graph at ${p} is not valid JSON: ${errorMessage(err)}
```

hint 分岐条件も逐語で確認: `process.env.AIDLC_STAGE_GRAPH` が truthy のときだけ後者 (**空文字列は falsy なので既定 hint に落ちる**)。

`stageGraphPath()` / `scopeGridPath()` (`:8509-8518`):

```
8509	function stageGraphPath(): string {
8510	  return process.env.AIDLC_STAGE_GRAPH ?? join(resolveDataDir(), "stage-graph.json");
8511	}
```
```
8516	export function scopeGridPath(): string {
8517	  return process.env.AIDLC_SCOPE_GRID ?? join(resolveDataDir(), "scope-grid.json");
8518	}
```

**`??` (nullish coalescing) なので空文字列の env はそのまま採用される** — hint 判定 (`? :` = truthy 判定) との非対称が実装上存在する。`resolveDataDir()` = `resolveHarnessPath(["tools","data"])` (`:8497-8499`)。

グリッドのフォールバック (`:8634-8641`) — **`scope-grid.json` は throw しない**:

```
8634	function loadScopeGridForMapping(): ScopeGridForMapping {
8635	  const p = scopeGridPath();
8636	  try {
8637	    return JSON.parse(readFileSync(p, "utf-8")) as ScopeGridForMapping;
8638	  } catch {
8639	    return transposeScopeGridForMapping(loadStageGraph());
8640	  }
8641	}
```
```
8618	function transposeScopeGridForMapping(stages: StageEntry[]): ScopeGridForMapping {
8619	  const scopeNames = new Set<string>();
8620	  for (const stage of stages) {
8621	    for (const name of stage.scopes ?? []) scopeNames.add(name);
8622	  }
8623	  const grid: ScopeGridForMapping = {};
8624	  for (const scope of [...scopeNames].sort()) {
8625	    const stagesMap: Record<string, "EXECUTE" | "SKIP"> = {};
8626	    for (const stage of stages) {
8627	      stagesMap[stage.slug] = (stage.scopes ?? []).includes(scope) ? "EXECUTE" : "SKIP";
8628	    }
8629	    grid[scope] = { stages: stagesMap };
8630	  }
8631	  return grid;
8632	}
```

**`initialization` フェーズの特例は `transposeScopeGridForMapping` に存在しない** — 純粋な `scopes.includes(scope)` のみ。ファイル内の `initialization` 特例は `stageEnabledBySelection` (`:456`、プラグイン選択の免除) と summary-confirmation の免除 (`:4146`) のみ。

`Unknown scope: "…". Valid scopes: …` の逐語は **`aidlc-lib.ts` には存在しない** (`:8901` に *"own validation owns the canonical `Unknown scope` error."* というコメントのみ)。`aidlc-graph.ts` 側の担当であり、本タスクでは未採取。

### 8.3 scope metadata loader の逐語 (全 6 形、逐語)

```
8643	export function loadScopeMetadataAll(): Record<string, ScopeMetadata> {
8644	  if (_scopeMetadataAll !== null) return _scopeMetadataAll;
8645	  const dir = scopesDir();
8646	  const out: Record<string, ScopeMetadata> = {};
8647	  const nameToFile = new Map<string, string>();
8648	  let files: string[];
8649	  try {
8650	    // Sort so readdirSync order is platform-independent — the derived
8651	    // scope set + the designer-export `scopes` key order stay deterministic
8652	    // across machines (same discipline as loadAgents()).
8653	    files = readdirSync(dir).filter((f) => f.endsWith(".md")).sort();
8654	  } catch {
8655	    files = [];
8656	  }
8657	  for (const f of files) {
8658	    const filePath = join(dir, f);
8659	    const body = readFileSync(filePath, "utf-8");
8660	    const fm = frontmatterBlock(body);
8661	    if (fm === null) throw new Error(`Scope file missing frontmatter: ${filePath}`);
8662	    const name = scalarField(fm, "name");
8663	    if (!name) throw new Error(`Scope file ${filePath} missing required frontmatter: name`);
8664	    const previousFile = nameToFile.get(name);
8665	    if (previousFile) {
8666	      throw new Error(
8667	        `Duplicate scope name "${name}" in ${filePath}: already declared in ${previousFile}. Rename one of them.`
8668	      );
8669	    }
8670	    nameToFile.set(name, filePath);
8671	    const meta: ScopeMetadata = {
8672	      name,
8673	      depth: scalarField(fm, "depth"),
8674	      description: scalarField(fm, "description"),
8675	      keywords: listField(fm, "keywords"),
8676	      skeleton: false,
8677	    };
8678	    const plugin = scalarField(fm, "plugin");
8679	    if (plugin) {
8680	      // `aidlc-` is core's namespace: scope-runner dirs are `aidlc-<name>` for
8681	      // core scopes but the bare name for plugin scopes, so an aidlc--prefixed
8682	      // plugin would land its runner on a core path and silently clobber it
8683	      // (same invariant compile enforces for stage frontmatter).
8684	      if (plugin.startsWith("aidlc-")) {
8685	        throw new Error(
8686	          `Scope file ${filePath} declares plugin "${plugin}"; the "aidlc-" prefix is reserved for core (it collides with core runner paths). Rename the plugin.`
8687	        );
8688	      }
8689	      meta.plugin = plugin;
8690	    }
8691	    const ts = scalarField(fm, "testStrategy");
8692	    if (ts) meta.testStrategy = ts;
8693	    const runner = scalarField(fm, "runner");
8694	    if (runner === "true" || runner === "false") meta.runner = runner === "true";
8695	    const skeleton = scalarField(fm, "skeleton");
8696	    if (skeleton) {
8697	      if (skeleton !== "on" && skeleton !== "off") {
8698	        throw new Error(
8699	          `Scope file ${filePath} has invalid skeleton value "${skeleton}". Expected "on" or "off".`
8700	        );
8701	      }
8702	      meta.skeleton = skeleton === "on";
8703	    }
8704	    if (scalarField(fm, "freeform_default") === "true") meta.freeformDefault = true;
8705	    const reviewCap = scalarField(fm, "review_cap");
8706	    if (reviewCap) {
8707	      if (
8708	        reviewCap !== "adversarial" &&
8709	        reviewCap !== "advisory" &&
8710	        reviewCap !== "none"
8711	      ) {
8712	        throw new Error(
8713	          `Scope file ${filePath} has invalid review_cap value "${reviewCap}". Expected "adversarial", "advisory", or "none".`
8714	        );
8715	      }
8716	      meta.reviewCap = reviewCap;
8717	    }
8718	    out[name] = meta;
8719	  }
8720	  _scopeMetadataAll = out;
8721	  return out;
8722	}
```

`freeform_default` の一意性検査は **`loadScopeMetadataAll` ではなく `loadScopeMetadata` 側** (= プラグイン選択でフィルタした「有効な」集合に対して) にある:

```
8772	export function loadScopeMetadata(): Record<string, ScopeMetadata> {
8773	  if (_scopeMetadata !== null) return _scopeMetadata;
8774	  const all = loadScopeMetadataAll();
8775	  const selected = pluginsEnabled();
8776	  const enabled: Record<string, ScopeMetadata> = {};
8777	  for (const [name, meta] of Object.entries(all)) {
8778	    const owner = meta.plugin ?? "aidlc";
8779	    if (selected === null || selected.has(owner)) enabled[name] = meta;
8780	  }
8781	  const nominated = Object.values(enabled)
8782	    .filter((meta) => meta.freeformDefault === true)
8783	    .map((meta) => meta.name)
8784	    .sort();
8785	  if (nominated.length > 1) {
8786	    throw new Error(
8787	      `Multiple enabled scopes declare freeform_default: true (${nominated.join(", ")}). ` +
8788	        "At most one enabled scope may nominate the freeform default."
8789	    );
8790	  }
8791	  _scopeMetadata = enabled;
8792	  return enabled;
8793	}
```

**逐語文言 6 形 (展開)**:

```text
Scope file missing frontmatter: ${filePath}
Scope file ${filePath} missing required frontmatter: name
Duplicate scope name "${name}" in ${filePath}: already declared in ${previousFile}. Rename one of them.
Scope file ${filePath} declares plugin "${plugin}"; the "aidlc-" prefix is reserved for core (it collides with core runner paths). Rename the plugin.
Scope file ${filePath} has invalid skeleton value "${skeleton}". Expected "on" or "off".
Scope file ${filePath} has invalid review_cap value "${reviewCap}". Expected "adversarial", "advisory", or "none".
Multiple enabled scopes declare freeform_default: true (${names.join(", ")}). At most one enabled scope may nominate the freeform default.
```

(計 7 形。研究文書の「skeleton」「review_cap」「name 重複」「plugin prefix」「freeform_default」の 5 形 + missing frontmatter / missing name の 2 形。)

補足契約 (研究文書に無かった実装詳細):

- **`readdirSync(dir)` の失敗は `files = []` に劣化する (throw しない)。** 一方 **個別 `.md` の `readFileSync` は try/catch されていないので raw ENOENT/EACCES がそのまま伝播する。**
- ファイルは **`.md` で絞ったうえで `.sort()`** — プラットフォーム非依存の決定性を明記。
- `scalarField` は不在時 **空文字列 `""` を返す** (null ではない)。`depth` / `description` は空文字列でも黙って通る。`name` だけ `if (!name)` で拒否。
- `runner` は `"true"` / `"false"` の**厳密 2 値以外は無視** (キー自体を設定しない)。
- `freeform_default` は `=== "true"` の厳密比較。
- frontmatter パーサは手書き: `frontmatterBlock` (`:9043-9046`, `/^---\r?\n([\s\S]*?)\r?\n---/`)、`scalarField` (`:9055-9068`, YAML 折返し記号 `>` `|` `>-` `|-` を拒否、前後の引用符を剥がす)、`listField` (`:9078-9092`, インデント `- ` 行のみ、ダッシュ後の空白 1 つ以上を必須)。**12-workflow-definition §3.3 の「frontmatter パーサは手書き」規範は upstream 実装と一致。**

---

## 9. `resolveReviewClass` の完全実装 (3 段単調下方解決)

```
8732	export const REVIEW_CLASSES = ["none", "advisory", "adversarial"] as const;
8733	export type ReviewClass = (typeof REVIEW_CLASSES)[number];
8734	
8735	const REVIEW_RANK: Record<ReviewClass, number> = {
8736	  none: 0,
8737	  advisory: 1,
8738	  adversarial: 2,
8739	};
8740	
8741	function asReviewClass(v: string | null | undefined): ReviewClass | null {
8742	  return v === "none" || v === "advisory" || v === "adversarial" ? v : null;
8743	}
```

```
8745	/** The effective review class for one stage run. `stageClass` is the compiled
8746	 *  node's review_class (undefined when the stage declares no reviewer -
8747	 *  resolves to "none"). `scope` names the active scope (its review_cap is
8748	 *  read from scope metadata; unknown scope or absent cap = no cap).
8749	 *  `stateContent` supplies the per-run `Review Override` field when present.
8750	 *  An override or cap can only LOWER the stage's declared class, never raise
8751	 *  it: min() everywhere, so `--review adversarial` on an advisory stage keeps
8752	 *  advisory, and neither can revive a reviewer the stage never declared. */
8753	export function resolveReviewClass(
8754	  stageClass: string | undefined,
8755	  scope: string,
8756	  stateContent?: string | null
8757	): ReviewClass {
8758	  const declared = asReviewClass(stageClass);
8759	  if (declared === null) return "none"; // no reviewer on the stage
8760	  let effective: ReviewClass = declared;
8761	  const cap = loadScopeMetadata()[scope]?.reviewCap;
8762	  if (cap && REVIEW_RANK[cap] < REVIEW_RANK[effective]) effective = cap;
8763	  const override = asReviewClass(
8764	    stateContent ? getField(stateContent, "Review Override") : null
8765	  );
8766	  if (override && REVIEW_RANK[override] < REVIEW_RANK[effective]) {
8767	    effective = override;
8768	  }
8769	  return effective;
8770	}
```

3 段の解決順序と意味論 (確定):

| 段 | 入力 | 規則 |
| --- | --- | --- |
| 1 | `stageClass` (コンパイル済みノードの `review_class`) | `asReviewClass` で 3 値以外は `null` → **即 `"none"` を返して終了** (cap/override を一切見ない)。未宣言ステージにレビュアーを蘇生させない |
| 2 | scope metadata の `reviewCap` | `cap && RANK[cap] < RANK[effective]` のときのみ下げる。**未知スコープ (`?.` で undefined) / cap 不在 = cap 無し** |
| 3 | state ファイルの `Review Override` | `getField(stateContent, "Review Override")` を `asReviewClass` に通す。3 値以外や不在は `null` = 無効。`RANK[override] < RANK[effective]` のときのみ下げる |

- **順位は `none: 0 < advisory: 1 < adversarial: 2`。すべて strict `<` の min 適用なので単調下方のみ。同値では代入しないが結果は同じ。**
- `stateContent` が `null`/`undefined`/空文字列のとき `getField` を呼ばずに `null` を渡す (**空文字列も falsy なので `getField` は呼ばれない**)。
- **`loadScopeMetadata()` は「有効な (プラグイン選択で残った)」集合。無効化されたスコープの cap は適用されない。**

---

## 10. `acquireAuditLock` のループ実装 — reap 成功時の予算消費

```
7138	export function acquireAuditLock(
7139	  projectDir: string,
7140	  maxRetries = 50,
7141	  retryMs = 100,
7142	  intent?: string,
7143	  space?: string,
7144	  reapLiveOwnerAfterStale = true,
7145	): boolean {
7146	  const lockDir = auditLockDir(projectDir, intent, space);
7147	  for (let i = 0; i <= maxRetries; i++) {
7148	    try {
7149	      mkdirSync(lockDir);
7150	      writeOwnerStamp(lockDir, reapLiveOwnerAfterStale);
7151	      return true;
7152	    } catch {
7153	      // EEXIST: someone holds it. Before sleeping, try to reap a dead/stale
7154	      // holder so a SIGKILL'd owner doesn't wedge every waiter for the full
7155	      // retry budget. If we reap, retry the mkdir immediately (next loop turn).
7156	      if (reapStaleLock(lockDir)) {
7157	        try {
7158	          mkdirSync(lockDir);
7159	          writeOwnerStamp(lockDir, reapLiveOwnerAfterStale);
7160	          return true;
7161	        } catch {
7162	          // another waiter beat us to the freed dir — fall through to sleep
7163	        }
7164	      }
7165	      if (i < maxRetries) {
7166	        Bun.sleepSync(retryMs);
7167	      }
7168	    }
7169	  }
7170	  return false;
7171	}
```

**「reap 成功時の予算非消費」の正確な意味 (確定)**:

- reap 成功時、upstream は **`continue` せず、同じイテレーション内でインラインに 2 回目の `mkdirSync` を試みる**。成功すれば即 `return true` — **その周回の予算 (`i`) は消費されるが、sleep は発生しない**。
- **インライン mkdir が失敗した場合は `continue` せずそのまま下へ落ち、`Bun.sleepSync(retryMs)` を実行して `i++`。つまり予算は消費される。**
- したがって「reap に成功すれば予算を無限に温存できる」わけではない。**reap 成功は「sleep を 1 回省ける」だけであり、ループ回数の上限 `maxRetries + 1` は常に有効**。これがライブロック防止になっている。
- 総 mkdir 試行回数の上限は `(maxRetries + 1) × 2` (毎周回 2 回試みうる)。総 sleep 回数は最大 `maxRetries` 回 (最終周回 `i === maxRetries` では sleep しない)。
- **`mkdirSync(lockDir)` に `mode` 指定は無い** (既定 `0o777 & ~umask`)。比較: `acquireActiveDirectiveLock` (`:7109`) は `{ mode: 0o700 }` を指定する — **audit ロックと active-directive ロックでディレクトリ権限が異なる**。
- **`writeOwnerStamp` の失敗は無視される** (戻り値を捨てている) — スタンプ無しでも acquire は成功する。
- 呼出側の翻訳文言は 2 系統存在する:
  - `aidlc-audit.ts:543` / `:782` / `:897` / `:1150` → `Failed to acquire audit lock after retries`
  - `aidlc-lib.ts:7599` (`withAuditLock`) → `` `Failed to acquire audit lock for ${key} after retries` `` (**key = identity 文字列を含む別文言**)
  - `aidlc-audit.ts:1375` (`audit-merge`) → `` `Failed to acquire audit lock after ${lockRetries} × ${lockRetryMs}ms = ${(lockRetries*lockRetryMs/1000).toFixed(1)}s retries; another merge in flight?` ``

---

## 11. amadeus-ng の現実装との食い違い (全列挙)

### A. `modules/core/interface-adapter/src/workspace/fs_workspace_lock.rs`

| # | 箇所 | 現実装 | upstream `3c3146cf` | 判定 |
| --- | --- | --- | --- | --- |
| **A1** | `:45-57` `impl PartialEq for OwnerStamp` (`TODO(golden)` 付) | `pid` + `started_at_ms` の 2 フィールドのみ比較。コメントで *「`reapLiveOwnerAfterStale` は保持者が宣言するポリシーであって同一性の一部ではない」* と明記 | `stampMatches` (`:6972-6977`) は **`pid` / `startedAtMs` / `reapLiveOwnerAfterStale` / `token` の 4 フィールドを比較**。ポリシーフラグも同一性の一部 | **食い違い (確定)**。`TODO(golden)` の想定と逆。ユニットテスト `owner_stamp_equality_is_holder_identity_not_policy` (`:305-326`) は upstream 挙動を否定するアサーションになっている |
| **A2** | `try_reap` (`:164-229`) | `reap_eligible(alive, age, stale_ms)` のみを見て `reap_live_owner_after_stale` を**一切読まない** | `:7037` `if (!owner.reapLiveOwnerAfterStale) return false;` — **生存所有者に対してのみだが確実に読む** | **食い違い**。`reapLiveOwnerAfterStale: false` を宣言した長時間ジョブ (リポジトリクローン等) が閾値超過で奪われる |
| **A3** | `try_reap` の `matches` (`:213-217`) `(None, None) => true` | rename 後に「未スタンプのまま」だけを確認 | `stampMatches(dir, null)` (`:6962-6970`) は「未スタンプのまま **かつ** mtime が今も grace 超過」の 2 条件。競合者が隙間で再 mkdir した新鮮な未スタンプ dir は mtime がリセットされて不一致になり **restore される** | **食い違い**。CAS の防御が 1 段弱い |
| **A4** | `try_reap` の rename 失敗 (`:204-207`) | `return true` (「lock_dir が空いた可能性が高いので mkdir 再試行させる」) | `:7047-7049` `catch { return false; }` | **食い違い**。しかも `acquire` 側が `try_reap == true` で `continue` (予算非消費) するため、rename が恒常的に失敗する条件 (EACCES 等) で**無限ループ**になりうる |
| **A5** | `acquire` の reap 成功パス (`:256-258`) | `continue` — 予算を一切消費せず `create_dir` 再試行 | reap 成功 → 同一周回でインライン mkdir、失敗したら sleep + `i++` (**予算消費**) | **食い違い (ライブネス)**。上限が保証されない |
| **A6** | `write_owner_stamp` (`:154-161`) + `acquire` (`:248-251`) | `io::Result` を返し、失敗時 `AcquireError::Io` で acquire を失敗させる。**その際 `create_dir` 済みのロック dir が残置される** | `:6843-6854` **best-effort。例外を握り潰して `null` を返し、acquire は成功する** (*"a missing stamp degrades the reaper to age-only on the next waiter … never to incorrectness"*) | **食い違い**。upstream は「未スタンプでも保持は成立」、現実装は「失敗 + dir リーク」 |
| **A7** | `serialize_owner_stamp` (`:59-64`) | `format!("{{\"pid\":{},\"startedAtMs\":{},\"reapLiveOwnerAfterStale\":{}}}")` | `JSON.stringify({pid, startedAtMs, reapLiveOwnerAfterStale})` = 同一キー順・同一形 | **一致** (キー順・インデント無し・末尾改行無しまで一致) |
| **A8** | `fs::write(lock_dir.join("owner.json"), …)` (`:160`) | パーミッション指定なし (既定 `0o666 & ~umask`) | `writeFileSync(..., { mode: 0o600 })` | **食い違い (軽微)**。11-workspace §9 が「stage-0/1 併用期はロックの物理形式も互換維持」と規定している以上、逸脱台帳マター |
| **A9** | `parse_owner_stamp` (`:68-77`) | `s.contains("\"reapLiveOwnerAfterStale\":true")` で真偽を判定 | `readOwnerStamp` (`:6866`) は **`parsed.reapLiveOwnerAfterStale !== false`** — すなわち**キー欠落・非 boolean は `true` 扱い** (*"Older stamps have no field and retain the historical over-age reaping."*) | **食い違い**。現実装はキー欠落を `false` と読むため、旧スタンプに対して逆の意味になる |
| **A10** | `parse_owner_stamp` の `token` | 未対応 (フィールドが無い) | `token?: string` が存在し、`stampMatches` の比較対象。audit ロック経路では常に不在なので実害はないが、active-directive ロックと同一ディレクトリ形式を共有する将来拡張では必要 | **未実装 (記録のみ)** |
| **A11** | `release` (`:274-290`) | 深度台帳に無い identity では lock dir に触れない (`release_requires_ownership` 相当の防御) | `releaseAuditLock` (`:7173-7186`) は**所有権検査なしで `rmSync(lockDir, {recursive, force})`** | **意図的な強化 (逸脱)**。upstream より厳格側。逸脱台帳に記録要 |
| **A12** | 未スタンプ猶予のアンカー (`:177-186`) | `fs::metadata(lock_dir).modified()` = **mtime** | `statSync(lockDir).mtimeMs` = **mtime** | **一致**。doc コメント `:9-10` の「birthtime に依存しない」も upstream と同じ選択 |
| **A13** | reap 適格の境界 | `reap_eligible` = `!alive \|\| elapsed > stale` (**厳密超過**) | `:7040` `if (now - startedAtMs <= lockStaleMs()) return false;` = 厳密超過のみ reap | **一致** |
| **A14** | dead dir 名 (`:194-202`) | `<lockDirName>.dead.<pid>-<counter>` | `` `${lockDir}.dead.${reapSuffix()}` `` = 同形 | **一致** |
| **A15** | `reapUnstamped` 引数 | 無し | `reapStaleLock(lockDir, reapUnstamped = true)` の第 2 引数。`acquireAuditLock` からは常に既定 `true` | **未実装 (現時点で実害なし)** |
| **A16** | ロック dir パーミッション | `fs::create_dir` (既定) | `mkdirSync(lockDir)` (既定、mode 指定なし) | **一致** |

### B. `modules/shared/message-catalog/src/lib.rs` (SpecQuotedOnly 4 件)

**4 件すべてピン留めソースでバイト一致を確認。`GoldenStatus::Captured` へ昇格可能。**

| 定数 | 出典 (カタログ記載) | ピン留め実測 | 判定 |
| --- | --- | --- | --- |
| `state::field_not_found` | `aidlc-lib.ts:6564` | `:6564` に関数、文言は `:6572` <br>`Field not found in state file: "${field}". Cannot update — refusing to silently no-op.` | **一致** (em dash `—` 含む) |
| `state::file_not_found` | `aidlc-lib.ts:6453` | `:6453` に関数、文言は `:6456` <br>`State file not found: ${path}` | **一致** |
| `lock::acquire_failed` | `aidlc-audit.ts:543` | `:543` `throw new Error("Failed to acquire audit lock after retries");` | **一致** |
| `bolt::invalid_mode` | `aidlc-bolt.ts:808` | `:808` `` error(`Invalid --mode: ${flags.mode}. Must be 'autonomous' or 'gated'.`) `` | **一致** |

**カタログ側の不足 (追加要検討)**:

- `withAuditLock` の別文言 `` `Failed to acquire audit lock for ${key} after retries` `` (`aidlc-lib.ts:7599`)。現カタログの `acquire_failed()` を唯一の acquire 失敗文言として扱うと**この経路が再現できない**。
- `audit-merge` の予算付き文言 (`aidlc-audit.ts:1375`)。
- 同一文言 `Failed to acquire audit lock after retries` は `aidlc-audit.ts` の **4 箇所** (`:543`, `:782`, `:897`, `:1150`) にあり、うち `:897` / `:1150` は throw ではなく `jsonError(...)` — **JSON エンベロープ経路と例外経路で同一文言**という契約。

### C. `docs/specs/12-workflow-definition.md` §4

| §4 の項 | ピン留め実測 | 判定 |
| --- | --- | --- |
| #1 `stage-graph.json` が読めない → fatal + env hint 分岐 | `:8565-8570` で逐語確認。文言・hint 2 形とも研究文書 §5.5 とバイト一致 | **一致・確定 (★ 解消)** |
| #2 不正 JSON → `Stage graph at <p> is not valid JSON: <err>` | `:8579-8581` で逐語確認 | **一致・確定 (★ 解消)** |
| #3 `scope-grid.json` は fatal にせず転置導出 | `loadScopeGridForMapping` (`:8634-8641`) の `catch` で `transposeScopeGridForMapping(loadStageGraph())` — **確認**。ただし逐語コメント *"callers never see a hard ENOENT for a derivable artifact"* は `aidlc-lib.ts` には**存在しない** (`aidlc-graph.ts` 側) | **挙動は一致。引用の出典が aidlc-lib.ts でない点だけ要訂正** |
| #4 `Unknown scope: "<s>". Valid scopes: <csv>` | **`aidlc-lib.ts` に存在しない。** `:8901` のコメント *"own validation owns the canonical `Unknown scope` error."* が `aidlc-graph.ts` 所在を示す | **未採取 (本タスク担当外)。★ は据置** |
| #8 `initialization` の 3 ステージは全スコープ列で EXECUTE (転置の特例) | **`transposeScopeGridForMapping` (`:8618-8632`) に特例は無い。** 純粋に `(stage.scopes ?? []).includes(scope)` のみ | **食い違い**。少なくとも「リーダ側のフォールバック転置」には特例が無い。特例が compile 側 (`aidlc-graph.ts`) にしか無いなら、amadeus-ng がフォールバック転置に特例を入れると本家と分岐する。§4 #8 の適用範囲 (compile 限定か、転置全般か) の裁定が必要 |
| §3.3 frontmatter 検証 5 種 + missing 2 種 | `:8661`, `:8663`, `:8666-8668`, `:8685-8687`, `:8698-8700`, `:8712-8714`, `:8786-8789` で全 7 形を逐語確認 | **一致・確定 (★ 解消)** |
| §3.3 「frontmatter パーサは手書き」 | `frontmatterBlock` / `scalarField` / `listField` (`:9043-9092`) はすべて手書き regex。zero-dep を明記したコメントあり | **一致** |
| §4 一般 | `readdirSync(scopesDir())` の失敗は `files = []` に劣化するが、**個別 `.md` の `readFileSync` は catch されず raw で伝播する** | **12 §4 の表に無い挙動。追記推奨** |

### D. 研究文書 (`docs/specs/research/`) の訂正必要箇所

| 文書 | 該当 | 訂正内容 |
| --- | --- | --- |
| `workspace-audit-ledger.md:227` (§5.2) | 「`readAllAuditShards` は**ファイル名順**に連結」 | **不正確**。正しくは「ディレクトリ群順 (space 群 → intent 群) → 各群内でファイル名昇順」。連結子は `"\n"` |
| `workspace-audit-ledger.md:22` (§1.1) | 「shard 内容を `\n` で連結」 | **正確** (§5.2 と §1.1 で記述が不整合だったが §1.1 が正しい) |
| `workspace-lock-fork-worktree.md:44` (§1.4) | 奪取可能条件を「ESRCH **または** 経過 > staleMs」と 2 条件で記載 | **不完全**。生存所有者枝には `reapLiveOwnerAfterStale` の第 3 条件がある (`:7037`)。死亡所有者には無条件 |
| `workspace-lock-fork-worktree.md:38` (§1.3) | owner.json 内容 `{ pid, startedAtMs, reapLiveOwnerAfterStale, token? }` | **正確**。キー順もこの通り。`mode: 0o600` の追記推奨 |
| `workflow-definition-graph-reader.md:221, 268` | `aidlc-lib.ts:2837-2864` / `:2841-2860` を `loadStageGraph` 系として引用 | **行番号が v2.2.0 由来で誤り**。ピン留めでは `:8552` (`loadStageGraph`) / `:8558-8585` (`loadStageGraphAll` + エラー逐語) |
| `workflow-definition-graph-reader.md` §4.2 の行番号群 (`:8664-8670`, `:8684-8687`, `:8697-8700`, `:8706-8716`, `:8785-8790`, `:8674`) | — | **すべてピン留めで一致**。★ を外してよい |

---

## 12. 参考: 本タスクで新たに判明した、仕様書に未記載の実装事実

1. `auditLockIdentity` は `realpathSync` 失敗時に **lexical 絶対パスへフォールバックする** (`:6800-6806`)。birth 前・診断時のロックを成立させるため。
2. `lockStaleMs` / `unstampedGraceMs` の env 受理条件は `Number.isFinite(n) && n > 0` — **0・負値・NaN は既定値へ落ちる**。
3. `isPidAlive` (`:6892`) は非整数・`<= 0` の pid を **not alive** と判定する (未スタンプ/破損スタンプの年齢のみ reap を可能にするため)。`EPERM` は alive。
4. `readAllAuditShards` / `readAuditShardEvents` は symlink 連鎖検査を**読取後にもう一度**行い、その際 `realpathSync(projectDir)` を使う。`auditShards()` 側の同検査は正規化しない projectDir を使う (**非対称**)。
5. `setStageSuffix` の `\b` により、`SKIP: <reason>` → `EXECUTE` の flip で **理由節が保存される** (`EXECUTE: <reason>` になる)。
6. `setCheckbox` だけが区切りを `" <slug> —"` (厳密な空白 1 個 + em dash) で要求し、`parseCheckboxes` / `setStageSuffix` は `\s*—\s*` を許す。
7. `classifyStateVersion` は `"08"` を `past` に分類する (regex 通過 → 文字列不等 → 数値不等ではない)。
8. `stageGraphPath` / `scopeGridPath` は `??` を使うので**空文字列の env をパスとして採用する**が、`loadStageGraphAll` の hint 分岐は truthy 判定なので**空文字列では既定 hint が出る**。
9. `withAuditLock` の `finally` は `depth <= 1` で `releaseAuditLock` を呼ぶ。`releaseAuditLock` は所有権を検査せず `rmSync` する (`:7176-7180`) — **他プロセスのロックをパス名だけで消しうる経路が upstream には存在する**。一方 doctor 経路 (`detectLeakedLocks`, `:7684-7688`) は `reapStaleLock` の CAS を通し *"Never remove a fresh replacement lock by pathname."* と明記しており、**同一ファイル内で防御レベルが非対称**。
10. `detectLeakedLocks` の 4 reason (`:7653`): `"dead-owner" | "over-age" | "unstamped" | "legacy-transaction"`。`over-age` 判定は `owner.reapLiveOwnerAfterStale && …` で**ここでもフラグを読む** (`:7677`, `:7702`)。

---

## 13. 未解決のまま残る項目 (本タスク担当外)

- `Unknown scope: "<s>". Valid scopes: <csv>` の逐語 → `core/tools/aidlc-graph.ts` の採取が必要。
- `initialization` 3 ステージの全スコープ EXECUTE 特例が compile 側 (`aidlc-graph.ts` の `canonicalStageGraphJson` / grid emit) のどこにあるか。
- `readAppendOnlyFileNoFollowOrThrow` の `<what>` 語彙は `aidlc-lib.ts` / `aidlc-audit.ts` / `aidlc-bolt.ts` の範囲では `"audit shard"` のみ。他ツール (`aidlc-state.ts`, `aidlc-runtime.ts`, `aidlc-log.ts` 等) の呼出サイト未確認。
- `FIELD_ORDER` 28 エントリ順序・`enabled` の出力有無・`summary_confirmation` の値域は `dist/claude/.claude/tools/data/stage-graph.json` の実バイト採取が必要 (本タスクの対象ファイルには無い)。

## RESOLVED OPEN QUESTIONS
- 【workspace-audit-ledger.md §5.2 / タスク項目 3】readAllAuditShards の連結順序 → 確定: 「ファイル名順」は不正確。auditShards() (:4530-4560) がディレクトリを 2 群 (intent===undefined && space!==undefined のときだけ前置される space レベル audit dir → 解決済み intent の audit dir) の順に走査し、各ディレクトリ内でのみ entries.sort() をかける。readAllAuditShards (:4568-4585) はその返り順をそのまま使い、再ソートせず parts.join("\n") で連結する (連結子は "\n"、"\n---\n" ではない)。
- 【fs_workspace_lock.rs:50 の TODO(golden: stage-0) / タスク項目 1】stampMatches が比較するフィールド → 確定: pid / startedAtMs / reapLiveOwnerAfterStale / token の 4 フィールドすべて (:6972-6977)。すなわち upstream では reapLiveOwnerAfterStale は「保持者の同一性の一部」である。現実装の PartialEq (pid + started_at_ms の 2 フィールド) およびその設計コメント「ポリシーであって同一性の一部ではない」は upstream と食い違う。
- 【タスク項目 1 の核心】reapLiveOwnerAfterStale は reap 判定で読まれるか → 確定: 読まれる。ただし ownerAlive(owner) が真の枝でのみ (:7037 `if (!owner.reapLiveOwnerAfterStale) return false;`)。所有者が死亡 (ESRCH) している場合はフラグも年齢も見ずに無条件で reap する。未スタンプ枝では owner.json が無いので読めない。detectLeakedLocks (:7677, :7702) も over-age 判定で同フラグを読む。
- 【タスク項目 1】writeOwnerStamp の出力バイト → 確定: JSON.stringify(owner) でインデント無し・末尾改行無し。キー順は pid → startedAtMs → reapLiveOwnerAfterStale → (token)。token は引数 truthy のときだけキーが出現し、acquireAuditLock は token を渡さないので audit ロックの owner.json に token フィールドは存在しない。writeFileSync のオプションは { encoding: "utf-8", mode: 0o600 } (token 付きのときのみ flag: "wx" を追加)。失敗は握り潰して null を返す best-effort であり acquire は成功する。
- 【タスク項目 2】未スタンプ猶予のアンカーは birthtime か mtime か → 確定: mtime。lockDirMtimeMs() (:6937-6943) が statSync(lockDir).mtimeMs を返す。renameSync が inode の mtime を保存することが stampMatches の未スタンプ枝 (:6962-6970) の正当性根拠として逐語コメントに明記されている (:6952-6954)。amadeus-ng の実装 (fs_workspace_lock.rs:177-186 / doc コメント :9-10) はこの点で一致。
- 【タスク項目 3】findAllEvents のタイブレーク実装 → 確定: results.sort((a,b) => a.timestamp !== b.timestamp ? (a.timestamp < b.timestamp ? -1 : 1) : a.pos - b.pos) (:7799-7802)。時刻は ISO-8601 文字列の辞書順比較 (Date パースなし)、pos は連結バッファ全体のブロック索引 (非マッチブロックでもインクリメントされる)。Timestamp 抽出は非 global の m フラグ regex なのでブロック内最初の一致のみ。
- 【タスク項目 4】readAppendOnlyFileNoFollowOrThrow の拒否文言 → 確定 4 種: `${what} is a symlink, which is not followed: ${path}` (ELOOP 経路と lstat フォールバック経路で同一文字列) / `${what} could not be opened: ${path} (${errorMessage(e)})` (err.code に元 errno を転記) / `${what} is not a regular file: ${path}` (非 regular の逐語) / `${what} changed while opening: ${path}`。<what> の実引数語彙は aidlc-lib.ts 内では "audit shard" のみ (:3786, :4574, :4610-4612)。aidlc-audit.ts / aidlc-bolt.ts には呼出サイトが存在しない。
- 【タスク項目 5】slugify / isoTimestamp の完全実装 → 確定。slugify (:1717-1728): toLowerCase → [^a-z0-9]+ を単一 - へ → 先頭末尾 - trim → slice(0,maxLength) → 切詰めで露出した末尾 - を再 trim → 先頭が [a-z] でなければ `intent-` 前置＋再 trim → 空なら "intent"。既定 maxLength=48、intent dir 名は 24。isoTimestamp (:9873-9875): new Date().toISOString().replace(/\.\d{3}Z$/, "Z") — タイムゾーンは常に UTC (Z 固定)、パディングは toISOString に委譲、秒精度。dateStamp (:1754-1758) は getUTC* + padStart(2,"0") で YYMMDD。
- 【タスク項目 6】classifyStateVersion の非整数比較の意味論 → 確定: 非整数の順序比較は起こらない。/^\d+$/ のテストで非負十進整数以外は全て unparseable に落ちてから Number() 比較に入る。ゼロ埋め "08" だけが「regex 通過・文字列不等・数値も大きくない」で past に分類され、メッセージ内も "08" で補間される (archive 先 aidlc.v08-archive)。CURRENT_STATE_VERSION は文字列 "8" (:10605)。4 分類の逐語は :10628-10667 に全文採取済み。
- 【タスク項目 7】checkbox suffix writer → 確定: setStageSuffix (:6733-6743) の regex は `^(- \[[ xSR?-]\] <slug>\s*—\s*)(EXECUTE|SKIP)\b` の m フラグ。\b により `SKIP: <reason>` の理由節は保存され `EXECUTE: <reason>` になる。非 global なので最初の 1 行のみ、不一致なら silent no-op。setCheckbox (:6713-6725) だけは区切りを厳密な " <slug> —" (空白 1 個 + em dash) で要求し、parseCheckboxes / setStageSuffix は \s*—\s* を許す非対称がある。
- 【workflow-definition-graph-reader.md §5.5 / §8 の ★ / 12-workflow-definition §4 #1 #2】グラフ読込エラー逐語 3 形 → ピン留めでバイト一致を確認 (:8565-8581)。`Stage graph not readable at ${p}: ${errorMessage(err)}. Reinstall the framework or re-run setup to restore the data file.` / 同前半 + `AIDLC_STAGE_GRAPH points to ${p}; unset it to use the default.` / `Stage graph at ${p} is not valid JSON: ${errorMessage(err)}`。hint 分岐は process.env.AIDLC_STAGE_GRAPH の truthy 判定。ただし文言の所在は v2.2.0 の :2837-2864 ではなく loadStageGraphAll (:8558-8585)。
- 【workflow-definition-graph-reader.md §5.5 / §8 の ★】scope frontmatter の逐語 → ピン留めでバイト一致を確認。`Scope file missing frontmatter: ${filePath}` (:8661) / `Scope file ${filePath} missing required frontmatter: name` (:8663) に加え、Duplicate scope name (:8666-8668) / plugin の aidlc- prefix 拒否 (:8685-8687) / invalid skeleton (:8698-8700) / invalid review_cap (:8712-8714) / Multiple enabled scopes declare freeform_default (:8786-8789) の計 7 形を採取。§4.2 表の行番号群 (8664/8674/8684/8697/8706/8785) はすべてピン留めで一致するため ★ を外せる。
- 【タスク項目 9】resolveReviewClass の 3 段単調下方解決 → 確定 (:8753-8770)。① stageClass を asReviewClass で検証し 3 値以外なら即 "none" 返却 (cap/override を見ない) ② loadScopeMetadata()[scope]?.reviewCap が REVIEW_RANK で厳密に小さいときのみ下げる (未知スコープ / cap 不在 = cap なし) ③ getField(stateContent,"Review Override") を asReviewClass に通し、厳密に小さいときのみ下げる。REVIEW_RANK = {none:0, advisory:1, adversarial:2}。stateContent が falsy (null/undefined/空文字列) なら getField を呼ばない。
- 【タスク項目 10】acquireAuditLock の reap 成功時の予算消費 → 確定 (:7147-7169)。reap 成功時は continue せず同一周回でインライン mkdirSync を試み、成功なら sleep なしで即 return true。インライン mkdir が失敗したらそのまま下へ落ちて Bun.sleepSync(retryMs) を実行し i++ する (= 予算を消費する)。つまり「reap 成功で予算を無限に温存できる」わけではなく、ループ上限 maxRetries+1 は常に有効でライブロックしない。mkdir 総試行は最大 (maxRetries+1)×2、sleep は最大 maxRetries 回。writeOwnerStamp の戻り値は捨てられ、失敗しても acquire は成功する。
- 【message-catalog SpecQuotedOnly 4 件】4 件すべてピン留めでバイト一致を確認 → Captured へ昇格可能。aidlc-lib.ts:6572 (Field not found in state file: "<f>". Cannot update — refusing to silently no-op.) / aidlc-lib.ts:6456 (State file not found: <path>) / aidlc-audit.ts:543 (Failed to acquire audit lock after retries) / aidlc-bolt.ts:808 (Invalid --mode: <m>. Must be 'autonomous' or 'gated'.)。ただしカタログには acquire 失敗の別 2 文言 (aidlc-lib.ts:7599 の `Failed to acquire audit lock for ${key} after retries`、aidlc-audit.ts:1375 の予算付き文言) が欠けている。

## VERIFIED COUNTS
- 【指示の期待値】aidlc-lib.ts のバイト数: 期待 450,663 bytes → 実測 450663 bytes (`wc -c`) — 一致
- 【03 Measurement M1】`wc -l core/tools/aidlc-lib.ts` → 期待 10668 → 実測 10668 — 一致。副次: aidlc-audit.ts 期待 1589 → 実測 1589 一致 (aidlc-bolt.ts は M1 対象外、実測 970)
- 【03 Measurement M14】`grep -cE '^export function (hooksHealthDir|recoveryFilePath|planFilePath|runtimeGraphPath|sensorsDir)' core/tools/aidlc-lib.ts` → 期待 5 → 実測 5 — 一致
- 【03 Measurement M15】`grep -nE "writeSync|writeFileSync|appendFileSync|writeBufferAtomic|copyFileSync|createWriteStream|truncateSync|ftruncateSync" core/tools/aidlc-audit.ts` → 期待「5 行 = import 2 (:14 writeSync, :33 writeBufferAtomic) + 呼出 3 (:603, :1239, :1252)」→ 実測 5 行・行番号すべて一致 — 一致
- 【03 Measurement M12 (aidlc-lib.ts 分)】`grep -n setOrInsertField core/tools/aidlc-lib.ts` → 期待「非呼出 3 行 (:6594 コメント, :6599 定義, :6616 コメント)、呼出サイト 0」→ 実測 3 行・行番号すべて一致、呼出サイトなし — 一致
- 【03 Measurement M12 (AUTONOMY_MODE_FIELD)】`grep -n AUTONOMY_MODE_FIELD core/tools/aidlc-lib.ts` → 期待「:6507 定数定義, :6510 getField 読み の 2 行のみ」→ 実測 2 行・行番号一致 — 一致
- 【研究文書の行番号照合 (aidlc-lib.ts, 48 箇所)】6753/6777/6784/6787/6799/6814/6824/6925/6960/7023/7138/7521/7570/7637/7761/4499/4512/4530/4568/1698/1717/1754/1765/1781/6453/6461/6487/6546/6564/6599/6620/6635/6647/6653/6662/6678/6713/6733/6745/9873/10605/10627/8753/8664/8684/8697/8706/8785 → 48/48 すべて期待シンボルと一致
- 【研究文書の行番号照合・不一致 1 件】workflow-definition-graph-reader.md:221,268 が挙げる `aidlc-lib.ts:2837-2864` / `:2841-2860` (loadStageGraph 系) → ピン留め実測では active-directive ロックトランザクション終端と freshActiveDirectiveMarker。正しい位置は loadStageGraph :8552、loadStageGraphAll + エラー逐語 :8558-8585 — 不一致 (v2.2.0 由来の行番号ドリフト)
- 【message-catalog 4 件の逐語照合】aidlc-lib.ts:6572 / aidlc-lib.ts:6456 / aidlc-audit.ts:543 / aidlc-bolt.ts:808 → カタログ文字列とバイト一致 4/4 — 一致 (em dash U+2014、引用符、末尾ピリオドまで含む)
- 【12-workflow-definition §4 #8 の検証】`grep -n initialization core/tools/aidlc-lib.ts` → transposeScopeGridForMapping (:8618-8632) 内に該当なし。ファイル内の initialization 特例は :456 (stageEnabledBySelection) と :4146 (summary-confirmation 免除) のみ — 期待 (全スコープ列 EXECUTE の転置特例) と不一致
- 【12-workflow-definition §4 #4 の検証】`grep -n 'Unknown scope' core/tools/aidlc-lib.ts` → 該当文言なし。:8901 に `own validation owns the canonical \`Unknown scope\` error.` のコメントのみ — aidlc-lib.ts には存在せず (aidlc-graph.ts 所在。本タスク担当外)
