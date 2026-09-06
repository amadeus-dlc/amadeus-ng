# developer-report-3 — 委任 1: FCC 11 型の新設（U2、Bolt b51、Step 0〜1）

AIDLC-UNIT: u2-domain-es-core
AIDLC-TESTING-CONTRACT: sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3

依頼書: `developer-brief-3.md`（承認済み計画 `code-generation-plan.md` 指紋
`sha256:dd1170c1a75b16e30a351f34d9f4ff57164bcbe65482361e94e6909de7f0634d` の Step 0〜Step 1）。
追加のみで既存の公開 API・既存テストは 1 行も変更していない。

## 1. Step 0 — 着手前の基線（2026-09-07 実測）

### 作業ツリーとブランチ

`git status --short` は**空ではない**。intent 記録の未コミット変更（`aidlc-state.md` /
`audit/` シャード / code-generation の履歴退避リネーム 5 件 / 新規 4 ファイル）が残っており、
依頼書の指示どおり**そのまま触っていない**。`modules/` 配下は着手前に変更なし（クリーン）。

`git log --oneline origin/main..HEAD`（記録コミットのみ、4 本）:

```
3bf8432f chore: U2 NFR 設計を現行コードと 2026-09-07 裁定へ再走同期（advisory NOT-READY を記録）
e35733e0 chore(aidlc): U2 NFR 要求を 2026-09-07 再走（Modify）で現行裁定・実コード・CI 実測へ同期 …
9f6a039a chore(aidlc): U2 機能設計を 2026-09-07 再走（Modify）で同期 …
b674645d chore(aidlc): U10 CI ガバナンスの実装記録を 2026-09-06 の実態へ同期（Code Generation 再確認、レビュー READY）
```

### テスト件数の基線

| 対象 | コマンド | 件数 |
|---|---|---|
| ドメイン lib（ユニット + PBT） | `PROPTEST_RNG_SEED=20260823 cargo test -p core-command-domain --lib` | 591 passed |
| 共通契約 | `cargo test -p core-command-domain --test collection_contract_test` | 1 passed |
| ITF 準拠（engine_loop） | `cargo test -p core-command-domain --test engine_loop_conformance` | 1 passed |
| doc-tests | 同上（`cargo test -p core-command-domain`） | 3 passed |

### カバレッジの基線

| 計測 | Regions | Functions | Lines |
|---|---|---|---|
| `cargo llvm-cov --package core-command-domain --summary-only` | 98.69% | 98.20% | **98.66%** |
| 同上 + `--ignore-filename-regex '…/(workflow_definition\|workspace)/'`（orchestration 単独） | 99.00% | 99.28% | **99.07%** |

いずれも `PROPTEST_RNG_SEED=20260823` 付き。クレート全体の行カバレッジ 98.66% は
`unit-test-instructions.md` §4 の基準値と一致した。

### `# Panics` の所在（`rg -n '# Panics' modules/core/command/domain/src`）

```
src/orchestration/intent_execution.rs:35   （冒頭 doc の「# Panics を持つ公開 API は無い」— §9 #4 で是正予定）
src/orchestration/intent_execution.rs:380
src/orchestration/intent_execution.rs:1534
src/orchestration/intent_execution.rs:2389
src/workflow_definition/workflow_definition.rs:213
```

### 生の `&[..]` 公開（`rg -n 'pub fn .*-> &\[' src/orchestration src/workspace`）— **14 件**

`intent_execution.rs:441 stage_keys` / `intent_execution_event/gate_opened.rs:40 artifacts` /
`workspace/practices_promotion.rs:113,119,125 sections,mandated,forbidden` /
`intent_execution_event/recomposed.rs:37,43 skipped,added` /
`intent_execution_event/started.rs:68 stages` / `intent.rs:260 stages` /
`intent_event/created.rs:87 stages` / `review_attempt.rs:66 closed` /
`intent_execution_event/practices_affirmed.rs:61,67,73 sections,mandated,forbidden`。

これらの解消は Step 2（委任 2）の担当であり、本委任では 14 件のまま**不変**である（受入 3 で再実測）。

## 2. Step 1 — 型ごとの Red → Green → Refactor

TDD は型ごとに「失敗するテストを先に書く」→「最小実装」→「緑のまま整理」で回した。Red は
`#[cfg(test)] mod tests` だけを持つファイルを作り `mod` 宣言を足した状態で Unit 限定コマンドを
走らせ、コンパイル失敗の実出力を記録した（依頼書 §A の許容形）。

### 2.1 `ArtifactPaths`（`orchestration/artifact_paths.rs`、テスト 7 本）

Red（`PROPTEST_RNG_SEED=20260823 cargo test -p core-command-domain --lib artifact_paths`）:

```
error[E0432]: unresolved import `super::ArtifactPaths`
 --> modules/core/command/domain/src/orchestration/artifact_paths.rs:5:9
  |
5 |     use super::ArtifactPaths;
  |         ^^^^^^^^^^^^^^^^^^^^ no `ArtifactPaths` in `orchestration::artifact_paths`
```

Green: `Vec<String>` を包む素通しの列。`empty` / `new` / `len` / `is_empty` / `at` /
`fold_left` / `filter -> Self` と `FirstClassCollection`（`Item<'a> = &'a str`、
`Filtered = Self`）。順序・重複を保持し、集合ではないので `combine` / `divide` は持たない
（NFR4.4）。Refactor なし（初回で最小形）。

### 2.2 `RuleLines`（`workspace/rule_lines.rs`、テスト 7 本）

Red:

```
error[E0432]: unresolved import `super::RuleLines`
 --> modules/core/command/domain/src/workspace/rule_lines.rs:5:9
  |
5 |     use super::RuleLines;
  |         ^^^^^^^^^^^^^^^^ no `RuleLines` in `workspace::rule_lines`
```

Green: `ArtifactPaths` と同形。Refactor: 畳み込みテストが `&line[..6]` でスライスしていたのを
`split_whitespace().next()` へ置換（`indexing_slicing` deny はテストコードにも効くため）。

### 2.3 `StageIndexSet`（`orchestration/stage_index_set.rs`、テスト 10 本 — うち proptest 2 本）

Red:

```
error[E0432]: unresolved import `super::StageIndexSet`
 --> modules/core/command/domain/src/orchestration/stage_index_set.rs:5:9
  |
5 |     use super::StageIndexSet;
  |         ^^^^^^^^^^^^^^^^^^^^ no `StageIndexSet` in `orchestration::stage_index_set`
```

Green: `BTreeSet<StageIndex>`。`empty` / `singleton` / `range(from, to_exclusive)` /
`new(impl IntoIterator)` / `contains` / `combine` / `divide` / `at`（昇順）/ `fold_left` /
`filter -> Self`、`Default` = 空。proptest（シード 20260823）で和集合の結合法則・左右単位元・
冪等・交換、差集合の `A \ A = ∅` / `A \ ∅ = A` / `(A ∪ B) \ B ⊆ A` を固定。

Green 途中で 2 件詰まり、いずれもその場で直した（記録）:

- `BTreeSet::len` / `is_empty` は const fn として未安定 → `const` を外した。
- 共通契約の `fold_left` は `Self::Item<'a>` を通じて `'a` が early-bound になるため、
  実装側の署名も `impl FnMut(A, Self::Item<'a>) -> A` と**関連型で**書く必要があった
  （具体型 `StageIndex` を直書きすると E0195）。以降の全型で同じ書き方を採用した。

### 2.4 `StageSlugSet`（`orchestration/stage_slug_set.rs`、テスト 8 本 — うち proptest 2 本）

Red:

```
error[E0432]: unresolved import `super::StageSlugSet`
 --> modules/core/command/domain/src/orchestration/stage_slug_set.rs:5:9
  |
5 |     use super::StageSlugSet;
  |         ^^^^^^^^^^^^^^^^^^^ no `StageSlugSet` in `orchestration::stage_slug_set`
```

Green: `BTreeSet<StageSlug>`（辞書順）。API は `StageIndexSet` と同型（`range` なし）。同じ
Monoid 則・差集合則を proptest で固定。doc に「並び順は辞書順であり文書順ではない — 逐語一致が
要る投影側は計画の位置で並べ直す」と明記した（計画 §2 補足の `Recomposed` 投影順序）。
Refactor: テストヘルパ `from_raw` を `into_iter` 消費へ（`needless_pass_by_value` deny）。

### 2.5 `TransitionSteps` / `TransitionStepsError`（テスト 8 + 4 本）

Red:

```
error[E0432]: unresolved import `super::TransitionSteps`
 --> modules/core/command/domain/src/orchestration/transition_steps.rs:5:9
error[E0432]: unresolved import `crate::orchestration::TransitionStepsError`
 --> modules/core/command/domain/src/orchestration/transition_steps.rs:6:48
error[E0432]: unresolved import `super::TransitionStepsError`
 --> modules/core/command/domain/src/orchestration/transition_steps_error.rs:5:9
```

Green: `Vec<TransitionStep>`（重複なし）。`new -> Result<_, Duplicate { step }>` / `single` /
`contains` / `is_single` / `is_pair` / `at` / `fold_left` / `filter -> Self`。
`TransitionStep` は `Copy` + `PartialEq` + `Hash` を持つことを実装で確認したので `Item<'a>` は
値渡しにした。段分岐は**名前付きクエリ**（`is_single` / `is_pair`）で書ける形にし、
`pub(crate)` のスライス公開は採らなかった（計画 §2 が開発者判断としていた二択の前者）。

### 2.6 `ReviewClosures`（テスト 7 本）

Red:

```
error[E0432]: unresolved import `super::ReviewClosures`
 --> modules/core/command/domain/src/orchestration/review_closures.rs:5:9
  |
5 |     use super::ReviewClosures;
  |         ^^^^^^^^^^^^^^^^^^^^^ no `ReviewClosures` in `orchestration::review_closures`
```

Green: `Vec<ReviewClosure>`（記録順）。`empty` / `new` / `record`（コマンド、`&mut self`）/
`has_terminal(&ReviewPolicy)` / `at` / `fold_left` / `filter -> Self`。`has_terminal` は現行
`ReviewAttempt::has_terminal` の判定を**写しただけ**で、`ReviewAttempt` 自体は触っていない
（写した先は `fold_left` で書き、イテレータを外へ出していない）。

### 2.7 `PendingIterations`（`pub(crate)`、テスト 7 本）

Red:

```
error[E0432]: unresolved import `super::PendingIterations`
 --> modules/core/command/domain/src/orchestration/pending_iterations.rs:5:9
  |
5 |     use super::PendingIterations;
  |         ^^^^^^^^^^^^^^^^^^^^^^^^ no `PendingIterations` in `orchestration::pending_iterations`
```

Green: `BTreeSet<u32>`。`empty` / `with` / `without`（コマンド、`&mut self`）/ `contains` /
`at` / `fold_left` / `filter -> Self`。ファサード非公開（`pub(crate)`）。

### 2.8 `PromotedSections` / `PromotedSectionsError`（テスト 7 + 3 本）

Red:

```
error[E0432]: unresolved import `super::PromotedSections`
 --> modules/core/command/domain/src/workspace/promoted_sections.rs:5:9
error[E0432]: unresolved import `crate::workspace::PromotedSectionsError`
 --> modules/core/command/domain/src/workspace/promoted_sections.rs:6:45
error[E0432]: unresolved import `super::PromotedSectionsError`
 --> modules/core/command/domain/src/workspace/promoted_sections_error.rs:5:9
```

Green: `Vec<PromotedSection>`（順序保持・見出し一意）。
`new -> Result<_, DuplicateHeading { heading }>` / `at` / `fold_left` / `filter -> Self` /
`headings() -> Collection<String>`（計画どおり `fold_left` で組んだ）。

### 2.9 `StageEntries`（テスト 10 本）

Red:

```
error[E0432]: unresolved import `super::StageEntries`
 --> modules/core/command/domain/src/orchestration/stage_entries.rs:5:9
  |
5 |     use super::StageEntries;
  |         ^^^^^^^^^^^^^^^^^^^ no `StageEntries` in `orchestration::stage_entries`
```

Green: `Vec<StageEntry>`。`new` は現行 `StageEntry::check_plan` を**呼び出して再利用**し、
`PlanError` の 4 変種を同じ順序・同じ変種で返す（`Empty` → `InitializationMustExecute` →
`InitializationMustBeUnconditional` → `DuplicateSlug`）。`at(StageIndex)` /
`position_of(&StageSlug)` / `first_of(PhaseId, PlanAction)` / `slugs_at(&StageIndexSet)` /
`fold_left` / `filter -> Collection<StageEntry>`。

Refactor: `position_of` の初回実装に、`fold_left` で書こうとして残った無意味な分岐
（常に `None` を返す畳み込みと `.or_else`）が混入していた。緑のまま `iter().position()` の
1 式へ整理した。

### 2.10 `StageSlot`（テスト 9 本）

Red:

```
error[E0432]: unresolved import `super::StageSlot`
 --> modules/core/command/domain/src/orchestration/stage_slot.rs:5:9
  |
5 |     use super::StageSlot;
  |         ^^^^^^^^^^^^^^^^ no `StageSlot` in `orchestration::stage_slot`
```

Green: 値の組（`Clone` + `PartialEq` + `Eq` + `Debug`）。`genesis(key, plan_action)`（Pending・
未承認・0・`ReviewAttempt::default()`・未受領）と `new(全属性)`（DTO 境界）。アクセサ名は現行
集約の綴りに合わせた（`checkbox` / `approved` / `revision_count` / `review_attempt` /
`practices_affirmed`）。コマンドは `&mut self` で戻り値なし: `mark` / `record_approval` /
`invalidate_approval` / `bump_revision`（飽和加算）/ `override_plan` / `reset_attempt`
（会計 reset + affirmed = false）/ `record_review_request` / `record_review_verdict` /
`affirm_practices`。レビュー系は `ReviewAttempt` の既存 `pub(super)` メソッドへ委譲した
（`review_attempt.rs` は未変更）。

### 2.11 `StageSlots` / `StageSlotsError`（テスト 12 + 3 本）

Red:

```
error[E0432]: unresolved import `super::StageSlots`
 --> modules/core/command/domain/src/orchestration/stage_slots.rs:5:9
error[E0432]: unresolved import `crate::orchestration::StageSlotsError`
 --> modules/core/command/domain/src/orchestration/stage_slots.rs:9:30
  |
9 |         StageKey, StageSlot, StageSlotsError,
  |                              ^^^^^^^^^^^^^^^ no `StageSlotsError` in `orchestration`
```

Green: `Vec<StageSlot>`（非空・slug 一意）。`new -> Result<_, StageSlotsError>` /
`genesis(&StageEntries)` / `at(StageIndex)` / `stage_key(StageIndex)` / `position_of` /
`fold_left` / `filter -> Collection<StageSlot>`。位置指定コマンド 9 本はすべて
`-> Result<(), StageSlotsError>` で、範囲外は `OutOfRange { stage }` で拒否する（無言 no-op に
しない）。一括コマンド `mark_all` / `invalidate_approvals` / `reset_attempts_all` は
戻り値なし。Green 途中で `StageSlotsError` の `Copy` derive が `String` フィールドと衝突した
ため derive から外した（`Debug, Clone, PartialEq, Eq`）。

### 2.12 共通契約への登録（`tests/collection_contract_test.rs`）

新規テスト `the_orchestration_and_workspace_collections_share_the_traversal_contract` を
1 本追加し、公開 9 型を `check(..)` へ登録した（空を許す型は空と非空の 2 例、非空型
`StageEntries` / `StageSlots` は非空例のみ）。既存テストと既存の `check` ヘルパは未変更。

`StageIndex` の公開構築経路はクレート外に無いため、統合テストからは
`StageEntries::position_of(&slug)` で位置を得て `StageIndexSet::singleton` を組んだ。

Refactor: 合成ヘルパを自由関数にすると `clippy.toml` の `allow-unwrap-in-tests` が効かず
`unwrap_used` で落ちたため、`#[test]` 本体のクロージャへ移した。

## 3. 受入（Step 1 の完了条件）

### 受入 1 — `PROPTEST_RNG_SEED=20260823 cargo test -p core-command-domain` 全緑

```
Running unittests src/lib.rs
test result: ok. 693 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.67s
Running tests/collection_contract_test.rs
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Running tests/engine_loop_conformance.rs
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
Doc-tests core_command_domain
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

| 対象 | Step 0 | Step 1 | 増分 |
|---|---|---|---|
| lib | 591 | 693 | **+102** |
| 契約試験 | 1 | 2 | +1 |
| ITF 準拠 | 1 | 1 | 0 |
| doc-tests | 3 | 3 | 0 |

新設テスト合計 **103 本**（型ごとの内訳: `ArtifactPaths` 7 / `RuleLines` 7 /
`StageIndexSet` 10 / `StageSlugSet` 8 / `TransitionSteps` 8 / `TransitionStepsError` 4 /
`ReviewClosures` 7 / `PendingIterations` 7 / `PromotedSections` 7 /
`PromotedSectionsError` 3 / `StageEntries` 10 / `StageSlot` 9 / `StageSlots` 12 /
`StageSlotsError` 3、共通契約 1）。うち性質試験は 4 本（集合型 2 つ × Monoid 則・差集合則）。
各コンポーネント 5〜8 本の目安に対し、集合型と `StageSlots` は操作数が多いため上回っている。

### 受入 2 — lint

```
$ cargo fmt --all --check        → 出力なし（PASS）
$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.89s   （警告 0）
$ cargo lint; echo "EXIT=$?"     → EXIT=0
```

参考: `PROPTEST_RNG_SEED=20260823 cargo test --workspace` も全緑を確認した（追加のみで
兄弟クレートに影響がないことの裏取り）。

### 受入 3 — diff がドメインクレートに閉じている

`git status --short -- modules`（コミット前）:

```
 M modules/core/command/domain/src/orchestration/mod.rs
 M modules/core/command/domain/src/workspace/mod.rs
 M modules/core/command/domain/tests/collection_contract_test.rs
?? modules/core/command/domain/src/orchestration/{artifact_paths,pending_iterations,review_closures,
   stage_entries,stage_index_set,stage_slot,stage_slots,stage_slots_error,stage_slug_set,
   transition_steps,transition_steps_error}.rs
?? modules/core/command/domain/src/workspace/{promoted_sections,promoted_sections_error,rule_lines}.rs
```

`git diff --stat -- Cargo.lock Cargo.toml modules/core/command/domain/Cargo.toml` は**空**
（依存追加なし、NFR4.1）。`# Panics` の所在は Step 0 と同一（増えていない）。
生の `&[..]` 公開も 14 件のまま（解消は Step 2）。

### 受入 4 — カバレッジが基線を下回らない

| 計測 | Step 0 | Step 1 | 判定 |
|---|---|---|---|
| クレート全体・行 | 98.66% | **98.87%** | PASS（+0.21pp） |
| クレート全体・リージョン | 98.69% | 98.83% | PASS |
| クレート全体・関数 | 98.20% | 98.45% | PASS |
| orchestration 単独・行 | 99.07% | **99.33%** | PASS（+0.26pp） |

### 受入 5 — コミット

`feat(domain): FCC 11 型を新設し契約試験へ登録` 1 本。`git add` は
`modules/core/command/domain/` に限り、`aidlc/` の記録はコミットしていない（回収はコンダクタ）。
push は行っていない。

## 4. 計画からの逸脱

1. **`PendingIterations` を共通契約ハーネスへ登録できない**（計画 §4 Step 1 と §2 の記載の衝突）。
   計画は「`tests/collection_contract_test.rs` の `check(..)` に新設型を登録」と書く一方、同じ型を
   `pub(crate)`・ファサード非公開と定めている。統合テストは別クレートなので `pub(crate)` 型に
   触れない。**解決**: `check` と同一の検査（`len` / `is_empty` / `fold_left` / `at` の範囲外 2 種 /
   `filter(true) == self` / `filter(false).is_empty()`）を空・非空の 2 基数で回すテストを
   `pending_iterations.rs` のインライン `#[cfg(test)]` に置いた
   （`the_shared_traversal_contract_holds_for_both_cardinalities`）。登録型は公開 9 型。

2. **`Collection<T>` 側に cross-type `PartialEq` を 2 本追加した**。契約ハーネスの `check` は
   `C::Filtered: PartialEq<C>` を要求する。`Filtered` が `Self` でない型（`StageEntries` /
   `StageSlots`、計画 §2 の表どおり `Collection<StageEntry>` / `Collection<StageSlot>`）を登録
   するには `impl PartialEq<StageEntries> for Collection<StageEntry>` が要る（孤児規則上は
   合法 — トレイト引数側が自クレート型）。各型のファイル末尾に理由付き doc を添えて置いた。
   `check` ヘルパ自体は依頼書の制約どおり未変更。

3. **一括コマンドの範囲外の扱いを doc で確定した**。依頼書は位置指定コマンドにのみ
   `Result<(), StageSlotsError>` を指示し、`mark_all` / `invalidate_approvals` /
   `reset_attempts_all` には戻り値を指示していない。署名は依頼書どおりにしたうえで、
   「位置集合は区間や述語から組むので、この列に**在る位置だけ**を動かす集合演算である」と
   doc と専用テスト（`a_bulk_command_naming_a_position_past_the_end_touches_only_what_exists`）で
   固定した。無言の失敗ではなく全域の集合演算として定義した、という整理である。

4. **`PendingIterations` に `#[cfg_attr(not(test), expect(dead_code, reason = "…"))]` を 1 か所
   置いた**。この型を `ReviewAttempt` へ差し込むのは Step 2 なので、Step 1 単独では非テスト
   ビルドで未使用になり `-D warnings` が落ちる。`allow` ではなく `expect` にしたので、Step 2 で
   実際に使われた時点で「期待が満たされない」と赤くなり、委任 2 が属性を消す動機になる。

5. **`entities.md` の業務操作のうち本委任で作らなかったもの**（依頼書 §A の操作一覧に無いため）:
   `StageEntries::map` / `combine` / `divide`、`StageSlots` の `active_count` / `positions` /
   `next_effective_execute_after` / `first_effective_execute` / `with_slot` / `with_slots` /
   `clear_receipts` / `map` / `combine` / `divide`、`ArtifactPaths` / `RuleLines` の
   `combine` / `divide`。`first-class-collections.md` の「使われない共通メソッド群を機械的に
   追加しない」に従い、Step 2 が必要とした時点で委任 2 が足す想定である。

## 5. 次の委任（Step 2、切替）への申し送り

- **`StageKey` の構築**: `StageKey::new(slug: StageSlug, phase: PhaseId)` は `pub const fn` で
  公開済み。`StageSlots::genesis` はこれを `StageEntries` の各要素から
  `StageKey::new(entry.slug().clone(), entry.phase())` で組んでいる。
- **`ReviewAttempt::default`**: `#[derive(Default)]` 済み。`StageSlot::genesis` はこれを使う。
  `record_request` / `record_verdict` / `reset` は `pub(super)`（= `orchestration` から可視）
  なので、子モジュール `orchestration::stage_slot` からそのまま委譲できた。
- **`TransitionStep` の trait**: `Debug, Clone, Copy, PartialEq, Eq, Hash` を持つ。よって
  `TransitionSteps` の `Item<'a>` は値渡し、`filter` の述語も値受けである。
- **`StageIndex::new` は `pub(crate)`**。`StageIndexSet::range` / `new` は同一クレートなので
  問題ないが、**クレート外（統合テスト・兄弟クレート）から位置集合を組む公開経路が無い**。
  現状の公開経路は `IntentExecution::stage_index(usize)` と、本委任で足した
  `StageEntries::position_of` / `StageSlots::position_of` の 3 本。Step 3 で
  interface-adapter / RMU が `StageIndexSet` を組む必要が出たら、公開の構築口をどこに置くかを
  裁定してほしい（現時点では DTO 側が位置集合を組む必要は見当たらない）。
- **誕生時の initialization = Completed** は `StageSlots::genesis` に入れていない（依頼書 §A の
  指示どおり全位置 Pending）。b34 の「誕生が initialization 3 段を Completed にしてカーソルを
  最初の実ステージへ立てる」は集約側の誕生変換の責務として Step 2 で書く。
- **`StageSlugSet` は辞書順**。`Recomposed` の投影（RMU `projection.rs:1097-1098`）は文書順で
  描く必要があるため、`plan.stages()` の位置で並べ直してから描く（計画 §2 補足のとおり）。
  型側の順序は変えないこと。
- **`ReviewClosures::has_terminal` は `ReviewAttempt::has_terminal` の写し**であり、現時点で
  同じ判定が 2 か所にある。Step 2 で `ReviewAttempt.closed` を `ReviewClosures` へ差し替えた
  ら、`ReviewAttempt::has_terminal` は `self.closed.has_terminal(policy)` への委譲 1 行にして
  重複を解消してほしい。
- **`StageSlotsError` に `OutOfRange` を足した**（計画 §2 の表は `Empty` / `DuplicateSlug` の
  2 変種）。位置指定コマンドを無言 no-op にしないために要る 3 変種目である。集約側が
  `IntentExecution::stage_index` で範囲を保証してから呼ぶ経路では発生しないので、Step 2 は
  この `Err` をそのまま上位へ流すか、`IntentExecutionError` へ写すかを選ぶことになる。

## 6. 設計判断が要る問題

**0 件**。上記 §4 の 5 点はいずれも依頼書 §A / 計画 §2 の範囲内で解決できたため、止まらずに
完了した。§5 の 3 点目（`StageIndexSet` のクレート外構築口）は Step 3 で必要になったときの
裁定事項として申し送るにとどめる。
