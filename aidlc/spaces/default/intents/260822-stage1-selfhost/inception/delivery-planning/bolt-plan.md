# bolt-plan — Bolt の順序（10 Bolt、直列）

> Delivery Planning（Inception 2.9）成果物。出典: `../units-generation/unit-of-work.md`（10 Unit）、
> `../units-generation/unit-of-work-dependency.md`（依存 DAG）、`../units-generation/unit-of-work-story-map.md`
> （FR → Unit）、`../contract-design/contract-summary.md`（契約と未解決項目）、`../domain-design/components.md`、
> `../requirements-analysis/requirements.md`（DoD）、`../practices-discovery/team-practices.md`（Bolt = PR・直列・
> squash-merge・skeleton: off・TDD・CI 3 ジョブ）、確認質問 `delivery-planning-questions.md`（Q1〜Q8 回答済み・
> Looks correct）。user-stories / mockups / team-formation は Skip のため参照なし。
>
> **Bolt** = 1 つの Unit を構築フェーズ（設計 → 実装 → テスト）に 1 回通す作業の単位。本プロジェクトでは
> **1 Bolt = 1 Unit = 1 PR**（`main` へ squash-merge、コミット名 = Bolt slug、オープン PR は常に 1 本）。

## 1. 方針（確認質問の裁定）

- 着手方針: **土台先行 + リスク早出し**（Q1 = A）。点数モデルは使わない（Q2 = A）。
- Quint（状態機械の形式検証）は毎 PR の受入ゲート（`scripts/quint-gate.sh`）として維持し、意味論を変える
  U2・U3 の Bolt にモデル改訂を同梱する（Q2a = A）。
- 1 Bolt = 1 Unit、直列のみ（Q3 = A / Q4 = A）。外部依存は実質なし（Q5 = A）。
- 構築の回し方は **unit-major**（Unit ごとに設計 → 実装を完結。Q7 = A）。承認は**毎 Bolt でゲート**
  （Q8 = A）。**Walking skeleton は作らない**（team.md: skeleton: off。全体疎通は B10 のドッグフードで実証）。
- ブランチ運用: base = `main`、target = `main`、squash-merge（team.md Way of Working）。

## 2. Bolt 順序

> **改訂 2026-08-22 UTC（U1 code-generation 完了時、オーナー裁定）**: ワークフローエンジンは Unit を依存バッチ順（u1 → u10 → u2 → u9 → …）で歩くため、
> 旧 B6 = U10 を **B2** に前倒しし、旧 B2〜B5 を B3〜B6 へ繰り下げた。依存列と後続 Bolt 番号は連動して振り直した。根拠:
> U10 は他 Unit に依存せず、CI の機械強制を早く入れるほど以降の Bolt が守られる。`team-allocation.md` /
> `risk-and-sequencing-rationale.md` / `external-dependency-map.md` の B 番号は旧番号のまま（U 名で読み替える）。

| Bolt | Unit | kind | 規模 | 依存（満たされる Bolt） | ねらい（順序の理由） |
|---|---|---|---|---|---|
| B1 | U1 `u1-canon-json-goldens` | library | M | なし | 互換の正解データ（ゴールデン）を最初に確保し、以降の全 Bolt の TDD のオラクルにする（心配 B） |
| B2 | U10 `u10-ci-governance` | packaging | M | なし | 【2026-08-23 改訂】toolchain 固定・forbid 昇格・audit・カバレッジ除外・branch protection を B1 直後に入れ、以降の全 Bolt の PR を機械強制下に置く（エンジンの依存バッチ順 u1 → u10 → u2 → u9 をオーナーが受け入れ、旧 B6 を前倒し） |
| B3 | U2 `u2-domain-es-core` | library | L | なし | 最大の設計リスク（ES 化・FSM・PlanAction 完全移動）を早く潰す（心配 A）。Quint `engine_loop` 維持 |
| B4 | U9 `u9-canon-docs` | spec | S | なし | U3 着手前に正本（`store` 注記・旧称除去）を直す。短い |
| B5 | U3 `u3-event-store-repository` | library | L | B3 | ストア + ロック退役 + `audit_lock.qnt` 改訂（心配 A）。B3 の直後で設計の連続性を保つ |
| B6 | U4 `u4-read-model-updater` | library | M | B3, B5 | 投影の upstream 互換を B1 のゴールデンで早期に実証（心配 B） |
| B7 | U5 `u5-report-use-case` | library | M | B3, B5, B6 | 最初のユースケース（書く側）。再水和 → decide → store → 投影の定型を確立 |
| B8 | U6 `u6-next-continue-use-case` | library | L | B1, B3, B5 | 21 分岐ラダー + continue_token。I8（読取専用）の型強制 |
| B9 | U7 `u7-cli-dispatcher-hooks` | service | L | B1, B6, B7, B8 | バイナリとして初めて動く。フック 4 本の実機動作（心配 C）をここで確認 |
| B10 | U8 `u8-doctor-dogfood` | service | M | B9 | doctor + 実地スモーク（DoD）。全体疎通の実証（心配 D）— walking skeleton の代わり |

依存 DAG（`unit-of-work-dependency.md`）のすべての辺を満たす（各 Bolt の依存 Unit はそれより前の Bolt）。
トポロジカル順からの逸脱はない。根（U1/U2/U9/U10）の並べ方が本ステージの判断で、理由は
`risk-and-sequencing-rationale.md`。

## 3. 各 Bolt の定義

共通の Definition of Done（全 Bolt）: ① 当該 Unit の合格基準（`unit-of-work.md` §3）を満たす。② TDD
（レイヤーごとに red-green-refactor）で書かれ、CI 3 ジョブ（check / quint / coverage）が green。③ PR が
オーナー承認のうえ `main` へ squash-merge 済み（コミット名 = Bolt slug）。④ coding-rules 正本に準拠。

### B1 — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

- **含む Unit**: U1。**walking skeleton**: いいえ。
- **Done の追加条件**: FR7.1 受入表の全行一致（FR7.3）、FR7.2 の CLI 実行出力・状態ファイル差分・監査行・
  フック入出力のゴールデンをコミット、再採取スクリプト同梱。
- **仮説（出荷して分かること）**: upstream の正準 JSON 仕様を Rust で再現できる（hash-canonical 全行一致）。
  ゴールデンの採取手順が再現可能で、以降の互換テストの正解として使える。
- **デモ**: `cargo test -p canon-json`（受入表 green）とゴールデン一覧。
### B2 — U10 CI・ガバナンス整備（`u10-ci-governance`）

- **含む Unit**: U10。**walking skeleton**: いいえ。
- **Done の追加条件**: FR9.1〜9.5（branch protection・サプライチェーン 4 件・tools/lint CI・PBT シード固定 +
  相対ゲート 0.01（実装時の実測で暫定 0.05 — Bolt B2 ゲートのオーナー裁定 2026-08-22 UTC、U3 のロック退役後に 0.01）・カバレッジ除外）、NFR2 / NFR4 の受入。
- **仮説**: 以降の PR（特に B9 の main.rs 配線）で CI が突然赤にならず、branch protection が機械強制される。
- **デモ**: `gh api` で required checks を確認、`cargo audit` clean、CI green。
### B3 — U2 ドメイン ES コア（`u2-domain-es-core`）

- **含む Unit**: U2。**walking skeleton**: いいえ。
- **Done の追加条件**: FR8.3（PlanAction 完全移動、再輸出なし）、FR8.4（畳み込みの集約メソッド化）、
  `WorkflowExecution` が decide / apply_event の ES 形 FSM（イベント 11 変種・version・seq_nr）、
  `engine_loop.qnt` の ITF 準拠維持、既存 PBT green、Quint モデル改訂（必要分）を同梱。
- **仮説**: 集約 = FSM + 1 コマンド 1 イベントの規律で 21 分岐ラダーと upstream 契約（engine_loop）を崩さずに
  ES 化できる。PlanAction 完全移動が 1 PR に収まる。
- **デモ**: ITF 準拠テスト + 集約ユニットテストの green、`orchestration` に PlanAction 定義が無いことの grep。
- **規模リスク**: L。着手時に見積もり、1 日を超えそうなら中断してオーナーと分割を相談（`risk-and-sequencing-rationale.md` §4）。
### B4 — U9 正本・仕様の canon 追従（`u9-canon-docs`）

- **含む Unit**: U9。**walking skeleton**: いいえ。
- **Done の追加条件**: FR8.1（`use-case-rules.md` load → find、`gateway-taxonomy.md` §4 散文、§2b の `store`
  注記、§2 実例リストの旧称除去）、FR8.2（11/01/10/12 号の canon 追従）、FR9.6（エラーハンドリング規則の
  文面をオーナー確認のうえ 1 ファイル追加）。
- **仮説**: 正本と仕様が B5 以降の実装レビューの基準として矛盾なく使える。
- **デモ**: diff レビュー。
### B5 — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

- **含む Unit**: U3。**walking skeleton**: いいえ。
- **Done の追加条件**: FR1.2（改訂版 `audit_lock.qnt` ITF 準拠）、FR1.3（store → find_by_id ラウンドトリップ、
  genesis 経路含む — contract-summary のレビュー所見）、InMemory 先行、mkdir ロック機構の退役、逸脱台帳に
  「SQLite ファイル追加・ロック dir 非生成」を登録、Quint `audit_lock.qnt` の改訂を同梱（Q2a）。
- **仮説**: SQLite Tx + 楽観 version で並行制御が 1 機構に集約でき、クラッシュ後にジャーナルから集約を
  再構成できる（NFR3 の書く側）。
- **デモ**: ラウンドトリップ・競合・クラッシュ再構成テスト green、Quint ゲート green。
- **規模リスク**: L（B3 と同じ扱い）。
### B6 — U4 ReadModelUpdater（`u4-read-model-updater`）

- **含む Unit**: U4。**walking skeleton**: いいえ。
- **Done の追加条件**: FR1.1（投影出力が 0a 逐語契約に一致、位置付き横断読取）、NFR3（ジャーナル → 集約 → 投影
  の再生成・冪等）、逸脱台帳に「互換ファイルはリードモデル」を登録。
- **仮説**: ドメインイベントから upstream 互換の監査行・状態ファイルをバイト一致で描ける（B1 のゴールデンで判定）。
- **デモ**: 投影テスト（イベント列 → 監査シャード・状態ファイル）のゴールデン突合 green。
### B7 — U5 report ユースケース（`u5-report-use-case`）

- **含む Unit**: U5。**walking skeleton**: いいえ。
- **Done の追加条件**: FR2.1（0a 契約マップ一致 + `engine_loop` ITF 準拠）、FR2.2。
- **仮説**: 再水和 → decide → store → 投影の定型が InMemory でテストでき、ゲート往復の契約が成立する。
- **デモ**: `ReportUseCase<InMemoryWorkflowExecutionRepository>` のテスト green。
### B8 — U6 next / continue ユースケース（`u6-next-continue-use-case`）

- **含む Unit**: U6。**walking skeleton**: いいえ。
- **Done の追加条件**: FR3.1（分岐網羅テスト）、FR3.2（load-steering 分割配信・continue_token・continue）、
  FR3.3（`next_decision` が `WorkflowExecution` のクエリメソッドでユースケース層に判断が無いことの確認）。
- **仮説**: `next` を Repository 非注入 + 参照渡しで読取専用に型強制できる。continue_token が B1 の正準化で
  安定する。
- **デモ**: 21 分岐のテーブルテスト green、continue_token のゴールデン一致。
### B9 — U7 CLI ディスパッチャ・文言配線・フック 4 本（`u7-cli-dispatcher-hooks`）

- **含む Unit**: U7。**walking skeleton**: いいえ（ただしここで初めてバイナリ全体が動く）。
- **Done の追加条件**: FR4.1（CLI 実行出力ゴールデン一致）、FR4.2（逐語文言のバイト一致）、FR5.1〜5.4
  （フック 4 本のゴールデン一致 + Claude Code 上での実働確認）。
- **仮説**: ROUTES の写像で upstream と同じ出力が得られ、フック 4 本が Claude Code のフック機構で
  upstream 同様に発火・ブロックする（心配 C）。
- **デモ**: 本リポジトリの `.claude/settings.json` をバイナリ呼出に切り替えた状態で `aidlc next` 等を手動実行。
### B10 — U8 doctor とドッグフード（`u8-doctor-dogfood`）

- **含む Unit**: U8。**walking skeleton**: いいえ（代替としての全体疎通実証）。
- **Done の追加条件**: FR6.1（doctor green）、FR6.2（本リポジトリで bugfix 相当の小 intent を 1 本、
  amadeus-ng バイナリをエンジンに開始 → ゲート承認 → 完了、CI green、Issue #7 close）。
- **仮説**: stage-1（セルフホスト切替）が成立する — 切替 5 条件（00-policy §4）の統合受入。
- **デモ**: 実地スモークのセッション記録（監査シャード・状態ファイル）。

## 4. 構築フェーズの運用

- **反復**: unit-major — 各 Bolt で functional-design → nfr-requirements → nfr-design → code-generation を
  その Unit について完結させ、build-and-test / ci-pipeline は全 Bolt 終了後に 1 回（ステージ定義どおり）。
  infrastructure-design と OPERATION はスコープ外（SKIP 済み）。
- **承認**: 毎 Bolt でゲート（gated）。自律スウォームは使わない。
- **Bolt 着手時の見積もり**: L 規模（B2・B4・B8・B9）は functional-design 後に PR 規模を見積もり、
  1 日超が見込まれれば中断してオーナーと分割を相談する（`risk-and-sequencing-rationale.md` §4）。
