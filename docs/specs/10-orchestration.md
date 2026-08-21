# orchestration コンテキスト仕様

> **位置づけ**: コンテキスト別仕様の第 1 号。`01-domain-model.md` の裁定（B1・B5・B9・B10・B11・B13・B14）と D3/D4/D10、ADR 0001〜0004 に従う。
> **契約コーパス**: upstream `02-orchestration-engine.md`（主）、`04-stage-protocol.md`・`07-hooks.md` §7・`09-cli-tools.md` §5-6（従）。精密抽出は [`research/orchestration-next-ladder.md`](research/orchestration-next-ladder.md)（`next` 21 分岐・scope 解決・load-steering・per-unit）、[`research/orchestration-report-guards.md`](research/orchestration-report-guards.md)（`report` 13 段ガード・verdict・CheckboxState 遷移）、[`research/orchestration-directives-verbs.md`](research/orchestration-directives-verbs.md)（Directive カタログ・jump/park/single/recompose/autonomy/Stop フック）に収録済み。本書は**構造の規範**を担い、逐語の完全列挙は抽出文書と upstream を正とする。
> **状態**: ドラフト（フェーズ A。slice 1 = 決定論コア、slice 2 = Construction 実行機構 §7 — 2026-08-22 追補済み。精密抽出は [`research/orchestration-bolt-verbs.md`](research/orchestration-bolt-verbs.md)・[`research/orchestration-swarm-protocol.md`](research/orchestration-swarm-protocol.md)・[`research/orchestration-unit-wave-loopback.md`](research/orchestration-unit-wave-loopback.md)）
> **策定日**: 2026-08-22

---

## 1. 責務と境界

orchestration は「**次に何が起こるか**」を所有する。engine（`next` の 21 分岐ラダー / `report` の 13 段ガード）、`Directive`（10 種の判別共用体・28KiB 上限）、ApprovalGate の 3 層構造、jump / park / recompose、Construction 実行機構（Bolt / swarm / per-unit 反復）、Stop フックの forwarding loop がここに属する。

境界の要点（01 の裁定の引き受け）:

- **B1**: scope grid は workflow-definition の不変の成果物として**読むだけ**。recompose の flip はこのコンテキストのコマンドであり、`effectivePlanAction`（grid ＋オーバレイの合成読み）はここが所有する read model。永続化は workspace に委ねる。
- **B5**: 監査台帳の mechanics は workspace 所有。swarm のマージ失敗 converged unit の復旧（「監査行なし」中間状態から）は本コンテキストの**サーガ**。
- **B9**: `HUMAN_TURN` の記録は workspace の事実、`humanActedSinceGate`（同秒 fail-closed を含む導出述語）と昇格可否の政策は本コンテキスト。
- **B10**: レビューレシートの鮮度・凍結述語は **verification 所有のクレートを依存として呼ぶ**。二重実装しない。
- **B11**: walking skeleton の stance 解決はここのプロセス。アンカー計算（scope で最初の Construction EXECUTE ステージ）は workflow-definition の純関数を呼ぶ。
- **B13**: HOLD-MERGE は本コンテキストの政策値。workspace には opaque な保存 API としてだけ渡す。
- **B14**: directive protocol（`next` / `report`、Directive スキーマ、`continue_token`、ask）は本コンテキストが公開する **Published Language**。stage runner はそれに conform する distribution の生成物。

（Construction 実行機構の全体は Bolt / swarm / per-unit 反復に **Build-and-Test loop-back**（3.6 → 3.5 の公認後方ジャンプ、intent あたり最大 3 回）を加えた 4 つで、01 §3.2 のとおり。詳細は §7。）

## 2. ドメイン層

### 2.1 集約: `WorkflowExecution`

1 つの Intent の実行状態を束ねる集約ルート。カーソル（`Current Stage`）、ステージごとの `CheckboxState`、`Status`（Running / Completed）と**直交する park マーカー**（upstream の `Parked` / `Parked At Stage` フィールド ≒ `Option<(Timestamp, StageSlug)>`。「マーカーが残ったままカーソルが先へ進んだ」stale-by-progress 状態が正規に存在し、parked 分岐の発火は「マーカー有 ∧ Parked At Stage == Current Stage ∧ 再入フラグ無」の**導出述語**で判定する — 3 値 enum に畳むと D6 の状態ファイル互換と Branch 2.5/2.6 の再現が壊れる。Quint モデルの 3 値 status はこの直交対の簡約でモデルヘッダに注記済み）、recompose オーバレイ、`AutonomyMode`、`SkeletonStance`、`MergeHeld` を内包する。

- **トランザクション境界**: 集約 1 更新 = `withAuditLock` 1 区間 = 監査 emit → 状態書き込みの 1 クリティカルセクション（audit-first）。upstream の設計がそのまま「1 トランザクション 1 集約」の DDD 規範に一致している。
- **コマンド**: 集約コマンドの全集合は、エンジン所有 11 動詞（`set` / `checkbox` / `advance` / `finalize` / `complete-workflow` / `gate-start` / `approve` / `reject` / `revise` / `skip` / `park` — Rust 名は `set_field` / `set_checkbox` 等へ機械的に写す。外部からの直接呼び出し拒否面は §9 S3）に、**`unpark` / `recompose_flip` / `set_autonomy` を加えたもの**（B1 により flip は本コンテキストのコマンド。upstream では unpark は直接呼出可の state 動詞、set-autonomy は bolt 動詞だが、集約のコマンド面としてはここに属する）。集約の外からチェックボックスやカーソルを直接書く経路は**型として存在させない**（upstream の state-transition guard の内側版）。
- **発行イベント**: `STAGE_*` / `GATE_*` / `WORKFLOW_*` ファミリに加え、advance のフェーズ境界トリオ（`PHASE_COMPLETED` / `PHASE_VERIFIED` / `PHASE_STARTED` の固定順）、`RECOMPOSED`、`AUTONOMY_MODE_SET`（監査イベントスキーマ = Published Language クレートの型で発行）。
- **主要不変条件**: §6 の表。

`Bolt` 集約と `SwarmBatch` サーガは §7（slice 2）で規定する。

### 2.2 Domain Primitive（E1/E2 の受け皿）

| 型 | 定義 | 強制 |
| --- | --- | --- |
| `DirectiveKind` | 10 種の閉集合（`load-steering` / `run-stage` / `dispatch-subagent`※ / `invoke-swarm` / `present-gate`※ / `ask` / `print` / `error` / `done` / `parked`。※は placeholder — 投機実装禁止） | E1 |
| `Directive` | kind による判別共用体。`validateDirective` 相当の検証（未知キー拒否・型/必須・cross-field・`gate` は boolean か `"unresolved"` のみ）を**コンストラクタで**行う | E1+E2 |
| `Verdict` | 受理 10 語。`approved`/`completed`/`complete`/`done` は同義語として正規化、`resume`/`resumed` 同義 | E2 |
| `CheckboxState` | 6 値（`[ ]`/`[-]`/`[?]`/`[R]`/`[x]`/`[S]`） | E1 |
| `PlanAction` | EXECUTE / SKIP | E1 |
| `EffectivePlan` | overlay が grid に勝つ合成読み（B1 の read model） | E1（合成関数） |
| `AutonomyMode` | **2 境界型に分離**: 状態読取側は `"autonomous"` 厳密一致のみ autonomous、それ以外（未設定・空・未知値）はすべて gated（fail-closed リーダ — 初期化は失敗しない）。CLI `--mode` 引数側（`AutonomyModeArg`）は autonomous / gated の 2 値**厳密パース**で、不正値は逐語拒否 `Invalid --mode: <m>. Must be 'autonomous' or 'gated'.`（Controller 規約の初期化失敗経路。1 型に畳むと本家の拒否文言が発生不能になる） | E2（両面） |
| `SkeletonStance` | on / off / scope-dependent | E2 |
| `JumpDirection` | forward / backward / redo（インデックス比較から導出） | E1 |
| `ContinueToken` | HMAC-SHA256 封筒 `{p, m}`。ペイロード 18 キーの厳密型表、`timingSafeEqual` 検証、4 ダイジェスト束縛（bundle / directive / route / state） | E2+E3 |
| `ProgressSignature` | `stage::stateSha256(Last Updated 行除外)::directiveFingerprint` | E1（構成関数） |
| `DirectiveMaxBytes` | 28 KiB（28,672 bytes）。超過 Directive は**構築不能**ではなく emit 拒否（half-emitted を出さない） | E3（Presenter） |

### 2.3 ドメインサービス（純関数）

| サービス | 入力 → 出力 | 対応する upstream |
| --- | --- | --- |
| `next_decision` | 観測状態＋コンパイル済みグラフ → `Directive` ちょうど 1 つ。**書き込みなし** | `handleNext` 21 分岐ラダー |
| `report_dispatch` | (verdict, checkbox, gated, final, moved-on, explicit-stage) → 遷移コマンド列 or 拒否。**対応範囲は段 10 の gate-lifecycle アームと §7.3 forward ディスパッチ表に限る**。段 1〜9・11〜13 は `Report` ユースケースが所有（§3） | verdict → サブコマンド選択は**エンジンが**行い、呼び出し側は選ばない |
| `human_acted_since_gate` | 監査行の射影 → bool。fail-closed になるのは**同秒かつ別シャード**のときのみ。同一シャードはシャード内 pos 順で決定的に判定 | B9 の導出述語 |
| `jump_resolve` | (target, cursor) → `JumpDirection` ＋帰属検証 | `aidlc-jump resolve`（純読取）と `execute`（コミット）の分離 |

## 3. ユースケース層

CLI 動詞・フック応答 1 つ = ユースケース 1 つ。ポート（trait)はこの層で定義する。

**ユースケース**（slice 1 の範囲）: `Next`（読み取り専用）、`Continue`（トークン検証＋再構築＋トランザクショナルなカーソル前進）、`Report`（13 段ガード → 集約コマンド）、`Park` / `Unpark`、`JumpResolve` / `JumpExecute`、`SetAutonomy`、`Recompose`、`SingleStageRun`。slice 2: Bolt 8 動詞、Swarm 3 動詞。

`Report` は 13 段ガードの実行主体である。ただし段 2 の `--single` は **Controller が `Report` より前に `SingleStageRun` へ分岐**させ、`Report` には到達させない — これが I10 の E1（遷移ポート非注入）が成立する条件で、turn-shape marker と state-version guard は `SingleStageRun` 側でも実施する。段 11（completion-evidence: pipeline link レシート / per-unit カバレッジ / paused-unit 拒否 / ensemble contribution 証跡）と段 12（practices promotion レシート）、および approve 側の前提スタック（verifyStageArtifacts / summary confirmation / pipeline link / Practices Affirmed Timestamp / 冪等 replay guard / next-slug 非 SKIP — upstream 03 §5.7）は、レビュアー述語（B10）と同様に verification / workspace への依存として観測する。per-unit カバレッジと paused-unit 拒否の詳細は §7.3〜7.4。

**ポート**:

| ポート | 責務 | 実装（Gateway） |
| --- | --- | --- |
| `StateFileReader` / `StateFileWriter` | 状態ファイルの読取／アトミック書込。**`Next` ユースケースには Writer を注入しない**（read-only 不変条件の型による強制） | workspace コンテキストの公開サービスへ委譲 |
| `AuditLedgerAppender` | audit-first の追記（emit 失敗で状態書込を中止） | 同上 |
| `AuditLedgerReader` | `humanActedSinceGate` 等の述語用の射影読取 | 同上 |
| `StageGraphReader` | `stage-graph.json` / `scope-grid.json`（Published Language）の読取 | canon-json コーデックを内部に持つ Gateway |
| `WorkspaceLock` | `withAuditLock` 相当（再入・reap は workspace 所有） | workspace の供給サービス |
| `MarkerStore` | active-directive marker / turn マーカー / steering MAC キー（`.aidlc-steering-token-key` — I8 の例外 2 つの書込面）のマシンローカル書き込み。原則 advisory — 失敗は throw しない。**例外**: Copilot-commit アームのみ発行失敗が work directive の発行自体を拒否する fail-closed（Copilot ハーネスは D5 の初期スコープ外だが、ポート契約として記録） | Gateway |

**verification への依存**（ポートではない）: レシート鮮度・凍結述語（`verifyReviewerPrecondition` 相当）は verification 所有クレートの公開 API を直接呼ぶ（B10）。フロアイベントの列挙はイベントスキーマ側の宣言を読む。

## 4. インターフェイスアダプタ層

- **Controllers**: `aidlc-orchestrate` の引数パース（`next` / `report --stage --result --user-input` / `continue <token>` / `park`）、Stop フックの stdin JSON（forwarding loop の入口）、resume メニュー（1–4 正規化）。**バリデーションは Domain Primitive の初期化可否で判定する**: 生の引数を `Verdict::parse` / `StageSlug::parse` / `ContinueToken::parse` 等に通し、成功した**型付き値オブジェクトをユースケースのメソッドに渡す**（例: `Report::execute(stage: Option<StageSlug>, verdict: Verdict, user_input: Option<UserInput>)` — **明示 `--stage` の有無はそれ自体が契約**なので `Option` で型に運ぶ: 省略時は Current Stage に作用、`--single` と skipped 受理は明示必須、in-progress gated の回復も明示必須。省略時の解決は Controller ではなくユースケース側で行う）。`--single` フラグを検出した場合は `Report` ではなく `SingleStageRun` へルーティングする（§3）。ユースケースの引数に素の String を置かない。初期化失敗は Presenter が文言カタログの拒否文言（`Unknown --result "<v>". accepted outcomes: <list>.` 等）に変換する。Controller 自身は検証ロジックも業務判断も持たない。
- **Presenters**: Directive → stdout **1 行 JSON**（ADR 0001 `contract-compact`）。28KiB 超と malformed の emit 拒否（逐語は文言カタログ）。exit code、stderr の verbatim 中継（sibling 遷移拒否の `Transition rejected by …` 形式）。
- **Gateways**: 上記ポートの実装。状態ファイル・監査台帳の**形式**は workspace コンテキストの所有物なので、Gateway は workspace の公開サービス／コーデックへ委譲し、形式知識を二重化しない。**I/O 責務はすべてここ**（01 §7）。テスト用に in-memory Gateway 一式を最初に用意する（§7 実装順序）。

## 5. インフラストラクチャ層の利用

正準 JSON（A2）・文言カタログ（A3）・ハッシュは純粋部品としてドメイン層からも利用可。プロセス spawn 基盤（A4）とアトミック書き込みを呼べるのは Gateway のみ。`tracing` 計装（A10）は application / adapter 層 — `Next` / `Report` 1 呼び出しがターントレース内のトップレベルスパンになり、監査イベントは append 成功後にのみ `aidlc.*` 属性でイベント発行する。

## 6. 不変条件表（強制手段つき）

E4 の定義名は I2〜I7 が [`formal/orchestration/engine_loop.qnt`](formal/orchestration/engine_loop.qnt)（slice 1 v2）、I16〜I17 が [`formal/orchestration/stop_hook.qnt`](formal/orchestration/stop_hook.qnt)（v1）、I14 が [`formal/workspace/audit_lock.qnt`](formal/workspace/audit_lock.qnt)（v3）に実在する — 第一陣 3 モジュールすべて green・mutation 検査力確認済み。

| # | 不変条件 | 強制 | E4 定義名 / 備考 |
| --- | --- | --- | --- |
| I1 | 1 回の呼び出しで Directive をちょうど 1 つ emit。malformed・28KiB 超は emit 自体を拒否（half-emitted なし） | E2+E3 | Presenter の refuse 2 形（文言カタログ） |
| I2 | 有効プランが SKIP のステージに run-stage を emit しない | E4+E3 | `engine_loop::no_run_stage_for_skip`（観測射影）＋`engine_loop::cursor_in_scope`（状態レベル — 単一ガード破壊を直接検出する実働の守り） |
| I3 | gated ステージの完了は承認経由のみ（ゲート迂回トレース不存在）。backward jump は承認履歴を無効化する | E4+E3 | `engine_loop::no_gate_bypass`。実行時は handleApprove 側強制＋state-transition guard |
| I4 | park → resume でカーソル位置が保存される | E4 | `engine_loop::parked_position`＋`engine_loop::unpark_restores_position`（resume 側） |
| I5 | stale re-report（カーソル通過済み completed への再報告）は何もコミットせず冪等 done | E4 | `engine_loop::stale_rereport_yields_done`＋`engine_loop::stale_rereport_frame`（フレーム条件 — 「何もコミットしない」を prev 状態スナップショットで検査） |
| I6 | アクティブ（in-progress / awaiting-approval / revising）なステージは高々 1 つ | E4 | `engine_loop::at_most_one_active`（upstream 未明文化の派生不変条件。**暫定規範**として採用し、A7 追従で反例が出たら降格） |
| I7 | gate-lifecycle の checkbox 前提は厳密（`awaiting-approval`←`in-progress`、`rejected`←`in-progress\|awaiting-approval`、`revised`←`revising`） | E2+E4 | `engine_loop::gate_lifecycle_preconditions`（prev 状態スナップショット経由でガード除去 mutation を検出） |
| I8 | `next` は読み取り専用（例外は steering MAC キーと active-directive marker の 2 つのみ、いずれも advisory） | **E1**+E5 | `Next` ユースケースに Writer ポートを注入しない構成で型強制 |
| I9 | 状態遷移はエンジン所有 11 動詞のみ。外部からの直接呼び出しは PID 束縛マーカーで拒否 | E1+E3 | 内側は集約メソッドの可視性、外側（マルチコール経由）は guard |
| I10 | `--single` は本流を進められない（advance / approve / complete-workflow に**構造的に到達不能**、synthetic workflow id は本流の完了証跡を満たさない） | **E1** | `SingleStageRun` ユースケースには遷移ポート自体を注入しない |
| I11 | autonomous への昇格のみ human presence を要する（降格は不要）。**同秒かつ別シャード**の HUMAN_TURN とゲート解決は fail-closed（同一シャードは pos 順で判定） | E2+E3 | `AutonomyMode` パーサ＋`human_acted_since_gate` 述語。E4 は audit_lock モデル（workspace 第一陣）と合流 |
| I12 | `continue_token` の MAC 不一致・ダイジェスト移動・型表違反はすべて fail-closed（fresh `next` からやり直し） | E2+E3 | fail-closed の逐語は handleContinue 4 形＋transportRunStage 2 形の計 6 形（文言カタログ） |
| I13 | `skipped` は routed lifecycle outcome（明示 `--stage`・CONDITIONAL または plan SKIP・非空 `--reason`・Current Stage 厳密一致・checkbox 前提） | E2+E3 | — |
| I14 | 監査 emit が状態書込に先行し、emit 失敗時は状態を書かない（audit-first） | E3+E4 | workspace 所有（11 §6 W1）。`audit_lock::audit_first` |
| I15 | HARD STOP RULE（ゲート提示後ターン即終了）。Construction / Operation ゲートは Approve / Request Changes の**厳密 2 択**（Ideation / Inception は skip 済みステージ再追加の第 3 選択肢可）。同一ステージ 3 回目の Request Changes 以降のゲートに Accept as-is が追加され、2 回目時点で予告義務 | E5+E3 | プロトコル文書＋Stop フック（forwarding loop）で補強 |
| I16 | Stop フックの forwarding は cap で必ず解放される（block は cap 未満のときのみ・cap はモード依存 8/2・記録は判定前に書く・done/parked 許可で streak ゼロ化・署名変化でカウントリセット・seed-2 は記録なし＆active 連鎖時のみ） | E3+E4 | `stop_hook::blocked_below_cap` / `cap_is_mode_dependent` / `record_before_decision` / `reset_on_terminal` / `signature_resets_count` / `seed2_on_active_chain`（v1 — green・mutation 7/7・witness 6 本＋決定的シナリオ `r_cap_release_*`） |
| I17 | carve-out（固定順の許可経路）は forwarding に勝つ。autonomous 下の parked は carve-out にならない（許可は他の carve か cap 解放のみ） | E3+E4 | `stop_hook::carve_beats_forwarding` / `parked_autonomous_guard` |

## 7. Construction 実行機構（slice 2）

逐語契約の完全列挙は research 3 本（bolt-verbs / swarm-protocol / unit-wave-loopback）を正とする。E4 は第二陣モデル（`bolt.qnt` / `swarm_convergence.qnt` — 未着手）成立後に付与し、それまで本節の不変条件は E2/E3/E5 のみを主張する。

### 7.1 `Bolt` 集約

「1 つの Unit（または依存で結ばれた少数の Unit 群）に対するステージ 3.1–3.5 の 1 回の実行」。8 動詞（start / complete / fail / abort / set-autonomy / dispatch-event / hold-merge / release-merge）のユースケースと `BOLT_STARTED` / `BOLT_COMPLETED` / `BOLT_FAILED` / `AUTONOMY_MODE_SET` の唯一の発行者。

- **合成のみ・重複禁止**（t48 emitter-pairing 規則）: sibling プリミティブ（state fork/merge・audit fork/merge・fragment fork/merge・worktree discard）を合成し、それらが所有する状態変異（Bolt Refs / Worktree Path）を決して重複実装しない。Rust では workspace の `WorktreeService`（11 §3）への依存として実現する。
- **順序規律 3 形**（逐語根拠つき — research bolt-verbs §3）: (a) `start --worktree` は検証 → `BOLT_STARTED` → state-fork → audit-fork → fragment-fork（検証先行は「orphan BOLT_STARTED を残さない」ため。各 fork 失敗はリカバリ `BOLT_FAILED` を発行してから失敗）。(b) `complete --merge` は **hold-merge チェック最前段** → `BOLT_COMPLETED` → state/audit/fragment-merge。(c) `abort --discard` のみ**意図的逆順**（discard 先・audit 後 — 先に emit すると「worktree が残っているのに掃除済みと主張する」ため）。
- **失敗エンベロープ**: `{"ok":false, "slug", "stage"(5 値), "reason"(17 値), "detail"}` で exit 1（contract-compact）。`error()` 経路（`ERROR_LOGGED`）とは明示的に別。sibling spawn は一律 30s タイムアウト、`SIGTERM` で `*-timeout` / `*-failed` を判別。
- **`BOLT_ABORTED` は存在しない**: abort は `BOLT_FAILED` + `Reason: aborted` を再利用（「audit count を安定に保ち、サブ分類はフィールドタクソノミで」）。
- **HOLD-MERGE**: 本コンテキストの政策値（B13）。halt-and-ask シーケンス中のマージ着地を防ぎ、`complete --merge` の最前段で逐語拒否。保存は workspace の `OpaqueFlagStore`。
- `--batch` は `/^[1-9][0-9]*$/`（E2）で、swarm の attempt 相関の join key。

### 7.2 `SwarmBatch` サーガ

責務三分割（逐語）: 「conductor が fan-out ＋ループ駆動（知識労働）、swarm ツールが収束判定＋マージ＋監査（決定論）、人間が autonomy を付与しバトンを受け取る」。prepare / check / finalize の 3 動詞で、**check は advisory・finalize が権威**。

- **finalize の 6 段ガード**（lying-conductor guard）: SwarmAttemptStamp `{stage, floor}` 一致 → attempt 一致 → worktree 存在 → confinement → tamper（保護テストファイルの git diff）→ green 再実行＋レシート＋binding 照合。conductor の申告を信用せず全 claim を再検証する。
- **`SWARM_*` 6 イベントの唯一の発行者は swarm ツール**（conductor prose は監査を発行しない — CLI_PROTECTED）。バッチ前進のキーは `SWARM_UNIT_CONVERGED` 監査行（ディスク成果物ではない）。
- **「行なし」中間状態とサーガ復旧**（B5）: 6 段ガードを通過したが merge-back に失敗した unit は `SWARM_UNIT_CONVERGED` も `SWARM_UNIT_FAILED` も得ない — converged 行はエンジンのバッチ前進信号なので、main に着地していない unit で前進させないため。復旧は**その unit にスコープした finalize の再実行**（`release-merge` は冪等、**prepare の再実行は禁止** — 既存 worktree がエラーになる）。行が着地して初めてバッチが前進する。exit 2 でバトンは construction モジュールの halt-and-ask seam に返る。
- **settle 再入**: 全 unit 収束後の `swarm_settled: true` run-stage は gate 専用（ステージ本体もレビュアーも再実行禁止）。中間バッチ後に approved を report してはならない（「後続バッチが未ビルドのままステージが完了してしまう」）。
- **reviewer 付き claim**: check 通過だけでは claim 不可。unit の worktree 内で `REVIEW_REQUESTED` → dispatch → terminal `REVIEW_COMPLETED`（`--project-dir` が worktree を指す）。`GATE_REJECTED` の捏造は禁止。
- **autonomous Code Generation gate**: prepare は worktree fork 前に全 unit の `CodeGenerationApproval`（7 チェック固定順・reason 逐語 6 形）を要求。worker brief は `AIDLC-UNIT:` ＋ `AIDLC-TESTING-CONTRACT:` マーカーで正確に始まる（§12b Plan Contract）。

### 7.3 Unit ライフサイクル

`unit start / pause / resume / complete`（conductor が直接叩く動詞 — エンジン所有 11 には含まれないが、委譲サブエージェントからは遮断）。

- **単一アクティブユニット不変条件**（E3）: 同一ステージで別 unit が open の間 start は拒否。自律 swarm がステージ所有中も拒否。unit は正本 DAG（`unit-of-work-dependency.md`）に存在必須。
- **complete は成果物のディスク実在検証をしてからレシートを commit**（「artifact walk が『遷移そのもの』から『遷移が検査するもの』へ移った」— claim-1 の逆転）。
- **receipt mode は粘着**: ライフサイクル行が 1 つでも存在したら以後の試行はすべて receipt mode（「artifact ファイルだけでは Unit は決済されない」）。
- **paused は最優先 hard-stop**: エンジンは `unit_state: paused` の ask を emit し、明示的 `unit resume` まで他の作業を開始できない。
- state の unit 4 フィールドはキャッシュで真実源は `UNIT_*` レシート（B9 と同型）。4 レシートは CLI_PROTECTED かつ MERGE_PROTECTED。

### 7.4 wave（stage-major 並列面）と unit-major

- **wave** `{batch_index, entries[]}` は 4 つの inline per-unit 設計ステージ専用（code-generation は不適格 — 共有 workspace に書き Plan Approval で hard-stop するため）。entry 検証は duplicate-unit と `required_produces ⊆ produces`（E2）。wave builder は serial lifecycle 動詞を呼ばず、**wave directive がそのまま batch checkpoint**。entry の review-state 語彙は閉集合（outstanding / retry-required / repair-required / recovery-required / escalation-required ＋ READY / NOT-READY / not-required — E1）。
- **unit-major**（`Construction Iteration: unit-major`、opt-in）: Unit-outer / stage-inner の歩行順。**autonomous swarm は unit-major では決して発火しない**。conductor の標準規則は「常に directive 自身の `directive.stage` + `directive.unit` に基づいて行動し、`Current Stage` に基づかない」（E5 — Stop フックの Plan Approval 例外と run-sensors の marker-first 解決がこれを補強）。
- **per-unit ルーティング**は slice 1 §2.3 の `next_decision` に属する: stage-major は最初の未カバー unit を `gate=false` で emit（report せず next 再実行）、ゲートは最後の unit の再 emit でのみ発火。`produces_kinds` は directive の produces パスとカバレッジ集合の**両方**を刈り込む（「何からも免除されない」）。

### 7.5 Build-and-Test loop-back（3.6 → 3.5）

NO EMERGENT BEHAVIOR RULE への公認例外。失敗した build-and-test 実行は意図的に in-flight のまま残し、ゲートも §13 儀式も成功実行まで繰延（stage diary はループを跨いで持続）。

- **回数の正本は artefact ledger**: `test-results.md` の `## Loop-Back Log` エントリ数がそのまま上限（**intent あたり最大 3**）。監査ではなく ledger が正本なのは「後方ジャンプを生き延びる」ため。append-only、**人間指示のジャンプはカウントしない**、resume 時のカウントは常に ledger のエントリ数。
- **ジャンプは必ずエンジン経由**: `next --stage code-generation` → エンジンの print した `aidlc-jump.ts execute` コマンドを **verbatim** 実行（手組み禁止）。`STAGE_JUMPED` は全先行レビューレシートを無効化し、per-unit ステージでは全 unit が current-attempt の terminal receipt を要する。
- **Plan Approval は生存**: 記録済み answer は権威のまま（loop-back のために `[Answer]:` を空欄に戻してはならない）。gated では人間の「Retry with fix」が re-approval。
- **halt-and-ask 2 変種**: impact-estimated 変種（Retry with fix / Accept failure / Abort ＋工数・コスト・リスク）と、候補 fix が無い場合の **no-fix 変種（Retry with fix を丸ごと省略）** — 「候補 fix なしの Retry 提示は捏造 fix と同罪」。プレースホルダでスロットを埋めることは禁止（E5）。
- **クラッシュ復旧**: ledger に planned fix があるのに対応する `STAGE_JUMPED` が無ければ「log と jump の間で死んだ」— 再診断せずジャンプを再実行。

## 8. 実装順序（D10 × domain-model-first）

1. **ドメイン例をユビキタス言語のテストとして書く**（例: 「gated ステージは awaiting-approval を経ずに completed にならない」「stale な再報告は状態を変えない」）。テスト名は 01 の正準用語を使う。
2. **Domain Primitive と `WorkflowExecution` 集約を TDD で実装**（§2 の表の E1/E2 を先に）。プロパティテスト（proptest）は `Verdict` 正規化・`EffectivePlan` 合成・`ProgressSignature` に適用。
3. **in-memory Gateway 一式**（StateFile / AuditLedger / StageGraph / Lock）でユースケーステストを回す。永続化・プロセスはまだ登場しない。
4. **ITF 準拠テストを接続**: `engine_loop.qnt` のトレース（`lastAction` 駆動）を domain 層のステップ関数に再生（ADR 0003 決定 5）。
5. **実 Gateway は最後**: workspace コンテキストの公開サービスに委譲し、ゴールデン互換層（upstream 実出力・実ワークスペース）で検証。

## 9. Quint ゲート実験 — 記録（試行条項）

- `engine_loop.qnt` slice 1（16 アクション・11 状態変数・不変条件 5 本）: `quint typecheck` 一発通過、`quint run --seed 固定 --max-samples 1000 --max-steps 30` で**違反 0**（349ms）。
- 到達性の反証チェック 4/4 成立（「完了に到達しない」「park に到達しない」「awaiting-approval に到達しない」「stale 再報告が起きない」という偽不変条件がすべて即座に反証される）— green が空回りでないことを確認済み。
- モデル化の副産物: I6（アクティブ高々 1）という upstream が明文化していない派生不変条件を顕在化。
- 未着手: skeleton stance / `gate:"unresolved"`、per-unit 反復、recompose 残り 4 ガード（slice 2）。`stop_hook.qnt` / `audit_lock.qnt` は次号。
- コスト実測: 契約抽出（3 エージェント並列 約 4.5 分）＋モデル執筆と検証（1 イテレーション）。ここまでの体感は「ゲートが実装を待たせる」より「仕様理解が先に進む」側。評価継続。

**第 2 回記録（同日、敵対的レビュー＋mutation テスト後）**:

- **素朴な green は検査力ゼロでありうる**ことが実証された。v1 の `no_run_stage_for_skip` はガードで違反状態が作れないだけの空回りで、チェックを外しても 10000 サンプル green。`stale_rereport_yields_done` は同一アクション内で両変数を代入する恒真式だった。忠実性バグも 1 件（jump forward の介在 Pending 素通し — 実トレースで upstream との乖離を確認）。
- v2 で修正: 状態レベルの `cursor_in_scope`、prev 状態スナップショットによる `stale_rereport_frame` / `unpark_restores_position` / `gate_lifecycle_preconditions` を追加し、9 不変条件で green（2000×40）。**mutation テスト 3/3**（jump スコープガード除去 → `cursor_in_scope`、stale への状態コミット注入 → `stale_rereport_frame`、gate-start 前提除去 → `gate_lifecycle_preconditions` がそれぞれ検出）。等価ミュータント（到達不能な単一ガード除去）の存在も確認 — mutation は意味のある変異を選ぶ必要がある。
- この学びを ADR 0003 の DoD に反映済み: **named invariant ごとに mutation テストで検査力を証明することを必須化**。

**第 3 回記録（同日、stop_hook.qnt — 第一陣完了）**:

- `stop_hook.qnt` v1: 8 不変条件 green（5000×60）＋ **mutation 7/7**（cap 除去・記録未書込・reset 不履行・carve 順崩し・署名継続・parked ガード除去・seed-2 破壊が各々狙いの不変条件で検出）。到達性 witness 6 本のうち深い経路（autonomous cap 8 の連続 block）はランダムウォークで到達不能だったため、**決定的シナリオ（`run r_*` ＋ `quint test`）を検査手段に追加** — witness の負形式実行と使い分ける（浅い経路は負形式、深い経路はシナリオ）。
- モデル化の副産物: seed-2 経路（記録なし＆stop_hook_active）が到達不能なことから、「fresh-session handoff が耐久カウンタをクリアし、payload 信号は独立に残る」という環境遷移の存在が要件として顕在化した（envHandoffReset として追加）。
- **第一陣 3/3 完了**（engine_loop v2 / audit_lock v2 / stop_hook v1）。試行条項の総括: 3 モデルとも「モデル執筆 → green → 敵対的レビュー/mutation で穴の実証 → 是正」のループが本物の欠陥（忠実性バグ 1・空回り/恒真 3・経路盲目 4・要件顕在化 2）を実装前に検出した。コストは 1 モデルあたり抽出込みで小さく、**マージゲートは第二陣にも継続適用する**（正式評価として ADR 0003 に記録）。

## 10. 実装ノート — sibling 合成の扱い（2026-08-22 オーナー裁定）

upstream の engine は変異を sibling CLI への subprocess spawn で合成するが、これは**実装**であり仕様ではない（00-policy §2 の判定原則）。維持必須の**仕様**は次の 4 点:

| # | 仕様（観測可能な契約） | Rust 実装での守り方 |
| --- | --- | --- |
| S1 | 遷移の意味論は単一で、どの経路から起きても同じ状態ファイルのバイト列・同じ監査行列を生む（エンジンは遷移ロジックを別実装しない） | `WorkflowExecution` 集約メソッドが唯一の遷移実装。CLI ラッパもエンジンのユースケースも同じ集約を呼ぶ |
| S2 | 各遷移は原子的（withAuditLock 区間で audit-first。クラッシュしても state と監査が食い違わない） | 集約トランザクションがロックと audit-first を所有。panic/exit 経路は A4 の規約と `audit_lock.qnt` が受け皿 |
| S3 | 非エンジンからの 11 動詞直接呼び出しは拒否＋逐語拒否文言。`AIDLC_ALLOW_DIRECT_STATE_TRANSITIONS=1` バイパスと `AIDLC_STATE_TRANSITION_OWNER` マーカーの受理意味論も維持（D6 の env 互換） | 11 動詞の CLI エントリにガード意味論を丸ごと実装（マーカー受理・bypass env・拒否文言込み）。in-process のエンジン経路は CLI エントリを通らないので正当に素通り |
| S4 | 拒否・エラー文言は verbatim。`Transition rejected by aidlc-state.ts <sub> for "<slug>": <stderr>` の書式ごと維持 | 文言カタログ（A3）＋ Presenter が同一書式で描画 |

その上で Rust 実装は **in-process 合成**を採る（subprocess の毎遷移プロセス起動コストとエラー伝搬の複雑さを除く）。仕様への準拠はゴールデン互換層で証明する（同じ入力に対して同じ state バイト列・同じ監査行・同じ stdout / stderr / exit code）。仕様を破っていないため逸脱台帳には載せない。

**M12 の裁定（2026-08-22 オーナー決定）**: upstream 既知バグ M12（birth が `Construction Autonomy Mode` 行を書かず、`setFieldStrict` を使う set-autonomy が新規 state ファイルで必ず `State update failed: Field not found …` で失敗する — upstream 03 の文書化済み discrepancy）は**修正する**。birth で行を書く（または挿入つき書込で修復する）。これは仕様レベルの逸脱（分類: バグ修正）として逸脱台帳 #2 に記録し、ゴールデン互換テストはこの 1 点のみ期待値を分岐させる。

## 11. 未決事項

- engine のターンコンテキスト解決（並行セッション時）— ADR 0004 の宿題。ここで確定する。
- load-steering チャンク配送の詳細規範化（材料は `research/orchestration-next-ladder.md` §4 に収録済み — slice 1 の §2.2 `ContinueToken` と I12 が骨子は既定）。
- 第二陣 Quint モデル（`bolt.qnt` — 三層 fork の順序と失敗リカバリ、`swarm_convergence.qnt` — 6 段ガードと「行なし」中間状態からのサーガ復旧）。成立後、§7 の不変条件に E4 を付与する。
- wave の並列実行と Stop フック / センサーの相互作用の精密化。
