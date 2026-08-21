> 抽出元: upstream as-built 仕様 (awslabs/aidlc-workflows v2 @ 3c3146cf, v2.6.40)。2026-08-22 の 3 エージェント精密抽出。11-workspace.md と audit_lock.qnt の執筆材料。

調査完了。以下が最終成果物です。

---

# タスク 1 抽出結果: 状態ファイルと Space/Intent の契約の完全列挙

典拠: `docs/upstream/specs/03-state-audit-runtime.md` (upstream v2.6.40, commit `3c3146cf`)。表中の「03:Lnnn」は同ファイルの行番号、`aidlc-*.ts:nnn` は upstream コードの行番号 (仕様に記載のまま転記)。補助典拠として `08-memory-rules-learnings.md` (以下 08)、`09-cli-tools.md` (以下 09) を明示のうえ使用。

---

## 1. `aidlc-state.md` の構造

### 1.1 形状の全体契約 (§5.1)

| 項目 | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| 配置 | intent ごとに 1 つの Markdown 文書 `<record>/aidlc-state.md` | §5.1, 03:L438-439; `stateFilePath` `aidlc-lib.ts:2545` (03:L367) |
| セクション数 | `##` セクションが **9 個**。全フィールドはトップレベル bullet `- **<Field>**: <value>` の厳密形 | §5.1, 03:L438-440 |
| 正準テンプレート | `core/knowledge/aidlc-shared/state-template.md`。ステージ列挙を明示的に拒否: *"the engine writes the concrete state file and enumerates stages from the compiled stage graph plus scope grid; this template must not hand-list shipped stages"* (`state-template.md:3-5`) | §5.1, 03:L442-447 |
| birth エミッタ | `aidlc-utility.ts:4229-4282`。同じ 9 セクション + リテラル bullet **30 個** + 補間 Phase Progress **5 行** (`phaseProgressLines`, `aidlc-utility.ts:4221-4227`) = 実行時 35 bullet。テンプレート側は 31 フィールド (`[Phase]` プレースホルダ行込み、M7) | §5.1, 03:L460-462; M7, 03:L1200-1210 |

### 1.2 9 セクションとフィールドの全列挙 (§5.1, 03:L448-458)

| # | セクション | フィールド (テンプレート順) |
| --- | --- | --- |
| 1 | `## Project Information` | Project, Project Type, Scope, Start Date, State Version, Active Agent, Worktree Path, Bolt Refs, Practices Affirmed Timestamp |
| 2 | `## Scope Configuration` | Stages to Execute, Stages to Skip, Depth, Test Strategy |
| 3 | `## Workspace State` | Project Root, Languages, Frameworks, Build System |
| 4 | `## Execution Plan Summary` | Total Stages, Completed, In Progress |
| 5 | `## Runtime State` | Revision Count |
| 6 | `## Phase Progress` | phase ごとに `- **<Phase>**: <status>` 1 行 |
| 7 | `## Stage Progress` | コンパイル済みステージごとにチェックボックス 1 行、`### <PHASE> PHASE` でグループ化 |
| 8 | `## Current Status` | Lifecycle Phase, Current Stage, Next Stage, Status, Construction Autonomy Mode, Last Updated |
| 9 | `## Session Resume Point` | Last Completed Stage, Next Action, Pending Artifacts |

birth とテンプレートの実在する差分 2 件 (§5.1, 03:L463-466; §5.8, 03:L631-632):

| 差分 | 内容 | 典拠 |
| --- | --- | --- |
| birth が書き、テンプレートに無い | `- **Review Override**:` を `## Scope Configuration` に書く (`aidlc-utility.ts:4247`) | §5.8, 03:L632 |
| テンプレートにあり、birth が書かない | `- **Construction Autonomy Mode**:` (`state-template.md:61` vs `aidlc-utility.ts:4271-4276`)。→ M12 問題 (後述 §2.2) | §5.8, 03:L631 |

### 1.3 値の文法と制約 (§5.2)

| 項目 | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| フィールド読取 regex | `getField` は `^- \*\*<Field>\*\*:[ \t]*(.*)$` (`m` フラグ)、trim した capture か `null` を返す | §5.2, 03:L470-471; `aidlc-lib.ts:6487` |
| 水平空白クラスの理由 | JS では `\s*` が `\n` にマッチするため、空値フィールドが次の bullet 行を飲み込む — 意図的に `[ \t]*` | §5.2, 03:L472-474; `aidlc-lib.ts:6489-6491` |
| 単一行制約 | `hasUnsafeSingleLineCharacter` がコードポイント走査で `<= 0x1f`, `0x7f`, `0x2028`, `0x2029` を拒否 (C0 制御, DEL, Unicode 行/段落区切り) | §5.2, 03:L475-479; `aidlc-lib.ts:6436-6448` |
| 適用先 | `validateStateLineValue` (`aidlc-state.ts:1073`) が呼び出し側指定の `--reason` / `--next-action` テキストに適用 | §5.2, 03:L479-480 |
| `Bolt Refs` のリスト文法 | 単一行のリスト値。`parseRefsList` (`aidlc-lib.ts:6635`) は `""`・リテラル `[empty list]`・角括弧カンマ区切りを受理。`emitRefsList` (`:6647`) は空なら常に `[empty list]`、非空ならソート済み角括弧リストを出す (round-trip 決定的)。`appendSlug` / `removeSlug` (`:6653`, `:6662`) は重複/不在 slug で throw (no-op しない) | §5.2, 03:L481-485 |

### 1.4 State Version 分類器 (§5.5)

| 項目 | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| 現行バージョン | `export const CURRENT_STATE_VERSION = "8";` (`aidlc-lib.ts:10605`) | §5.5, 03:L557-559 |
| 分類器 | `classifyStateVersion(stateContent)` (`aidlc-lib.ts:10627`)。runtime (`aidlc-orchestrate next`/`report`) と `--doctor` の**両方が同一の分類器を使う** (不一致不可能) | §5.5, 03:L561-563 |
| マッチ regex | `/^- \*\*State Version\*\*:[ \t]*(\S+)[ \t]*$/m` — 行末アンカーのため `State Version: 8 garbage` は `unparseable` に落ちる | §5.5, 03:L563-565 |
| 戻り値 | `{kind:"ok"} \| {kind:"unparseable"} \| {kind:"past"} \| {kind:"future"}` の 4 分類 | §5.5, 03:L565-566 |
| unparseable 時の指示 | メッセージはアーカイブして作り直せと指示: `mv aidlc aidlc.archive` | §5.5, 03:L566 |

### 1.5 ファイル I/O 契約 — アトミック書込と read-only バリア (§5.6)

| 項目 | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| 読取 | `readStateFile` (`aidlc-lib.ts:6453`) は不在時 `State file not found: <path>` を throw | §5.6, 03:L570 |
| 書込前チェック | `writeStateFile` (`:6461`) — 対象が存在すれば `accessSync(path, W_OK)` を先に呼び `EACCES` を伝播、存在しなければ親チェーンを `mkdir -p` | §5.6, 03:L571-573 |
| W_OK 事前チェックの理由 | 書込自体は `writeFileAtomic` (tmp + rename) 経由であり、*"POSIX rename overwrites a read-only TARGET (it only needs directory-write permission), so it would bypass that barrier"* (`:6463-6469`)。**read-only な `aidlc-state.md` は意図的な書込バリアとして扱う** | §5.6, 03:L573-576 |
| アトミック性 | 書込は tmp + rename でアトミック。クラッシュしても並行リーダに破断ファイルを見せない (`:6477-6481`) | §5.6, 03:L577-578 |

---

## 2. writer 4 種と使い分け・既知問題

### 2.1 4 writer の完全契約 (§5.3, 03:L489-505) — すべて純粋な string→string

| Writer | フィールド**存在**時 | フィールド**不在**時 | 典拠 |
| --- | --- | --- | --- |
| `setField` | 値を置換 | **無言 no-op** (content をそのまま返す) | `aidlc-lib.ts:6546` (03:L493) |
| `setFieldStrict` | 値を置換 | **throw**: `Field not found in state file: "<f>". Cannot update — refusing to silently no-op.` | `aidlc-lib.ts:6564` (03:L494) |
| `setOrInsertField(content, heading, field, value)` | 値を置換 | 指定 `## Heading` の末尾に bullet を追記 | `aidlc-lib.ts:6599` (03:L495) |
| `removeField` | bullet 行全体 (末尾改行込み) を削除 | no-op | `aidlc-lib.ts:6620` (03:L496) |

設計則 (逐語): `setFieldStrict` は *"in state-machine transitions where a silent no-op would cause undetected drift … if the field is missing, we want to know immediately, not ship a lie to the caller."* (`aidlc-lib.ts:6560-6563`; 03:L498-500)

**エンジン内の `setFieldStrict` 呼び出し 4 箇所すべて** (§5.3, 03:L502-505):

| 対象フィールド | 契機 | サイト |
| --- | --- | --- |
| `Bolt Refs` (追記) | fork 時 | `aidlc-state.ts:4042` |
| `Worktree Path` | worktree コピー時 | `aidlc-state.ts:4074` |
| `Bolt Refs` (除去) | worktree merge 経路 | `aidlc-state.ts:4217` |
| `Construction Autonomy Mode` | `aidlc-bolt set-autonomy` | `aidlc-bolt.ts:837` |

補助 writer: `setPhaseProgress` (`aidlc-lib.ts:6585`, 03:L507-510) は `setField` の薄い wrapper。phase slug を大文字化 ("ideation"→"Ideation") し `Pending | Active | Verified | Skipped` のいずれかを書く。行不在時は意図的 no-op — *"the section is display-only, so a missing row must never fail a transition"* (`:6582-6584`)。

**runtime-only フィールド** (base テンプレートに無く `setOrInsertField` で挿入; §5.3, 03:L514-524):

| フィールド | セクション | writer |
| --- | --- | --- |
| `Skeleton Stance` (`on`/`off`/`scope-dependent`) | `## Runtime State` | `aidlc-state.ts:724` (`set-skeleton-stance`) |
| `Construction Iteration` (`unit-major`/`stage-major`) | `## Runtime State` | `aidlc-state.ts:764` |
| `Parked` (ISO ts), `Parked At Stage` | `## Runtime State` | `aidlc-state.ts:814-815` (`park`); `unpark` が除去 `:831-832` |
| `Active Unit`, `Unit State`, `Unit Pause Reason`, `Unit Next Action` | `## Runtime State` | `aidlc-state.ts:1046-1055`; `unit complete` で 4 つとも除去 `:1041-1044` |
| `Merge-Held` (`true`/`false`) | `## Project Information` — **per-Bolt fork 済み state 限定** | `aidlc-bolt.ts:692` |

**`Practices Affirmed Timestamp` の例外** (§5.3, 03:L527-535): `setOrInsertField` で書かれる (`aidlc-state.ts:3743`) が runtime-only では**ない**。テンプレートフィールド (`state-template.md:20`) であり birth が空値 bullet を出す (`aidlc-utility.ts:4240`) ため、本エンジンが作った state では常に *replace* アームを通る。insert アームはレガシー修復専用: *"a state file missing the row (a hand-edited or pre-field file)"* (`:3739-3742`) — `setField` なら無言 no-op になり、このタイムスタンプを要求する approve ゲートが永久拒否し、その是正手段 ("run practices-promote") も no-op し続けるため。

### 2.2 既知の M12 問題 (§5.8, 03:L631; M12, 03:L1238-1249)

| 事実 | 典拠 |
| --- | --- |
| テンプレートは `Construction Autonomy Mode` を宣言 (`state-template.md:61`) するが birth エミッタは書かない (`aidlc-utility.ts:4271-4276`) | 03:L631 |
| リーダは `getField` → `null` → 非 autonomous 扱いで安全に劣化 | 03:L631 |
| **唯一の writer** `aidlc-bolt set-autonomy` は `setFieldStrict` (`aidlc-bolt.ts:837`) を使い、このフィールドの `setOrInsertField` サイトは**存在しない** (M12)。新規 birth 直後の state では `State update failed: Field not found in state file: "Construction Autonomy Mode". …` で必ず失敗する | 03:L631, 03:L1238-1249 |
| テストは製品経路でなく regex で行を注入して回避 (`tests/unit/t186-foreach-per-unit-iteration.test.ts:205`, `tests/unit/t215-bolt-dag-selfheal.test.ts:250`) | 03:L631 |
| M12 の計測: `setOrInsertField` 呼び出しサイト 10 箇所 (`aidlc-bolt.ts:692`, `aidlc-state.ts:724`, `:764`, `:814`, `:815`, `:1046`, `:1047`, `:1054`, `:1055`, `:3743`) のいずれもこのフィールドを指名しない。"Construction Autonomy Mode" 出現 142 行中 writer は `aidlc-bolt.ts:837` のみ | 03:L1238-1249 |

### 2.3 チェックボックス文法と setCheckbox / setStageSuffix の分離 (§5.4)

| 項目 | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| パース regex | `parseCheckboxes` (`aidlc-lib.ts:6678`) は `/^- \[([ xSR?-])\] (\S+)\s*—\s*(.*)$/gm` — 区切りは **em dash (—)** | §5.4, 03:L537-538 |
| 6 状態 | `[ ]`=`pending`, `[-]`=`in-progress`, `[?]`=`awaiting-approval`, `[R]`=`revising`, `[x]`=`completed`, `[S]`=`skipped` | §5.4, 03:L539-547 |
| 分離則 | `setCheckbox` (`:6713`) は**マーカーのみ**を書換え、`setStageSuffix` (`:6733`) は **`EXECUTE`/`SKIP` 末尾のみ**を書換える。逐語: *"setCheckbox owns the marker (run-state); this owns the suffix (the plan) - the two edit disjoint fields of the same line, so recompose and jump compose cleanly."* (`:6727-6731`) — 同一行の互いに素なフィールド | §5.4, 03:L549-552 |
| 集計 | `countCheckboxes` (`:6745`) が `Completed` フィールド同期に使われる (`aidlc-state.ts:2240-2241`) | §5.4, 03:L552-553 |
| 凡例コメントの不一致 (参考) | テンプレート (`state-template.md:48`)・エミッタ (`aidlc-utility.ts:4269`, 末尾 `[S] skipped via --stage/--phase jump`)・rewrite regex ヘッダ (`aidlc-utility.ts:5013`, `[?]`/`[R]` 欠落) の 3 種の文言が不一致。凡例は装飾であり `parseCheckboxes` はマーカーのみ読む | §5.8, 03:L633 |

---

## 3. 「監査が真実源、state はキャッシュ」の対象と規則

| 項目 | 内容 | 典拠 |
| --- | --- | --- |
| 対象フィールド (unit 系) | `Active Unit`, `Unit State`, `Unit Pause Reason`, `Unit Next Action` の 4 つ。逐語: *"audit stays the source of truth — these fields are a cache, exactly like Parked / Parked At Stage"* (`aidlc-state.ts:1036-1038`) | §5.3, 03:L525-526 |
| 同型の先行例 | `Parked` / `Parked At Stage` (park/unpark の挿入・除去、`aidlc-state.ts:814-815`, `:831-832`) | §5.3, 03:L520 |
| ライフサイクル | `unit start\|pause\|resume\|complete` (`aidlc-state.ts:861`) がフィールドを書き、`unit complete` (`:1041-1044`) で 4 つとも除去。真実源は監査イベント `UNIT_STARTED` `UNIT_PAUSED` `UNIT_RESUMED` `UNIT_COMPLETED` (Unit Lifecycle カテゴリ) | §5.3, 03:L521; §6.5, 03:L781 |
| 再構築の境界規則 | unit ライフサイクルイベントはカウンタなしで境界を確定する: `Run floor` は正確なトークン `<event>:<timestamp>#<ordinal>`、別シャードでの同時刻境界は決定的な `AMBIGUOUS:<timestamp>#<digest>` floor に劣化し、過去のレシートは一致し得ない (`audit-format.md:114-119`) | §6.4, 03:L750-753 |
| 保護 | 4 つの `UNIT_*` レシートは `CLI_PROTECTED_EVENT_TYPES` (直接 emit 拒否) かつ unit-lifecycle レシートとして `MERGE_PROTECTED_EVENT_TYPES` (worktree delta で移動不可) に含まれる | §6.6, 03:L815-818, L827-834 |

注: 03 は「キャッシュである」という原則と除去規則・イベント対応までを規定し、キャッシュ再構築の具体的アルゴリズム (audit リプレイ手順) 自体は明文化していない。再構築時の順序決定は §6.4 の一般順序契約 (timestamp ソート + バッファ位置 tiebreak、authority 比較のみ cross-shard tie で fail-closed) に従う。

---

## 4. エンジン所有 11 動詞と advance のガードスタック

### 4.1 遷移所有権ガード (§5.7, 03:L581-599)

| 項目 | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| エンジン所有 11 動詞 | `set, checkbox, advance, finalize, complete-workflow, gate-start, approve, reject, revise, skip, park` (`aidlc-state.ts:524-549`; 全 25 サブコマンド中 11、M10) | 03:L582-589, M10 03:L1227-1228 |
| ガード条件 | `process.env.AIDLC_STATE_TRANSITION_OWNER === `orchestrate:${process.ppid}`` — **PID 束縛マーカー** (静的トークンのコピーは無効) | 03:L590-591; `aidlc-state.ts:540` (03:L109) |
| バイパス env | `AIDLC_ALLOW_DIRECT_STATE_TRANSITIONS === "1"` | 03:L592; `aidlc-state.ts:541` (03:L110) |
| 拒否文言 (逐語) | `Direct aidlc-state.ts <sub> is blocked: workflow lifecycle transitions are engine-owned. Use aidlc-orchestrate.ts report --stage <slug> --result <awaiting-approval\|approved\|rejected\|revised\|completed\|skipped>; use aidlc-orchestrate.ts park to park, and next/jump for routing changes.` | 03:L592-594 |
| クリティカルセクション | 全 read-modify-write ハンドラは `withAuditLock(pd, …)` 内で実行 — read → decide → audit → write が 1 クリティカルセクション | §5.7, 03:L596-597 |
| audit-first 不変条件 | 監査行はロック内で emit され state 書込がその後に続く。**audit エラー throw は state 書込をスキップする** (`aidlc-state.ts:128-130`, 例 `:2255-2286`) | §5.7, 03:L597-599 |

### 4.2 advance のガードスタック — 全 8 項 (§5.7, 03:L601-618)

03 の番号は 6 項だが第 6 項が 3 precondition を束ねるため、展開すると 8 ガード:

| # | ガード | 契約 (逐語) | 典拠 |
| --- | --- | --- | --- |
| 1 | Scope 妥当性 | `Scope` が存在し `validScopes()` に含まれること — 無言の `feature` フォールバックではなく *"Refusing to advance"* (`aidlc-state.ts:2096-2106`) | 03:L602-604 |
| 2 | 完了 slug の一致 | 完了 slug は `Current Stage` に等しい **または** すでに `[x]` (`:2117-2131`) | 03:L605 |
| 3 | next slug の SKIP 禁止 | 呼び出し側指定の next slug は state の suffix にも scope マッピングにも `SKIP` であってはならない (`:2142-2150`) | 03:L606-607 |
| 4 | 冪等/リプレイガード | 遷移が既に完全適用済みならクリーンに exit (`:2174-2196`) | 03:L608-609 |
| 5 | reviewer 前提 | `verifyReviewerPrecondition` (`:1775`) — reviewer を持つステージは terminal な `REVIEW_COMPLETED` レシートが必要 | 03:L610-611 |
| 6 | 成果物検証 | `verifyStageArtifacts` (`:2210-2214`)。ステージ完了済みならスキップ | 03:L612-613 |
| 7 | サマリ確認前提 | `verifySummaryConfirmationPrecondition` (同上) | 03:L612-613 |
| 8 | パイプラインリンク前提 | `verifyPipelineLinkPrecondition` (同上) | 03:L612-613 |

通過後の動作 (03:L615-618): チェックボックス反転、**10 フィールド**更新、phase 境界で Phase Progress 行反転、`STAGE_COMPLETED` (+境界時 `PHASE_COMPLETED`/`PHASE_VERIFIED`/`PHASE_STARTED` の 3 点セット) と `STAGE_STARTED` を emit、state 書込。

関連ガード (§5.7, 03:L619-626):

| 動詞 | ガード |
| --- | --- |
| `park` | 自律 Construction 下で拒否 (`aidlc-state.ts:796-801`)、`Status` が `Completed` でも拒否 (`:803-805`) |
| `unit start\|pause\|resume\|complete` (`:861`) | 単一アクティブユニット不変条件を強制; 自律 swarm がステージ所有中は拒否 (`:906-912`); unit は権威 DAG に存在必須 (`:921-925`); `complete` はレシート commit **前**に全必須成果物のディスク存在を検証 (`:980-988`) — *"the claim-1 inversion — the artifact walk moved from 'is the transition' to 'is checked by the transition'"* (`:976-979`) |

---

## 5. Space の契約

### 5.1 4 サブツリーとパスヘルパ (§3.1)

各 space (`aidlc/spaces/<space>/`) は 4 サブツリーを持つ (03:L136-144):

| サブツリー | 内容 | ヘルパ | 典拠 |
| --- | --- | --- | --- |
| `memory/` | `org.md` `team.md` `project.md` `phases/` `templates/` | — | 03:L138 |
| `knowledge/` | 自由形式のチーム知識; `documents/` + `documentkb/` | `knowledgeDir` `aidlc-lib.ts:1324` | 03:L139, L158 |
| `codekb/<repo>/` | リポジトリ別コード知識 | `codekbDir` `aidlc-lib.ts:1436` | 03:L140, L159 |
| `intents/` | レジストリ + intent record 群 + `active-intent` カーソル | `intentsDir` `aidlc-lib.ts:1312` | 03:L141-144, L157 |

その他: `spacesRoot` = `aidlc/spaces` (`aidlc-lib.ts:1924`)、`spaceRecordRoot` = `intentsDir` (null-intent フォールバックルート、`:1669`)、`relativeSpaceRecordPrefix(space)` = posix 区切りの `aidlc/spaces/<space>/intents` (`:1679`) (03:L160-162)。

### 5.2 名前 regex と default space の特例 (§3.1)

| 項目 | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| 名前 regex | `SPACE_NAME_REGEX = /^[a-z][a-z0-9-]*$/` (`aidlc-lib.ts:1341`)。`--space` フラグは `validSpaceFlag` (`:1343`) で検証 — *"it is a path segment that must never reach `join()` raw"* | 03:L166-169 |
| 定数 | `DEFAULT_SPACE = "default"` (`aidlc-lib.ts:591`) | 03:L177 |
| 特例 1 | `activeSpace` は**決して throw しない** — *"NEVER throws — the default space is always valid even when nothing is on disk yet"* (`aidlc-lib.ts:1298-1299`)。カーソル不在/空なら `"default"` | 03:L156, L164-165 |
| 特例 2 | `listSpaces` (`:1962`) は `aidlc/spaces/` が存在しなくても常に `default` を報告 | 03:L165-166 |
| 配布シード | `dist/claude/aidlc/active-space` の内容は `"default"`、default space の memory ツリーのみ同梱 (intent record なし) | §3.5, 03:L262-267 |

### 5.3 新規スペースの継承規則 (典拠: **08 §2.4** — 03 には無い)

| 項目 | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| 作成 | `handleSpaceCreate` (`aidlc-utility.ts:4799-4862`) が `memory/`, `memory/phases/`, `memory/templates/`, `intents/`, `codekb/`, `knowledge/` を作成 | 08 §2.4 (08:L72) |
| 継承 | default space から **`org.md` のみ**コピー。`team.md` / `project.md` は 1 行スタブ `# Team practices` / `# Project overrides` を新規作成 (`aidlc-utility.ts:4837-4850`) | 08 §2.4 |
| 意図 (逐語) | *"A new team starts at the framework baseline and earns its OWN practices — it does NOT inherit another space's learnings"* (`aidlc-utility.ts:4795-4797`) | 08 §2.4 |
| knowledge floor | `space create` は空 committed dir を追跡させる `.gitkeep` を置く (`aidlc-utility.ts:4857-4858`) | 08 §7 (08:L507) |
| switch との分離 | 未知 space への switch はエラーで、作成を促さない: `Unknown space "<t>". Existing: … This command only switches between existing spaces. Do not create a space to recover from this error - creating one is a separate, deliberate move (/aidlc space create <name>, or legacy /aidlc space-create <name>).` | 09 (09:L182) |
| switch の副作用 | switch は per-user 書込 **2 つ**: gitignored `active-space` カーソル + `repointHarnessIncludes()` (`aidlc-utility.ts:4552-4562`) | 09 (09:L171) |

---

## 6. Intent の契約

### 6.1 識別子と dirName の分離 (§4.1)

| 項目 | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| 正準 id | **UUIDv7** — `uuidv7()` (`aidlc-lib.ts:1698`): 48-bit Unix-ms プレフィクス + version nibble `7` + `randomUUID()` 由来の暗号学的 tail (`Math.random` 不使用)。uuid 文字列ソート = 作成順 | 03:L306-309 |
| dirName | `<YYMMDD>-<short-label>` — `intentDirNameBase` (`aidlc-lib.ts:1765`)、`dateStamp()` は UTC `YYMMDD` (`:1754`)、`slugify(label, 24)` (`:1717`)。時刻トークンが*プレフィクス*なのは `ls` で時系列ソートさせるため、ラベルは 2–3 語の要点 (cap 24、旧 48 から縮小) (`:1731-1735`) | 03:L310-315 |
| 衝突 | `resolveUniqueIntentDir` (`:1781`) が `-2`, `-3`, … を `MAX_DIR_COLLISIONS = 1000` まで付与し、以後はループせず loud に throw | 03:L316-317 |
| 予約ラベル | `RESERVED_RECORD_NAMES` (`aidlc-lib.ts:836`) = `RESERVED_RECORD_NAME_LIST` (`:826`) = `"help"` ∪ `INTENT_VERBS` ∪ `SPACE_VERBS` ∪ `RESERVED_FUTURE` = **`help, list, switch, create, archive, rename, show, birth`**。`createIntent` は `…is a reserved name and cannot be an intent label` を throw (`:2335-2337`) | 03:L317-321 |

### 6.2 birth のチョークポイント (§4.1, 03:L322-329)

| ステップ | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| 単一入口 | `createIntent` (`aidlc-lib.ts:2319`) が birth のチョークポイント: uuid mint → dirName 解決 → record を `mkdir` | 03:L322-324 |
| stub 書込 | `if (!existsSync(statePath))` ガード (`:2351`) 下で**ヘッダのみの stub** `aidlc-state.md` = `# AI-DLC State Tracking\n` を書く (`:2352`) | 03:L324-326 |
| stub の理由 | `activeIntent()` は `aidlc-state.md` を持つディレクトリのみ実 record と見なすため、stub がないと mint〜full state 書込の間にカーソルが解決せず、**birth 後の書込が裸の space root に漏れる** (`:2343-2350`) | 03:L326-329 |
| status | birth はレジストリ status に `"in-flight"` を書く (workflow 完了時に terminal status) | §4.2, 03:L358-359 |
| 遅延スキャフォールド | `ensureWorkspaceDirs` (`aidlc-utility.ts:3764`) が birth 時に record dir・in-scope phase ごとの subdirectory・`verification/`・space-level `knowledge/` を遅延作成、*"never SEED"* (`:3782`)。scope 外 phase の dir は作られない (`phasesWithExecuteStages(scope)`, `:3771-3773`)。birth 監査は件数を記録: `WORKSPACE_SCAFFOLDED`, `Details: "<n> in-scope phase dirs + verification/ + space-level knowledge/ ensured (shell shipped by SEED)"` (`aidlc-utility.ts:4032-4036`) | §3.5 03:L269-274; §4.3 03:L386-390 |

### 6.3 `intents.json` レジストリ (§4.2)

行スキーマ (逐語、`aidlc-lib.ts:1874-1887`; 03:L334-344):

```ts
export interface IntentRegistryEntry {
  uuid: string;
  slug: string;
  dirName?: string;   // stored verbatim at birth; optional for pre-spike rows
  scope?: string;
  repos?: string[];
  status: string;
}
```

| 項目 | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| 配置 | `intentsRegistryPath` = `<space>/intents/intents.json` (`aidlc-lib.ts:1900`) | 03:L331-332 |
| 書込 | `appendIntentToRegistry` (`:1904`) — `writeFileAtomic` + 2-space JSON。不在/破損ファイルは fail せず新規リストを開始 | 03:L346-347 |
| 読取 | `readIntentRegistry` (`:1934`) は不在/破損で `[]` (同じ寛容性) | 03:L348 |
| 行→dir 結合則 | `recordDirMatches(entry, dirName)` (`:1893`) が唯一の結合ルール: `entry.dirName` の完全一致を優先、無ければレガシー `<slug>-<id8>` 形 (slug prefix + `idSuffix(entry.uuid, …)` の prefix となる末尾 hex run) にフォールバック | 03:L349-351 |
| 孤児 | `listIntents` (`:1991`) はレジストリ行とディスク上 dir を join し、**孤児 (行なし dir) を `uuid: ""`, `status: "unknown"` で追加**する | 03:L352-354 |
| status 更新 | `updateIntentStatus` (`:2372`) は行の `status` を in-place 反転 (birth=`"in-flight"`, 完了=terminal)。**workspace ロック下で実行必須** | 03:L355-357 |
| レジストリ非依存の列挙 | `listIntentDirs` (`:1353`) は `aidlc-state.md` を含む `intents/` エントリをソートして列挙 — 意図的にレジストリ非依存 (*"it must not depend on the registry being present"*, `:1352`) | 03:L359-361 |

**ロック規則** (§6.8, 03:L873-882): ロック識別子は `<realpath(projectDir)>\x00<space>\x00<intent>`、intent 省略時は `<realpath(projectDir)>\x00__workspace__` (`WORKSPACE_LOCK_SENTINEL`, `aidlc-lib.ts:6777`)。キーイング不変条件 2 つ (逐語要旨、`:6757-6768`):
1. intent 省略時は予約センチネルをハッシュし、**決して `activeIntent()` を解決しない** — birth 時点に active intent は無く、解決すると並行する 2 つの first-run が異なるバケットをキーして両方 birth してしまう。**すべての `intents.json` 変更はこのバケット (workspace センチネルロック) を取る**;
2. 複合 identity はロック dir と in-process depth/handler map の両方をキーする (さもなくば map が intent 間で衝突)。

### 6.4 `activeIntent()` の null 意味論 (§4.4)

優先順位 (`aidlc-lib.ts:1376`; 03:L394-400):

| 順位 | ソース | 条件 |
| --- | --- | --- |
| 1 | `explicit` 引数 | — |
| 2 | `active-intent` カーソル | **実際に `aidlc-state.md` を持つ**ディレクトリを指す場合のみ (`:1387`) |
| 3 | 唯一の intent | `listIntentDirs` がちょうど 1 件返すとき |
| 4 | `null` | 上記いずれも不成立 |

| 項目 | 契約 (逐語) | 典拠 |
| --- | --- | --- |
| null は仕様 | *"Returns null rather than throwing on ambiguity so the path helpers stay total; the verb/handler layer (P4) owns the error/prompt for the >1-intent-no-cursor case."* (`aidlc-lib.ts:1373-1375`) | 03:L402-405 |
| null 時の解決先 | 全絶対パスヘルパは `spaceRecordRoot` = 裸の `intents/` dir に解決。そこに `aidlc-state.md` が正当に存在することはない (`aidlc-lib.ts:579-587`) ため、存在ゲート付き consumer は正しく「workflow なし」と読む | 03:L407-409 |
| ガード例 (逐語) | `aidlc-log.ts` は emit 前にこれを確認 (`resolveActiveProjectDir`, `aidlc-log.ts:62-69`): `No active workflow — refusing to log an interaction event with no resolvable intent.` | 03:L410-412 |

### 6.5 active-space / active-intent カーソルの契約 (§3.2)

定数 (逐語、03:L174-178):

```ts
export const ACTIVE_SPACE_POINTER = "active-space";     // aidlc-lib.ts:589
export const ACTIVE_INTENT_POINTER = "active-intent";   // aidlc-lib.ts:590
export const DEFAULT_SPACE = "default";                 // aidlc-lib.ts:591
```

| カーソル | 内容 | writer | 契約 | 典拠 |
| --- | --- | --- | --- | --- |
| `aidlc/active-space` | space 名 | `setActiveSpaceCursor` (`aidlc-lib.ts:2067`) | **best-effort、失敗は swallow** — *"per-user cursor; best-effort"* | 03:L180-181 |
| `aidlc/spaces/<space>/intents/active-intent` | **record ディレクトリ名** (uuid ではない) | `setActiveIntentCursor` (`aidlc-lib.ts:2055`) | — | 03:L182-183 |
| (space カーソルの実体化) | — | `ensureActiveSpaceCursor` (`aidlc-lib.ts:2032`) | 並行 switch を潰さない: ステージング `aidlc/.aidlc-active-space-<pid>-<uuid>.tmp` に `flag: "wx"` で書き、`linkSync` (no-replace 意味論がアトミック) で設置後、ステージングを unlink | 03:L184-188 |
| 両者 | — | — | gitignored (§3.4) — fresh clone にはどちらも無い。ゆえに resolver は不在を許容 (`activeSpace` は default、`activeIntent` は `null`)。ignore glob (逐語): `aidlc/active-space`, `aidlc/spaces/*/intents/active-intent`。理由 (逐語): *"two teammates legitimately point at different spaces/intents at once; committing them would turn per-user navigation into shared state and cause conflicts on births and switches"* (`dot-gitignore:30-33`) | 03:L189-192, L220-221, L241 |

---

## 7. 執筆時の注意点 (11-workspace.md / audit_lock.qnt への含意)

| 注意点 | 内容 | 典拠 |
| --- | --- | --- |
| birth の並行性はロック keying に依存 | 2 つの first-run が両方 birth する競合は「intent 省略 = workspace センチネル」のキーイングで防がれる。`activeIntent()` 解決をロック keying に混ぜてはならない。Quint モデルの birth アクションはこのバケットを取ること | §6.8, 03:L877-880 |
| audit-first 不変条件 | ロック内順序は read → decide → **audit emit → state write**。audit throw で state write スキップ。R6 クラッシュモデルではこの中間状態 (audit 済み・state 未書込) が到達可能状態になる | §5.7, 03:L596-599 |
| state 書込のアトミック性 | tmp+rename で torn write なし。ただし read-only ターゲットは rename が貫通するため W_OK 事前チェックがバリアの実装 | §5.6, 03:L571-578 |
| advance の冪等ガード | リプレイ (ガード 4) はクラッシュ後再実行の受け皿 — Quint の crash-recovery 遷移と対応 | §5.7, 03:L608-609 |
| 03 に無いもの | 新規 space の継承規則は 08 §2.4、space verb の CLI 面は 09 §aidlc-utility が典拠。11-workspace.md で引用する場合は出典を 03 と混同しないこと | 08:L70-72, 09:L171-182 |

ソースファイル:
- `docs/upstream/specs/03-state-audit-runtime.md` (主典拠、全 1268 行読了)
- `docs/upstream/specs/08-memory-rules-learnings.md` (space 継承規則)
- `docs/upstream/specs/09-cli-tools.md` (space verb / switch エラー文言)