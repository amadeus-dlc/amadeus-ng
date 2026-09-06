# security-design — U2 ドメイン ES コア（`u2-domain-es-core`）

> NFR Design（Construction 3.3）成果物（Unit: U2、kind: library）。**2026-09-07 再走（Modify）** — 2026-08-23 の初版（旧世界:
> `WorkflowExecution`・12 変種・`from_snapshot` / `SnapshotError` / 公開 `ApplyError`・「どの検査も panic なし」・`next_decision` の
> `DefinitionMismatch`・`core-domain` 配置）を、2026-09-05 是正・2026-09-07 再走後の機能設計と現行コード `modules/core/command/domain/`
> へ同期した。旧 READY レビュー節（2026-08-23、Major 2 / Minor 2）と `pending-revision.md` は
> `security-design-review-history-2026-08-23.md` へ逐語退避した（質問票 P11）。
>
> 出典: `../nfr-requirements/security-requirements.md`（NFR1.1〜1.3 / NFR2.1〜2.5 / NFR3.1〜3.4 / NFR4.1〜4.5、STRIDE、データ分類、
> 末尾レビュー R-01〜R-08）、`../nfr-requirements/tech-stack-decisions.md`（依存追加なし、FCC、定義の識別子、PBT / ITF、契約試験
> ハーネス、手実装エラー型）、`../functional-design/functional-spec.md`（§2 API、W1〜W7、§4〜§5、§9 引継ぎ、末尾レビュー R-01〜R-10）、
> `../functional-design/rules.md`（BR1.0〜BR1.9 / BR2.1〜BR2.6 / BR3.x / BR5.1〜BR5.5）、`../functional-design/entities.md`、
> `../../../inception/contract-design/contract-summary.md`（C3 / C5 / C6）、`../../../inception/domain-design/decisions.md`（ADR-001〜010）、
> 確認事項 `nfr-design-questions.md`（前提 P5〜P11、Looks correct）。performance / scalability / reliability / observability の要求・
> 設計は kind = library のため存在しない。
>
> 設計ステージの制約に従い、コードは ≤15 行の例示のみ。用語: **FCC** = ファーストクラスコレクション（配列を不変条件と操作を持つ
> 専用型で包んだもの）、**DTO** = アダプタ層が保存・復元に使う写し、**RMU** = read-model-updater。

## 1. 設計方針

U2 は I/O・認証・認可・永続化・ログを持たない純粋な集約（`Intent` と `IntentExecution`）。セキュリティ設計は 4 点に絞る:
**(a) 検査点を二層に分ける** — 境界の検査付き変換とコマンド入口のガードは Err（状態不変）、型変換後の壊れた歴史は panic
（オーナー裁定 2026-08-30）; **(b) 集約間は ID で参照し取り違えを拒否する** — `IntentMismatch` と `DefinitionMismatch`;
**(c) 配列を FCC に閉じ込める** — 不変条件は構築時に検査し、境界では `fold_left` で平坦に写す（BR5.5、Q4 / Q4a）;
**(d) 境界を薄く保つ** — 依存追加なし・serde なし・人間入力は素通し・時計は `*EventId::generate` のみ。

## 2. 検査点の二層（NFR3.2 / NFR4.3 / NFR1.1）

| 層 | 検査点 | 何を検査するか | 違反の返し方 | 出典 |
|---|---|---|---|---|
| (1) Err | `Intent::create` | scope の妥当性（`UnknownScope`）、計画の非空・initialization の EXECUTE / 無条件・slug 重複・表示名の単一行 | `Err(IntentError::{UnknownScope, Empty, InitializationMustExecute, InitializationMustBeUnconditional, DuplicateSlug, StageDisplayNotSingleLine})` — 集約は生成されない | BR2.2、functional-spec W1 |
| (1) Err | `IntentExecution::new`（DTO → 集約基底、`intent_execution.rs:283`） | 空計画、列の長さ = stage_count、cursor < stage_count、parked_at = cursor（park 中）、seq_nr ≥ 1、slug 重複、適用後の状態不変条件（cursor in-scope / active ≤ 1 / gated Completed ⇒ approved） | `Err(IntentExecutionError)`（理由文字列を持つ 1 型）。U3 が封筒・通番・`aggregate_id` の不整合とともに `RepositoryError::Corrupt` へ写す（C3） | BR5.2、NFR3.2 (1)、C6 |
| (1) Err | FCC の構築（BR5.5 新設型） | `StageEntries`: 非空・slug 一意・文書順; `StageSlots`: 長さ = stage_count; `StageIndexSet`: 範囲内; `StageSlugSet`: 辞書順・重複なし; `PromotedSections`: 見出し一意; `combine` / `map` の衝突 | 型ごとの手実装 Error（enum + `Display` + `std::error::Error`）の Err。集合型の `combine` / `divide` は全域（Err なし） | BR5.5、first-class-collections |
| (1) Err | decide（16 コマンド）と書込前ガード（`jump_resolve` / `stale_report`） | `matches(intent)`（`IntentMismatch`）、`accepts_commands`（`NotRunning`）、checkbox 前提（`CheckboxPrecondition`）、対象の妥当性（`NotSkippable` / `InvalidTarget` / `UnknownStage`）、autonomy と人間の存在（`RefusedUnderAutonomy` / `HumanPresenceRequired`）、レビュー会計（`NoDeclaredReviewer` / `ReviewerMismatch` / `ReviewBudgetExceeded` / `ReviewOutOfSequence` / `NoPendingReview` / `ReviewReceiptMissing` / `PracticesReceiptMissing`）、`stale_report` の staleness（`NotStale`）。報告適用は `ReportRefusal`（13 変種）/ `ReportCommitError`、隔離実行は `SingleStageRunRefusal`、skeleton 立場は `SkeletonStanceRefusal` | `Err(CommandError::..)` ほか — ガード不成立では `self` に触れない（BR1.1） | BR1.x、functional-spec W2 / W5 / W6、§5 |
| (1) Err | `next_decision(&Intent, &NextRequest)` | `matches(intent)` | `Err(CommandError::IntentMismatch)`（Q5 = A。現行 `:1897` は `NextDecision` を直接返しており code-generation で Result 化） | BR2.6 / BR3.1、NFR3.4 |
| (2) panic | `replay(snapshot, delta)` / `apply_event(seq_nr, t, &event)`（`:385` / `:1542`） | 通番 = 現在値 + 1、ペイロードのステージが `slots` に存在、適用後の不変条件 | **panic**（回復しない）。`# Panics` を明記し `missing_panics_doc` 緑。`ApplyError` は `pub(crate)` で公開しない | オーナー裁定 2026-08-30、BR2.1 / BR2.3、NFR3.2 (2) |

- **panic の射程は (2) だけ**（NFR4.3）: 公開位置は `StageIndex`（構築は `stage_index(usize) -> Option<StageIndex>` と DTO 境界の検査付き `new`）で
  型保証し、FCC の添字アクセスは `at` の `Option` を通す。`unwrap` / `expect` はプロダクトコードで禁止（workspace lint）。`# Panics` を持つ
  公開 API は `replay` / `apply_event` に限る（`intent_execution.rs:35` の旧 doc「`# Panics` を持つ公開 API は無い」は実態と乖離しており、
  §9 #4 で修正する）。
- **1 コマンド 1 イベント・Err は無副作用**（BR1.1）: decide はガードをすべて通した後にイベントを構築し、`apply_event` を経て自身に適用する。
- **`stale_report` は第 5 のガード**: 旧版で decide 行に紛れていた `NotStale` は `stale_report(StageIndex)`（書込なしの受理ガード、BR1.9）
  が返す。

```text
// 検査点の形（例示）
fn new(id, intent_id, slots: StageSlots, cursor, status, ..) -> Result<IntentExecution, IntentExecutionError>; // 層 (1)
fn approve_gate(&mut self, intent: &Intent, policy: Option<&ReviewPolicy>, user_input, t)
    -> Result<IntentExecutionEvent, CommandError>;          // matches → ガード → イベント構築 → apply_event
fn next_decision(&self, intent: &Intent, req: &NextRequest) -> Result<NextDecision, CommandError>; // IntentMismatch
fn apply_event(&mut self, seq_nr: usize, t: DateTime<Utc>, event: &IntentExecutionEvent); // 層 (2): # Panics
fn replay(snapshot: IntentExecution, delta: impl IntoIterator<Item = (usize, DateTime<Utc>, IntentExecutionEvent)>)
    -> IntentExecution;                                     // 層 (2): # Panics
```

## 3. 集約参照の照合と来歴（NFR3.4 / BR2.6 / ADR-008）

- `Intent` は `definition_id: WorkflowDefinitionId`（系譜 ID）と `definition_revision: DefinitionRevision`（内容版、`CompiledDefinition` が
  `of_content` で導出 — ADR-008 改訂 2026-09-02）を `Created` に記録して来歴として保持する。revision の計算（canon-json）はアダプタ層で、
  ドメインは値を運ぶだけ。
- `IntentExecution` は `intent_id` のみを持ち、`&Intent` を受ける全コマンド・書込前ガード・`next_decision` が `matches(intent)`
  （`intent_execution.rs:417`）で照合して `CommandError::IntentMismatch` を返す。
- 定義 ID の照合は `Intent::resolve_review_policy(&WorkflowDefinition, &StageSlug)` が担い、不一致は `IntentReviewError::DefinitionMismatch`
  （現行実装名。質問票 P10 の「LineageMismatch」は誤記で、`LineageMismatch` は `workflow_definition` 文脈の `redefine` 用エラー）。
  `next_decision` は定義を参照しないので旧世界の `DefinitionMismatch` は失効。
- revision の差（ピン更新）は Err にしない — 計画は `Started` に自己完結し、upstream も dist 更新をまたいでワークフローを続ける。drift は
  `definition_revision()` アクセサで U4 / U7 が観測する。
- ITF 準拠テストは合成の `Intent`（固定 ID・合成計画）で集約を作る（BR2.5）。

## 4. ペイロードと情報の扱い（NFR4.4 / NFR3.1）

- 人間入力（`request` / `user_input` / `feedback` / `reason` / 成果物パス / 規則行）は `String` / `Option<String>` / 文字列 FCC
  （`ArtifactPaths` / `RuleLines`）の素通し。集約は内容を解釈・検証・切詰め・要約せず、順序と重複規則以外の加工をしない。`Display` 実装
  （エラー型）は材料（ID・索引・状態）だけを出し、人間入力を埋め込まない（文言はアダプタ層）。
- 集約は環境変数・乱数・ログ基盤を持たない。時計を読むのは `*EventId::generate`（`Uuid::now_v7`、4 型）だけで、`occurred_at` は封筒値として
  呼出側から受け取る。`core-command-domain` の `src/` に `std::time` / `std::env` / `rand` の利用が無いことをレビュー項目にする（NFR3.1、
  2026-09-06 実測 0 件）。
- 秘密情報・トークンを載せる経路は設けない（イベント型に資格情報のフィールドは無い）。

## 5. サプライチェーンと境界（NFR4.1 / NFR4.2 / NFR4.5）

- `core-command-domain` の依存はベースライン（runtime = `chrono` / `uuid` / `core-infrastructure`、dev = `proptest` / `serde_json`）から
  増やさない。FCC 化・`next_decision` 改修で外部クレートを足さず、`Cargo.lock` 不変が期待値。serde / canon-json はドメインに入れない
  （JSON 化は U3 の DTO、revision 計算はアダプタ層）。
- `unsafe_code = "forbid"`（workspace lint — U10 で昇格・実証済み）。
- デシリアライズ面を持たない — 外部バイト列はドメインに届かず、parse-don't-validate は U3 の DTO 境界。ドメインは型で受け取った値に
  §2 (1) の検査（`new`、FCC の構築検査）だけを適用する。FCC は境界で `fold_left` により平坦な表現へ写し、DTO のバイト表現は変えない。

## 6. 決定性と契約の維持（NFR1.1 / NFR1.2 / NFR1.3 / NFR2.2 / NFR2.5 / NFR3.1）

- decide / apply は純関数的（同じ状態 + 同じコマンド → 同じイベントと次状態。イベント ID だけが採番される）。PBT で (a) decide 後の
  状態 == 旧状態に同じイベント・通番・時刻を `apply_event` した状態、(b) `replay(snapshot, delta)` == 通常実行（version と新規イベント ID
  を除く）、(c) 通番単調と飛びの panic、(d) Quint 不変条件 7 本、(e) Err 無副作用、を `PROPTEST_RNG_SEED=20260823` 固定で固定する（NFR2.2）。
- ITF 準拠（NFR1.1）: `engine_loop.qnt`（v2.7、不変）のトレースを decide → apply 経路で再生し、rules.md 第 3 節の射影表で突き合わせる。
  射影は `StageSlots.at` で同じ観測を読む。追随対象（`next_decision` / `open_gate` / `recompose` の呼出）は logical-components §1。
- ゲート判定（NFR1.2）: `gated(stage) = phase ≠ initialization`（`StageEntry::is_gated` / `StageKey::is_gated`）。誕生が initialization 全段を
  Completed にする。実グラフ（initialization 3 ステージ）の索引 0〜2 非ゲート / 3 以降ゲート / initialization への jump = `InvalidTarget`
  をユニットテストで固定。
- イベント語彙（NFR1.3）: 16 変種の `enum` と網羅 `match`（`#[non_exhaustive]` は付けない）。ペイロードの列は FCC（`StageEntries` /
  `ArtifactPaths` / `StageSlugSet` / `PromotedSections` / `RuleLines`）だが DTO の列表現は不変。
- FCC の契約（NFR2.5）: 共通契約（`len` / `at` / `fold_left` / `filter`）は `tests/collection_contract_test.rs`、集合型の Monoid 則・差集合則と
  列の連結衝突拒否は型ファイル同居の性質試験。用途の無い型に `combine` / `divide` を足さない。登録対象の確定列挙は機能設計レビュー
  R-01 の定義確定が前提で、functional-design ゲートの Request Changes に先行して載せる。
- 時計・乱数・環境の不使用（NFR3.1）: §4。

## 7. 失敗の扱い

- 層 (1) の失敗はすべて `Result` で呼出側へ返す。沈黙の失敗なし（ガード不成立・境界の検査違反・FCC の衝突はそれぞれ専用の Err）。
- 層 (2) の失敗（型変換後の壊れた歴史）は panic で停止する。真実源（SQLite ジャーナル）が破損している状態なので進まないのが正で、
  復旧は U3 の責務。ブラストラディウスは再構成を行った 1 プロセス（logical-components §3）。
- エラー型は手実装 enum（`IntentExecutionError` は理由文字列の struct）+ `fmt::Display`（材料のみ）+ `std::error::Error` 手実装
  （house style、thiserror / anyhow 不使用）。FCC ごとの Error も同じ様式で新設し、既存型の変種は増やさない。
- `IntentMismatch` / `DefinitionMismatch` を受けたユースケース（U6）は処理を中断して上位へ返す。RMU の `NextAnswerRow::of` は
  `next_decision` の Err を投影の束縛不整合として扱う（§9 #2）。

## 8. 要求への対応

| 要求 | 設計上の手当て |
|---|---|
| NFR1.1 | ITF 準拠を decide → apply 経路で再生、合成計画 + 射影表、モデル不変、追随対象の列挙（§6、logical-components §1） |
| NFR1.2 | `gated = phase ≠ initialization`、誕生が initialization 全段を完了、実グラフ索引のユニットテスト（§6） |
| NFR1.3 | 16 変種 enum + 網羅 match、ペイロード列の FCC 化、DTO のバイト表現不変（§6） |
| NFR2.1 / NFR2.3 / NFR2.4 | TDD・カバレッジ（クレート全体 + orchestration 単独）・機械強制と BR4.1 判定式は logical-components §4 の受入手順 |
| NFR2.2 | PBT 5 性質、シード固定、生成器（§6） |
| NFR2.5 | FCC の共通契約はハーネス、Monoid 則・差集合則・連結衝突は型ファイル同居の性質試験、登録対象は R-01 確定が前提（§6） |
| NFR3.1 | 純関数的 decide / apply、時計は `*EventId::generate` のみ、環境・乱数なし（§4 / §6） |
| NFR3.2 | 検査点の二層 — 層 (1) は Err、層 (2) は panic と `# Panics`（§2） |
| NFR3.3 | スナップショット = 集約自身、DTO が全状態を写し `new` を通した復元が同値、`slots` 統合でも DTO の列表現は不変（§2 / §5） |
| NFR3.4 | `&Intent` を受ける全 API と `next_decision` の `IntentMismatch`、`resolve_review_policy` の `DefinitionMismatch`、revision は観測のみ（§3） |
| NFR4.1 / NFR4.2 / NFR4.5 | 依存追加なし・`unsafe` forbid・デシリアライズ面なし・FCC は境界で `fold_left`（§5） |
| NFR4.3 | `StageIndex` の型保証、FCC の `at` は `Option`、`unwrap` / `expect` 禁止、panic は層 (2) に限定し `# Panics` 明記（§2） |
| NFR4.4 | 人間入力と文字列 FCC の素通し、`Display` は材料のみ、秘密情報の経路なし（§4） |

## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-06T16:35:32Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action |
|---|---|---|---|---|
| R-01 | Critical | `construction/u2-domain-es-core/nfr-design/logical-components.md` > §1「兄弟クレートへ及ぶ追随（U2 の Bolt に含める — P8）」表、および §2「越境の裁定」 | 追随表が実測に対して不完全で、表のとおり着手すると未掲載の生産コードでビルドが壊れる。表は 3 クレート・10 ファイルを挙げるが、FCC 化する公開アクセサを生産コードで呼ぶ未掲載箇所が少なくとも次にある — `modules/core/read-model-updater/src/orchestration/dto/started_dto.rs:31`（`payload.stages()`）、`gate_opened_dto.rs:26`（`payload.artifacts()`）、`recomposed_dto.rs:25,26`（`payload.skipped()` / `added()`）、`practices_affirmed_dto.rs:42,49,50`（`payload.sections()` / `mandated()` / `forbidden()`）、`intent_dto.rs:93`（`intent.stages()`）; `modules/core/read-model-updater/src/workspace/projection.rs:1080,1083,1097,1098,1403,1416,1420,1432,1443,1444`（`recomposed.skipped()` / `added()`、`affirmed.sections()` / `mandated()` / `forbidden()`）; `modules/core/command/use-case/src/orchestration/promote_practices_use_case.rs:190,194`（`affirmed.sections()` / `mandated()`）; `modules/app/aidlc/src/scaffold.rs:46,161,182`（`intent.stages()`）。いずれも `.iter()` / `.to_vec()` / `for` の要素列挙であり、戻り値が `&[T]` から FCC 型へ変わればコンパイルエラーになる。とくに `modules/app/aidlc` は §2 越境の裁定が対象とする「3 クレート」に含まれておらず、宣言境界（`unit-of-work.md` U2「`core-command-domain` クレート内」）を越える 4 つめのクレートが裁定の射程外に残る。これにより §2 末尾の「機能設計レビュー R-10 / NFR 要求レビュー R-03 の未解決はこの記載で閉じ」は成立しない — NFR 要求レビュー R-03 の要求アクションは「`next_answer_row.rs` を含む**全呼出元**を列挙する」であり、上記が漏れている | 追随表を実測で作り直す（対象 API は `Intent::stages` / `IntentExecution::stage_keys` / `open_gate` / `recompose` / `apply_report` / `next_decision` と、`Created` / `Started` / `GateOpened` / `Recomposed` / `PracticesAffirmed` / `PracticesPromotion` / `ReviewAttempt` の列アクセサ全部）。`modules/app/aidlc` を越境の裁定の対象クレートに加えるか、対象外とする根拠を書く。テストファイル（interface-adapter / RMU / app の `tests/`）も同じ理由でビルド対象なので、Bolt の作業範囲見積りに含める |
| R-02 | Major | `.../nfr-design/security-design.md` > §2「検査点の二層」表の層 (2) 行、および直下の箇条書き「`# Panics` を持つ公開 API は `replay` / `apply_event` に限る」 | 実測と食い違い、二層モデルに第 3 の panic 点が抜けている。`modules/core/command/domain/src/orchestration/intent_execution.rs` の `# Panics` は 3 か所（`:380` = `replay`、`:1534` = `apply_event`、`:2389` = `impl From<(Started, DateTime<Utc>)> for IntentExecution`（宣言は `:2368`））。3 つめは誕生（genesis）変換の公開経路で、doc は「壊れた歴史 (空の計画・slug 重複・集約不変条件の違反)」で panic すると書く。これは層 (1) の `IntentExecution::new` が同じ違反を `IntentExecutionError` の Err で返すのと**同一条件**であり、どちらの経路を通るかで Err と panic が入れ替わる。この分岐は設計本文のどこにも書かれていないため、実装者は誕生経路の失敗の扱いを設計者へ問い直すことになる。なお `modules/core/command/domain/src/workflow_definition/workflow_definition.rs:213`（`WorkflowDefinition::replay`）にも `# Panics` があり、クレート全体で見ても「限る」は成立しない | §2 の層 (2) 表に誕生変換 `From<(Started, DateTime<Utc>)>` の行を足し、`new`（Err）と誕生変換（panic）が同じ不変条件違反をどう振り分けるかを明記する。箇条書きの「`replay` / `apply_event` に限る」を実測に合わせて書き直す（`workflow_definition` 文脈を射程外とするならその旨も書く） |
| R-03 | Major | `.../nfr-design/logical-components.md` > §1 `intent_execution` 行の「冒頭 doc（`:35`「`# Panics` を持つ公開 API は無い」「memento」）を実態へ修正」 | 是正指示が実測より狭く、修正漏れがそのまま残る。同じ冒頭 doc ブロックには他にも実態と乖離した記述がある — `intent_execution.rs:5`「**状態遷移**(12 の decide コマンド)」（本設計自身が 16 コマンド・16 変種と書いており、`src/orchestration/intent_execution_event/` は実測 16 ファイル）、同 `:26-28`「**楽観 version は持たない** (ADR-010 / B7) … 集約が持つ順序番号は `seq_nr` **だけ**」（実測: `IntentExecution` struct は `version: usize` フィールドを持ち、同 struct の doc が「オーナー裁定 2026-08-30」で集約が版を持ち回ると書いていて、同一ファイル内で矛盾している）。指示どおり `:35` と memento だけ直すと、より誤解を招く 2 文が残る | 冒頭 doc の是正対象を実測で列挙する（少なくとも decide コマンド数、楽観 version の保持、`# Panics` の 3 点）。`unit-of-work.md` U2 の「`version` は失効（ADR-010 / B7）」という宣言と 2026-08-30 裁定の関係も 1 行で決着させる |
| R-04 | Minor | `.../nfr-design/logical-components.md` > §1「契約試験ハーネス」行の「現行 7 型: BoltRefs / Checkboxes / OrderedAuditEvents / AuditFields / StageGraph / ScopeGrid + infrastructure 側」 | 実測と合わない。`modules/core/command/domain/tests/collection_contract_test.rs` が `check(..)` へ登録するのは列挙どおりの 6 型のみで、`use` は `core_command_domain::{workflow_definition, workspace}` と `core_infrastructure::collections::FirstClassCollection` だけであり、infrastructure 側の FCC 型は当ファイルに登録されていない。正本 `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/first-class-collections.md` は「既存7型のtrait適合は `collection_contract_test.rs` と `modules/core/infrastructure/tests/collections_test.rs` で検査する」と 2 ファイルに分けており、7 型はその合算 | 「現行 6 型（当ファイル）+ infrastructure 側は `modules/core/infrastructure/tests/collections_test.rs`」と書き分ける。新設 FCC をどちらのファイルへ登録するかも 1 行で指定する |
| R-05 | Minor | `.../nfr-design/logical-components.md` > §1「ID 型」行「`orchestration/{intent_id, intent_execution_id, intent_event_id, intent_execution_event_id}.rs` … `generate()`（`Uuid::now_v7`）が時計を読む唯一の箇所」 | 置き場の記述が実測と食い違う。`Uuid::now_v7` を持つのは実測 4 か所で、そのうち `orchestration/` にあるのは `intent_event_id.rs:49` と `intent_execution_event_id.rs:49` の 2 つだけ。`intent_id.rs` / `intent_execution_id.rs` に `generate` は無く、残る 2 か所は `workflow_definition/workflow_definition_event_id.rs:50` と `workflow_definition/compiled_definition_event_id.rs:50`。security-design §4 の「`*EventId::generate`（`Uuid::now_v7`、4 型）」自体は正しいが、この行を読むと NFR3.1 のレビュー範囲が `orchestration/` に閉じると誤解する | 行の対象を `*EventId` 2 型に限るか、時計を読む 4 か所が 2 文脈にまたがる旨を書く（NFR3.1 のレビュー項目はクレート `src/` 全体である旨と揃える） |
| R-06 | Minor | `.../nfr-design/traceability.json` > NFR2.4 の target「機械強制（fmt / clippy 50 / cargo lint）」 | 数の帰属が不正確。ルート `Cargo.toml` の実測は `[workspace.lints.rust]` 5 + `[workspace.lints.rustdoc]` 1 + `[workspace.lints.clippy]` 44 = 計 50 であり、clippy 単独は 44。上流 `nfr-requirements/security-requirements.md` 末尾レビューの検証表も同じ内訳（「rust 5 + rustdoc 1 + clippy 44 = 50」）を記録している | 「lints 計 50（rust 5 / rustdoc 1 / clippy 44）」と書き換える |
| R-07 | Info | `.../nfr-design/security-design.md` > §6 / §8、`.../nfr-design/logical-components.md` > §4「横断適合（U3 / U4 所有）」行 | 上流 NFR 要求レビュー R-08（Info、凍結中）の受け止めが本文に見えない。R-08 は「`audit_lock.qnt` の ITF 準拠とクラッシュ再構成テスト（ジャーナル → 集約 → 投影）は U3 / U4 の検収」と明記せよと求めているが、本設計の ITF 記述は `engine_loop.qnt` のみで、§4 の横断適合行も DTO 往復とゴールデンだけを挙げる。R-01〜R-07 は本文で受け止めが追えるのに、R-08 だけ追えない | §4 の横断適合行か §5 に「`audit_lock.qnt` の ITF 準拠とクラッシュ再構成は U3 / U4 の検収」を 1 行足す。あるいは R-08 は上流の記載事項であり本設計の射程外である旨を明記する |

### Validation Tool Results

| 検査 | 結果 | 解釈 |
|---|---|---|
| `aidlc-sensor-required-sections.ts --stage nfr-design`（`security-design.md`） | pass、H2 8 本、所見 0 | 必須見出しは充足。`validation-20260907.md` の記録と一致 |
| `aidlc-sensor-required-sections.ts --stage nfr-design`（`logical-components.md`） | pass、H2 5 本、所見 0 | 同上 |
| `aidlc-sensor-traceability.ts --stage nfr-design` | pass、gaps / orphans / missing_from_table / missing_from_upstream_ids / invalid_entries / invalid_targets すべて空 | NFR1.1〜NFR4.5 + NFR2.5 の 17 ID は構造的に過不足なし。R-06 は target 文言の精度で、センサーの守備範囲外 |
| `orchestration/` のエントリ数（`ls \| wc -l`） | 53（`ls -p \| grep -v /` で 51 ファイル + `intent_event/` / `intent_execution_event/`） | §1 の「53 エントリ = 51 ファイル + 2 ディレクトリ」は正確。質問票 P7 の「55 ファイル」を訂正した記載も正しい |
| `IntentExecution` の並列列（struct 実測） | `stage_keys` / `overlay` / `checkbox` / `review_attempts` / `practices_affirmed` / `approved` / `revision_count` の 7 列 | §1 の `StageSlots` 統合対象 7 列は正確 |
| 引用行番号（`intent_execution.rs`） | `new`:283 / `replay`:385 / `stage_keys`:441 / `stage_index`:464 / `open_gate`:821 / `recompose`:1060 / `apply_event`:1542 / `next_decision`:1897 / `apply_report`:2024、`matches`:417 | security-design §2 / §3 と logical-components §1 の引用はすべて実測どおり |
| 生の `Vec` / `&[..]` 公開（引用箇所の実測） | `intent.rs:260` / `stage_entry.rs:100` / `review_attempt.rs:66` / `created.rs:87` / `started.rs:68` / `gate_opened.rs:40` / `recomposed.rs:37,43` / `practices_affirmed.rs:61,67,73` / `practices_promotion.rs:113,119,125` すべて該当 | §1「本再走の変更」列の引用は正確 |
| 変種数（`command_error.rs` / `report_refusal.rs` / `intent_execution_event/`） | `CommandError` 17、`ReportRefusal` 13、イベント 16 | §2 と §1 の記載どおり |
| `IntentExecutionError` の形 / `ApplyError` の可視性 | 理由文字列 1 本の `pub struct` / `pub(crate) enum ApplyError`（`apply_error.rs:15`、`mod.rs:67` は `mod apply_error;` のみで `pub use` なし） | §2 / §7 の記載どおり |
| `IntentError` / `IntentExecution::new` の検査項目 | `UnknownScope` / `Empty` / `InitializationMustExecute` / `InitializationMustBeUnconditional` / `DuplicateSlug` / `StageDisplayNotSingleLine` の 6 変種、`new` は empty plan → 列長 → cursor 範囲 → parked_at → `seq_nr == 0` → slug 重複 → 不変条件の順に Err | §2 の層 (1) 表 2 行は実測と完全一致 |
| `# Panics` の所在（`src/` 全体） | `intent_execution.rs:380,1534,2389`、`workflow_definition/workflow_definition.rs:213` | §2 の「`replay` / `apply_event` に限る」は不成立（R-02） |
| 冒頭 doc の乖離（`intent_execution.rs:1-40`） | `:5`「12 の decide コマンド」、`:26`「楽観 version は持たない」、`:35`「`# Panics` を持つ公開 API は無い」 | §1 の是正指示は `:35` と memento のみを挙げる（R-03） |
| 時計・環境・乱数（`src/` の grep） | `Uuid::now_v7` 4 か所（`intent_event_id.rs:49` / `intent_execution_event_id.rs:49` / `workflow_definition_event_id.rs:50` / `compiled_definition_event_id.rs:50`）、`std::time` / `std::env` / `rand` は 0 件 | security-design §4 の「4 型」は正確。logical-components §1 の置き場記述は不正確（R-05） |
| 文脈間の参照方向 | `src/workspace/` → `orchestration` 0 件 | §2 の一方向依存の主張どおり |
| workspace lints（ルート `Cargo.toml`） | rust 5 + rustdoc 1 + clippy 44 = 50、`unsafe_code = "forbid"` は `[workspace.lints.rust]` に実在（`:29`） | §5 の `unsafe_code` forbid は正確。traceability の「clippy 50」は帰属誤り（R-06） |
| 追随表の引用箇所（兄弟クレート 4 ファイル群） | `intent_dto.rs:85` / `created_dto.rs:47` / `intent_execution_dto.rs:142` / `intent_execution_event_dto.rs:113,121`、`read_tables.rs:239,284` / `stage_lookup.rs:23` / `resolved_plan.rs:49` / `next_answer_row.rs:58`、`commit_verdict_use_case.rs:212,218` / `test_support.rs:114,856,889`、`engine_loop_conformance.rs:356,449,488` — すべて該当 | 挙げられた箇所は正確。問題は網羅性（R-01） |
| 未掲載の呼出箇所（同じ API を `rg` で全 workspace 走査） | RMU 生産コード 6 ファイル（`workspace/projection.rs`、`orchestration/dto/` 5 本）、use-case 1 ファイル（`promote_practices_use_case.rs`）、`modules/app/aidlc/src/scaffold.rs` ほかテスト多数 | 追随表の不足を実証（R-01） |
| `FirstClassCollection` trait（`modules/core/infrastructure/src/collections/first_class_collection.rs`） | `len` / `is_empty` / `at` / `fold_left` / `filter`（`filter` は `type Filtered` を要求） | §6 の共通契約の記述どおり。`Filtered` の具体型が未決である旨を §1 が R-04 として引き継いでいるのも妥当 |
| `tests/collection_contract_test.rs` の登録型 | `check(&..)` 9 呼出 / 6 型（BoltRefs / Checkboxes / OrderedAuditEvents / AuditFields / StageGraph / ScopeGrid）、infrastructure 側の型は登録なし | §1 の「現行 7 型 … + infrastructure 側」は不正確（R-04） |
| `cargo` の実行 | 未実施（依頼の指示による） | ビルド・テスト・カバレッジ・Quint ゲートは本レビューの根拠に含めていない |

### Summary

実測との照合精度は総じて高い。引用した行番号・変種数・エラー型の形・`ApplyError` の可視性・`unsafe_code` forbid・文脈間の参照方向・`new` の検査順序まで、確認したものはほぼすべて現行コードと一致し、旧世代（`WorkflowExecution` / `from_snapshot` / 「panic なし」）の失効も正しく反映されている。センサー 3 本も緑である。上流の凍結中レビュー所見も、R-01（RMU は FCC 型を保持しない）・R-04（判定式の `-e` 2 本と検出力の裏取り）・R-06（用途の無い `combine` を足さない）・R-07（`orchestration` 単独計測）は本文で受け止めが追える。

NOT-READY の理由は 2 点に集中する。第 1 に、兄弟クレートへの追随表が実測に対して不完全で、read-model-updater の DTO 5 本と `workspace/projection.rs`、use-case の `promote_practices_use_case.rs`、そして越境の裁定が対象にすらしていない `modules/app/aidlc` の生産コードが漏れている。実装者はビルドエラーに突き当たった時点で「`app` クレートまで U2 の Bolt で触ってよいか」を設計者へ問い直すことになり、これは上流 NFR 要求レビュー R-03 が指摘した射程の問題がそのまま残っていることを意味する。第 2 に、二層モデルの panic 側が誕生変換 `From<(Started, DateTime<Utc>)>` を取りこぼしており、`IntentExecution::new` が Err で返すのと同じ不変条件違反が誕生経路では panic するという分岐が本文に無い。どちらも設計者への追加照会なしには実装に入れない性質の欠落である。R-03〜R-07 は文言の精度の問題で、単独なら判定を左右しない。
