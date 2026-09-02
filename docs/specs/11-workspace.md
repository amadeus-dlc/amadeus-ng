# workspace コンテキスト仕様

> **改名裁定（2026-08-29 / Bolt B12）**: 集約 `WorkflowExecution` は **`Intent` 構造体 +
> `IntentExecution` 集約**へ分割された（`Intent` = 静的な intent: 識別子・依頼・scope・解決済み
> 計画・定義ピン / `IntentExecution` = 1 回の実行: `IntentExecutionId` で識別、1 intent : n 実行、
> 実行時状態のみ保持し計画は `&Intent` 引数で受ける）。本文中の `WorkflowExecution` は文脈により
> どちらかへ読み替える。本文の全文追従は後続 Bolt で行う（正本の裁定記録:
> `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/intent-aggregate-rename/brief-1.md`）。
>
> **優先順位（2026-08-30 / Bolt B13）**: 本文のうち集約の構築・再構成・エラー設計に触れる記述
> （`from_material` / memento 型 / スナップショット種の再水和 / リポジトリ別エラー型 /
> `Created` の集約埋め込み 等）は**歴史記録・非規範**である。現行の正は
> `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（aggregate-commands「再構成の形」・
> factory-naming・error-handling）が持ち、本文と矛盾する場合は coding-rules が常に優先する。
> 本文の全文追従は後続 Bolt（範囲: 改名 + ES 再構成の意味論）で行う。


> **位置づけ**: コンテキスト別仕様の第 2 号。`01-domain-model.md` の裁定（B5・B9・B12・B13）と D3/D4/D10、ADR 0001〜0004 に従う。
> **契約コーパス**: upstream `03-state-audit-runtime.md`（主）、`09-cli-tools.md` §5-7・`07-hooks.md` §3-4・`08-memory-rules-learnings.md` §2.4・`11-plugin-system.md` §5（従）。精密抽出は [`research/workspace-state-intent.md`](research/workspace-state-intent.md)（状態ファイル・writer 4 種・Space/Intent）、[`research/workspace-audit-ledger.md`](research/workspace-audit-ledger.md)（shard 文法・86 イベント・authority・順序・ロック）、[`research/workspace-lock-fork-worktree.md`](research/workspace-lock-fork-worktree.md)（reap 詳細・三層 fork/merge・Worktree・compose ロック共有）に収録済み。本書は**構造の規範**を担い、逐語の完全列挙は抽出文書と upstream を正とする。
> **状態**: ドラフト（フェーズ A。スコープは決定論コアの縦切り — DocumentKB のストレージ供給詳細と Windows 対応は後続）
> **策定日**: 2026-08-22

---

## 1. 責務と境界

workspace は**永続化の機構**を所有する。Space / Intent、状態ファイル `aidlc-state.md`、監査台帳（clone ごとの shard）、~~ワークスペースロック~~（ADR-007 / Bolt B5 で退役）、三層 fork/merge、Worktree、committed vs ignored の規律がここに属する。

境界の要点（01 の裁定の引き受け）:

- **B5**: 台帳の mechanics（追記・fork/merge・prefix-hash・audit-first・ロック）を**イベント意味論から独立に**所有する。イベント行は opaque であり、merge-protected 属性も監査イベントスキーマ（Published Language）上の宣言として受け取る — upstream では `MERGE_PROTECTED_EVENT_TYPES` が監査ツール内にハードコードされているが、amadeus-ng ではスキーマ駆動宣言に昇格する。
- **B9**: `HUMAN_TURN` の**記録**（事実）はここ。`humanActedSinceGate` 述語（同秒×別シャードの fail-closed を含む）は orchestration の所有で、本コンテキストは shard 列挙と位置付き読取の材料を供給する。「状態ファイルはキャッシュ、真実源は監査」を境界規約として全 API に適用する（unit 4 フィールド・`Parked`/`Parked At Stage` が代表）。
- **B12**: diary / memory / DocumentKB の**文書スキーマとライフサイクルは knowledge 所有**。本コンテキストは space / intent スコープの汎用ストレージと存在保証（self-heal）を供給し、内容には関与しない。
- **B13**: worktree / fork / merge の機構と `WORKTREE_*` イベントはここ。HOLD-MERGE は orchestration の政策値で、本コンテキストは **opaque な保留フラグの保存 API**（set / clear の冪等性、欠落ファイルへの非対称エラー）だけを提供する。
- **~~ロックサービスの供給~~（退役 — ADR-007 / Bolt B5）**（B5 の Shared Kernel 解体）: upstream では compose がインストール済み lib の関数 import ＋ `AIDLC_WORKSPACE_LOCK_OWNER_PID` 環境変数でロックを物理共有する。~~amadeus-ng では workspace が単独所有のロックサービスを公開し、orchestration / plugin はその顧客になる（依存方向として固定）~~ → **失効**: mkdir ロック機構は ADR-007 で退役し、workspace はロックサービスを公開しない。`WorkflowExecution` 集約の書込は SQLite Tx（本家 event-store-adapter-rs）＋ 楽観 version に置換されたが、**登録簿（`intents.json`）側の直列化機構は ADR-010（Bolt B6）で再び未決に戻った**（§2.1 `LockIdentity` 行・§10 参照。U7 で裁定）。

## 2. ドメイン層

### 2.1 集約

| 集約 | ルートと内包 | トランザクション境界 |
| --- | --- | --- |
| `Intent` | record ディレクトリ＋レジストリ行（`intents.json` の uuid / slug / dirName と生死）。birth は `createIntent` の単一チョークポイント（uuid mint → dirName 解決 → mkdir → **ヘッダのみの stub state**。stub がないと mint〜full 書込の間にカーソルが解決せず書込が space root に漏れる）。`StateFile` は**内包しない** — リードモデルである（下記、ADR-004） | `intents.json` の全変更は 1 トランザクション（直列化の機構は §10 の未決事項） |
| `Space` | 4 サブツリー（memory / knowledge / codekb / intents）＋レジストリ＋カーソル。default space は「ディスクに何もなくても常に有効」の特例。新規 space は default の `org.md` のみ継承（team/project は 1 行スタブ — 「新チームは自分のプラクティスを自分で獲得する」） | 同上（`intents.json` の全変更は単一 DB = 単一バケットへ集約する方針だが、~~直列化の機構は確定（ADR-007 / Bolt B5）~~ → 直列化の機構自体は §2.2・§10 の未決事項、2026-08-27 / ADR-010） |
| `Worktree` | `.aidlc/worktrees/bolt-<slug>` ＋ブランチ `bolt-<slug>`（**導出であり引数渡しではない**）。absent → created → merged / discarded。record ミラー（同一相対レイアウト）と main clone-id のスレッディング。`--repo` 指定時は**ターゲットリポジトリの checkout に再アンカー**（multi-repo — §2.4） | 変異 3 動詞（create / merge / discard）は**すべて監査を伴う**。emit と効果の逆順が認められるのは aidlc-bolt の `abort --discard`（orchestration 側の動詞、slice 2）のみで、本表の対象外 |

**リードモデル**（集約ではない — ADR-003 / ADR-004）: `StateFile`（`aidlc-state.md`）と `AuditShard`（clone ごとの監査シャード群）。真実源は SQLite ジャーナル（C6）であり、両者は ReadModelUpdater（U4）が投影として**バイト互換**で再生成する。監査台帳は集約 `WorkflowExecution` のイベントログであって独立した集約ではない（ADR-001 / ADR-003）。シャードが**追記専用**であること・行が opaque であること・他クローンのシャードが読み取り専用の外部入力であることは、投影の規範として維持する（唯一の例外は audit-fork による worktree ミラー shard の tmp+rename 確立）。

**退役**: `WorkspaceLock`（旧: 集約ではなく本コンテキストが所有・公開する並行性サービス）。ES 化により read-modify-write のトランザクションが SQLite に入ったため、mkdir ロック機構は退役し、ロック dir は生成しない。`WorkflowExecution` 集約の書込は SQLite Tx（本家 event-store-adapter-rs）＋ 楽観 version が並行制御を担う（ADR-007）。~~並行制御は SQLite Tx ＋ 楽観 version が担う~~ → **2026-08-27 訂正 / ADR-010**: これは `WorkflowExecution` 集約の書込に限った話であり、登録簿（`intents.json`）の read-modify-write の直列化機構は、代替として想定していた `within_write_transaction` が削除されたため**再び未決**である（§2.1 `LockIdentity` 行・§10 参照。U7 で裁定）。逸脱台帳 [`deviations.md`](deviations.md) 参照。

### 2.2 Domain Primitive（E1/E2 の受け皿）

| 型 | 定義 | 強制 |
| --- | --- | --- |
| `SpaceName` | `/^[a-z][a-z0-9-]*$/` — 「生のまま `join()` に到達してはならないパスセグメント」 | E2 |
| `IntentId` | UUIDv7（48-bit Unix-ms プレフィクス＋暗号学的 random tail）。文字列ソートの順序保証は**ミリ秒粒度**（同一ミリ秒内は非保証 — upstream 同等。単調カウンタは導入しない） | E2 |
| `IntentDirName` | `<YYMMDD>-<slug(label,24)>` の kebab 表記。衝突は `-2`… `-1000` まで、以後 loud throw。予約ラベル 8 語（help / list / switch / create / archive / rename / show / birth）拒否。**`IntentId`（UUIDv7）とは別の値**で、リードモデルの投影先パス解決に使う（01 §3.3、オーナー裁定 2026-08-23） | E2 |
| `CloneId` | `/^[a-z0-9]{1,32}$/`。欠如時 12 hex mint → **再読で並行初回鋳造が単一トークンに収束**。machine-local（gitignore）が本質 | E2＋E5（運用） |
| `ShardName` | `<host(小文字化・[a-z0-9-]圧縮・48 字上限・空なら"host")>-<cloneId>.md` | E1（構成関数） |
| `StateVersion` | 現行 `"8"`。分類器は `ok / unparseable / past / future` の 4 値で、**runtime と doctor が同一関数を使う**（不一致が構造的に不可能） | E1＋E2 |
| `StateFieldValue` | 単一行必須 — C0 制御・DEL・U+2028/U+2029 をコードポイント走査で拒否 | E2 |
| `BoltRefs` | 単一行リスト値。空は常に `[empty list]`、非空はソート済みブラケットリスト（round-trip 決定的）。append/remove は重複・不在で **throw**（no-op しない） | E2 |
| `CheckboxState` | 6 値（`[ ]` / `[-]` / `[?]` / `[R]` / `[x]` / `[S]`）。**本コンテキストの所有**であり orchestration（10 §2.2）は参照のみ（設計監査 C12） | E1 |
| `CheckboxLine` | マーカー（6 値）＋ em dash ＋ EXECUTE/SKIP サフィックス。**marker writer と suffix writer は同一行の互いに素なフィールドを編集する別 API**（recompose と jump が合成できる根拠） | E1（2 writer 分離） |
| `EventType` | 86 語 22 カテゴリの閉集合＋MANDATORY 8。型は監査イベントスキーマ PL クレート（01 §3.3 の注のとおり） | E1 |
| `AuthorityClass` | CLI_RESERVED(8) / CLI_PROTECTED(18) / MERGE_PROTECTED(26＋`DOCUMENT_*` prefix) の 3 deny-list。**スキーマ側宣言**（B5） | E1＋E3（拒否点） |
| `AuditFieldKey` | `/^[A-Za-z][A-Za-z0-9 ._()/-]*$/`。`Event` は呼出側供給禁止（第二の `**Event**:` 行偽造の防止）、`Timestamp` は受理するが値は捨てる | E2 |
| `LockIdentity`（退役 — ADR-007 / Bolt B5） | 登録簿（`intents.json`）の read-modify-write の直列化は ~~`EventStoreImpl::within_write_transaction`（同一 DB の `BEGIN IMMEDIATE`）に置換された。旧 keying 規範（全 space の登録簿変更を単一バケットへ集約し、`activeIntent()` は keying に使わない）は「単一 DB = 単一バケット」でそのまま満たされる（U3 FD Q2 = A）~~ → **失効（2026-08-27 / ADR-010・Bolt B6）**: 本家 event-store-adapter-rs は接続もトランザクションも露出しないため `within_write_transaction` は口ごと削除され、**登録簿の直列化機構は現時点で未定**である。ADR-010 は「登録簿を SQLite のテーブルへ移す（リードモデルとして RMU が投影する）」を筋と書いているが、**U7 で裁定**する。`LockIdentity` 自体の退役（ADR-007）は変わらない | 退役 |

### 2.3 ドメインサービス（純関数）

**骨格生成は投影の責務外**（2026-08-29 / Bolt B10 オーナー裁定 A）: genesis の状態ファイル骨格
102 行は環境値（Project Root 等）を含みジャーナルから導けないため、書くのは合成ルート（U7）で
あり、投影（RMU）は既存本文への差分適用に徹する（骨格欠落は `ScaffoldMissing` で前提違反）。
NFR3 の冪等再構成は差分適用に適用され、骨格は環境成果物（全損時は archive & recreate）。

本節の純関数のうち、**状態ファイル・監査ブロックの描画**にあたるもの（`render_audit_block` / `state_writers`）は、ES 化により**投影の責務**へ移る — 描くのは ReadModelUpdater（U4）であって、ドメイン層ではない（ADR-003 / ADR-004。~~コードの移動は U4 の Bolt で実施する~~ → **実施済み（2026-08-29 / Bolt B8 — `core-read-model-updater`（中間クレート。旧称 ~~`core-query-read-model-updater`~~、同日中に是正）の投影 API へ転居）**）。ドメインに残るのは値オブジェクトの Always Valid 検証（`StateFieldValue` の単一行検査、`EventType` の閉集合、行終端エスケープによる行偽造不能性）と、集約に置けない横断の判断である（01 §7.1 原則 2）。~~`find_all_events`（他クローンのシャード横断読取）と `classify_state_version` は本コンテキストに残る。~~ → **分割（2026-08-29 / Bolt B8）**: `find_all_events` は責務が割れた。**ドメインに残るのは順序付けの純関数のみ**（timestamp ソート＋バッファ位置 tiebreak を、渡された行列に適用する。実装は `core_command_domain::workspace` — ドメインはコマンド側の持ち物、`coding-rules/cqrs-boundaries.md`）。**シャード列挙とファイル読取（I/O）は投影側（中間クレート `core-read-model-updater` の `workspace/audit_shard.rs::read_all`）へ移った** — 11-workspace が「domain に残す」としていた記述と unit-of-work.md U4 の「横断読取は U4 の責務」を、純関数とその I/O 呼び出し元とに分けて両立させた（`construction/u4-read-model-updater/developer-report-1.md` §7-7）。`classify_state_version` は本コンテキストに残る（純関数・I/O 無し）。

| サービス | 内容 |
| --- | --- |
| `render_audit_block`（~~→ 投影 API — ReadModelUpdater、U4~~ → **実施済み（2026-08-29 / Bolt B8）**: 中間クレート `core-read-model-updater`（`workspace/audit_block.rs`）） | `## Heading` / `**Timestamp**` / `**Event**` / フィールド行 / `\n---\n`。値の行終端（`\r\n?` `\n` U+2028 U+2029）を `\n` リテラルへエスケープし、第二のフィールド行・イベント行の偽造を防ぐ |
| `find_all_events`（**分割 — 2026-08-29 / Bolt B8**。下記参照） | 順序付けの純関数（ドメインに残置。実装 `core_command_domain::workspace`）は timestamp（秒精度 ISO）ソート＋バッファ位置 tiebreak。**通常読取は決して fail-closed しない**（authority 比較の同秒 fail-closed は orchestration の述語側 — B9）。出力は順序付き専用型（外部から構築・再ソート不能 — W15 の E1 装置）。**シャード列挙とファイル読取（I/O）は投影側（中間クレート `core-read-model-updater` の `workspace/audit_shard.rs::read_all`）へ移った** |
| `classify_state_version` | 4 分類の単一実装（W7） |
| `state_writers`（~~→ 投影 API — ReadModelUpdater、U4~~ → **実施済み（2026-08-29 / Bolt B8）**: 中間クレート `core-read-model-updater` の投影 API） | `with_field_if_present`（無言 no-op）/ `with_field`（不在で throw — 「無言 no-op は検出不能なドリフト」）/ `with_field_or_insert` / `without_field` の 4 種。純粋な string→string |

### 2.4 multi-repo・カーソル・committed vs ignored

- **`WorkspaceManifest`（repos.json）**: multi-repo checkout のマニフェスト `{org, repos[]}`。01 衝突台帳の `RepoWorkspace` — `Workspace`（aidlc/ ツリー）とは別物。repo 名は単一パスセグメント（`..`・区切り文字禁止）・重複禁止、解決パスはワークスペースルートの immediate child（封じ込め再検査）、**ランタイムはディスク実態が真実（disk wins at runtime）**。`IntentRegistryEntry` の `repos?: string[]` と worktree 動詞の `--repo` 再アンカーがこの上に載る。reconcile（workspace-sync）は distribution 側の ACL（01 §2）で、本コンテキストはマニフェストのスキーマと解決規則を所有する。
- **カーソル 2 種**（`aidlc/active-space`、`spaces/<space>/intents/active-intent`）: per-user・best-effort（書込失敗は swallow）・両方 gitignore。「2 人のチームメイトが正当に別 space / intent を指す」ため共有状態にしない。active-space の実体化はステージングファイル＋ `linkSync`（no-replace 意味論）のアトミック設置で、並行 switch を潰さない。
- **`activeIntent()` の 4 段解決**: explicit 引数 → カーソル（**実際に `aidlc-state.md` を持つ** dir を指す場合のみ）→ 唯一の intent → `null`。null は例外ではなく「まだワークフローがない」正当な状態（パスヘルパは total に保ち、>1 intent かつカーソル無しのエラー/プロンプトは動詞層の責務）。
- **committed vs ignored**: 経験則は「per-user カーソルとマシンローカルなランタイム/派生状態は ignore、共有作業（メソッド・レジストリ・state・監査 shard・artifact）はコミット」。ignore glob は 11 個で各々にインライン根拠（upstream 03 §3.4 が逐語の正本）。**否定的決定**として `.gitattributes merge=union` は意図的に置かない — 複数行監査ブロックを破損させると実証済み。

## 3. ユースケース層

**ユースケース**（= CLI 動詞・提供サービス）: audit 5 動詞（append / append-batch / append-raw / audit-fork / audit-merge）、worktree 6 動詞（create / merge / discard / list / verify / info）、intent / space 管理動詞、runtime-graph compile（**器のみ** — センサー区画の折り込み規則は verification 提供、B8）、および state の**非遷移**動詞。**遷移系動詞（エンジン所有 11 ＋ unpark ＋ unit 系）のユースケースと S3 ガード（PID マーカー・bypass env・逐語拒否）は orchestration のアダプタ所有**（01 §3.2「遷移動詞 11 個の唯一の所有者は `WorkflowExecution`」、10 §9 S1「CLI ラッパもエンジンも同じ集約を呼ぶ」）。マルチコール composition root は該当動詞をガード通過後に orchestration ユースケースへディスパッチし、workspace はリードモデル（状態ファイル・監査シャード）の読取と §3 の供給サービスに徹する — これで 01 §2 の依存方向（orchestration → workspace の C/S）が保たれる（ADR-003 / ADR-004）。

**ポート**（[`coding-rules/gateway-taxonomy.md`](../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md) の語彙。Gateway 責務は Repository と外部システムクライアントの 2 種類だけで、判定は「**どのユースケースがこのポートを消費するか**」で行う — 設計監査 R3 / C3 / C4 / C11）:

| ポート | 消費するユースケース | 契約 | 実装の所在 |
| --- | --- | --- | --- |
| `WorkflowExecutionRepository` | orchestration の `Report` / `Continue` / `Park` / `Jump*` / `Recompose` / `SetAutonomy`（10 §3） | 集約 `WorkflowExecution` の ES 形 Repository。~~`store(event, aggregate)`（単一イベント＋適用後集約を同一 Tx）~~ → **失効（2026-08-29 / ADR-010・Bolt B7）**: `store(&mut self, event, aggregate, expected_version: usize)` — `expected_version` を明示引数に取り、新規・更新とも `persist_event_and_snapshot` で同一 Tx 永続化する（分岐は封筒の `seq_nr == 1` から導出）。楽観 `version` = **ストアが採番する不透明トークン** — ドメインは解釈も比較もしない（この性質自体は不変。**2026-08-27 補足**: 現行の event-store-adapter-rs バックエンドと genesis に 1 を載せる採番規約の組み合わせでは `version` は結果としてジャーナル長と一致する（J3 `journal_protocol::version_equals_journal`）が、これは**現行 adapter の観測された性質であってドメイン契約ではない** — ドメイン・Repository はこの一致を前提条件として使わない）。~~`find_by_id(&IntentId)`（スナップショット＋差分 replay で完全再構成）~~ → **失効（2026-08-29 / ADR-010・Bolt B7）**: `find_by_id(&IntentId)` は最新スナップショット＋差分 replay で**再水和レコード `RehydratedWorkflowExecution`**（集約 + ストア採番 version）を返す — 楽観 version は集約から外れ、集約の外を持ち回る形になった — C3 / ~~ADR-006~~ → **ADR-010** | `WorkflowExecutionRepositoryImpl<S>`（**本家 event-store-adapter-rs ~~v2.0.0~~ → v3.0.0（2026-08-29 / Bolt B7）のイベントストアを内包** — C6 のスキーマ、ストアファイルは `aidlc/spaces/<space>/intents/.aidlc-store.sqlite`（U3 FD Q1 = A））。`S` はバックエンドで `open()` = SQLite / `in_memory()` = memory。~~`EventStoreImpl` を内包 / 登録簿の直列化は `EventStoreImpl::within_write_transaction`（U3 FD Q2 = A）/ テストダブルは `InMemoryWorkflowExecutionRepository`~~ → **失効**（2026-08-27 / ADR-010・Bolt B6。登録簿の扱いは U7 で裁定） |
| 外部システムクライアント（Git。例 `GitWorktreeClient`） | orchestration（Bolt / swarm — slice 2）、worktree 6 動詞 | worktree add / merge 3 戦略 / branch 削除 / conflict 検出 `/^CONFLICT \(/m`。別プロセスとの RPC であって集約の永続化ではない（gateway-taxonomy §1） | アダプタ層の Gateway（spawn 基盤 A4 経由、30s タイムアウト、SIGTERM でタイムアウトと失敗を区別） |

**ポートではないもの**（同規則の帰結。ポート表に載せると Gateway 責務の分類が濁る）:

- `FileStore`（アトミック書込 tmp+rename、追記専用 open、シンボリックリンク連鎖検査、封じ込め検査）は **Repository 実装と投影ライタの内部部品**であって、ユースケースが消費するポートではない。「読むだけの Gateway だから Reader」式のポート造語（`Store` / `Reader` / `Writer` / `Source` / `Provider`）と、`StateFileRepository` のような媒体名の Repository は禁止（gateway-taxonomy §2・§3）。
- `Clock` / `Tmpdir` は**アダプタ層の機構**であり、実装は機構モジュールに置き、差し替えは composition root が配線する（gateway-taxonomy §1、設計監査 C4）。`ProcessProbe` は**退役**（reap 機構の消滅に伴う — ADR-007 / Bolt B5）。
- 監査台帳の追記サービス（旧称）は**退役**した。監査シャードは ReadModelUpdater（U4）の投影であり、専用のポートを持たない（ADR-003）。同様に、ロックのサービスも退役し、並行制御は SQLite Tx ＋ 楽観 version が担う（ADR-007）。

**他コンテキストへの供給面**（Customer/Supplier の supplier 側。ポートではなく本コンテキストが公開する API）:

| サービス | 顧客 | 契約の要点 |
| --- | --- | --- |
| `WorktreeService` | orchestration（Bolt/swarm） | 三層 fork/merge の workspace 側（state-fork/merge、audit-fork/merge、fragment-fork/merge）と Worktree ライフサイクル。Git 操作は上記の外部システムクライアント経由 |
| `OpaqueFlagStore` | orchestration | HOLD-MERGE 等の政策値の保存。set/clear 冪等、欠落 forked state への set は hard error（非対称は保存 API の仕様 — B13） |
| `ScopedStorage` | knowledge | space / intent スコープのストレージと存在保証（self-heal・「default tree never churns」）。内容不干渉（B12） |
| `SessionStampStore` | フック（session-start / session-end / rebuild） | セッション → intent スタンプ（`aidlc/.aidlc-sessions/<session_id>`、gitignore、per-user）と handoff receipt（TTL 5 分 — `SESSION_INTENT_HANDOFF_TTL_MS`）の保存・照合材料。スタンプ済みセッションへの intent birth は上書きせず receipt を書く（Stop フック carve-out 0 が消費）。SESSION_ENDED の fail-closed 帰属（未知 intent へのスタンプ・未スタンプセッションの shared-cursor fallback 拒否）の判定材料もここが供給する |

状態ファイルと監査シャードの**読取**（`aidlc-state.md` の描画結果、shard 列挙、位置付き読取 — B9 の述語材料）は、リードモデルの読取として本コンテキストが供給する。**書込**は投影（U4）の責務であり、供給面には現れない（ADR-003 / ADR-004）。

## 4. インターフェイスアダプタ層

- **Controllers**: 各 CLI 動詞の引数を Domain Primitive の `parse` で検証（`--slug` は `SLUG_RE`、`--space` は `SpaceName` 等）し、型付き値をユースケースへ渡す（01 §7 の規約）。**遷移系動詞の Controller と S3 ガードは orchestration 所有**（§3）で、本コンテキストの Controller は非遷移動詞と供給サービスの CLI 面のみを持つ。
- **Presenters**: worktree conflict JSON（worktree 保存＋`conflict_files` — ADR 0001 contract-compact）、`[merge-succeeded:<sha>]` プレフィクス付き post-merge エラー（doctor が「merge 全失敗」と「着地済み・クリーンアップ孤児」を区別する contract）。逐語は文言カタログ。bolt の失敗エンベロープ（reason 17 値）は orchestration slice 2 の CLI 面、workspace-sync の exit code は distribution の ACL に属し、ここでは規定しない。
- **配置の規範**（§3 の帰結）: `Clock` は**アダプタ層の機構モジュール**（コンテキストの外、クレート root）に置き、実物と fake の差し替えは composition root が配線する（`ProcessProbe` は退役 — reap 機構の消滅に伴う、ADR-007 / Bolt B5）。`FileStore`（アトミック書込・追記専用 open・封じ込め検査）と正準 JSON（A2）・ハッシュは **Repository 実装と投影ライタの内部部品**として実装側に閉じる。いずれも use-case 層に trait を置かない（[`coding-rules/gateway-taxonomy.md`](../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md) §1、設計監査 C4）。
- **Gateways**: `FileStore` 実装（`O_RDWR|O_APPEND|O_CREAT|O_NOFOLLOW|O_NONBLOCK` open、fstat regular-file 検査、書込前後の記述子同一性再検証 — 書込中 rename は行方不明の行ではなく**囲んでいる audit-first トランザクションの失敗**になる。**nlink の意図的非対称**: 通常 append 経路は `nlink != 1` を拒否**しない** — rsync `--link-dest` / `cp -al` スナップショットへの拒否が「以後の全 gate/hook 追記をフレームワーク全体で文鎮化した」実績による。厳格な多重リンク拒否は fork/merge 経路のみ。Rust 再実装で防御を「強化」すると同じ障害を再現する）、外部システムクライアント（Git）実装（spawn 基盤 A4 経由、30s タイムアウト、SIGTERM でタイムアウトと失敗を区別）。ロック dir 実装（mkdir-EEXIST、owner.json スタンプ、rename CAS reap）は**退役**し、並行制御は `WorkflowExecutionRepositoryImpl` の SQLite Tx ＋ 楽観 version に移る（ADR-007。逸脱台帳 [`deviations.md`](deviations.md) 参照）。**I/O 責務はすべてここ**。テスト用 in-memory 実装を最初に用意する。

### 4.1 構造化リードモデル（`read_*` 表）— CLI 読取コマンド向けの第 2 系統（2026-09-02 追加、b39）

リードモデルは **2 系統**ある（オーナー裁定 2026-09-02、`construction/query-side-audit/read-model-spec.md`）。

| 系統 | 読み手 | 形 | 更新者 |
| --- | --- | --- | --- |
| (1) 人・upstream ツール向けファイル面 | 人が開く、upstream の hook / ステージが読む | `aidlc-state.md`、監査シャード、配布束の面 `stage-graph.json` / `scope-grid.json` | RMU（バイト互換、golden 固定） |
| (2) CLI 読取コマンド向け構造化リードモデル | `next` / `continue` /（将来）`--status` / `doctor` の DAO | イベントストアと同じ SQLite ファイル（`aidlc/spaces/<space>/intents/.aidlc-store.sqlite`）の接頭辞 `read_` の表 | RMU（`catch_up` ごとに**全履歴から再計算し全差し替え**、チェックポイント前進と**同一トランザクション**） |

(2) の規範:

- **行の値はすべて集約のクエリメソッドの戻り値の写し**である。RMU はジャーナル 3 ストリーム（intent / 実行 / 定義）から `replay` で集約を起こし（投影核の入口はイベント列 — cqrs-boundaries 規則 3）、`next_decision` / `jump_resolve` / `scope_cost` / 述語面を**呼んで**行に書く。判断を RMU で再実装しない。
- **非正規化**する — 読取コマンドが 1 回の引当（`WHERE`）で答えを得る形に置く。クエリ側の DAO はキーで引くだけで、行に無い事実を作らない（作れば CQRS 違反 — オーナー裁定）。
- 全行が `as_of`（投影に使った最後のジャーナル通番 = `GlobalSeqNr`）を持つ。壁時計は読まない（冪等・決定性）。
- 表の作成は `CREATE TABLE IF NOT EXISTS`（`amadeus_projection_checkpoint` と同じ流儀）、RMU の `JournalReaderImpl::open` が行う。

表カタログ（b39 で作る 13 表。列の定義と計算元は `construction/b39-rmu-read-tables/design.md` §4.1 が正本。b40 で `read_run_stage` / `read_scope_change` / `read_config_current` / `read_steering_plan` / `read_steering_part` を追加する）:

| 表 | キー | 出所の集約 |
| --- | --- | --- |
| `read_definition` / `read_definition_stage` / `read_definition_scope` / `read_definition_scope_keyword` / `read_definition_scope_stage` / `read_definition_scope_phase_entry` | definition_id（× stage_slug / scope / keyword / phase） | `WorkflowDefinition`（`Defined` / `Redefined` の再生） |
| `read_intent` / `read_intent_stage` | intent_id（× stage_index） | `Intent`（`Created`） |
| `read_execution` / `read_execution_stage` | execution_id（× stage_index） | `IntentExecution`（`Started` からの再生） |
| `read_next_answer` | execution_id × request_kind ∈ {`bare`, `resume`, `free-text`, `reentry`}（kebab-case） | `IntentExecution::next_decision` |
| `read_next_jump` / `read_next_jump_phase` | execution_id × target_index / phase | `IntentExecution::jump_resolve` / `first_in_scope_of_phase` |

前提となるドメインの是正（b39）: `Started` は**集約 id と解決済み計画の写し**を運ぶ（従来は `intent_id` のみで、genesis が `&Intent` を要し自ストリームだけで再生できなかった）。`IntentExecution: From<(Started, DateTime<Utc>)>` が genesis イベントからの唯一の状態導出であり、`start` はそれを通る（`Intent` / `WorkflowDefinition` と同型 — coding-rules/aggregate-commands.md）。

## 5. インフラストラクチャ層の利用

正準 JSON（A2 — `intents.json` は 2-space、`runtime-graph.json` は決定性契約「同一監査ログ → バイト同値」）、文言カタログ（A3）、ハッシュ（SHA-256 prefix-hash）は純粋部品。MD5 のロック dir 名はロック機構の退役に伴い不要になる（ADR-007。stage-0/1 併用期の互換維持は §9・§10 の論点）。アトミック書込・spawn 基盤（A4）を呼ぶのは Gateway のみ。`tracing` 計装（A10）は application/adapter 層で、派生イベント発行のチョークポイントは**ジャーナル追記の Tx コミット成功後**（`WorkflowExecutionRepositoryImpl` の `store`）に置く（ADR-003）。**POSIX 前提**（O_NOFOLLOW・`kill(pid,0)`・mkdir ロック・rename 意味論）は方針書 R3 のとおり初期フェーズの明示的制約で、Windows はフェーズ D で防御の等価物定義とセット。

## 6. 不変条件表（強制手段つき）

E4 の定義名は J1〜J6（旧 W1〜W5 に相当する区間 — mkdir ロック時代の規範は ADR-007 で退役、§2.1）について [`formal/orchestration/journal_protocol.qnt`](../../formal/orchestration/journal_protocol.qnt)（ジャーナル / スナップショット / version / チェックポイント協定モデル、**不変条件 8 / witness 4**）に実在する — `formal/workspace/audit_lock.qnt`（旧: v3、10 不変条件 green・mutation 10/10・witness 7 本）を退役して置換した（ADR-007 / Bolt B5）。

| # | 不変条件 | 強制 | E4 定義名 / 備考 |
| --- | --- | --- | --- |
| J1 | 楽観 `version` 不一致の書込（`store_conflict`）はジャーナル・スナップショットのいずれも変えない（競合拒否） | E3+E4 | `journal_protocol::conflict_rejected` |
| J2 | スナップショットの `seq_nr` は常にジャーナル末尾と一致する | E3+E4 | `journal_protocol::snapshot_tracks_journal` |
| J3 | スナップショットの `version` は常に永続化済みイベント数と一致する（**2026-08-27 補足 / ADR-010**: これは現行 event-store-adapter-rs バックエンド＋genesis=1 の採番規約が両立させている**adapter 固有の観測された性質**であり、`version` を不透明トークンとするドメイン契約そのものが要求するわけではない。§3 ポート表参照） | E3+E4 | `journal_protocol::version_equals_journal` |
| J4 | チェックポイントは単調に増加し（後退しない）、ジャーナル末尾を超えない | E3+E4 | `journal_protocol::checkpoint_monotone` ＋ `journal_protocol::checkpoint_bounded` |
| J5 | 投影（readModelSeq）はジャーナルを超えて進まず、直前と同じチェックポイントからの再実行では値が変わらない（冪等） | E3+E4 | `journal_protocol::projection_idempotent` ＋ `journal_protocol::truth_is_journal` |
| J6 | 書込成功（`store_ok`）が起きるのは、書込主体が読み取った `version` が直前のスナップショット `version` と一致するときのみ（lost update 防止） | E3+E4 | `journal_protocol::no_lost_update` |
| W6 | 状態ファイルのフィールド値は単一行（C0 / DEL / U+2028 / U+2029 拒否） | E2 | `StateFieldValue` |
| W7 | State Version の分類は runtime と doctor で同一関数（乖離が構造的に不可能） | **E1** | 装置: 分類結果型 `StateVersionClassification` のコンストラクタを分類器モジュール内 private とし、`classify_state_version` 経由以外で値を**生成不能**にする（別実装の分類器は戻り値型を作れない） |
| W8 | 状態書込は tmp+rename でアトミック。read-only ターゲットは W_OK 事前チェックで書込バリアとして尊重（rename 貫通を塞ぐ） | E3 | — |
| W9 | **構造化ブロック**のイベント型は 86 閉集合のみ・呼出側 `Event` キー供給禁止・値の行終端エスケープで行偽造不能。append-raw の event なし note（Error / Recovery 形 — timestamp ちょうど 1）は**別枠の正当ブロック**で、merge delta 検証もこれを受理する | E1+E2 | `EventType`／`AuditFieldKey`／`render_audit_block` |
| W10 | authority 3 deny-list（RESERVED はパース前拒否、PROTECTED は append で拒否＋bypass env、MERGE_PROTECTED は delta で拒否）。宣言はイベントスキーマ側（B5） | E1+E3 | 拒否文言は文言カタログ（逐語 3 形） |
| W11 | audit-merge は delta のみ追記し、ブロック境界・既知イベント・非 merge-protected・worktree スナップショットのバイト/inode 一致・**main 先頭 boundary バイトの prefix-hash 一致**を全て検証（mid-Bolt tampering / truncation を逐語で区別） | E2+E3 | R7 の受け皿。authoritative fork 行は main から回収（書込可能な worktree コピーを信用しない） |
| W12 | main shard を書き換えるコードパスは存在しない（追記専用）。唯一の例外は audit-fork の worktree ミラー確立（tmp+rename）で、以後は再び追記のみ | **E1** 候補+E3 | 装置: main shard と worktree ミラーを**パスの型で分離**し（`MainShard` / `WorktreeMirrorShard`）、置換 API は `WorktreeMirrorShard`（audit-fork ユースケース専用型）にのみ定義する — main shard 型には追記しか存在しない |
| W13 | birth は単一チョークポイント。`intents.json` の全変更は ~~`EventStoreImpl::within_write_transaction`（単一 DB = 単一バケット）下で~~、keying に `activeIntent()` を使わない（並行 first-run の二重 birth 防止）。**単一チョークポイントという要求は不変だが、それを担う機構は未定**（2026-08-27 / ADR-010 — `within_write_transaction` は削除済み。U7 で裁定） | E1+E3 | **未定（U7 で裁定）**。~~`EventStoreImpl::within_write_transaction`~~（2026-08-27 失効）／旧: `LockIdentity` の構成関数が唯一の入口 — ADR-007 / Bolt B5 で置換 |
| W14 | 状態機械遷移は strict writer（不在フィールドは throw — 無言 no-op は検出不能ドリフト）。M12 は修正方針確定済み（逸脱台帳 #2。実装形は §10 のとおり実装時に確定） | E2+E3 | `with_field` |
| W15 | shard 横断の順序は timestamp ソート＋バッファ位置 tiebreak で、通常読取は決して fail-closed しない（同秒 fail-closed は orchestration の authority 述語のみ — B9） | **E1** | 装置: `find_all_events` の出力を順序付き専用型（外部から構築・再ソート不能）にし、順序付きイベント列はこの型経由でしか得られない |
| W16 | runtime-graph は純観測者（state を変異しない・質問しない）で、同一監査ログから**バイト同値**を再現する | E3＋A2 | 決定性は正準 JSON（A2）が前提。折り込み規則は verification 提供（B8） |

## 7. 実装順序（D10 × domain-model-first）

1. **ドメイン例のテスト**: 「emit が throw したら state は変わらない」「reap は死んだ所有者か閾値超過のみ」「BoltRefs の append は重複で throw」「stub のない record は activeIntent に解決されない」等を正準用語で書く。
2. **Domain Primitive → 集約の TDD**: §2.2 の E1/E2 を先に。proptest は `BoltRefs` round-trip・`render_audit_block` エスケープ（任意入力で行偽造不能）・`ShardName` 構成・checkbox parse に適用。
3. **in-memory 実装**（~~`InMemoryWorkflowExecutionRepository`~~ → `WorkflowExecutionRepositoryImpl::in_memory()`（2026-08-27 改訂 / ADR-010 — 本家の memory バックエンドを内包し、実装コードは SQLite と同一）と外部システムクライアント（Git）の fake、機構の `Clock` の fake）でユースケーステスト。`FileStore` は Repository 実装の内部部品なので、そのフェイクも実装側に閉じる（§3）。並行制御のテスト（楽観 version の競合と再試行）は loom 等の検討を含め実装時に確定。
4. **ITF 準拠**: [`formal/orchestration/journal_protocol.qnt`](../../formal/orchestration/journal_protocol.qnt)（ADR-007 により `audit_lock.qnt` を退役して置換した「ジャーナル / スナップショット / version / チェックポイント協定」モデル — Bolt B5）のトレース（`lastAction`/`lastActor` 駆動）を ~~`InMemoryEventStore`~~ → **`WorkflowExecutionRepositoryImpl` ＋ `JournalReaderImpl`**（2026-08-27 改訂 / ADR-010）＋フェイク投影に再生。**モデルは 1 文字も変えずに通った** — 本家へ載せ替えても同じトレースが再生できることが乗り換えの意味論的な検収である。
5. **実 Gateway は最後**: ゴールデン互換層で upstream 実ワークスペース（TS 版が書いた実物）を読ませ、state バイト列・監査行・shard 名・レジストリ JSON の一致を検証（stage-1 切替の前提そのもの）。

## 8. Quint ゲート実験 — 第一陣 3 号の記録

- `audit_lock.qnt` v1（不変条件 5 本）は green・mutation 3/3 だったが、**敵対的レビューが v1 の重大な検査力の穴を実証**した: 所有権の移転がアクションラベル経由でしか守られておらず、(a) acquire の不在ガード除去（＝保持中ロックの横取り）、(b) release の所有権ガード除去（＝非保持者の解放）、(c) reap の reaper 生存ガード除去、(d) **crash がロックを消す変異（R6 の核心の直接否定）** — の 4 変異がすべて全不変条件を素通りした。engine_loop v1 と同型の教訓: 「観測ラベル依存の不変条件は、経路を変えた違反に盲目」。
- v2 で状態遷移レベルの不変条件 4 本を追加（`lock_no_steal` — 経路非依存の移転条件で相互排他の実質検査 / `release_requires_ownership` / `reaper_alive` / `crash_leaves_lock`）。**9 不変条件 green（5000×40）＋ mutation 9/9**（9 変異がそれぞれ狙いの不変条件で検出。多重防御も確認 — reap ガード除去は `no_reap_of_live_fresh` と `lock_no_steal` の両方が捕捉）。
- **到達性 witness 5 本をモジュール内に定義**（`w_threshold_reap` / `w_dead_owner_reap` / `w_deep_release` / `w_emit_fail` / `w_full_unwind`）。CI は負形式（`--invariant "not(w_x)"`）で実行し violation = 経路実在 = pass と読む。green のままなら経路がモデルから消えた退行（例: tick 凍結で閾値 reap が到達不能になる）を検出できる — ファイル外の一時的な反証 run では退行に盲目だった穴の是正。
- v2 でも維持する抽象化はモデルヘッダに完全列挙（二相 acquire と未スタンプ猶予・取得予算・reapLiveOwnerAfterStale・rename CAS の内部状態・バッチ原子性 — slice 2 候補）。「解放または reap 可能」の temporal liveness は定義名を与えて `quint verify`（nightly）に載せるまで E4 と数えない（W5）。
- **v3（PR #2 レビュー反映）**: audit-first のクリティカルセクションを emit / write の 2 段に分割し、「監査済み・state 未書込」の crash 中間状態（R6 の核心）を到達可能にした（`w_crash_mid_txn` → reap 回復 `w_recovery_after_mid_txn_crash`）。stale 境界を upstream の厳密超過（`>`）に一致。10 不変条件 green（5000×50）＋ mutation 10/10（emit なし state 書込の新変異を含む）＋ witness 7/7。
- 第一陣は 3/3 完了（`engine_loop` v2 / `audit_lock` v2 / `stop_hook` v1）。総括は 10-orchestration §9 の第 3 回記録と ADR 0003 試行条項。
- **退役と置換（2026-08-23、Bolt B5）**: `audit_lock.qnt`（v3）は ADR-007 のロック機構退役に伴い退役し、協定モデル [`formal/orchestration/journal_protocol.qnt`](../../formal/orchestration/journal_protocol.qnt)（不変条件 8 / witness 4）へ置換した。上記の v1〜v3 の記録は退役前の実験履歴として保持する。

## 9. 実装ノート — 仕様と実装の分離

orchestration §10 の裁定（in-process 合成、S1〜S4 維持）は本コンテキストの提供面にそのまま適用される。追加の分類:

- **仕様**: エンジン所有 11 動詞の外部拒否面（S3 — マーカー受理・bypass env・逐語文言）、25 動詞の CLI 語彙、shard 名・ブロック文法・イベント語彙・authority 拒否、ロックの奪取条件と予算の**意味論**、fork/merge の検証と拒否文言、worktree のパス/ブランチ導出とイベント。
- **実装**: ~~ロック dir の物理配置（tmpdir + md5 名）や owner.json の形式は…初期フェーズはロックの物理形式も互換維持する~~ → **ADR-007 でロック機構そのものを退役させ、ロック dir を生成しないことにしたため、この互換維持の前提は失効した**（逸脱台帳 [`deviations.md`](deviations.md) に「ロック dir の非生成」として登録済み）。stage-0/1 併用期に upstream プロセスと同一ワークスペースを触る場合の相互排他は**担保しない**（stage-1 は単一クローン運用 — U3 FD P7 / Q1 = A、§10 確定、Bolt B5）。`AIDLC_WORKSPACE_LOCK_OWNER_PID` の env 名と受理意味論は D6 の対象として維持する。

## 10. 未決事項

- `ScopedStorage`（B12）の API 詳細は knowledge コンテキスト仕様（13 号 — 12 号は workflow-definition が使用）と同時に確定する。
- `SessionStampStore` の rebind offer 文言と Codex/Copilot 差分（session-start フックの詳細）は検証コンテキスト仕様（フック帰属分）で確定する。
- **stage-0/1 併用期の相互排他 — 確定（2026-08-23、Bolt B5）**: 担保しない。stage-1 は単一クローン運用とする（U3 FD P7 / Q1 = A）。upstream プロセス（mkdir ロックを取る）との併用時の相互排他は本 intent のスコープ外とし、後続 intent の課題とする（§9 の旧前提は失効のまま）。
- ~~**`intents.json` の直列化機構 — 確定（2026-08-23、Bolt B5）**: `EventStoreImpl::within_write_transaction`（同一 DB の `BEGIN IMMEDIATE`）に一本化した（U3 FD Q2 = A）。W13（birth の単一チョークポイント）と旧 `LockIdentity` の keying 規範（単一バケットへの集約・`activeIntent()` を keying に使わない）は、この機構でそのまま満たされる（§2.2、§6 W13）。~~
  → **失効（2026-08-27 / ADR-010・Bolt B6）**。本家 event-store-adapter-rs は接続もトランザクションも露出しないため `within_write_transaction` を実現できず、口ごと削除した。**`intents.json` の直列化機構は再び未決であり、U7 で裁定する**。ADR-010 は「登録簿（リードモデル）をコマンド側が Tx で守る構造自体が CQRS の境界に反する」として、登録簿を SQLite のテーブルへ移し RMU の投影対象にする案を筋と書いている。W13 の要求（birth は単一チョークポイント、keying に `activeIntent()` を使わない）はそのまま有効である。
- ~~ロック keying（複数 identity・センチネル 2 成分形）と二相 acquire・未スタンプ猶予のモデル化~~ → 確定（2026-08-23、Bolt B5）: mkdir ロックの退役（ADR-007）により `audit_lock.qnt` は退役し、協定モデル [`formal/orchestration/journal_protocol.qnt`](../../formal/orchestration/journal_protocol.qnt)（不変条件 8 / witness 4）へ置換した（U3 FD Q4 = A）。
- runtime-graph のセンサー区画折り込み規則の受け渡し形式（B8 — 宣言的規則の表現）は verification 仕様で確定。
- M12 修正の実装形（birth で行を書く vs `with_field_or_insert` 化）は実装時に選び、ゴールデンの分岐点を 1 か所に固定する。
- Windows の防御等価物（O_NOFOLLOW / kill(0) / mkdir ロック / rename）はフェーズ D（R3）。
