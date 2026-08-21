> 抽出元: upstream as-built 仕様 (awslabs/aidlc-workflows v2 @ 3c3146cf, v2.6.40)。2026-08-22 の 3 エージェント精密抽出。11-workspace.md と audit_lock.qnt の執筆材料。

以下、upstream as-built 仕様からの精密抽出。典拠は「仕様書 §節 (仕様書内行番号 L)」＋原典ソース `path:line`(仕様書が保存している逐語引用) の二重表記。逐語契約(メッセージ・フィールド名・regex・パス)は原文のまま英語で保存。

# タスク 3: ロック・fork/merge・Worktree の契約の完全列挙

主典拠: `docs/upstream/specs/03-state-audit-runtime.md`(以下 **03**)、補助: `09-cli-tools.md`(**09**)、`11-plugin-system.md`(**11**)、`07-hooks.md`(**07**)、amadeus-ng 側 `docs/specs/01-domain-model.md`(**NG-01**)。

---

## 1. withAuditLock の完全な機構 (03 §6.8, §2.3)

### 1.1 ロックの物理形態と識別子

| 要素 | 契約 | 典拠 |
| --- | --- | --- |
| 物理形態 | **`os.tmpdir()` 内の `mkdir`-EEXIST ディレクトリによるクロスプロセス mutex** | 03 §6.8 L870-871; `aidlc-lib.ts:6753-6755` |
| 識別子合成 (intent あり) | `<realpath(projectDir)>\x00<space>\x00<intent>` | 03 §6.8 L873-875; `aidlc-lib.ts:6799` |
| 識別子合成 (intent 省略) | `<realpath(projectDir)>\x00__workspace__`。センチネル定数 `WORKSPACE_LOCK_SENTINEL` | 03 §6.8 L875-876; `aidlc-lib.ts:6777` |
| キー不変条件 1 | intent 省略時は予約センチネルをハッシュし、**決して `activeIntent()` を解決しない** — birth 時点で active intent は無く、解決すると並行 first-run 2 つが別バケットをキーして両方 birth してしまう。**`intents.json` の全ミューテーションはこのバケットを取る** | 03 §6.8 L878-881; `aidlc-lib.ts:6757-6768` |
| キー不変条件 2 | 複合 identity はロックディレクトリと in-process の深度/ハンドラマップの両方をキーする。さもなくばマップが intent 間で衝突 | 03 §6.8 L882-883 |
| ロックディレクトリ | `join(tmpdir(), ".aidlc-audit-" + md5(identity).slice(0,8) + ".lock")` | 03 §6.8 L884; `aidlc-lib.ts:6814` |

### 1.2 取得 (acquire)

| 要素 | 契約 | 典拠 |
| --- | --- | --- |
| シグネチャ | `acquireAuditLock(projectDir, maxRetries=50, retryMs=100, intent?, space?, reapLiveOwnerAfterStale=true)` | 03 §6.8 L886-887; `aidlc-lib.ts:7138` |
| ループ | `mkdirSync(lockDir)` → `writeOwnerStamp`。`EEXIST` なら `reapStaleLock` を試み、成功したら即 `mkdir` 再試行、失敗なら `retryMs` sleep | 03 §6.8 L887-889 |
| 予算超過 | `false` を返す。呼び出し側は `Failed to acquire audit lock after retries` に翻訳 | 03 §6.8 L890-891; `aidlc-audit.ts:543` |
| デフォルト予算 | 50 × 100 ms = 5 s。`audit-merge` は `AIDLC_AUDIT_LOCK_RETRIES` / `_RETRY_MS` で **200 × 100 ms = 20 s** に拡大 (並列 Bolt 競合対策) | 03 §2.3 L115, §6.9 L975; `aidlc-audit.ts:1363-1371` |
| フック側予算 | review-freeze フックは audit-lock 予算 **5 × 50 ms** | 07 L425; `aidlc-review-freeze.ts:821` |

### 1.3 owner.json スタンプ

| 要素 | 契約 | 典拠 |
| --- | --- | --- |
| 内容 | `{ pid, startedAtMs, reapLiveOwnerAfterStale, token? }` | 03 §6.8 L893-894; `aidlc-lib.ts:6824-6826` |

### 1.4 reap 規則 (「生きている閾値未満の保持者からは決して奪わない」の正確な条件)

| 要素 | 契約 | 典拠 |
| --- | --- | --- |
| 奪取可能条件 (iff) | `process.kill(pid, 0)` が `ESRCH` を投げる (所有者死亡) **または** スタンプ経過時間 > `lockStaleMs()` | 03 §6.8 L896-897 |
| 閾値 | `DEFAULT_LOCK_STALE_MS = 10 * 60 * 1000` (10 分)。`AIDLC_LOCK_STALE_MS` で上書き可 | 03 §6.8 L897-898, §2.3 L113; `aidlc-lib.ts:6784`, `:6787` |
| 不奪取保証 | **生存中かつ閾値未満の保持者は決して奪われない** (*"A live, under-threshold holder is never robbed"*) | 03 §6.8 L898-899; `aidlc-lib.ts:6771-6774` |
| 未スタンプ猶予 | mkdir は成立したが `owner.json` 未書込のディレクトリは `unstampedGraceMs()` (デフォルト **5000 ms**、`AIDLC_LOCK_UNSTAMPED_GRACE_MS`) で保護 — 取得途中の生存プロセスから奪わない | 03 §6.8 L899-901, §2.3 L114; `aidlc-lib.ts:6925-6932` |
| **奪取は rename CAS** | `reapStaleLock` (`aidlc-lib.ts:7023`) はロックディレクトリを reaper 専用パス `<lockDir>.dead.<pid>-<counter>` へ rename で退避 → 移動後ディレクトリに対し `stampMatches` (`:6960`) で「判定したものと同一のロックを掴んだか」確認。不一致なら rename で戻す | 03 §6.8 L902-905 |
| 残余レース | 戻しの rename が `EEXIST` で失敗しうる (第三プロセスが隙間で再 `mkdir` した場合)。そのときは生きたロックが既に存在するので private dir は単に破棄 | 03 §6.8 L905-907; `aidlc-lib.ts:6993-7014` |

### 1.5 再入と解放経路

| 要素 | 契約 | 典拠 |
| --- | --- | --- |
| 再入 | `withAuditLock` (`aidlc-lib.ts:7570`) は **identity ごとの深度カウンタ**を保持。保持中セクション内のネスト呼び出しは再取得せず、早期解放もしない | 03 §6.8 L909-911 |
| 正常解放 | 深度が 0 に戻るとき (`depth <= 1`) に `rm -rf lockDir` + exit ハンドラ除去 | 03 §6.8 mermaid L929-931 |
| exit ハンドラ | 初回取得時に `process.on("exit")` で lock dir を `rm -rf` するハンドラを設置 — *"if the body calls process.exit (Bun skips `finally` in that case) … so the project isn't poisoned for ~5s on the next invocation"* | 03 §6.8 L911-914; `aidlc-lib.ts:7601-7609` |
| クラッシュ後 | kill -9 等でハンドラも走らない場合は §1.4 の reap (死亡判定 ESRCH は即時、それ以外は 10 分閾値) が受け皿 | 03 §6.8 L896-899 |
| 自己デッドロック回避 | `holdsAuditLock` (`aidlc-lib.ts:7637`) は複合 identity 下の exit ハンドラ存在を探査。`emitAudit` (`aidlc-state.ts:141`) と `emitError` (`aidlc-lib.ts:9977`) はこれで分岐して `appendAuditEntryUnlocked` を選ぶ | 03 §6.8 L914-915 |

---

## 2. audit-first クリティカルセクション (03 §5.7, §6.10)

| 要素 | 契約 | 典拠 |
| --- | --- | --- |
| 順序 | すべての read-modify-write ハンドラは `withAuditLock(pd, …)` 内で実行され、**read → decide → audit emit → state write が単一クリティカルセクション** | 03 §5.7 L596-598 |
| emit 失敗時 | 不変条件は *audit-first*: 監査行はロック内で emit され state 書込が後続。**audit エラーが throw されたら state 書込はスキップ** | 03 §5.7 L597-599; `aidlc-state.ts:128-130`, 例 `:2255-2286` |
| バッチ原子性 | `appendAuditEntries` (`aidlc-audit.ts:770`) は audit 専用トランザクション: 全エントリをディスク接触**前**に検証し、1 ロック内 1 write で全ブロック書込 — *"a malformed later entry cannot leave an earlier entry committed, and no concurrent emitter can interleave between the blocks"* | 03 §6.10 L1014-1017; `aidlc-audit.ts:765-769` |
| append 中の rename 検出 | `verifyPathStillNamesDescriptor()` は write の**前後両方**で実行 — 書込途中の rename は「もはや発見不能な行を報告する」のではなく**囲んでいる audit-first トランザクション全体を失敗させる** | 03 §6.7 L852-856; `aidlc-audit.ts:677-690` |
| worktree ツールでの audit-first | `aidlc-worktree create` / `merge` は state を変更する git コマンドの**前に** emit — emit と効果の間のクラッシュは doctor が調停できる phantom event として現れる (kill-9 window 明示)。`abort --discard` は**意図的に逆順** (discard 先・audit 後) | 09 §14 L1046; `aidlc-worktree.ts:266-268`, `aidlc-bolt.ts:562-569` |
| remote fetch の位置 | rebase の remote **存在チェック**は pre-audit、`git fetch` は post-audit — *"fetch mutates remote-tracking refs — running it before the audit emit would leave a kill-9 window where refs moved without a corresponding audit row"* | 09 §7.2 L492; `aidlc-worktree.ts:484-488` |

---

## 3. 三層 fork/merge

### 3.1 state fork/merge (`aidlc-state.ts fork` / `merge`)

| 要素 | 契約 | 典拠 |
| --- | --- | --- |
| 動詞 | `fork` (case `:613`)、`merge` (case `:616`) — aidlc-state.ts の 25 サブコマンドに含まれる (engine-owned 11 個には**含まれない**) | 09 §3 L84, §16 L1093; 03 §5.7 L581-589 |
| fork 時の state 変更 | `Bolt Refs` へ slug 追加 — `setFieldStrict` (欠落フィールドなら throw) | 03 §5.3 L502-503; `aidlc-state.ts:4042` |
| worktree コピー時 | forked state に `Worktree Path` を `setFieldStrict` で書込 | 03 §5.3 L503-504; `aidlc-state.ts:4074` |
| merge 時 | worktree merge 経路で `Bolt Refs` から slug 除去 (`setFieldStrict`) | 03 §5.3 L504; `aidlc-state.ts:4217` |
| `Bolt Refs` 値文法 | 単一行リスト値: `parseRefsList` は `""`・リテラル `[empty list]`・ブラケット付きカンマリストを受理; `emitRefsList` は空なら常に `[empty list]`、非空ならソート済みブラケットリスト (往復決定的)。`appendSlug`/`removeSlug` は重複/欠落で **throw** (no-op しない) | 03 §5.2 L481-485; `aidlc-lib.ts:6635`, `:6647`, `:6653`, `:6662` |
| `Merge-Held` | per-Bolt **forked** state のみに存在するフィールド (`## Project Information` 下に初回 hold で挿入)。詳細は §4.6 | 03 §5.3 L522; `aidlc-bolt.ts:692` |
| イベント | `STATE_FORKED` / `STATE_MERGED` (Worktree カテゴリ 7 種の一部)。両者とも merge-protected (referee bookkeeping、main 側で emit) | 03 §6.5 L790, §6.6 L831-834 |
| 陳腐化コメント | `aidlc-state.ts:4071` には旧フラット `aidlc-docs/` パスを述べる stale コメントが残存 (実際の live パスは `worktreeStateFilePath`) | 03 §6.11 L1024 |

### 3.2 audit-fork (`aidlc-audit.ts audit-fork --slug <s> [--intent <i>] [--space <sp>]`)

手順 (03 §6.9 L945-957; `aidlc-audit.ts:1123`):

| # | ステップ | 契約 (逐語) | 典拠 |
| --- | --- | --- | --- |
| 1 | pre-emit ガード (fail clean) | `main audit not found at <p>; start a workflow first …` / `worktree directory not found at <p>; run aidlc-worktree create first` | L948-950 |
| 2 | per-intent ロック下で main をスナップショット | `boundary = bytes.length`、`sourceHash = sha256(bytes)` — **バイト長 + SHA-256 のピン留め** | L951-952 |
| 3 | `AUDIT_FORKED` emit | フィールド: `Bolt slug`、`Source Audit Hash`、`Fork Boundary`。`expectedIdentity` prefix チェックでピン — スナップショットと emit の間に並行 append が滑り込めない | L953-955 |
| 4 | clone-id トークンを worktree へコピー → shard を worktree に**ホールファイル tmp+rename** で書込 (`writeBufferAtomic(wtAuditPath, mainAfterFork)`) — aidlc-audit.ts 内で append でない唯一の台帳バイト書込 | | L956-957; `aidlc-audit.ts:1232-1239`, `:1252` |

再 fork の許容 (03 §6.9 L959-966):

| 条件 | 拒否文言 (逐語) |
| --- | --- |
| AUDIT_FORKED 後に未 merge の作業がある | `…with unmerged work after AUDIT_FORKED; merge the delta with audit-merge, or discard the worktree` |
| AUDIT_FORKED 行が main の authoritative 行と不一致 | `…its AUDIT_FORKED row does not match the authoritative main row` |
| fork prefix が main と相違 | `…its fork prefix differs from main` |

3 ガードと `alreadyCurrent` 短絡はすべて `if (existingFork)` (`:1161-1188`) 内 — **この slug の `AUDIT_FORKED` 行を持たない既存 worktree shard はどれにも該当せず、step 4 でホールセール置換される** (L964-966)。

### 3.3 audit-merge (`aidlc-audit.ts audit-merge --slug <s>`)

delta = `wtContent.slice(fork.end)` のみ append (03 §6.9 L968-984; `aidlc-audit.ts:1320`):

| 検証 | 拒否文言 (逐語) | 典拠 |
| --- | --- | --- |
| delta がブロック境界で終端 | `worktree audit delta ends with an incomplete block` | `validateMergeDelta`, `aidlc-audit.ts:974` |
| 各ブロックに `Event` と `Timestamp` がちょうど 1 つ (または event なし・timestamp ちょうど 1 の完全な append-raw ノート) | (同上関数) | L970-972 |
| イベントが `VALID_EVENT_TYPES` に属する | `worktree audit delta contains unknown event <E>` | L973 |
| merge-protected でない | `worktree audit delta contains protected authority event <E>` | L974 |
| ロック内で main を再スナップショット; worktree スナップショットは pre-lock read と**バイト・inode 同一** | `worktree audit changed while merge was preparing; retry the merge` | L976-978 |
| authoritative fork 行は **main から回収** (書込可能な worktree コピーを信用しない)。全相関フィールド一致必須 | | L979-980; `:1404-1411` |
| main 先頭 `boundary` バイトの SHA-256 == 記録済 `Source Audit Hash` | `main audit prefix-hash at byte <n> does not match recorded Source Audit Hash; refusing to merge (mid-Bolt tampering suspected)`。main が boundary より短い場合: `… (main-audit truncation suspected)` | L981-984 |

- `AUDIT_MERGED` フィールド: `Bolt slug`、`Entries Merged`、`Source Audit Hash`、`Fork Boundary`、`Fork Timestamp` (L986)。
- 順序: per-Bolt エントリ順は保存、cross-Bolt 順は merge 完了順 (`audit-format.md:211`、L987-988)。
- fork/merge 経路は厳格: `readAuditSnapshot` (`:705`) は **多重リンク shard (nlink≠1) を拒否** (`:719-721`); `verifyExpectedPrefix` (`:657`) は merge append 中に nlink + expected prefix の SHA-256 を再検査 (03 §6.7 L863-866)。※通常 append 経路は `nlink != 1` を拒否**しない** (rsync --link-dest / cp -al 対応、L860-863)。
- 相違点: legacy な二重 Timestamp ブロックは `validateMergeDelta` が拒否 (`aidlc-audit.ts:987-989`) するため worktree から merge 不能 (03 §6.11 L1025)。

### 3.4 fragment fork/merge (`aidlc-runtime.ts`) (03 §7.5 L1141-1153)

| 要素 | 契約 | 典拠 |
| --- | --- | --- |
| `fragment-fork --slug` | main の `runtime-graph.json` を Bolt worktree にバイトコピー。**single-read プロトコル** (1 回読み、そのバッファから書き、同じバッファをハッシュ) で並行 compile とのバイトコピー/ハッシュレースを閉じる | `aidlc-runtime.ts:1120-1122` |
| main にグラフがない場合 | fragment パスへ空グラフを書く (`writeEmptyGraph`, `:813`) | L1148-1150 |
| `fragment-merge --slug` | worktree fragment を除去 (**冪等**) | L1142-1143 |
| audit イベント | **どちらも emit しない** — *"the fork boundary is already triple-attested by BOLT_STARTED + STATE_FORKED + AUDIT_FORKED, the merge boundary by BOLT_COMPLETED + STATE_MERGED + AUDIT_MERGED"* | `aidlc-runtime.ts:1104-1107` |
| 内容マージ | **意図的に無し** — main のグラフは post-Bash フックが main audit からイベントソーシングで再構築; 内容マージは compile と競合する | `aidlc-runtime.ts:1109-1112` |

### 3.5 Bolt fork/merge の固定順序 (09 §5.3 L237-245)

| 経路 | 固定順序 | 記録された理由 |
| --- | --- | --- |
| `start --worktree` | state ファイル形状検証 → `BOLT_STARTED` emit → state-fork → audit-fork → fragment-fork | 検証が emit に先行するのは *"so a missing state file doesn't leave an orphan BOLT_STARTED"* (`aidlc-bolt.ts:221-223`)。各 fork 失敗は失敗前にリカバリ `BOLT_FAILED` を emit |
| `complete --merge` | hold-merge チェック → `BOLT_COMPLETED` emit → state-merge → audit-merge → fragment-merge | `aidlc-bolt.ts:387-489` |
| `abort --discard` | **discard 先、audit 後** | 先に emit すると *"would claim the Bolt was aborted-and-cleaned-up while the worktree directory still existed on disk and the slug remained in main's Bolt Refs"* (`:562-586`) |

- 全 sibling spawn は **30 s タイムアウト**; `signal === "SIGTERM"` がタイムアウトと exit-code 失敗を区別し `*-timeout` reason を選択 (09 L245)。
- `--worktree` / `--merge` は single-bolt のみ。CSV `--name` は拒否: `--worktree requires a single bolt name; got csv: "<n>". Issue one start --worktree per bolt.` (09 L235)。
- 失敗エンベロープ (09 §5.4 L249-255): `{"ok": false, "slug": "…", "stage": "…", "reason": "…", "detail": "…"}` で exit 1。`stage` ∈ {`start-worktree`, `complete-merge`, `abort-discard`, `hold-merge`, `release-merge`}。`reason` は 17 値: `state-read-failed`, `audit-emit-failed`, `state-fork-failed`, `state-fork-timeout`, `audit-fork-failed`, `audit-fork-timeout`, `fragment-fork-failed`, `fragment-fork-timeout`, `merge-held`, `state-merge-failed`, `state-merge-timeout`, `audit-merge-failed`, `audit-merge-timeout`, `fragment-merge-failed`, `fragment-merge-timeout`, `discard-failed`, `discard-timeout`。

---

## 4. Worktree (09 §7, 03 §4.5)

### 4.1 パス・ブランチ導出規則

| 要素 | 契約 | 典拠 |
| --- | --- | --- |
| ディレクトリ | `<projectDir>/.aidlc/worktrees/bolt-<slug>` — 導出であり引数渡しではない | 09 §7.1 L462; `aidlc-worktree.ts:260`; 03 §4.5 L417 (`worktreePath`, `aidlc-lib.ts:4639`) |
| ブランチ | `bolt-<slug>` | 09 §7.1 L462; `:914` |
| slug 検証 | `SLUG_RE = /^[a-z][a-z0-9-]*$/`; 拒否: `Invalid --slug: "<s>". Must be kebab-case (lowercase letter then [a-z0-9-]).` | 09 §7.1 L460, §7.2 L474 |
| record ミラー | worktree 内に**同一相対レイアウト**でミラー: `worktreeDocsDir` = `<wt>/<recordPrefix>`、`worktreeStateFilePath` = `<wt>/<recordPrefix>/aidlc-state.md`、`worktreeAuditFilePath` = `<wt>/<recordPrefix>/audit/<shardName>`、`worktreeRuntimeGraphPath` | 03 §4.5 L420-426; `aidlc-lib.ts:6189-6209` |
| clone-id スレッディング | `worktreeAuditFilePath` は **main の** projectDir を取り、shard 名に main clone のトークンを埋める — *"the fork and merge subprocesses are both spawned from the main checkout, so threading the main clone-id makes them resolve the SAME worktree shard across the two PIDs"* | 03 §4.5 L427-431; `aidlc-lib.ts:6198-6203` |

### 4.2 6 サブコマンド (09 §7.1 L449-458)

不明動詞: `Unknown subcommand: <x>. Valid: create, merge, discard, list, verify, info` (`aidlc-worktree.ts:1171`)。

| Subcommand | フラグ | Audit | Read-only |
| --- | --- | --- | --- |
| `create` | `--slug`, `--base`, `[--repo] [--intent] [--space]` | `WORKTREE_CREATED` | no |
| `merge` | `--slug`, `--target`, `--strategy`, `[--message] [--repo] [--intent] [--space]` | `WORKTREE_MERGED` | no |
| `discard` | `--slug`, `[--repo] [--intent] [--space]` | `WORKTREE_DISCARDED` | no |
| `list` | — | none | yes |
| `verify` | `--event`, `--slug`, `[--max-age-seconds]` | none | yes |
| `info` | `--slug` | none | yes |

`VALID_VERIFY_EVENTS = {WORKTREE_CREATED, WORKTREE_MERGED, WORKTREE_DISCARDED}` (`:43-47`)。`verify` の鮮度窓デフォルト **60 秒**、3 結果: `{verified:true, event, slug, audit_timestamp}` (exit 0) / `reason:"absent"` (exit 1) / `reason:"stale (last seen <ts>)"` (exit 1) (09 L549)。`info` は最新 `WORKTREE_CREATED` ブロックの `Worktree path` と `Branch name` を印字、スキーマは `knowledge/aidlc-shared/worktree-info-schema.md` にピン (09 L551; ただし同スキーマ `:42` は stale な `aidlc-docs/` パスを記載 — 03 §6.11 L1024)。

### 4.3 「メイン checkout からのみ実行可」と create ガード

| ガード | 拒否文言 (逐語) | 典拠 |
| --- | --- | --- |
| sibling-worktree 拒否 (`assertNotSiblingWorktree`) | `aidlc-worktree must run from the main repo checkout, not from a sibling worktree at <top>. Bolt worktrees are siblings of the main checkout, not nested.` — `git rev-parse --show-toplevel` と `dirname(git rev-parse --git-common-dir)` を `realpathSync` 正規化のうえ比較 (*"macOS symlinks `/var → /private/var`"*)。`create`/`merge`/`discard` で実行、`list` は意図的にスキップ (*"list is read-only and useful from anywhere"*)。`--repo` 下ではターゲットリポジトリの checkout に再アンカー | 09 §7.2 L466-470; `aidlc-worktree.ts:155-175`, `:147-148`, `:150-154`, `:911-912` |
| base 実在 (pre-audit) | `Base branch does not exist locally: <base>` | 09 §7.2 L479 |
| ディレクトリ既存 (pre-audit) | `Worktree directory already exists: <path>` | L480 |
| ブランチ既存 (pre-audit) | `Branch already exists: bolt-<slug>` | L481 |
| merge HEAD チェック | `expected branch <target>, found detached HEAD` / `expected branch <target>, found <actual>` — `--target` はリポジトリ cwd で checkout 済みブランチでなければならない | L485-486 |
| rebase remote 要件 | `rebase strategy requires a remote for <target>; got none` (存在チェックは pre-audit、fetch は post-audit — §2 参照) | L490-492 |

作成コマンド: `git worktree add <wtPath> -b bolt-<slug> <base>` (`:281`)。

### 4.4 merge strategy 3 値とコンフリクト時の保存

`VALID_STRATEGIES = {squash, merge, rebase}` (`:42`); 拒否: `Invalid --strategy: "<s>". Must be one of: squash, merge, rebase.`

| Strategy | 実行 checkout | コマンド (09 §7.3 L510-514) |
| --- | --- | --- |
| `squash` | ターゲットリポジトリの main checkout (`repoCwd`) | `git merge --squash <mergeTarget>` → `git commit --no-edit -m <message>` |
| `merge` | `repoCwd` | `git merge --no-ff --no-edit -m "Merge bolt <slug>" <mergeTarget>` |
| `rebase` | worktree (`wtPath`) → `repoCwd` | `git fetch <remote>` + `git rebase <target>` (wtPath)、続けて `git merge --ff-only <ffTarget>` (repoCwd) |

- `squash`/`merge` の git 引数は `--target` **ではなく** `mergeTarget` = **Bolt 側** (`bolt-<slug>` ブランチ、source-bound/bypassed なら不変レビュー済みコミット / bypass ブランチ OID)。`--target` は repoCwd に checkout 済みブランチで、Bolt はそこへマージ**される**。`rebase` のみ `flags.target` を直接取る (09 L508)。
- `--message` デフォルト: `Bolt <slug>` (`:414`)。
- **コンフリクト検出**は git の正準マーカー `/^CONFLICT \(/m` (stdout+stderr 結合) にアンカー (旧 `/conflict/i` は false-positive したため置換)。コンフリクトパス列挙は同じ cwd での `git diff --name-only --diff-filter=U` (09 §7.4 L520-522)。
- **コンフリクト時の保存** — exit 1 で以下を印字し worktree は保存:

```json
{"status":"conflict","slug":"…","worktree_path":"…","conflict_files":[…],
 "detail":"Merge produced conflicts in worktree at <path>. Worktree preserved for inspection."}
```

- **post-merge クリーンアップ**: マージコミット着地後のクリーンアップ失敗は merge 失敗と読ませない。全 post-merge エラーに `[merge-succeeded:<commitSha>]` プレフィクス (doctor が「merge 全失敗」と「merge 着地・クリーンアップ孤児残存」を区別するため)。クリーンアップは binding 別 3 種 (bound: `git reset --hard <mergeTarget>` → `git worktree remove --force`; bypass: OID 不変検証 (`bypassed Bolt branch changed during the merge; worktree and branch preserved` で fail-closed) → 3 framework pathspec 限定 `git clean -ffdx` → 非 forced remove + `update-ref -d` による OID チェック付きブランチ削除; neither: 素の `git worktree remove` + `git branch -D`) (09 §7.5 L531-541)。

### 4.5 source-bound の扱い (09 §7.2 L494-500)

| 分類 | 判定 | 帰結 |
| --- | --- | --- |
| *source-bound* | 最新 `SWARM_UNIT_CONVERGED` 行が `Source Fingerprint` + `Source Commit` を持つ | merge ターゲットは可動 `bolt-<slug>` ブランチでなく**不変コミットオブジェクト** |
| *bypassed* | 同行が `Source Freshness Bypass` を持つ | bypass ブランチ OID で merge |
| どちらでもない | swarm を経ていない | 素通り |

逐語拒否: `refusing to rebase a source-bound convergence: rebase before review/finalize, then merge the immutable reviewed commit` (`:448`) / `refusing to merge: reviewed Source Commit <sha> is unavailable` (`:404`) / `refusing to merge: the bypassed Bolt has uncommitted or ignored application paths not represented by its branch (<detail>); commit, remove, or discard those paths before retrying` (`:477-479`)。

### 4.6 HOLD-MERGE (09 §5.5 L257-270)

| 要素 | 契約 |
| --- | --- |
| 保管場所 | per-Bolt **forked** state ファイル `<projectDir>/.aidlc/worktrees/bolt-<slug>/…/aidlc-state.md` の `Merge-Held` (`true`/`false`)、初回 hold で `## Project Information` に挿入 |
| 冪等性 | 両方向で冪等。**audit emit なし** (*"Merge-Held is internal coordination state, not a user-visible event"*) |
| 欠落時 | forked state ファイル欠落は *not held* と読む (`isMergeHeld` false)。ただし `setMergeHeld` は hard error: ``No per-Bolt forked state file for slug "<s>" — was `aidlc-bolt start --worktree --slug <s>` run?`` |
| 執行点 | `complete --merge`。拒否 (逐語): ``Merge held by HOLD-MERGE invariant; resolve the failed-sibling halt-and-ask sequence and run `aidlc-bolt release-merge --slug <slug>` before retrying.`` |

### 4.7 discard の冪等性・list・孤児回収

| 要素 | 契約 | 典拠 |
| --- | --- | --- |
| discard 冪等 | ディレクトリ・ブランチ・retained source ref のいずれも無いとき `{"emitted":null,"slug":"…","worktree_path":"…","reason":"already-discarded"}` を印字して emit せず return。それ以外は `WORKTREE_DISCARDED` を `Reason: agent-discard` で emit (**audit-first**) → `git worktree remove --force` + `git branch -D` | 09 §7.6 L545 |
| list の二重条件 | basename が `bolt-` 開始 **かつ** 親ディレクトリが正確に `<projectDir>/.aidlc/worktrees` — 名前空間外の `bolt-other` を偽装させない。パス比較は `pathKey` (正規化 + win32 lowercase) | 09 §7.6 L547 |
| 孤児回収 | doctor が検出: *Orphan worktrees, stale `bolt-*` branches, orphan per-Bolt state files, orphan audit shards* | 09 §11 L842 |
| workspace-sync | `aidlc-workspace-sync.ts` は workspace `withAuditLock` 下で 1 `reconcile()`。`--force` は保守的 preflight + 隔離検査後にのみ孤児除去を許可 — *"Cached refs/remotes/* and advertised OIDs alone never prove recoverability: matching object graphs must be fetched into an isolated probe before removal"*。exit 0/2/1 | 09 §12 L904; `aidlc-workspace-sync.ts:1155-1173`, `:16-19` |

### 4.8 WORKTREE_* イベント

- Worktree カテゴリは 7 種: `WORKTREE_CREATED` `WORKTREE_MERGED` `WORKTREE_DISCARDED` `STATE_FORKED` `STATE_MERGED` `AUDIT_FORKED` `AUDIT_MERGED` (03 §6.5 L790)。
- すべて `MERGE_PROTECTED_EVENT_TYPES` (26 + `DOCUMENT_*` prefix) 側の "referee bookkeeping (fork/merge/swarm/bolt/worktree rows, emitted main-side)" に属し、worktree delta では移動不可 (03 §6.6 L827-834; `aidlc-audit.ts:395`, prefix 規則 `:426-429`)。逆に `STAGE_*`/`SENSOR_*`/reviewer receipts/`ARTIFACT_*` は worktree の正当な作業産物として delta を通る — *"the referee's defence against a lying conductor is artifact re-verification at finalize, not delta filtering."* (`aidlc-audit.ts:377-394`)。

---

## 5. compose とのロック共有と B5 関連事実

### 5.1 upstream の物理的ロック共有 (11 §5.1, §5.3)

| 事実 | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| ロック共有の前提プローブ | インストール済み lib が `acquireAuditLock`/`releaseAuditLock` を export しない、またはインストール済み `aidlc-graph.ts` が `AIDLC_WORKSPACE_LOCK_OWNER_PID` トークンを欠く場合、drop: `plugin compose skipped: installed engine lacks shared compose/graph workspace-lock support; re-copy the current dist/<harness>/ shell and retry` | 11 §5.1 L390; `compose.ts:391-402`, `:116-125` |
| 取得予算 | `COMPOSE_LOCK_RETRIES = 600`。取得不能なら drop: `plugin compose skipped: could not acquire the shared workspace lock` | 11 §5.1 L391; `compose.ts:74`, `:404-410` |
| ロック継承の受け渡し | compose がロック保持中に spawn するツール (graph compile / runner-gen) にはピン留め環境 `AIDLC_PROJECT_DIR`, `AIDLC_STAGE_GRAPH`, `AIDLC_SENSORS_DIR` に加え **`AIDLC_WORKSPACE_LOCK_OWNER_PID`** が渡る — 子プロセスがロックを再取得せず「保持済み」として動く仕組み | 11 §5.3 L485-488; `compose.ts:276-298` |
| ロック下トランザクション | ロック保持下でスナップショット式書込トランザクション (no-clobber コピー → contribution merge → 変更時 recompile)。compile 失敗は全書込ロールバック + retry marker `<project>/aidlc/.plugin-compose-retry-<PLUGIN_KEY>` | 11 §5 L374-382, §5.3 L474-477 |
| 失敗の非伝播 | compose はホストセッションを決して壊さない — catch が `compose threw: <msg>` を記録して return | 11 §5.1 L398-400; `compose.ts:1851-1856` |
| エンジン側の同一ロック使用 | plugin sync の downstream surface 更新も workspace lock 内 (`aidlc-utility.ts:847-931`) | 11 L303 |

### 5.2 NG-01 裁定 B5/B4 に関わる事実の対応

| NG-01 裁定 | 対応する upstream 事実 |
| --- | --- |
| 「compose とエンジンが共有するワークスペースロックは workspace 単独所有のロックサービスとし、**plugin はその顧客に格下げする**」(NG-01 L63) | upstream では compose が**インストール済み lib の関数 import + env トークン**でロックを物理共有 (上表)。この「同一実装の二重ロード + PID 環境変数」構造が、Rust では Customer/Supplier の依存方向に置換される対象 |
| B5「workspace は台帳の mechanics (fork/merge、prefix-hash、audit-first、ロック) をイベント意味論から独立に所有し、イベント行は opaque。merge-protected はイベントスキーマ (Published Language) 上の宣言」(NG-01 L169) | upstream の `MERGE_PROTECTED_EVENT_TYPES` は prefix 族でなく**明示列挙 26 + `DOCUMENT_*` prefix** (`aidlc-audit.ts:395`, `:426-429`) で、判定は audit ツール内にハードコード — NG ではスキーマ駆動宣言に昇格 |
| B5「swarm のマージ失敗 converged unit の復旧は orchestration の**サーガ**」(NG-01 L169, 10-orchestration L17) | upstream 事実: merge-back が失敗した converged unit は `SWARM_UNIT_CONVERGED` も `SWARM_UNIT_FAILED` も得ない (「監査行なし」中間状態)。理由 (逐語): converged 行は *"the engine's batch-advance signal, and emitting it for a unit whose metadata never landed on main would advance the run past an unmerged unit"*; 失敗エンベロープ + exit 2 が merge 結果を運び、行は scoped retry で着地 (09 §6.5 L378; `aidlc-swarm.ts:1099-1103`) |

---

## 6. セッションスタンプ・handoff receipt・SESSION_* の所有

### 6.1 `.aidlc-sessions` セッションスタンプ

| 要素 | 契約 | 典拠 |
| --- | --- | --- |
| 場所 | `aidlc/.aidlc-sessions/` — per-conversation session→intent map (gitignored)。スタンプは `aidlc/.aidlc-sessions/<id>` (id は Claude Code session_id) | 03 §3.1 L134, §3.4 L224/L244 (*"per-user runtime state keyed by Claude Code session_id, never shared truth"*); 07 §4.1 L118 |
| スタンプ書込 | SessionStart フックが STARTED 系イベント時に live intent UUID をスタンプ。`resume` で**別の・まだ解決可能な** UUID がスタンプ済みなら `INTENT REBIND OFFER: This conversation was working …` で始まるオファーを合成 (Codex では `$aidlc`、他では `/aidlc` の正確な切替コマンドを名指し)。**オファー提示と同時に live intent へ即再スタンプ** — 辞退されても usage が旧 workflow に付かない | 07 §4.1 L118; `aidlc-session-start.ts:181-192` |
| intent birth 時の束縛 | PostToolUse (`aidlc-rebuild-stage-graph.ts`) の `bindCreatedIntentToInvokingSession` が tool response を regex `/(?:Intent created:|Migrated flat workspace into intent:)\s*([A-Za-z0-9._-]+)\s+\(space:\s*([A-Za-z0-9._-]+)\)/` で照合し、新 intent の UUID を呼び出し session_id へスタンプ | 07 §7 L323; `:74-76` |

### 6.2 handoff receipt の TTL

| 要素 | 契約 | 典拠 |
| --- | --- | --- |
| 発生 | session が**既にスタンプを持つ**場合、上書きせず *handoff receipt* を書く — Stop フックの carve-out 0 がこれを消費 | 07 §7 L323 |
| TTL | `SESSION_INTENT_HANDOFF_TTL_MS = 5 * 60 * 1000` (**5 分**) | 07 L291, L425; `core/tools/aidlc-lib.ts:2147` |
| carve-out 0 の条件 | fresh な receipt の `from`/`to` UUID が session スタンプと live cursor に**まだ一致**していること (`aidlc-continue-workflow.ts:1148-1170`) | 07 §7.4 L291 |

### 6.3 SESSION_* イベントの所有

Session カテゴリは 5 種で **hook-owned** と分類: `SESSION_STARTED` `SESSION_RESUMED` `SESSION_COMPACTED` `SESSION_ENDED` `HUMAN_TURN` (03 §6.5 L777)。

| イベント | 所有フック | 契約 | 典拠 |
| --- | --- | --- | --- |
| `SESSION_STARTED` / `SESSION_RESUMED` | `aidlc-session-start.ts` (SessionStart) | source マッピング (逐語): `startup → SESSION_STARTED`, `clear → SESSION_STARTED`, `resume → SESSION_RESUMED`, `malformed → SESSION_STARTED`, `compact`/`unknown` → **emit なし** | 07 §4.1 L117; `:134-139` |
| `SESSION_COMPACTED` | `aidlc-validate-state.ts` (PreCompact) が所有 — *"firing it twice would pollute the audit trail"*。フィールド `Current Stage`・`State Validity` (`valid`/`invalid`)、audit ファイル存在時のみ | 07 §4.3 L141, §4.1 L117; `aidlc-session-start.ts:17-18` |
| `SESSION_ENDED` | `aidlc-session-end.ts` (SessionEnd) | `Reason` フィールド付き (stdin に無ければ `unknown`)。帰属は fail-closed: 未知 intent へのスタンプは drop `session <id> is stamped to unknown intent <uuid>; refusing active-cursor fallback`; active UUID を持つ workspace での未スタンプ session は shared-cursor fallback 拒否。*"ending a session does NOT complete the workflow. This event is observability only"* | 07 §4.4 L143-147 |
| `HUMAN_TURN` | prompt-submit フック (`aidlc-record-human-turn.ts`; Kiro 系はアダプタが `markHumanTurn` seam へインライン、Copilot は audit 半分のみインライン) | `CLI_PROTECTED_EVENT_TYPES` に属し、audit CLI 直接 emit は `AIDLC_ALLOW_DIRECT_AUDIT_EVENTS=1` なしでは拒否 | 03 §6.6 L815-819; 07 L387 |
| (関連) authority 拒否文言 | `Direct emission of <E> is blocked: it is an authority-bearing receipt owned by its emitting tool or hook (gate resolutions and approvals come from aidlc-orchestrate.ts report, interview answers and reviews from aidlc-log.ts, human presence from the prompt-submit hook). The audit CLI appends diagnostic events only.` / 予約セット (8 種、構成員は 03 では非列挙): `<E> is reserved for its owning hook/tool and cannot be appended through the public audit CLI.` | 03 §6.6 L821, L825 |

### 6.4 audit_lock.qnt に関わる補足の順序事実 (03 §6.4)

- 監査行に**シーケンス番号は無い**。秒精度 ISO timestamp のみ。shard 内は append 順 = buffer 順で保存; shard 横断は `findAllEvents` が timestamp ソート + buffer 位置 tiebreak で**決して fail-closed しない** (03 §1 L27-31, §6.4 L726-738)。
- **authority-bearing 比較 (`humanActedSinceGate`) のみ cross-shard tie で fail-closed**: 最新 `HUMAN_TURN` と別 shard の最新 gate resolution が同一秒を共有したら *"execution order is unknowable and the check fails CLOSED (require a fresh turn)"*。勝利条件は全 latest resolution が `resolution.shard === human.shard && resolution.pos < human.pos` を満たすこと (03 §6.4 L739-748; `aidlc-lib.ts:3752-3754`, `:3838-3853`)。
- clone-id (`aidlc/.aidlc-clone-id`, `/^[a-z0-9]{1,32}$/`, 欠落時 12 hex mint 後**再読**で並行 first-run が単一トークンに収束) と shard 名 `<host>-<cloneId>.md` が git merge 衝突を除去する設計 (03 §3.3 L193-211)。

---

**関連ファイル (絶対パス)**:
- /Users/j5ik2o/orca/workspaces/amadeus-ng/docs/docs/upstream/specs/03-state-audit-runtime.md (§2.3, §3.3-3.4, §4.5, §5.3, §5.7, §6.4-6.11, §7.5)
- /Users/j5ik2o/orca/workspaces/amadeus-ng/docs/docs/upstream/specs/09-cli-tools.md (§5.2-5.5, §6.5, §7, §11 L842, §12 L904, §14 L1046)
- /Users/j5ik2o/orca/workspaces/amadeus-ng/docs/docs/upstream/specs/11-plugin-system.md (§5, §5.1, §5.3)
- /Users/j5ik2o/orca/workspaces/amadeus-ng/docs/docs/upstream/specs/07-hooks.md (§2, §4.1-4.4, §7 L291/L323, L425)
- /Users/j5ik2o/orca/workspaces/amadeus-ng/docs/docs/specs/01-domain-model.md (L63, L145-147, L168-170: 裁定 B4/B5/B6)