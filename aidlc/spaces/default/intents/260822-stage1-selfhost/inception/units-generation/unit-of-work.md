# unit-of-work — stage-1 の Unit 定義（10 Unit）

> Units Generation（Inception 2.7）成果物。出典: `../domain-design/components.md`（11 コンポーネント）、
> `../domain-design/decisions.md`（ADR-001〜007。ADR-005 は 2026-08-22 再入で完全移動へ改訂）、
> `../requirements-analysis/requirements.md`（FR1〜FR9 / NFR1〜NFR5、2026-08-22 改訂版）、
> `../practices-discovery/team-practices.md`（Bolt = PR、直列運用、squash-merge）、確認質問
> `units-generation-questions.md`（Q1〜Q9a 回答済み・Looks correct 確認済み・分割計画 Approve Plan）。
> `user-stories` は Skip（`../user-stories/user-stories-assessment.md`）のため、ストーリーの代わりに
> **FR → Unit** でトレースする（`unit-of-work-story-map.md`）。
>
> 本ファイルは Unit の**定義と境界**だけを扱う。Unit 間の依存の形は `unit-of-work-dependency.md`、
> どの Unit から着手するか（Bolt の順序・クリティカルパス）は次の delivery-planning（2.9）が決める。

## 1. 分割方針（確認質問の裁定）

- **境界**: ハイブリッド（Q1 = C）。縦串（ユースケース・CLI・フック・doctor）は FR 起点、横断の基盤
  （ES ストア・投影・canon-json）と非コード作業（docs 正本修正・CI）は作業種別起点で独立 Unit。
- **粒度**: 中（Q2 = B）。1 Unit = 1 Bolt = 1 PR（数時間〜1 日、直列運用）。
- **非コード作業**は独立 Unit（Q3 = A）: FR8 docs = `spec`、FR9 CI/ガバナンス = `packaging`。
- **FR7**（canon-json + 0b ゴールデン採取）は基盤 Unit 1 つ（Q4 = A）。
- **ES 基盤**は書く側と描く側で 2 Unit（Q5 = A）。
- **kind** は役割ごとに使い分け（Q8 = A）: コードの Unit は `library`、CLI バイナリ+フック+doctor は
  `service`、docs 正本修正は `spec`、CI/ガバナンスは `packaging`。
- **デプロイモデル**: 本プロジェクトは単一 CLI バイナリ `aidlc`（配布は別 intent）。コードの Unit はすべて
  その 1 バイナリに **embedded**（独立デプロイなし）。`spec` / `packaging` の Unit は成果物がリポジトリ内の
  文書・CI 設定であり、デプロイ対象を持たない（`standalone`）。

## 2. Unit 一覧

| Unit ID | Directory | Unit 名 | kind | 規模 | デプロイ | 主担当 FR / NFR |
|---|---|---|---|---|---|---|
| U1 | `u1-canon-json-goldens` | canon-json とゴールデン採取 | library | M | embedded | FR7.1, FR7.2, FR7.3, NFR1（正準化面） |
| U2 | `u2-domain-es-core` | ドメイン ES コア（集約 FSM・ドメインイベント・PlanAction 完全移動） | library | L | embedded | FR8.3, FR8.4（+ FR1/FR2/FR3 の土台） |
| U3 | `u3-event-store-repository` | SQLite EventStore と WorkflowExecutionRepository | library | L | embedded | FR1.2, FR1.3, NFR3（書く側） |
| U4 | `u4-read-model-updater` | ReadModelUpdater（状態ファイル・監査シャード投影） | library | M | embedded | FR1.1, NFR3（描く側）, FR5.4 の投影側 |
| U5 | `u5-report-use-case` | report ユースケース | library | M | embedded | FR2.1, FR2.2 |
| U6 | `u6-next-continue-use-case` | next / continue ユースケース | library | L | embedded | FR3.1, FR3.2, FR3.3 |
| U7 | `u7-cli-dispatcher-hooks` | CLI ディスパッチャ・文言配線・フック 4 本 | service | L | embedded（`aidlc` バイナリ本体） | FR4.1, FR4.2, FR5.1, FR5.2, FR5.3, FR5.4, NFR1（CLI 面） |
| U8 | `u8-doctor-dogfood` | doctor とドッグフード（実地スモーク） | service | M | embedded | FR6.1, FR6.2 |
| U9 | `u9-canon-docs` | 正本・仕様の canon 追従 | spec | S | standalone（文書） | FR8.1, FR8.2, FR9.6 |
| U10 | `u10-ci-governance` | CI・ガバナンス整備 | packaging | M | standalone（CI 設定） | FR9.1, FR9.2, FR9.3, FR9.4, FR9.5, NFR2, NFR4 |

規模は相対値（S < M < L < XL）。XL は無い — 粒度 Q2 = B の帯（7〜10 Unit）に収めるため、U7 は CLI 本体と
フック 4 本を 1 Unit にまとめたが（フックはサブコマンド — CliDispatcher の behaviour）、L の上限とみなす。

## 3. Unit 定義

### U1 — `u1-canon-json-goldens`（library, M）

- **責務**: `canon-json` クレート（upstream 互換の正準 JSON 直列化 + sha256 — `components.md` の
  CanonJson）と、0b の正解データ採取（upstream ツールを bun で実行し、hash-canonical 受入表と CLI
  実行出力ゴールデン = stdout JSON・状態ファイル差分・監査行 をコミット）。
- **境界**: 入力は upstream ピン `3c3146cf` の実行結果のみ。他 Unit のコードに依存しない（依存ゼロの
  純粋部品）。ゴールデンは `tests/` 配下の固定資産として他 Unit の受入に使われる。
- **合格**: FR7.1 受入表の全行一致（FR7.3）、FR7.2 ゴールデンのコミット。
- **実装ノート**: 採取スクリプトは再現可能にする（A3 前提）。ゴールデンの upstream ピン更新は別 intent。

### U2 — `u2-domain-es-core`（library, L）

- **責務**: `core-domain` の `WorkflowExecution` を ES 形の FSM にする — ドメインイベント語彙
  （`WorkflowExecutionEvent`、コマンドと 1:1 の 11 変種程度）、decide（`&mut self` コマンドが単一イベントを返す）
  / `apply_event` 分離、~~`version` /~~ `seq_nr` 保持（`version` は**失効（2026-08-29 / ADR-010・Bolt B7）**: 楽観 version は集約の外へ — `RehydratedWorkflowExecution` が持ち回る）、`next_decision` クエリメソッド（ADR-002）、有効プラン
  畳み込みの集約メソッド化（FR8.4 / R2）、`PlanAction` の `workflow_definition` への**完全移動**（FR8.3 /
  ADR-005 改訂 — 再輸出なし、呼出側パスの一斉修正を同 Unit に含む）。
- **境界**: `core-domain` クレート内（orchestration / workflow_definition / workspace コンテキスト）。I/O なし・
  純粋・同期。Repository・ストア・投影は持たない（U3 / U4）。
- **合格**: FR8.3（`orchestration` に `PlanAction` の定義・再輸出が無く全参照が `workflow_definition::PlanAction`）、
  FR8.4（畳み込みが `WorkflowExecution` のメソッド、`WorkflowDefinition` にはグリッド照会のみ）、
  `engine_loop.qnt` ITF 準拠維持、既存ユニットテスト + PBT green。
- **実装ノート**: 1 コマンド 1 イベント（絶対）。typestate 不採用。`Eq` はドメイン同値で手実装
  （coding-rules/domain-equality）。フィールド private + アクセサ（field-visibility）。

### U3 — `u3-event-store-repository`（library, L）

- **責務**: ~~`core-interface-adapter`~~ に SQLite EventStore（journal / snapshot / checkpoint テーブル、
  `persist_event_and_snapshot` は同一 Tx + 楽観 version 条件付き書込）と `WorkflowExecutionRepositoryImpl`
  （store = イベント + スナップショット永続化、find_by_id = 最新スナップショット + seq_nr 以降 replay）を
  実装する。mkdir ロック機構（`FsWorkspaceLock` / `WorkspaceLock` / `LockProtocol` / `reap_eligible` /
  `OwnerStamp`）を退役し、`audit_lock.qnt` を「ジャーナル / スナップショット / version / チェックポイント協定」
  の検証モデルへ改訂する（ADR-007）。`InMemoryWorkflowExecutionRepository` を先に書く（gateway-taxonomy §6）。
  → **失効（2026-08-29 / Bolt B8）**: `core-interface-adapter` はコマンド側とクエリ側に分割された。
  `WorkflowExecutionRepositoryImpl` は **`core-command-interface-adapter`**（本 Unit の実体）が
  引き続き所有し、`JournalReaderImpl`（本行が当初含意していた読取側実装）はクエリ側
  **`core-query-read-model-updater`**（U4）へ移動済み（`crate-structure-proposal.md` §1、
  `construction/u4-read-model-updater/developer-report-1.md` §1）。
- **境界**: ポート trait（`WorkflowExecutionRepository`、EventStore 同形 trait）はユースケース層に置く
  （U5/U6 より先に本 Unit が定義する）。ドメイン型（イベント・集約）は U2 のものを使う。投影は持たない。
- **合格**: FR1.2（改訂版 `audit_lock.qnt` ITF 準拠）、FR1.3（store → find_by_id ラウンドトリップ）、
  クラッシュ再構成（ジャーナル → 集約）テスト（NFR3 の書く側）。
- **実装ノート**: `store` は ES 拡張語彙（ADR-006。正本注記は U9 FR8.1 が同梱）。逸脱台帳に「SQLite ファイル
  追加・ロック dir 非生成」を登録。Clock は機構モジュール（Gateway に数えない）。

### U4 — `u4-read-model-updater`（library, M）

- **責務**: ReadModelUpdater — チェックポイント以降のイベントをジャーナルから読み、`aidlc-state.md`（状態
  ファイル）と監査シャード `<record>/audit/<host>-<clone>.md`（1 ドメインイベント → upstream 監査行 N 行、
  86 語彙・見出し・フィールド順は逐語互換）へ投影し、チェックポイントを進める冪等な差分関数。単一ファイル
  原子性（tmp+rename）。**監査シャード横断の位置付き読取**（timestamp ソート + バッファ位置 tiebreak —
  FR1.1。domain-design レビュー Minor の読み側合流をここに置く）。~~`state_file_io` を投影ライタ部品へ転生。~~
  → **実施済み（2026-08-29 / Bolt B8）**: `state_file_io` は `workspace/state_file.rs` へ転生。
  `render_audit_block` / `state_writers`（11-workspace §2.3）・`AuditFieldKey` 等の Domain Primitive
  （11-workspace §2.2）・監査ブロック描画（`audit_block.rs`）・投影規則 12 変種（`projection.rs`）も
  本 Unit で実装済み。
- **境界**: ~~入力は U3 のジャーナル読取 API と U2 のイベント型。~~ → **失効（2026-08-29 / Bolt B8）**:
  `JournalReader` ポートとその実装 `JournalReaderImpl`（旧 U3 所有）は本 Unit へ移動した。入力は
  U2 のドメインイベント型と、U3（`core-command-interface-adapter`）が書き込む SQLite ジャーナル
  （同じ DB ファイルへの別接続）。**本 Unit は独立クレート `core-query-read-model-updater` として
  実装済み**であり、コマンド側の `core-command-use-case` / `core-command-interface-adapter` の
  `Cargo.toml` に一切現れない（相互独立が物理強制 — `crate-structure-proposal.md` §2）。書込先は
  upstream 互換ファイルのみ。常駐しない（コマンド末尾で同期実行 — 起動は U7）。
- **合格**: FR1.1（投影出力が 0a 逐語契約に一致）、NFR3（ジャーナル → 集約 → 投影の再生成、冪等性）。
  FR5.4（write-audit-log）の「監査行を描く」側はここ、フックの発火側は U7。
- **実装ノート**: 投影規則（イベント → 行）は contract-design で形式化（Q7 = B）。**embedded 表記の
  補足（2026-08-29 / Bolt B8）**: §2 表の「embedded」はデプロイ形態（`aidlc` バイナリへの静的リンク、
  独立プロセスなし）を指し、クレート境界の独立性を意味しない。本 Unit は Bolt B8 で独立クレート
  `core-query-read-model-updater` として実装され、U3 とは相互独立が物理強制されている。

### U5 — `u5-report-use-case`（library, M）

- **責務**: `report` ユースケース（approve / reject / revise / skip / awaiting-approval / resumed の遷移
  コミット）。典型形: find_by_id で再水和 → decide（1 イベント）→ store → 投影キャッチアップ起動。
  B10 述語（ゲート受理の最小前提）と verification モジュール最小面。
- **境界**: ユースケース層。ビジネスロジック禁止（判断は U2 の集約）。trait のみ依存（DIP）、静的束縛。
  ユースケース間呼出禁止。
- **合格**: FR2.1（0a 契約マップ一致 + `engine_loop` ITF 準拠）、FR2.2。
- **実装ノート**: テストは `XxxUseCase<InMemoryWorkflowExecutionRepository>` の素の値で組む。

### U6 — `u6-next-continue-use-case`（library, L）

- **責務**: `next` の 21 分岐ラダー（判断は U2 の `next_decision`、ユースケースはフロー制御のみ）、
  load-steering 分割配信と `continue_token`（U1 の正準 JSON + ハッシュ）、`continue` 動詞。I8（`next` は
  読み取り専用）は Repository 非注入 + `&WorkflowExecution` 参照渡しで型強制。
- **境界**: ユースケース層。`next` は書かない（Controller が U3 の find_by_id で載せて `&` で渡す）。
- **合格**: FR3.1（分岐網羅テスト green）、FR3.2（continue_token / continue）、FR3.3（`next_decision` が
  `WorkflowExecution` の `&self` クエリメソッドで、ユースケース層に判断ロジックが無いことのレビュー確認）。

### U7 — `u7-cli-dispatcher-hooks`（service, L）

- **責務**: マルチコールバイナリ `aidlc` — tokio（current_thread）の async main、ROUTES 表（逸脱台帳 #1 の
  綴り写像）、composition root（実物 / InMemory の結線はここだけ）、Presenter（directive JSON・逐語文言）、
  `message-catalog` の逐語文言配線（FR4.2）、コマンド末尾の ReadModelUpdater 起動、フック 4 本
  （Stop forwarding loop / HUMAN_TURN 記録 / state-transition guard / write-audit-log）のサブコマンド化
  （FR5、upstream の発火条件・出力・ブロック挙動互換）、`harness-claude` の配線データ。
- **境界**: app / harness 層。ユースケース（U5/U6）・Repository 実装（U3）・投影（U4）を結線するだけで
  ロジックを持たない。main.rs の配線部はカバレッジ除外対象（U10 FR9.5）。
- **合格**: FR4.1（0b CLI 実行出力ゴールデン一致）、FR4.2（LLM 分岐条件文言のバイト一致）、FR5.1〜5.4
  （0b ゴールデン一致 + 実地スモークでの実働）。
- **実装ノート**: `unsafe_code` forbid は workspace lints（U10）。`dyn` はユースケースに持ち込まない。

### U8 — `u8-doctor-dogfood`（service, M）

- **責務**: `--doctor` サブセット（stage-1 で必要な検査項目）と、DoD の実地スモーク（本リポジトリで bugfix
  相当の小 intent を 1 本、amadeus-ng バイナリをエンジンにして開始 → ゲート承認 → 完了まで通す。
  切替条件 1・2・3 の統合受入）、Issue #7 の close。
- **境界**: doctor は U7 のサブコマンドとして追加。ドッグフードはコードを増やさず U1〜U7 の統合を実証する
  工程（walking skeleton を別立てしない裁定の実体 — team.md）。
- **合格**: FR6.1（doctor green）、FR6.2（スモーク完走 + CI green）。

### U9 — `u9-canon-docs`（spec, S）

- **責務**: FR8.1 — `coding-rules/use-case-rules.md:38` の `repository.load()` → `find()`、
  `gateway-taxonomy.md` §4 の「load / save」散文修正、§2b への ES 拡張語彙 `store` の注記追加、§2 実例リスト
  からの旧称 `AuditLedgerRepository` 除去（requirements.md のレビュー所見どおり §2）。FR8.2 — 11 号 §2.3/§3
  ポート・供給面表、01 号 §3 集約候補表、10 号 §3「同上」、10/12 号の PlanAction・CheckboxState 所有一意化、
  12 号 §2.3/§5/§39 整合。FR9.6 — エラーハンドリング様式規則の文面起草（オーナー確認のうえ 1 ファイル追加）。
- **境界**: 文書のみ（`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`、`docs/specs/`）。コード変更なし。
- **合格**: 各修正がレビューで確認でき `coding-rules/README.md` の一覧と矛盾しない。

### U10 — `u10-ci-governance`（packaging, M）

- **責務**: FR9.1 `main` の branch protection（required checks: check / quint / coverage）、FR9.2 サプライ
  チェーン 4 件（`cargo audit` CI 追加 — tools/lint の独立 `Cargo.lock` 含む、`rust-toolchain.toml` 固定、
  `unsafe_code = "forbid"` の workspace lints 昇格、CI `permissions: contents: read`）、FR9.3 tools/lint の
  CI 3 ステップ、FR9.4 PBT シード固定とカバレッジ相対ゲート 0.5pp → 0.01、FR9.5 カバレッジ除外
  （`scripts/coverage.sh`、composition root のみ）。
- **境界**: `.github/workflows/ci.yml`、`Cargo.toml` workspace lints、`rust-toolchain.toml`、`scripts/`、
  GitHub 設定。プロダクトコードは触らない（`unsafe_code` 昇格でビルドが赤になるクレートがあれば U7 で直す）。
- **合格**: NFR2 / NFR4 の受入（CI 3 ジョブ green、audit clean、branch protection が `gh api` で確認できる）。

## 4. 横断事項

- **NFR1（upstream 互換）**: U1（正準化・ゴールデン）、U4（監査行・状態ファイル）、U7（CLI 語彙・文言）の
  3 Unit で検収する。主担当は最終の互換面である U7。
- **NFR5（性能）**: 数値目標なし（非目標の明示）— Unit 割当なし。
- **逸脱台帳**（`docs/specs/deviations.md`）への登録は U3（SQLite ファイル・ロック dir 非生成）と U4
  （互換ファイルはリードモデル）の Bolt で行う。
- **Walking skeleton**: 作らない（team.md）。全体疎通は U8 のドッグフードで実証する。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-22T09:51:42Z
**Iteration:** 1（advisory）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | unit-of-work.md U7・U8／components.md EngineUseCases | `components.md` の `EngineUseCases`（依存ゼロの独立クレート — Rationale 欄に「DIP の機械強制点（クレート分離 = E0432）」と明記）は behaviour に `NextUseCase / ReportUseCase / ContinueUseCase / DoctorUseCase / フック4ユースケース` を列挙しており、フック4本・doctor もユースケース層の一員としてクレート境界で分離される設計になっている。ところが `unit-of-work.md` U7 の責務は「フック 4 本（Stop forwarding loop / HUMAN_TURN 記録 / state-transition guard / write-audit-log）のサブコマンド化」を直接内包し、U7 の境界は「ユースケース（U5/U6）・Repository 実装（U3）・投影（U4）を結線するだけでロジックを持たない」と自己宣言している。U8 も「doctor は U7 のサブコマンドとして追加」とだけ書き、doctor 用ユースケースの所属先を示さない。結果として、フック4本と doctor のユースケース層コード（`EngineUseCases` が本来所有するはずの部分）がどの Unit のどのクレートに実装されるのか — U7（CLI クレート、ロジック無し宣言と矛盾）なのか、U5/U6（どちらも該当 FR を明記していない）に新設するのか — が本成果物内で確定していない。DIP のクレート分離強制点がフックと doctor に対して機能しなくなるリスクがあり、実装者が着手前に判断を迫られる。 | U7 の責務記述から「フック4本のロジック実装」を切り離し、フック4ユースケース・DoctorUseCase の実装先クレート（新設 Unit か、既存 U5/U6 への追加か）を明記する。U7 は composition root としての「サブコマンド登録・結線」のみに限定する。 |
| 2 | Major | unit-of-work-dependency.md §3 | ステージ定義（units-generation.md 冒頭 NOTE および Step 6 の NOTE）は「2.7 MUST NOT recommend an implementation order or identify a critical path — those are 2.9's economic-sequencing decisions」と明記しているが、`unit-of-work-dependency.md` §3 は「直列に並ぶ鎖: U2 → U3 → U4 → U5 → U7 → U8（最長の依存連鎖 — これは幾何であって推奨順ではない）」と、DAG 上の最長経路（＝重み無し critical path の定義そのもの）を名指しで書き出している。「推奨順ではない」という注記は付いているが、最長依存連鎖を明示すること自体が「クリティカルパスの特定」に該当し、2.9（delivery-planning）の経済的判断領分に踏み込んでいる。 | §3 から「直列に並ぶ鎖」の行を削除するか、「最長経路の長さ」ではなく「並列可能な組」の列挙のみに留める（Q6 = A が求めているのは並列機会の明示であり、最長鎖の特定ではない）。 |
| 3 | Minor | unit-of-work.md 全体 | `components.md` の 11 コンポーネントのうち `PublishedLanguage`・`InfraIo`・`HarnessClaude` の 3 つが、`unit-of-work.md`／`unit-of-work-dependency.md` のどこにも Unit 名として言及されていない（grep で該当 0 件）。`InfraIo` は「既存実装を維持」と明記されており新規作業なしと読めるが、`PublishedLanguage` は「R4 で Gateway 直書き分を移設」という未完了の移設作業を示唆しており（`R4` は `decisions.md` に存在しない識別子）、`HarnessClaude` はフック配線（FR5 と直結）の当事者であるにもかかわらず、この移設作業・配線更新がどの Unit の Bolt に属するか本成果物からは判定できない。 | `PublishedLanguage`（R4 移設作業の要否）と `HarnessClaude`（フック配線更新）を該当 Unit（例: U7）の責務に明示するか、「brownfield 既存実装のため本 intent のスコープ外」と明記する。 |
| 4 | Minor | unit-of-work.md §2 注記／U2・U7 | Q2 = B（粒度: 中、7〜10 Unit、1 Unit = 1 PR が数時間〜1 日）に対し、U2（FSM 再設計 + イベント語彙新設 + `PlanAction` 完全移動と全呼出側一斉修正）と U7（CLI dispatcher + composition root + Presenter + 文言配線 + フック4本）はいずれも L 規模で、成果物自身が「L の上限とみなす」と認めているが、それ以上の緩和策（サブタスク化・Bolt 内分割の指針など）は示されていない。両 Unit とも複数の独立した変更理由（FSM 契約変更と参照パス一斉修正／CLI 配線とフック実装）を1 Bolt に同居させており、直列 PR 運用・数時間〜1 日という team.md の制約に対する実現可能性の検証が本成果物内で完結していない。 | U2・U7 が実際に 1 Bolt/PR に収まるか、delivery-planning（2.9）または後続の Bolt 計画で早期に見積もりを取り、収まらない場合の分割方針（例: U2 を「FSM/イベント語彙」と「PlanAction 完全移動」に分ける）を検討する。 |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `aidlc-sensor-required-sections`（`unit-of-work-dependency.md`） | `pass:true`, `edge_block:"ok"`, `h2_count:5` | 機械可読 `yaml` 辺ブロックは整形済み・非循環。手動でも DAG を追跡し循環なしを確認（U1・U2・U9・U10 が根、U8→U7→{U1,U4,U5,U6}→…→U2 まで戻り辺なし） |
| `aidlc-sensor-traceability`（`traceability.json`） | `pass:false`, `findings_count:81`（全件 GAP + invalid_targets） | ダイスパッチブリーフの事前注記どおり、upstream センサーが `story-map` の行を `USx.y` 形式でしか認識できず、FR 直結行を誤検知しているだけ（stories.md 不在パスに未対応の既知の限界）。手動突合の結果、`requirements.md` の FR/NFR **43 ID**（親 FR1〜FR9・NFR1〜NFR5 + 子 38 件）が `traceability.json` `upstream_ids` と過不足なく一致し、各 `OK`/`N/A` の `target`（U1〜U10）は `unit-of-work-story-map.md` §1/§2 の該当行と一致することを確認した。誤検知として所見に数えない |
| ID 突合（requirements.md ↔ unit-of-work.md ↔ story-map ↔ traceability.json、手動） | 一致 | 43 ID すべてに Unit 割当があり、未割当の FR/NFR は無い（NFR5 のみ意図的に N/A）。Unit 側も U1〜U10 すべてが最低 1 件の FR/NFR を持つ |
| コンポーネント突合（components.md 11 個 ↔ unit-of-work.md、手動） | 不一致（8/11） | `OrchestrationEngine`／`WorkflowDefinitionModel`／`WorkspaceModel`（→ U2 の「core-domain クレート」表現で間接網羅）、`PersistenceGateways`（→ U3）、`ReadModelUpdater`（→ U4）、`CliDispatcher`（→ U7）、`CanonJson`（→ U1）は追跡できたが、`PublishedLanguage`／`InfraIo`／`HarnessClaude` の 3 個は Unit 名として一度も出現しない（所見 3） |
| Q7（A, B, C, D）と §4 統合点表の突合 | 一致 | ポート trait／ドメインイベント語彙と投影規則／SQLite スキーマ／CLI 動詞・directive JSON・フック入出力 の 4 行が Q7 回答と 1:1 対応 |

### Summary

DAG は機械的に非循環でトレーサビリティの ID 突合も 43 件すべて解決しており、基礎となる `FR → Unit` の割当漏れは無い。ただし、フック4本・doctor のユースケース層コードの帰属が `components.md` の `EngineUseCases`（クレート分離による DIP 強制点）と `unit-of-work.md` の U7（「ロジックを持たない」自己宣言）の間で整合しておらず（所見1）、また `unit-of-work-dependency.md` §3 がステージ定義の明示的な禁則（クリティカルパスの特定禁止）に抵触する「最長依存連鎖」を書き出している（所見2）。この2件は Major だが、advisory 判定の閾値（≤2 Major）内であり、DAG・トレーサビリティという構造的な健全性は保たれているため READY とする。承認前にこの2件の Major と、コンポーネント帰属漏れ・粒度リスクの Minor 2件を人間が重みづけされたい。
