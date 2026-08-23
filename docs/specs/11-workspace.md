# workspace コンテキスト仕様

> **位置づけ**: コンテキスト別仕様の第 2 号。`01-domain-model.md` の裁定（B5・B9・B12・B13）と D3/D4/D10、ADR 0001〜0004 に従う。
> **契約コーパス**: upstream `03-state-audit-runtime.md`（主）、`09-cli-tools.md` §5-7・`07-hooks.md` §3-4・`08-memory-rules-learnings.md` §2.4・`11-plugin-system.md` §5（従）。精密抽出は [`research/workspace-state-intent.md`](research/workspace-state-intent.md)（状態ファイル・writer 4 種・Space/Intent）、[`research/workspace-audit-ledger.md`](research/workspace-audit-ledger.md)（shard 文法・86 イベント・authority・順序・ロック）、[`research/workspace-lock-fork-worktree.md`](research/workspace-lock-fork-worktree.md)（reap 詳細・三層 fork/merge・Worktree・compose ロック共有）に収録済み。本書は**構造の規範**を担い、逐語の完全列挙は抽出文書と upstream を正とする。
> **状態**: ドラフト（フェーズ A。スコープは決定論コアの縦切り — DocumentKB のストレージ供給詳細と Windows 対応は後続）
> **策定日**: 2026-08-22

---

## 1. 責務と境界

workspace は**永続化の機構**を所有する。Space / Intent、状態ファイル `aidlc-state.md`、監査台帳（clone ごとの shard）、ワークスペースロック、三層 fork/merge、Worktree、committed vs ignored の規律がここに属する。

境界の要点（01 の裁定の引き受け）:

- **B5**: 台帳の mechanics（追記・fork/merge・prefix-hash・audit-first・ロック）を**イベント意味論から独立に**所有する。イベント行は opaque であり、merge-protected 属性も監査イベントスキーマ（Published Language）上の宣言として受け取る — upstream では `MERGE_PROTECTED_EVENT_TYPES` が監査ツール内にハードコードされているが、amadeus-ng ではスキーマ駆動宣言に昇格する。
- **B9**: `HUMAN_TURN` の**記録**（事実）はここ。`humanActedSinceGate` 述語（同秒×別シャードの fail-closed を含む）は orchestration の所有で、本コンテキストは shard 列挙と位置付き読取の材料を供給する。「状態ファイルはキャッシュ、真実源は監査」を境界規約として全 API に適用する（unit 4 フィールド・`Parked`/`Parked At Stage` が代表）。
- **B12**: diary / memory / DocumentKB の**文書スキーマとライフサイクルは knowledge 所有**。本コンテキストは space / intent スコープの汎用ストレージと存在保証（self-heal）を供給し、内容には関与しない。
- **B13**: worktree / fork / merge の機構と `WORKTREE_*` イベントはここ。HOLD-MERGE は orchestration の政策値で、本コンテキストは **opaque な保留フラグの保存 API**（set / clear の冪等性、欠落ファイルへの非対称エラー）だけを提供する。
- **ロックサービスの供給**（B5 の Shared Kernel 解体）: upstream では compose がインストール済み lib の関数 import ＋ `AIDLC_WORKSPACE_LOCK_OWNER_PID` 環境変数でロックを物理共有する。amadeus-ng では workspace が**単独所有のロックサービス**を公開し、orchestration / plugin はその顧客になる（依存方向として固定）。

## 2. ドメイン層

### 2.1 集約

| 集約 | ルートと内包 | トランザクション境界 |
| --- | --- | --- |
| `Intent` | record ディレクトリ＋レジストリ行（`intents.json` の uuid / slug / dirName と生死）。birth は `createIntent` の単一チョークポイント（uuid mint → dirName 解決 → mkdir → **ヘッダのみの stub state**。stub がないと mint〜full 書込の間にカーソルが解決せず書込が space root に漏れる）。`StateFile` は**内包しない** — リードモデルである（下記、ADR-004） | `intents.json` の全変更は 1 トランザクション（直列化の機構は §10 の未決事項） |
| `Space` | 4 サブツリー（memory / knowledge / codekb / intents）＋レジストリ＋カーソル。default space は「ディスクに何もなくても常に有効」の特例。新規 space は default の `org.md` のみ継承（team/project は 1 行スタブ — 「新チームは自分のプラクティスを自分で獲得する」） | 同上（`intents.json` の全変更は単一バケットに集約する — §2.2 `LockIdentity` の keying 規範は維持） |
| `Worktree` | `.aidlc/worktrees/bolt-<slug>` ＋ブランチ `bolt-<slug>`（**導出であり引数渡しではない**）。absent → created → merged / discarded。record ミラー（同一相対レイアウト）と main clone-id のスレッディング。`--repo` 指定時は**ターゲットリポジトリの checkout に再アンカー**（multi-repo — §2.4） | 変異 3 動詞（create / merge / discard）は**すべて監査を伴う**。emit と効果の逆順が認められるのは aidlc-bolt の `abort --discard`（orchestration 側の動詞、slice 2）のみで、本表の対象外 |

**リードモデル**（集約ではない — ADR-003 / ADR-004）: `StateFile`（`aidlc-state.md`）と `AuditShard`（clone ごとの監査シャード群）。真実源は SQLite ジャーナル（C6）であり、両者は ReadModelUpdater（U4）が投影として**バイト互換**で再生成する。監査台帳は集約 `WorkflowExecution` のイベントログであって独立した集約ではない（ADR-001 / ADR-003）。シャードが**追記専用**であること・行が opaque であること・他クローンのシャードが読み取り専用の外部入力であることは、投影の規範として維持する（唯一の例外は audit-fork による worktree ミラー shard の tmp+rename 確立）。

**退役**: `WorkspaceLock`（旧: 集約ではなく本コンテキストが所有・公開する並行性サービス）。ES 化により read-modify-write のトランザクションが SQLite に入ったため、mkdir ロック機構は退役し、ロック dir は生成しない。並行制御は SQLite Tx ＋ 楽観 version が担う（ADR-007。逸脱台帳 [`deviations.md`](deviations.md) 参照）。

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
| `LockIdentity`（退役予定 — ADR-007） | `<realpath(projectDir)>\x00<space>\x00<intent>`。intent 省略時は **`<realpath(projectDir)>\x00__workspace__` の 2 成分形（space 成分ごと落ちる）** — 全 space の `intents.json` 変更が単一バケットに集約される。ロック dir 名は `md5(identity).slice(0,8)` から導出されるため、このバイト列は stage-0/1 併用期の相互排他互換（§9）に直結する。**`activeIntent()` を keying に決して使わない**（並行 birth が別バケットに割れて二重 birth する）。**ロック dir の生成そのものは退役**（ADR-007）— 残るのは「`intents.json` の変更を単一バケットへ集約する」keying の規範だけで、md5 dir 名を含む物理形式の互換維持は §9 の stage-0/1 併用期の論点として §10 に移す | E1（構成関数）。E4 は keying をモデル化する slice 2 で付与 |

### 2.3 ドメインサービス（純関数）

本節の純関数のうち、**状態ファイル・監査ブロックの描画**にあたるもの（`render_audit_block` / `state_writers`）は、ES 化により**投影の責務**へ移る — 描くのは ReadModelUpdater（U4）であって、ドメイン層ではない（ADR-003 / ADR-004。コードの移動は U4 の Bolt で実施する）。ドメインに残るのは値オブジェクトの Always Valid 検証（`StateFieldValue` の単一行検査、`EventType` の閉集合、行終端エスケープによる行偽造不能性）と、集約に置けない横断の判断である（01 §7.1 原則 2）。`find_all_events`（他クローンのシャード横断読取）と `classify_state_version` は本コンテキストに残る。

| サービス | 内容 |
| --- | --- |
| `render_audit_block` | `## Heading` / `**Timestamp**` / `**Event**` / フィールド行 / `\n---\n`。値の行終端（`\r\n?` `\n` U+2028 U+2029）を `\n` リテラルへエスケープし、第二のフィールド行・イベント行の偽造を防ぐ |
| `find_all_events` | shard 横断の順序: timestamp（秒精度 ISO）ソート＋バッファ位置 tiebreak。**通常読取は決して fail-closed しない**（authority 比較の同秒 fail-closed は orchestration の述語側 — B9）。出力は順序付き専用型（外部から構築・再ソート不能 — W15 の E1 装置） |
| `classify_state_version` | 4 分類の単一実装（W7） |
| `state_writers` | `set_field`（無言 no-op）/ `set_field_strict`（不在で throw — 「無言 no-op は検出不能なドリフト」）/ `set_or_insert_field` / `remove_field` の 4 種。純粋な string→string |

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
| `WorkflowExecutionRepository` | orchestration の `Report` / `Continue` / `Park` / `Jump*` / `Recompose` / `SetAutonomy`（10 §3） | 集約 `WorkflowExecution` の ES 形 Repository。`store(event, aggregate)`（単一イベント＋適用後集約を同一 Tx、楽観 `version`）/ `find_by_id(&IntentId)`（スナップショット＋差分 replay で完全再構成）— C3 / ADR-006 | `WorkflowExecutionRepositoryImpl`（SQLite EventStore — C6 のスキーマを内包）。テストダブルは `InMemoryWorkflowExecutionRepository`（gateway-taxonomy §5） |
| 外部システムクライアント（Git。例 `GitWorktreeClient`） | orchestration（Bolt / swarm — slice 2）、worktree 6 動詞 | worktree add / merge 3 戦略 / branch 削除 / conflict 検出 `/^CONFLICT \(/m`。別プロセスとの RPC であって集約の永続化ではない（gateway-taxonomy §1） | アダプタ層の Gateway（spawn 基盤 A4 経由、30s タイムアウト、SIGTERM でタイムアウトと失敗を区別） |

**ポートではないもの**（同規則の帰結。ポート表に載せると Gateway 責務の分類が濁る）:

- `FileStore`（アトミック書込 tmp+rename、追記専用 open、シンボリックリンク連鎖検査、封じ込め検査）は **Repository 実装と投影ライタの内部部品**であって、ユースケースが消費するポートではない。「読むだけの Gateway だから Reader」式のポート造語（`Store` / `Reader` / `Writer` / `Source` / `Provider`）と、`StateFileRepository` のような媒体名の Repository は禁止（gateway-taxonomy §2・§3）。
- `Clock` / `ProcessProbe` / `Tmpdir` は**アダプタ層の機構**であり、実装は機構モジュールに置き、差し替えは composition root が配線する（gateway-taxonomy §1、設計監査 C4）。
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
- **配置の規範**（§3 の帰結）: `Clock` / `ProcessProbe` は**アダプタ層の機構モジュール**（コンテキストの外、クレート root）に置き、実物と fake の差し替えは composition root が配線する。`FileStore`（アトミック書込・追記専用 open・封じ込め検査）と正準 JSON（A2）・ハッシュは **Repository 実装と投影ライタの内部部品**として実装側に閉じる。いずれも use-case 層に trait を置かない（[`coding-rules/gateway-taxonomy.md`](../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md) §1、設計監査 C4）。
- **Gateways**: `FileStore` 実装（`O_RDWR|O_APPEND|O_CREAT|O_NOFOLLOW|O_NONBLOCK` open、fstat regular-file 検査、書込前後の記述子同一性再検証 — 書込中 rename は行方不明の行ではなく**囲んでいる audit-first トランザクションの失敗**になる。**nlink の意図的非対称**: 通常 append 経路は `nlink != 1` を拒否**しない** — rsync `--link-dest` / `cp -al` スナップショットへの拒否が「以後の全 gate/hook 追記をフレームワーク全体で文鎮化した」実績による。厳格な多重リンク拒否は fork/merge 経路のみ。Rust 再実装で防御を「強化」すると同じ障害を再現する）、外部システムクライアント（Git）実装（spawn 基盤 A4 経由、30s タイムアウト、SIGTERM でタイムアウトと失敗を区別）。ロック dir 実装（mkdir-EEXIST、owner.json スタンプ、rename CAS reap）は**退役**し、並行制御は `WorkflowExecutionRepositoryImpl` の SQLite Tx ＋ 楽観 version に移る（ADR-007。逸脱台帳 [`deviations.md`](deviations.md) 参照）。**I/O 責務はすべてここ**。テスト用 in-memory 実装を最初に用意する。

## 5. インフラストラクチャ層の利用

正準 JSON（A2 — `intents.json` は 2-space、`runtime-graph.json` は決定性契約「同一監査ログ → バイト同値」）、文言カタログ（A3）、ハッシュ（SHA-256 prefix-hash）は純粋部品。MD5 のロック dir 名はロック機構の退役に伴い不要になる（ADR-007。stage-0/1 併用期の互換維持は §9・§10 の論点）。アトミック書込・spawn 基盤（A4）を呼ぶのは Gateway のみ。`tracing` 計装（A10）は application/adapter 層で、派生イベント発行のチョークポイントは**ジャーナル追記の Tx コミット成功後**（`WorkflowExecutionRepositoryImpl` の `store`）に置く（ADR-003）。**POSIX 前提**（O_NOFOLLOW・`kill(pid,0)`・mkdir ロック・rename 意味論）は方針書 R3 のとおり初期フェーズの明示的制約で、Windows はフェーズ D で防御の等価物定義とセット。

## 6. 不変条件表（強制手段つき）

E4 の定義名は W1〜W5 について [`formal/workspace/audit_lock.qnt`](../../formal/workspace/audit_lock.qnt)（v3 — **10 不変条件 green・mutation 10/10・到達性 witness 7 本モジュール内定義**）に実在する。**注記（2026-08-23）**: W1〜W5 の文言と E4 定義名は upstream の mkdir ロック時代の規範（旧）である。ADR-007 でロック機構は退役したため（§2.1）、`audit_lock.qnt` を「ジャーナル / スナップショット / version / チェックポイント協定」モデルへ改訂したうえで Bolt B5 が W1〜W5 と定義名を差し替える（改訂して存続 — 本 Unit では変更しない）。

| # | 不変条件 | 強制 | E4 定義名 / 備考 |
| --- | --- | --- | --- |
| W1 | audit-first: 監査 emit が state 書込に先行し、emit 失敗（throw）時は state を書かない。read → decide → emit → write が 1 クリティカルセクション。**emit 成功〜state 書込の間のクラッシュは「監査済み・state 未書込」という到達可能な中間状態**であり（R6）、回復は reap ＋冪等 replay guard が受け皿 | E3+E4 | `audit_lock::audit_first`（state 書込は emit 済みトランザクション内でのみ）＋`audit_lock::pending_only_with_lock`。中間状態と回復の到達性は `w_crash_mid_txn` / `w_recovery_after_mid_txn_crash` |
| W2 | 生きている閾値未満のロック保持者からは決して奪わない（reap は ESRCH または stale **厳密超過**（年齢 > 閾値）のみ — upstream の `> lockStaleMs()` に一致）。**経路を問わず**保持者間の直接移転も同条件に限る（acquire 経由の横取りも不可）。未スタンプ dir の猶予保護は E3 のみ（モデルは常時スタンプ済みの抽象 — slice 2 候補） | E3+E4 | `audit_lock::no_reap_of_live_fresh`（reap 経路）＋`audit_lock::lock_no_steal`（経路非依存の移転条件 — 本モデルにおける相互排他の実質検査）＋`audit_lock::reaper_alive` |
| W3 | 台帳・状態への書込はロック保持者のみ。解放も保持者のみ | E3+E4 | `audit_lock::writes_require_ownership`＋`audit_lock::release_requires_ownership` |
| W4 | ロックは identity ごとの深度カウンタで再入可。深度 0 でのみ解放。深度とロック在否は常に整合 | E3+E4 | `audit_lock::depth_consistent`＋`audit_lock::reentrant_release_keeps_lock` |
| W5 | クラッシュ（exit ハンドラなし）ではロックが**そのまま残って** stale 化し（状態を変えない）、以後 reap で回復可能 | E3+E4（safety）/ E4 予定（liveness） | safety は `audit_lock::crash_leaves_lock`。回復経路の実在はモジュール内 witness（`w_dead_owner_reap` / `w_threshold_reap` — CI は負形式で violation = pass）。「解放または reap 可能」の temporal liveness は定義名を与えた上で `quint verify`（nightly）に載せるまで E4 と数えない |
| W6 | 状態ファイルのフィールド値は単一行（C0 / DEL / U+2028 / U+2029 拒否） | E2 | `StateFieldValue` |
| W7 | State Version の分類は runtime と doctor で同一関数（乖離が構造的に不可能） | **E1** | 装置: 分類結果型 `StateVersionClassification` のコンストラクタを分類器モジュール内 private とし、`classify_state_version` 経由以外で値を**生成不能**にする（別実装の分類器は戻り値型を作れない） |
| W8 | 状態書込は tmp+rename でアトミック。read-only ターゲットは W_OK 事前チェックで書込バリアとして尊重（rename 貫通を塞ぐ） | E3 | — |
| W9 | **構造化ブロック**のイベント型は 86 閉集合のみ・呼出側 `Event` キー供給禁止・値の行終端エスケープで行偽造不能。append-raw の event なし note（Error / Recovery 形 — timestamp ちょうど 1）は**別枠の正当ブロック**で、merge delta 検証もこれを受理する | E1+E2 | `EventType`／`AuditFieldKey`／`render_audit_block` |
| W10 | authority 3 deny-list（RESERVED はパース前拒否、PROTECTED は append で拒否＋bypass env、MERGE_PROTECTED は delta で拒否）。宣言はイベントスキーマ側（B5） | E1+E3 | 拒否文言は文言カタログ（逐語 3 形） |
| W11 | audit-merge は delta のみ追記し、ブロック境界・既知イベント・非 merge-protected・worktree スナップショットのバイト/inode 一致・**main 先頭 boundary バイトの prefix-hash 一致**を全て検証（mid-Bolt tampering / truncation を逐語で区別） | E2+E3 | R7 の受け皿。authoritative fork 行は main から回収（書込可能な worktree コピーを信用しない） |
| W12 | main shard を書き換えるコードパスは存在しない（追記専用）。唯一の例外は audit-fork の worktree ミラー確立（tmp+rename）で、以後は再び追記のみ | **E1** 候補+E3 | 装置: main shard と worktree ミラーを**パスの型で分離**し（`MainShard` / `WorktreeMirrorShard`）、置換 API は `WorktreeMirrorShard`（audit-fork ユースケース専用型）にのみ定義する — main shard 型には追記しか存在しない |
| W13 | birth は単一チョークポイント。`intents.json` の全変更は workspace センチネルロック（**2 成分形** — 全 space が単一バケット）下で、keying に `activeIntent()` を使わない（並行 first-run の二重 birth 防止） | E1+E3 | `LockIdentity` の構成関数が唯一の入口 |
| W14 | 状態機械遷移は strict writer（不在フィールドは throw — 無言 no-op は検出不能ドリフト）。M12 は修正方針確定済み（逸脱台帳 #2。実装形は §10 のとおり実装時に確定） | E2+E3 | `set_field_strict` |
| W15 | shard 横断の順序は timestamp ソート＋バッファ位置 tiebreak で、通常読取は決して fail-closed しない（同秒 fail-closed は orchestration の authority 述語のみ — B9） | **E1** | 装置: `find_all_events` の出力を順序付き専用型（外部から構築・再ソート不能）にし、順序付きイベント列はこの型経由でしか得られない |
| W16 | runtime-graph は純観測者（state を変異しない・質問しない）で、同一監査ログから**バイト同値**を再現する | E3＋A2 | 決定性は正準 JSON（A2）が前提。折り込み規則は verification 提供（B8） |

## 7. 実装順序（D10 × domain-model-first）

1. **ドメイン例のテスト**: 「emit が throw したら state は変わらない」「reap は死んだ所有者か閾値超過のみ」「BoltRefs の append は重複で throw」「stub のない record は activeIntent に解決されない」等を正準用語で書く。
2. **Domain Primitive → 集約の TDD**: §2.2 の E1/E2 を先に。proptest は `BoltRefs` round-trip・`render_audit_block` エスケープ（任意入力で行偽造不能）・`ShardName` 構成・checkbox parse に適用。
3. **in-memory 実装**（`InMemoryWorkflowExecutionRepository` と外部システムクライアント（Git）の fake、機構の `Clock` / `ProcessProbe` の fake）でユースケーステスト。`FileStore` は Repository 実装の内部部品なので、そのフェイクも実装側に閉じる（§3）。並行制御のテスト（楽観 version の競合と再試行）は loom 等の検討を含め実装時に確定。
4. **ITF 準拠**: `audit_lock.qnt`（ADR-007 により「ジャーナル / スナップショット / version / チェックポイント協定」のモデルへ改訂 — Bolt B5）のトレース（`lastAction`/`lastActor` 駆動）を Repository 実装の純粋遷移関数に再生。
5. **実 Gateway は最後**: ゴールデン互換層で upstream 実ワークスペース（TS 版が書いた実物）を読ませ、state バイト列・監査行・shard 名・レジストリ JSON の一致を検証（stage-1 切替の前提そのもの）。

## 8. Quint ゲート実験 — 第一陣 3 号の記録

- `audit_lock.qnt` v1（不変条件 5 本）は green・mutation 3/3 だったが、**敵対的レビューが v1 の重大な検査力の穴を実証**した: 所有権の移転がアクションラベル経由でしか守られておらず、(a) acquire の不在ガード除去（＝保持中ロックの横取り）、(b) release の所有権ガード除去（＝非保持者の解放）、(c) reap の reaper 生存ガード除去、(d) **crash がロックを消す変異（R6 の核心の直接否定）** — の 4 変異がすべて全不変条件を素通りした。engine_loop v1 と同型の教訓: 「観測ラベル依存の不変条件は、経路を変えた違反に盲目」。
- v2 で状態遷移レベルの不変条件 4 本を追加（`lock_no_steal` — 経路非依存の移転条件で相互排他の実質検査 / `release_requires_ownership` / `reaper_alive` / `crash_leaves_lock`）。**9 不変条件 green（5000×40）＋ mutation 9/9**（9 変異がそれぞれ狙いの不変条件で検出。多重防御も確認 — reap ガード除去は `no_reap_of_live_fresh` と `lock_no_steal` の両方が捕捉）。
- **到達性 witness 5 本をモジュール内に定義**（`w_threshold_reap` / `w_dead_owner_reap` / `w_deep_release` / `w_emit_fail` / `w_full_unwind`）。CI は負形式（`--invariant "not(w_x)"`）で実行し violation = 経路実在 = pass と読む。green のままなら経路がモデルから消えた退行（例: tick 凍結で閾値 reap が到達不能になる）を検出できる — ファイル外の一時的な反証 run では退行に盲目だった穴の是正。
- v2 でも維持する抽象化はモデルヘッダに完全列挙（二相 acquire と未スタンプ猶予・取得予算・reapLiveOwnerAfterStale・rename CAS の内部状態・バッチ原子性 — slice 2 候補）。「解放または reap 可能」の temporal liveness は定義名を与えて `quint verify`（nightly）に載せるまで E4 と数えない（W5）。
- **v3（PR #2 レビュー反映）**: audit-first のクリティカルセクションを emit / write の 2 段に分割し、「監査済み・state 未書込」の crash 中間状態（R6 の核心）を到達可能にした（`w_crash_mid_txn` → reap 回復 `w_recovery_after_mid_txn_crash`）。stale 境界を upstream の厳密超過（`>`）に一致。10 不変条件 green（5000×50）＋ mutation 10/10（emit なし state 書込の新変異を含む）＋ witness 7/7。
- 第一陣は 3/3 完了（`engine_loop` v2 / `audit_lock` v2 / `stop_hook` v1）。総括は 10-orchestration §9 の第 3 回記録と ADR 0003 試行条項。

## 9. 実装ノート — 仕様と実装の分離

orchestration §10 の裁定（in-process 合成、S1〜S4 維持）は本コンテキストの提供面にそのまま適用される。追加の分類:

- **仕様**: エンジン所有 11 動詞の外部拒否面（S3 — マーカー受理・bypass env・逐語文言）、25 動詞の CLI 語彙、shard 名・ブロック文法・イベント語彙・authority 拒否、ロックの奪取条件と予算の**意味論**、fork/merge の検証と拒否文言、worktree のパス/ブランチ導出とイベント。
- **実装**: ~~ロック dir の物理配置（tmpdir + md5 名）や owner.json の形式は…初期フェーズはロックの物理形式も互換維持する~~ → **ADR-007 でロック機構そのものを退役させ、ロック dir を生成しないことにしたため、この互換維持の前提は失効した**（逸脱台帳 [`deviations.md`](deviations.md) に「ロック dir の非生成」として登録済み）。stage-0/1 併用期に upstream プロセスと同一ワークスペースを触る場合の相互排他をどう担保するかは**未確定**であり、§10 の未決事項として立てる。`AIDLC_WORKSPACE_LOCK_OWNER_PID` の env 名と受理意味論は D6 の対象として維持する。

## 10. 未決事項

- `ScopedStorage`（B12）の API 詳細は knowledge コンテキスト仕様（13 号 — 12 号は workflow-definition が使用）と同時に確定する。
- `SessionStampStore` の rebind offer 文言と Codex/Copilot 差分（session-start フックの詳細）は検証コンテキスト仕様（フック帰属分）で確定する。
- **stage-0/1 併用期の相互排他**（2026-08-23 追加）: ADR-007 でロック dir を生成しなくなったため、upstream プロセス（mkdir ロックを取る）と同一ワークスペースを並行して触る場合の相互排他は担保されない。併用を許すか、許すならどの機構で担保するかはオーナー裁定待ち（§9 の旧前提は失効）。
- **`intents.json` の直列化機構の確定**（2026-08-23 追加）: 集約の遷移は SQLite Tx ＋ 楽観 version に移ったが（ADR-007）、`intents.json`（レジストリ）は SQLite ジャーナルの外にある。W13（birth の単一チョークポイント）と §2.2 `LockIdentity` の keying 規範をどの機構で満たすかは **U3 の設計で確定する**。本書は規範（単一バケットへの集約・`activeIntent()` を keying に使わない）だけを保持し、機構は未確定とする。
- ~~ロック keying（複数 identity・センチネル 2 成分形）と二相 acquire・未スタンプ猶予のモデル化~~ → mkdir ロックの退役（ADR-007）により `audit_lock.qnt` は「ジャーナル / スナップショット / version / チェックポイント協定」の検証モデルへ改訂する。temporal liveness の定義名付与も改訂後のモデルで行う。
- runtime-graph のセンサー区画折り込み規則の受け渡し形式（B8 — 宣言的規則の表現）は verification 仕様で確定。
- M12 修正の実装形（birth で行を書く vs `set_or_insert_field` 化）は実装時に選び、ゴールデンの分岐点を 1 か所に固定する。
- Windows の防御等価物（O_NOFOLLOW / kill(0) / mkdir ロック / rename）はフェーズ D（R3）。
