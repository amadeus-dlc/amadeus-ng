# B11 開発者報告 1 — U5 `report` ユースケース（FR2.1）

対象ブリーフ: [`brief-1.md`](brief-1.md)
ブランチ: `bolt/b11-u5-report-use-case`（origin/main 基準、**push なし・コミットは意味単位**）
検証の `CARGO_TARGET_DIR`: `target-delegate`（`cargo lint` のみ `target-delegate-lint`）

---

## 0. 結論（先に）

固定裁定 1〜5 はすべて守った。実装（A）（B）とテスト（C）は完了し、私の所有ツリーだけで
fmt / clippy / `cargo lint` / 該当クレートのテストはすべて緑である。

ただし**受入基準 4・5・6・8 は未通過**である。理由は 1 つだけで、裁定 2（`approve_gate` の
引数廃止）の追随が**ブリーフで禁止されたツリーの 3 ファイル 4 箇所**に必要であり、独断で
触れないと判断して止めたためである。詳細は §7。必要な差分は 4 トークンの削除だけで、
パッチも §7 に添えた。

---

## 1. 実装した経路と、対応する集約コマンド

`ReportUseCase::execute` の経路表。左端はユースケースの入力型
`ReportedVerdict` / `ReportedTransition` の変種、右端は打った集約コマンドである。

| 入力（正規化済み） | 対応する `Verdict` | 打つ集約コマンド | コミット | 備考 |
|---|---|---|---|---|
| `Transition(AwaitingApproval { artifacts })` | `AwaitingApproval` | `open_gate(artifacts, at)` | する | |
| 同上・ただし現在の印が `[?]` | `AwaitingApproval` | **打たない** | **しない** | `GateAlreadyOpen` を返す（golden `awaiting-approval-repeat` は監査行・状態差分とも空） |
| `Transition(Forward { user_input })`・カーソルがゲート付き | `Forward` | `approve_gate(user_input, at)` | する | どちらを打つかは集約の `gated(cursor)` クエリで決める |
| `Transition(Forward { user_input })`・カーソルが非ゲート | `Forward` | `complete_stage(at)` | する | 同上（initialization フェーズ） |
| `Transition(Rejected { feedback })` | `Rejected` | `reject_gate(feedback, at)` | する | 改訂回数の +1 は集約が行う（BR1.4） |
| `Transition(Revised)` | `Revised` | `revise_stage(at)` | する | |
| `Transition(Skipped { reason })` | `Skipped` | `skip_stage(reason, at)` | する | CONDITIONAL / 実効 SKIP の判定は集約（BR1.5） |
| `Resumed` | `Resume` | **打たない**（再構成もしない） | **しない** | `ResumeRouting` を返す |
| `stage` 引数がカーソル手前のステージを名指し | 経路によらず先に判定 | `stale_report(index)`（クエリ） | **しない** | `AlreadyDone` を返す（BR1.9） |

補足:

- **`Resumed` だけが型の外側にいる。** 入力型を
  `ReportedVerdict { Transition(ReportedTransition), Resumed }` の 2 段にしたのは、
  「再開は集約に届かない」を型の事実にするためである。1 段の 6 変種にすると
  ユースケース側の `match` に到達不能な腕が残り、`clippy::unreachable`（workspace で deny）
  を避けるための死んだ分岐がカバレッジに穴を開ける。
- **入力に生の文字列は無い。** `approved` / `completed` / `complete` / `done` の同義畳み込みは
  既存のドメイン型 `Verdict::parse` が持っており、そこから `ReportedVerdict` を組むのは U7 の
  仕事である。対応が 1:1 であることは
  `every_reported_verdict_projects_onto_one_domain_verdict` が固定した。
- **出力に文言は無い。** `ReportOutcome` は材料（イベント・ステージ・`NextDecision`）だけを
  運ぶ。「Committed approve for "…" (scope: …)」の逐語は U7 の Presenter が組む。
- **`Err` はそのまま伝播する。** `ReportError::Repository(..)` / `ReportError::Command(..)` は
  伝播のための封筒であり、握り潰し・言い換え・再試行はしない（`Conflict` も再試行しない）。
- **楽観 version は `find_by_id` が返した値そのもの**を `store` に渡す。`aggregate.seq_nr()`
  からは導かない。`the_write_presents_the_version_the_rehydration_returned` が、通番 2 の集約に
  版 7 を持たせて「渡ったのは 7 である」ことを固定している。

### `stage` 引数について（申し送りあり）

`execute` の第 2 引数 `stage: Option<&StageSlug>` は upstream の `report --stage <slug>` に
対応する。`None` はカーソル。冪等経路（BR1.9）はカーソル以外を名指ししたときにしか成立
しないため、この引数が無いと実装できない。

slug から `StageIndex` への解決は、集約の公開読取モデル（`stages()` / `stage_index()`）を
使ってユースケース側の私有関数 `locate` で行っている。集約には同じ役割の私有関数 `resolve`
があるが、公開されていない。**ブリーフの「domain の変更は裁定 2 の範囲のみ」に従い、公開
クエリの追加は行わなかった。** 申し送りは §8 に記載する。

---

## 2. 裁定 2 — フェーズ境界の集約内部導出

### 導出規則（実装）

`modules/core/command/domain/src/orchestration/workflow_execution.rs` に私有クエリを 1 本
追加し、`approve_gate` から呼ぶ。

```rust
fn crossed_phase_boundary(&self, stage: StageIndex) -> Option<PhaseBoundary> {
    let from = self.entry(stage)?.phase();
    let to = self.entry(self.next_in_scope(stage)?)?.phase();
    (from != to).then(|| PhaseBoundary::new(from, to))
}
```

規則を言葉にすると 3 つである。

1. 承認するステージの `phase` と、**次の実効 EXECUTE ステージ**の `phase` が違う → その 2 つで
   `PhaseBoundary` を作る。
2. 同じフェーズ → `None`。
3. 次が無い（= 最終ステージ）→ `None`。

走査は既存の `next_in_scope`（= `next_in_scope_slug` が使っているもの）と**同一**である。
したがって実効 SKIP のステージや recompose オーバレイで畳まれたステージは跨いで数える。
`GateApproved` の `next_stage` と `phase_boundary` が同じ走査から出る以上、両者が食い違うことは
構造的に起きない。

### upstream との突き合わせ根拠

| 出典 | 実測した内容 | 導出規則との一致 |
|---|---|---|
| `.claude/tools/aidlc-state.ts:2266`（`handleAdvance`） | `const crossesPhaseBoundary = completedStage.phase !== nextStage.phase;` | 規則 1・2 と同一。「完了したステージ」と「次のステージ」のフェーズ比較そのもの |
| `.claude/tools/aidlc-state.ts:3196`（`handleSkip`） | `nextStage !== null && stage.phase !== nextStage.phase` | 規則 3（次が無ければ境界を立てない）と同一 |
| `.claude/tools/aidlc-state.ts` の `nextInScopeStage` 経由 | 次ステージの決定は state ファイルの EXECUTE/SKIP 上書きと既存チェックボックスを尊重した「次の実効 EXECUTE」 | 集約の `next_in_scope`（`effective_plan == Execute`）と同一の走査 |
| ゴールデン `tests/golden/upstream-3c3146cf/cli/report/approved-across-phases` | `delivery-planning`（inception）承認 → `PHASE_COMPLETED` の `From phase: inception` / `To phase: construction`、続いて `STAGE_STARTED: functional-design` | 規則 1。承認ステージが inception、次の実効 EXECUTE が construction の `functional-design` |
| ゴールデン `tests/golden/upstream-3c3146cf/cli/report/approved` | `practices-discovery` 承認 → 境界 3 行は**無く**、`STAGE_STARTED: requirements-analysis` が直後に来る | 規則 2。どちらも inception |
| RMU 側の既存実装 `projection.rs:816`（`crossed_phase_boundary`、`Jumped` 用） | `let from = …phase(); let to = …phase(); Ok((from != to).then(|| PhaseBoundary::new(from, to)))` | 同一の式。投影が `Jumped` に対して計画から導いているのと**同じ導出**を、`GateApproved` では集約が行う |

### 外形不変の根拠（受入基準 8 の前半）

- **`GateApproved` の payload 形は 1 バイトも変えていない。** フィールドは
  `stage` / `user_input` / `next_stage` / `phase_boundary` の 4 つのままで、型も
  `Option<PhaseBoundary>` のままである。変えたのは**誰がその値を作るか**だけで、
  `GateApproved::new` の引数も 4 本のままである（`workflow_execution_event.rs` の diff は
  doc コメント 3 か所のみ、実装行の変更 0）。
- **既存の呼出は全部 `None` を渡していた。** 実測した 6 箇所（domain 2・RMU 1・
  interface-adapter 1・app 2）はすべてリテラル `None` である。
- **各フィクスチャの計画で導出結果も `None` になる。** 実測:
  - `modules/core/read-model-updater/tests/support/mod.rs` / 同 `interface-adapter` /
    同 `app`: いずれも `[Initialization, Ideation, Ideation]`。`approve_gate` は索引 1
    （ideation）で発火し、次の実効 EXECUTE は索引 2（ideation）→ 同一フェーズ → `None`。
  - `modules/app/aidlc/tests/journal_protocol_conformance.rs`: 索引 0 のみ initialization、
    以降すべて inception。中間は inception → inception → `None`、最終は次が無い → `None`。
  - `modules/core/command/domain/tests/*`: 同様に `None`。
- したがって**ジャーナルに載る payload バイトは変わらない**。投影ゴールデン
  （`projection_golden_test.rs` 19 本）は `approve_gate` を呼んでおらず、
  **ファイル自体は無改変**である。

**ただし「19 本が全緑であること」自体は実測できていない** — §7 の追随が入るまで
`core-read-model-updater` のテストがコンパイルできないためである。ここは推定であって実測
ではない、と明示しておく。

### 置き換えたテスト

`a_gate_approval_carries_the_caller_supplied_phase_boundary`（呼出側が渡した境界がそのまま
載ることを見ていた）は削除し、導出を見る 4 本に置き換えた。§4 参照。

---

## 3. 裁定 1・3・4・5 の遵守

| 裁定 | 遵守の証拠 |
|---|---|
| 1. U5 は RMU を呼ばない | `modules/core/command/use-case/Cargo.toml` の `[dependencies]` は `core-command-domain` と `chrono` のみ。`[dev-dependencies]` は `tokio` のみ。`core-read-model-updater` / `core-command-interface-adapter` は grep 0 件。テストダブルは本クレート内の `#[cfg(test)] mod test_support` に置いた |
| 2. フェーズ境界は集約が導出 | §2 |
| 3. `StoreVersion` newtype 化は却下 | `StoreVersion` の grep は `modules/` `tools/` `docs/` で 0 件。ポート doc の「U5/U6 の境界強化候補として記録してある」を削除し、却下理由（本家 v3.0.0 が `expected_version: usize` で定めた語彙であり、包み直しは Conformist 方針への違反。`coding-rules/upstream-contracts.md`）を書いた |
| 4. 「再水和」を使わない | 本 Bolt で新規に書いた散文・doc コメントはすべて「再構成」。既存 48 ファイルと型名 `RehydratedWorkflowExecution` の一括置換は行っていない（対象外） |
| 5. FR2.2 は対象外 | レシート述語・verification 面には一切触れていない |

---

## 4. 追加・変更したテスト（全 28 本）

### domain（`workflow_execution.rs` — 追加 4・削除 1、合計 313 → 316）

| テスト | 何を固定するか |
|---|---|
| `a_gate_approval_derives_the_boundary_it_crosses_from_its_own_plan` | 規則 1（跨ぐ）。ideation → inception |
| `a_gate_approval_inside_one_phase_derives_no_boundary` | 規則 2（同一フェーズ） |
| `approving_the_last_stage_in_scope_derives_no_boundary` | 規則 3（最終ステージ） |
| `the_boundary_skips_over_stages_that_are_not_in_scope` | 走査が `next_in_scope` と同一（実効 SKIP を跨ぐ） |
| ~~`a_gate_approval_carries_the_caller_supplied_phase_boundary`~~ | **削除**（呼出側供給という前提そのものが失効したため） |

補助フィクスチャとして `start_from_phased_plan` / `phased` / `approval_boundary` を追加した。

### use-case（追加 24 本、合計 18 → 42）

`report_use_case.rs`（19 本）:

| テスト | 分類 |
|---|---|
| `an_awaiting_approval_report_opens_the_gate` | 正常系（経路） |
| `a_repeated_awaiting_approval_report_commits_nothing` | no-op（golden `awaiting-approval-repeat`） |
| `a_forward_report_on_a_gated_stage_approves_the_gate` | 正常系（経路） |
| `a_forward_report_on_an_ungated_stage_completes_the_stage` | 正常系（`gated` クエリでの分岐） |
| `a_rejected_report_carries_the_feedback` | 正常系（経路） |
| `a_revised_report_re_enters_the_gate` | 正常系（経路） |
| `a_skipped_report_carries_the_reason` | 正常系（経路） |
| `a_resume_report_routes_without_touching_the_aggregate` | no-op。**空のストアでも成功する**ことが「再構成すらしていない」証拠 |
| `a_re_report_of_a_stage_the_cursor_has_passed_commits_nothing` | 冪等（BR1.9） |
| `naming_the_cursor_explicitly_still_takes_the_normal_route` | 境界条件（`--stage` がカーソル自身） |
| `a_report_that_names_a_stage_outside_the_plan_is_refused` | 異常系（`UnknownStage`） |
| `a_report_that_names_a_stage_the_cursor_has_not_reached_is_refused_by_the_aggregate` | 異常系（`CommandError::NotStale` の伝播） |
| `the_write_presents_the_version_the_rehydration_returned` | 楽観 version の往復 |
| `a_missing_aggregate_is_reported_as_not_found` | **異常系（`find_by_id` が `NotFound`）** |
| `a_write_that_lost_the_race_is_reported_as_a_conflict` | **異常系（`store` が `Conflict`）**。再試行しないことも同時に固定 |
| `a_command_the_aggregate_refuses_is_propagated_verbatim` | 異常系（`CheckboxPrecondition` の逐語伝播 + 1 バイトも書かない） |
| `the_phase_boundary_comes_from_the_aggregate_not_from_the_use_case` | 層の境界（裁定 2 の横断確認） |
| `approving_the_last_stage_reports_no_next_stage` | 境界条件（`next_stage` / `phase_boundary` とも `None`） |
| `every_reported_verdict_projects_onto_one_domain_verdict` | 入力型とドメイン `Verdict` の 1:1 |

`report_error.rs`（5 本）: `a_repository_failure_is_carried_verbatim` /
`a_refused_command_is_carried_verbatim` / `an_unknown_stage_names_the_slug_it_could_not_resolve` /
`the_failure_is_a_std_error` / `failures_compare_by_value`。

### TDD の進め方（実測）

1. domain: 新しい 2 引数シグネチャを前提にしたテスト 4 本を先に書き、
   `cargo test -p core-command-domain --lib` で **red**（`E0061: this method takes 3 arguments
   but 2 arguments were supplied`）を確認。
2. `approve_gate` の引数廃止と `crossed_phase_boundary` を実装して **green**（316 本）。
3. use-case: `report_use_case.rs` に**テストモジュールだけ**を書き、`mod.rs` へ配線して
   **red**（`E0583: file not found for module report_error / report_outcome / reported_verdict`、
   `E0432: unresolved import ReportUseCase`）を確認。
4. 4 型を実装して **green**（42 本）。

### テストダブルの統合

ブリーフの選択肢どおり、ポートのテストにあった `FakeRepository` は削除し、
`test_support::InMemoryWorkflowExecutionRepository` に一本化した
（`coding-rules/no-backward-compatibility.md` — 同じ役割の口を 2 つ並立させない）。
`Conflict` の注入は `holding_after_a_concurrent_write` が担う。`find_by_id` が実際より 1 つ古い版を
返すことで「読んだ直後に別の書き手が 1 件書いた」状況を、本物の並行実行なしに再現する。

---

## 5. 受入基準の実行結果

**実行したコマンドと、その実際の結果**である。通っていないものは通っていないと書く。

| # | 基準 | コマンド | 結果 |
|---|---|---|---|
| 1 | `cargo fmt --all --check`（`tools/lint` も） | `CARGO_TARGET_DIR=$PWD/target-delegate cargo fmt --all --check` | **緑**（`FMT OK`）。`tools/lint` は本 Bolt で 1 行も触っていない |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | `… cargo clippy -p core-command-domain -p core-command-use-case --all-targets -- -D warnings` | **部分的に緑**。所有 2 クレートは緑。workspace 全体は §7 によりコンパイル不能で**未実行** |
| 3 | `cargo lint` | `CARGO_TARGET_DIR=$PWD/target-delegate-lint cargo lint` | **緑**（exit 0、所見 0 件） |
| 4 | `cargo test --workspace` | `… cargo test --workspace` | **未通過**。§7 の 4 箇所が `E0061` でコンパイル不能。所有クレート単体は緑（domain lib 316 / use-case lib 42 / domain 統合テスト 6） |
| 5 | `scripts/quint-gate.sh` | — | **未実行**。formal/ に一切触れていないため退行は原理的に無いが、実測していないので「通った」とは書かない |
| 6 | `scripts/coverage.sh` | — | **未実行**。基準 4 が通らないと計測できない |
| 7 | プロダクトコードに `unwrap` / `expect` 0 件 | clippy（`unwrap_used` / `expect_used` deny + `clippy.toml` のテスト許可）+ 各ファイルの `#[cfg(test)]` 前を対象にした grep | **緑**。新規・変更 9 ファイルすべて非テスト部 0 件 |
| 8 | 外形不変の証明 | `GateApproved` payload 形の diff 実測 + フィクスチャ計画の実測 | **半分のみ実測**。payload 形不変とフィクスチャの導出値が `None` であることは実測済（§2）。`projection_golden_test.rs` 19 本の実行は基準 4 と同じ理由で**未実測** |
| 9 | use-case の依存が `core-command-domain` 1 本 | `cat modules/core/command/use-case/Cargo.toml` + grep | **要注意（§6 参照）**。RMU / interface-adapter は dev-dependencies を含め 0 件で**基準の主旨は満たす**が、外部クレート `chrono` を `[dev-dependencies]` から `[dependencies]` へ移した。字義の「1 本のまま」は満たしていない |
| 10 | `StoreVersion` grep 0 件・ポート doc 更新 | `grep -rn StoreVersion modules/ tools/ docs/` | **緑**（0 件）。ポート doc は却下理由付きで更新済 |
| 11 | 報告書の内容 | 本ファイル | **完了** |

### 参考: baseline（origin/main 相当、本 Bolt 着手前に実測）

`CARGO_TARGET_DIR=$PWD/target-delegate cargo test --workspace` → **744 passed / 0 failed**。
ブリーフの「既存 234 テスト」は古い数値であり、退行の基準は 744 である
（内訳の主なもの: domain lib 313 / RMU lib 126 / core-infrastructure lib 103 /
interface-adapter lib 35 / projection_golden_test 19 / use-case lib 18）。

---

## 6. 判断が要った点（すべて明記する）

### (a) `chrono` を `[dependencies]` へ移した — 基準 9 の字義との差

ユースケースは `occurred_at: DateTime<Utc>` を集約コマンドへ渡す。集約は時計を持たない
（NFR3.1）ので、この型を名指しせずにユースケース本体を書くことはできない。`chrono` は
すでに同クレートの `[dev-dependencies]` にあり、workspace 依存として全層で使われている
外部クレートである。

`coding-rules/cqrs-boundaries.md` は「判定の『相手』は側のクレートであり、共有層と外部
ライブラリは対象外」と明記しているので、CQRS 境界の観点では違反ではない。基準 9 が
括弧で名指ししている「RMU / interface-adapter が現れないこと」も満たしている。
**ただし「依存が `core-command-domain` 1 本のまま」という字義は満たしていない**ので、
勝手に「基準 9 は緑」とは書かず、ここに明示する。

### (b) `RepositoryError::Conflict` の doc が裁定と矛盾していた

既存の doc に「ユースケースは再水和して 1 回だけ再試行する」とあり、ブリーフの固定裁定
「再試行の政策は持たない（`Conflict` も再試行しない）」と正面から矛盾していた。所有ツリー内
であり、かつ裁定の**適用**であって逸脱ではないと判断し、「ユースケースは再試行しない
（オーナー裁定 2026-08-29）。旧文は失効」へ是正した。委任者へは事前に報告済み。

### (c) `PhaseBoundary` / `GateApproved` の doc が裁定 2 で失効した

`phase_boundary.rs` の「**呼出側（ユースケース）が定義から導出して渡す投影材料**であり、
集約は検証せずイベントに載せるだけ」と、`workflow_execution_event.rs` の同趣旨 2 か所を
是正した。裁定 2 の範囲内の変更である。**実装行は 1 行も変えていない**（diff は doc のみ）。

### (d) `ReportOutcome` に `clippy::large_enum_variant` の allow を付けた

`Committed` がドメインイベントを丸ごと運ぶため 248 バイト対 40 バイトになる。`Box` 化すると
呼出側にデリファレンスと 1 回のヒープ確保を強いるだけで、ワンショット CLI が 1 起動につき
1 個だけ作る値には見合わない。理由付き `#[allow(..., reason = "…")]`（ポートの
`async_fn_in_trait` と同じ house style）で通した。

### (e) CQS の逸脱 — `execute(&mut self) -> Result<ReportOutcome, _>`

`coding-rules/command-query-separation.md` の既定（Command は戻り値なし）から外れる。
判定フロー 3 は「分離不能ならオーナー許可のうえ理由をコメントに書く」と定める。分離すると
2 つ目の呼出が別トランザクションになり、コミットの有無と結果が食い違いうるため分離不能と
判断し、doc に理由を書いた。**根拠は既存の house 先例**である — 集約コマンド自身が
`&mut self -> Result<WorkflowExecutionEvent, CommandError>` であり、ES の「1 コマンド 1
イベント」契約がそう要求している。オーナー許可は取っていないので、レビューで裁定されたい。

### (f) 止めて相談した点 — 裁定 2 と所有ファイル規律の衝突

§7。委任者へ 2 度報告し、回答を待った。

---

## 7. 未完了 — 裁定 2 の追随（禁止ツリー 3 ファイル 4 箇所）

`approve_gate` の引数を廃止すると、リテラル `None` を渡していた呼出側が全部コンパイル
できなくなる。`coding-rules/no-backward-compatibility.md` は互換オーバーロードも旧署名の
薄い委譲も禁じているので、逃げ道は無い。所有ツリー内の 2 箇所
（`modules/core/command/domain/tests/engine_loop_conformance.rs` /
`…/upstream_event_store_conformance.rs`）は追随済み。残るのは次である。

| ファイル | 行 | ブリーフ上の扱い |
|---|---|---|
| `modules/core/read-model-updater/tests/support/mod.rs` | 194 | **禁止**ツリー |
| `modules/app/aidlc/tests/journal_protocol_conformance.rs` | 265 | **禁止**ツリー |
| `modules/app/aidlc/tests/crash_reconstruction_test.rs` | 79, 206 | **禁止**ツリー |
| `modules/core/command/interface-adapter/tests/support/contract.rs` | 40 | 許可にも禁止にも**記載なし** |

必要な差分は `None,` 1 個の削除だけである。

```sh
perl -0pi -e 's/approve_gate\(([^,]+), None, at\(\)\)/approve_gate($1, at())/g' \
  modules/core/read-model-updater/tests/support/mod.rs \
  modules/app/aidlc/tests/journal_protocol_conformance.rs \
  modules/app/aidlc/tests/crash_reconstruction_test.rs \
  modules/core/command/interface-adapter/tests/support/contract.rs
```

§2 のとおり、この 4 箇所は導出結果も `None` なので**イベントのバイトは変わらない**。
それでもブリーフが明示的に禁じたツリーなので、独断では触れていない。

---

## 8. 申し送り

1. **`WorkflowExecution` に slug → `StageIndex` の公開クエリが要る。** 現状はユースケース側の
   私有関数 `locate` が `stages()` の走査で解決しており、集約の私有 `resolve` と実質同じ
   ロジックが 2 か所にある。`coding-rules/domain-services.md`（導出は所有する型へ）の観点では
   集約側に寄せるのが正しい。本 Bolt では「domain は裁定 2 の範囲のみ」に従って見送った。
2. **`skip_stage` のフェーズ境界。** upstream の `handleSkip` は最終ステージの skip で
   `PHASE_COMPLETED`（`To phase: (end)`）を出すが、`StageSkipped` payload には
   `phase_boundary` が無く、RMU の `stage_skipped` も境界行を描かない。U5 の責務外だが、
   FR1.1（監査シャードの逐語互換）に関わる既知の穴として記録する。
3. **`ReportOutcome::AlreadyDone` が運ぶ `NextDecision` は常に `Done`** である
   （`stale_report` の戻り値がそれしか無い）。U7 が分岐に使う予定が無ければ、将来もっと
   狭い型に絞れる。
4. **`report --single` / `--skeleton-stance`** は U5 の入力型に含めていない（ブリーフの
   非スコープ）。U7 が `ReportUseCase` に到達する前に分岐させる必要がある — upstream も
   `handleReport` の Branch -1 / Branch 0 で手前分岐している。
5. **基準 5・6（quint / coverage）は未実行**。§7 の追随後にメインセッションで回してほしい。
   coverage については、`ReportUseCase` の分岐はすべてテストで到達している一方、
   `test_support` は `#[cfg(test)]` なので計測対象外である。

---

## 9. 変更したファイル一覧

新規（`modules/core/command/use-case/src/orchestration/`）:

- `report_use_case.rs`（641 行 — うちテスト約 430 行）
- `reported_verdict.rs`（85 行）
- `report_outcome.rs`（52 行）
- `report_error.rs`（107 行）
- `test_support.rs`（219 行、`#[cfg(test)]`）

変更:

- `modules/core/command/domain/src/orchestration/workflow_execution.rs`（裁定 2 の実装 + テスト）
- `modules/core/command/domain/src/orchestration/phase_boundary.rs`（doc のみ）
- `modules/core/command/domain/src/orchestration/workflow_execution_event.rs`（doc のみ）
- `modules/core/command/domain/tests/engine_loop_conformance.rs`（呼出追随 1 箇所）
- `modules/core/command/domain/tests/upstream_event_store_conformance.rs`（呼出追随 1 箇所）
- `modules/core/command/use-case/Cargo.toml`（`chrono` を dependencies へ）
- `modules/core/command/use-case/src/orchestration/mod.rs`（ファサードの `pub use`）
- `modules/core/command/use-case/src/orchestration/workflow_execution_repository.rs`
  （裁定 3 の doc 更新 + テストダブル一本化）
- `modules/core/command/use-case/src/orchestration/repository_error.rs`（`Conflict` の doc 是正）

**`tests/golden/**` は 1 バイトも変更していない**（読むだけ）。`docs/` `formal/` `coding-rules/`
`.claude/` `modules/harness/` にも触れていない。
