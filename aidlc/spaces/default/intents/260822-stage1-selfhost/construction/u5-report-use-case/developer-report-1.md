# B11 開発者報告 1 — U5 `report` ユースケース（FR2.1）

対象ブリーフ: [`brief-1.md`](brief-1.md) / 裁定の経緯: [`decisions-1.md`](decisions-1.md)
ブランチ: `bolt/b11-u5-report-use-case`（origin/main 基準、**push なし**）
検証の `CARGO_TARGET_DIR`: `target-delegate`（`cargo lint` のみ `target-delegate-lint`）

---

## 0. 結論（先に）

**受入基準 1〜11 はすべて緑**である。固定裁定 1〜6、2026-08-29 に訂正された `Conflict`
再試行の裁定、所有ファイル規律の例外承認（`None,` 削除 5 箇所）、結線テスト 1 本の追加、
そして**裁定 7（改名・CQS 適合形への再設計・`Resumed` の入力型からの削除）**をすべて
反映した。

ワークスペース全体で **772 テスト全緑**（baseline 744 から +28、退行 0）。
カバレッジは head 98.53229%（絶対 90% 床・相対ゲートとも PASS）。

裁定 7 により、当初 §7 (e) で「レビューで裁定されたい」としていた CQS 逸脱は
**設計の変更で解消**した（例外は不要になった）。同じく §7 (d) の
`clippy::large_enum_variant` の allow も、`ReportOutcome` の削除とともに消えた。

字義と実装がずれている点が 1 つだけある — 受入基準 9 の「依存が `core-command-domain`
1 本のまま」に対し、外部クレート `chrono` を `[dev-dependencies]` から `[dependencies]` へ
移した。理由は §6 (a)。基準が括弧で名指しする「RMU / interface-adapter が現れないこと」は
dev-dependencies を含め満たしている。

---

## 1. 実装した経路と、対応する集約コマンド

`CommitVerdictUseCase::execute` の経路表（**裁定 7 反映後**）。左端はユースケースの入力型
`ReportedTransition` の変種、右端は打った集約コマンドである。**成功時の戻り値は常に
`Ok(())`** で、経路の区別は外へ出さない。

| 入力（正規化済み） | 対応する `Verdict` | 打つ集約コマンド | コミット | 戻り値 | 備考 |
|---|---|---|---|---|---|
| `AwaitingApproval { artifacts }` | `AwaitingApproval` | `open_gate(artifacts, at)` | する | `Ok(())` | |
| 同上・ただし現在の印が `[?]` | `AwaitingApproval` | **打たない** | **しない** | `Ok(())` | golden `awaiting-approval-repeat` は監査行・状態差分とも空 |
| `Forward { user_input }`・カーソルがゲート付き | `Forward` | `approve_gate(user_input, at)` | する | `Ok(())` | どちらを打つかは集約の `gated(cursor)` クエリで決める |
| `Forward { user_input }`・カーソルが非ゲート | `Forward` | `complete_stage(at)` | する | `Ok(())` | 同上（initialization フェーズ） |
| `Rejected { feedback }` | `Rejected` | `reject_gate(feedback, at)` | する | `Ok(())` | 改訂回数の +1 は集約が行う（BR1.4） |
| `Revised` | `Revised` | `revise_stage(at)` | する | `Ok(())` | |
| `Skipped { reason }` | `Skipped` | `skip_stage(reason, at)` | する | `Ok(())` | CONDITIONAL / 実効 SKIP の判定は集約（BR1.5） |
| `stage` 引数がカーソル手前のステージを名指し | 経路によらず先に判定 | `stale_report(index)`（クエリ。戻り値は捨てる） | **しない** | `Ok(())` | BR1.9 |

補足:

- **`resume` / `resumed` は入力型に無い**（裁定 7）。再開は遷移をコミットせず、コマンド名を
  提示するだけのルーティングなので、その分岐は Controller（U7）が**ユースケースへ届く手前で**
  行う（`use-case-rules.md` §3「resume 4 択はルーティング（コマンド名の提示）のみ」
  「Controller が手前で分岐する」）。upstream も `handleReport` が `RESUME_RESULTS` を早期に
  `handleResumeReport` へ振り分けており、同じ構造である。**FR2.1 の「resumed」はこの U7 側
  ルーティングで満たされる。** 本 Bolt の当初形は入力型に `Resumed` を持たせており、規則に
  反していた。
- **入力に生の文字列は無い。** `approved` / `completed` / `complete` / `done` の同義畳み込みは
  既存のドメイン型 `Verdict::parse` が持っており、そこから `ReportedTransition` を組むのは
  U7 の仕事である。対応が 1:1 であることは
  `every_reported_transition_projects_onto_one_domain_verdict` が固定した
  （`Verdict` 6 分類のうち、ここへ来るのは `Resume` を除く 5 つ）。
- **成功時の戻り値が無い。** 「何をコミットしたか」は合成ルート（U7）が catch_up 後の
  リードモデルから導く（§7 (e)）。
- **集約とポートの `Err` はそのまま伝播する。** 握り潰し・言い換えはしない。持っている
  再試行の政策は `Conflict` 1 回だけである（§2）。
- **楽観 version は `find_by_id` が返した値そのもの**を `store` に渡す。`aggregate.seq_nr()`
  からは導かない。`the_write_presents_the_version_the_rehydration_returned` が、通番 2 の集約に
  版 7 を持たせて「渡ったのは 7 である」ことを固定している。

### `stage` 引数について（申し送りあり）

`execute` の第 2 引数 `stage: Option<&StageSlug>` は upstream の `report --stage <slug>` に
対応する。`None` はカーソル。冪等経路（BR1.9）はカーソル以外を名指ししたときにしか成立
しないため、この引数が無いと実装できない。

slug から `StageIndex` への解決は、集約の公開読取モデル（`stages()` / `stage_index()`）を
使ってユースケース側の私有関数 `locate` で行っている。集約には同じ役割の私有関数 `resolve`
があるが公開されていない。**ブリーフの「domain の変更は裁定 2 の範囲のみ」に従い、公開
クエリの追加は行わなかった。** 申し送りは §8 (1)。

---

## 2. `Conflict` の再試行（2026-08-29 訂正裁定）

初版ブリーフの「`Conflict` も再試行しない」は C3 ③ の誤読として撤回され、正本は
`contract-design-questions.md` Q6 = A（「ユースケースが 1 回だけ再構成して再試行、それでも
競合なら CLI がエラー終了」）と確定した。実装はこれに従う。

```rust
match self.attempt(intent_id, stage, transition.clone(), occurred_at).await {
    Err(CommitError::Repository(RepositoryError::Conflict { .. })) => {
        self.attempt(intent_id, stage, transition, occurred_at).await
    }
    settled => settled,
}
```

- `attempt` は**再構成からコミットまでの 1 回分**である。したがって再試行は必ず
  `find_by_id` からやり直し、新しい版の集約に改めてコマンドを打つ。古い集約に `store` だけ
  打ち直すのは「読んだ時点の版で書く」という楽観ロックの意味そのものを壊す。
- `Conflict` は `store` からしか来ない（`find_by_id` は返さない）ので、この分岐は
  「書込が競合した」と同値である。
- 2 回目も `Conflict` なら伝播する。再試行後に集約がコマンドを拒否した場合も伝播する。
- `repository_error.rs` の `Conflict` doc は正しいので、**origin/main の内容へ戻した**
  （初版ブリーフに従って一度書き換えていた）。

**「再構成からやり直したこと」の証拠**: テストダブルは現在の版を提示した書込しか受理しない。
`a_first_conflict_is_retried_once_from_the_rehydration` は 1 件の割り込み書込を仕込んで
2 回目が成功することを見ており、もし古い集約に `store` だけ打ち直していれば 2 回目も競合する。
2 回目が通ったこと自体が再読み込みの証拠である。あわせて `store_attempts() == 2` で
「再試行は 1 回だけ」も固定した。

---

## 3. 裁定 2 — フェーズ境界の集約内部導出

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
| RMU の既存実装 `projection.rs:816`（`crossed_phase_boundary`、`Jumped` 用） | `let from = …phase(); let to = …phase(); Ok((from != to).then(\|\| PhaseBoundary::new(from, to)))` | 同一の式。投影が `Jumped` に対して計画から導いているのと**同じ導出**を、`GateApproved` では集約が行う |

### 外形不変の証明（受入基準 8）

- **`GateApproved` の payload 形は 1 バイトも変えていない。** フィールドは
  `stage` / `user_input` / `next_stage` / `phase_boundary` の 4 つのままで、型も
  `Option<PhaseBoundary>` のままである。`GateApproved::new` の引数も 4 本のままで、
  `workflow_execution_event.rs` の diff は **doc コメント 3 か所のみ・実装行の変更 0**。
- **既存の呼出 7 箇所はすべてリテラル `None` を渡していた。** 各フィクスチャの計画を実測すると
  導出結果も `None` になる:
  - RMU / interface-adapter / app の各 `tests/support/mod.rs`: いずれも
    `[Initialization, Ideation, Ideation]`。`approve_gate` は索引 1（ideation）で発火し、
    次の実効 EXECUTE は索引 2（ideation）→ 同一フェーズ → `None`。
  - `modules/app/aidlc/tests/journal_protocol_conformance.rs`: 索引 0 のみ initialization、
    以降すべて inception。中間は inception → inception → `None`、最終は次が無い → `None`。
  - `modules/core/command/domain/tests/*`: 同様に `None`。
  したがってジャーナルに載る payload バイトは変わらない。
- **`projection_golden_test.rs` は無改変で 19 本全緑**（実測。`git status` でも当該ファイル・
  `tests/golden/**` とも差分 0）。
- **ゴールデンパリティ 9 本・監査ブロックゴールデン 1 本・クラッシュ再構成 5 本**も全緑。

### 置き換えたテスト

`a_gate_approval_carries_the_caller_supplied_phase_boundary`（呼出側が渡した境界がそのまま
載ることを見ていた）は削除し、導出を見る 4 本に置き換えた。§5 参照。

---

## 4. 裁定 1・3・4・5・6・7 の遵守

| 裁定 | 遵守の証拠 |
|---|---|
| 1. U5 は RMU を呼ばない | `modules/core/command/use-case/Cargo.toml` の `[dependencies]` は `core-command-domain` と `chrono` のみ、`[dev-dependencies]` は `tokio` のみ。`core-read-model-updater` / `core-command-interface-adapter` は grep 0 件。テストダブルは本クレート内の `#[cfg(test)] mod test_support` に置いた |
| 2. フェーズ境界は集約が導出 | §3 |
| 3. `StoreVersion` newtype 化は却下 | `StoreVersion` の grep は `modules/` `tools/` `docs/` で 0 件。ポート doc の「U5/U6 の境界強化候補として記録してある」を削除し、却下理由（本家 v3.0.0 が `expected_version: usize` で定めた語彙であり、包み直しは Conformist 方針への違反。`coding-rules/upstream-contracts.md`）を書いた |
| 4. 「再水和」を使わない | 本 Bolt で新規に書いた散文・doc コメントはすべて「再構成」。既存 48 ファイルと型名 `RehydratedWorkflowExecution` の一括置換は行っていない（対象外） |
| 5. FR2.2 は対象外 | レシート述語・verification 面には一切触れていない |
| 6. 新規コードで `CorruptCause` への結合を増やさない（本体の是正は B11 着地後の追随 PR） | 実測: 新規 6 ファイルの `CorruptCause` / `Corrupt` 参照は **0 件**。本 Bolt の `modules/` 差分の**追加行**で `Corrupt` に触れた行も **0 件**。`repository_error.rs` は origin/main と**バイト同一**（差分 0）なので、裁定 6 の追随 PR は本 Bolt の diff と衝突しない |
| 7. 改名・CQS 適合形・`Resumed` 削除 | ①`CommitVerdictUseCase` / `CommitError` へ改名し、ファイル名・`mod.rs` の `pub use`・結線テスト・doc の言及もすべて追随（`modules/` `tools/` の旧名 grep は 0 件）。②`execute(...) -> Result<(), CommitError>` へ変更し `report_outcome.rs` を削除。何もコミットしない成功 2 経路も `Ok(())`。③`ReportedVerdict` ラッパーを削除し入力は `ReportedTransition`（5 経路）を直接受ける。resume の分岐が U7 の責務であることは型の doc に明記。④集約（domain）は触っていない — `approve_gate` がイベントを返す形もそのまま |

---

## 5. 追加・変更したテスト（新規 28 本）

### domain（`workflow_execution.rs` — 追加 4・削除 1、313 → 316）

| テスト | 何を固定するか |
|---|---|
| `a_gate_approval_derives_the_boundary_it_crosses_from_its_own_plan` | 規則 1（跨ぐ）。ideation → inception |
| `a_gate_approval_inside_one_phase_derives_no_boundary` | 規則 2（同一フェーズ） |
| `approving_the_last_stage_in_scope_derives_no_boundary` | 規則 3（最終ステージ） |
| `the_boundary_skips_over_stages_that_are_not_in_scope` | 走査が `next_in_scope` と同一（実効 SKIP を跨ぐ） |
| ~~`a_gate_approval_carries_the_caller_supplied_phase_boundary`~~ | **削除**（呼出側供給という前提そのものが失効したため） |

補助フィクスチャとして `start_from_phased_plan` / `phased` / `approval_boundary` を追加した。

### use-case（追加 24 本、18 → 42）

裁定 7 でテストは**戻り値アサートから効果の観測へ**書き換えた。`execute` は成功しても値を
返さないので、コミット内容はテストダブルに残った痕跡（`committed()` / `store_attempts()` /
`version()`）で固定する。no-op 経路は「`Ok(())` かつ `store_attempts() == 0`」である。

`commit_verdict_use_case.rs`（19 本）:

| テスト | 分類 |
|---|---|
| `an_awaiting_approval_report_opens_the_gate` | 正常系（経路） |
| `a_repeated_awaiting_approval_report_commits_nothing` | no-op（golden `awaiting-approval-repeat`）。`store_attempts() == 0` で「書込を試みない」まで固定 |
| `a_forward_report_on_a_gated_stage_approves_the_gate` | 正常系（経路） |
| `a_forward_report_on_an_ungated_stage_completes_the_stage` | 正常系（`gated` クエリでの分岐） |
| `a_rejected_report_carries_the_feedback` | 正常系（経路） |
| `a_revised_report_re_enters_the_gate` | 正常系（経路） |
| `a_skipped_report_carries_the_reason` | 正常系（経路） |
| `a_re_report_of_a_stage_the_cursor_has_passed_commits_nothing` | 冪等（BR1.9）。同じく `store_attempts() == 0` |
| `naming_the_cursor_explicitly_still_takes_the_normal_route` | 境界条件（`--stage` がカーソル自身） |
| `a_report_that_names_a_stage_outside_the_plan_is_refused` | 異常系（`UnknownStage`） |
| `a_report_that_names_a_stage_the_cursor_has_not_reached_is_refused_by_the_aggregate` | 異常系（`CommandError::NotStale` の伝播） |
| `the_write_presents_the_version_the_rehydration_returned` | 楽観 version の往復 |
| `a_missing_aggregate_is_reported_as_not_found` | **異常系（`find_by_id` が `NotFound`）** — ブリーフ必須 1/3 |
| `a_first_conflict_is_retried_once_from_the_rehydration` | **異常系（1 回目 `Conflict` → 2 回目成功）** — ブリーフ必須 2/3 |
| `a_second_conflict_is_propagated_without_a_further_retry` | **異常系（2 回とも `Conflict` → 伝播）** — ブリーフ必須 3/3 |
| `a_command_the_aggregate_refuses_is_propagated_verbatim` | 異常系（`CheckboxPrecondition` の逐語伝播 + 1 バイトも書かない） |
| `the_phase_boundary_comes_from_the_aggregate_not_from_the_use_case` | 層の境界（裁定 2 の横断確認） |
| `approving_the_last_stage_reports_no_next_stage` | 境界条件（`next_stage` / `phase_boundary` とも `None`） |
| `every_reported_transition_projects_onto_one_domain_verdict` | 入力型 5 変種とドメイン `Verdict` の 1:1（`Resume` は来ない） |

`commit_error.rs`（5 本）: `a_repository_failure_is_carried_verbatim` /
`a_refused_command_is_carried_verbatim` / `an_unknown_stage_names_the_slug_it_could_not_resolve` /
`the_failure_is_a_std_error` / `failures_compare_by_value`。

### interface-adapter（新規ファイル 1・テスト 1 本）

`tests/commit_verdict_use_case_wiring_test.rs` —
`the_use_case_commits_a_transition_through_the_real_repository`。契約 C3 ④ を満たす結線テスト。
実物の `WorkflowExecutionRepositoryImpl::in_memory()` を `CommitVerdictUseCase` に注入して
1 遷移をコミットし、**同じストアを指す別の口（`reopened()`）から再構成**して行が本当に載った
ことを確かめる（`seq_nr == 2` / `version == 2` / カーソルが `intent-capture` へ進み着手済み）。
検証が戻り値ではなく読み直しなのは仕様である — 合成ルートも catch_up 後のリードモデルを
読んで Presenter の材料を得る。網羅は use-case 側の fake テストが持つ。

### TDD の進め方（実測）

1. domain: 新しい 2 引数シグネチャを前提にしたテスト 4 本を先に書き、
   `cargo test -p core-command-domain --lib` で **red**（`E0061: this method takes 3 arguments
   but 2 arguments were supplied`）を確認。
2. `approve_gate` の引数廃止と `crossed_phase_boundary` を実装して **green**（316 本）。
3. use-case: `report_use_case.rs` に**テストモジュールだけ**を書き、`mod.rs` へ配線して
   **red**（`E0583: file not found for module report_error / report_outcome / reported_verdict`、
   `E0432: unresolved import ReportUseCase`）を確認（型名は裁定 7 で改名される前のもの）。
4. 4 型を実装して **green**。
5. `Conflict` 再試行の訂正裁定を受けて、再試行の 2 本を先に書き足してから `attempt` の
   括り出しとテストダブルの台本化を実装。
6. 裁定 7（改名 + CQS 適合形 + `Resumed` 削除）は**既存テストの書き換え**なので red-green の
   サイクルは踏んでいない。戻り値アサートを効果の観測へ移し、resume のテストを削除し、
   全ゲートを回し直して緑を確認した。

### テストダブルの統合と台本化

ブリーフの選択肢どおり、ポートのテストにあった `FakeRepository` は削除し、
`test_support::InMemoryWorkflowExecutionRepository` に一本化した
（`coding-rules/no-backward-compatibility.md` — 同じ役割の口を 2 つ並立させない）。

`Conflict` は「読んでから書くまでの間に別の書き手が入った」ときにしか起きないので、単一
スレッドのテストからは自然に起こせない。そこで**割り込む書込の回数**を台本として持たせた
（`holding_behind_concurrent_writes(aggregate, version, writes)`）。台本が残っている間、
`store` はストアの版だけを 1 つ進めて `Conflict` を返す。割り込んだ相手が書いた**内容**までは
模していない — 版の進行だけが再試行の観測に要る材料だからである。

---

## 6. 受入基準の実行結果（すべて実測）

| # | 基準 | コマンド | 結果 |
|---|---|---|---|
| 1 | `cargo fmt --all --check` | `CARGO_TARGET_DIR=$PWD/target-delegate cargo fmt --all --check` | **緑**（exit 0）。`tools/lint` は本 Bolt で 1 行も触っていない |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | 同上 | **緑**（exit 0） |
| 3 | `cargo lint` | `CARGO_TARGET_DIR=$PWD/target-delegate-lint cargo lint` | **緑**（exit 0、所見 0 件） |
| 4 | `cargo test --workspace` | `CARGO_TARGET_DIR=$PWD/target-delegate cargo test --workspace` | **緑**。**772 passed / 0 failed**（baseline 744 から +28、退行 0） |
| 5 | `scripts/quint-gate.sh` | `CARGO_TARGET_DIR=$PWD/target-delegate bash scripts/quint-gate.sh` | **緑**（exit 0、`[PASS] quint gate: all steps green`） |
| 6 | `scripts/coverage.sh` | `… bash scripts/coverage.sh --base origin/main` | **緑**。head 98.53229%。絶対ゲート `[PASS] … >= 90.0%`、相対ゲート `[PASS] head (98.53229%) >= base (98.51646%) - tolerance (0.01)` |
| 7 | プロダクトコードに `unwrap` / `expect` 0 件 | clippy（`unwrap_used` / `expect_used` deny）+ 各ファイルの `#[cfg(test)]` 前を対象にした grep | **緑**。新規・変更ファイルすべて非テスト部 0 件 |
| 8 | 外形不変の証明 | `GateApproved` payload 形の diff 実測 + フィクスチャ計画の実測 + `projection_golden_test.rs` の実行 | **緑**。payload 形不変（diff は doc のみ）、`projection_golden_test.rs` は**無改変で 19 本全緑**、`tests/golden/**` の差分 0。§3 |
| 9 | use-case の依存が `core-command-domain` 1 本 | `cat Cargo.toml` + grep | **要注意**。RMU / interface-adapter は dev-dependencies を含め 0 件で**基準の主旨は満たす**が、外部クレート `chrono` を `[dev-dependencies]` から `[dependencies]` へ移した。下記 (a) |
| 10 | `StoreVersion` grep 0 件・ポート doc 更新 | `grep -rn StoreVersion modules/ tools/ docs/` | **緑**（0 件）。ポート doc は却下理由付きで更新済 |
| 11 | 報告書の内容 | 本ファイル | **完了** |

### baseline（origin/main 相当、本 Bolt 着手前に実測）

`cargo test --workspace` → **744 passed / 0 failed**。ブリーフの「既存 234 テスト」は古い数値で
あり、退行の基準は 744 である（主な内訳: domain lib 313 / RMU lib 126 /
core-infrastructure lib 103 / interface-adapter lib 35 / projection_golden_test 19 /
use-case lib 18）。+28 の内訳は domain +3・use-case +24・interface-adapter 結線テスト +1。

---

## 7. 判断が要った点（すべて明記する）

### (a) `chrono` を `[dependencies]` へ移した — 受入基準 9 の字義との差

ユースケースは `occurred_at: DateTime<Utc>` を集約コマンドへ渡す。集約は時計を持たない
（NFR3.1）ので、この型を名指しせずにユースケース本体は書けない。`chrono` はすでに同クレートの
`[dev-dependencies]` にあり、workspace 依存として全層で使われている外部クレートである。

`coding-rules/cqrs-boundaries.md` は「判定の『相手』は側のクレートであり、共有層と外部
ライブラリは対象外」と明記しているので CQRS 境界の観点では違反ではない。基準 9 が括弧で
名指ししている「RMU / interface-adapter が現れないこと」も満たしている。
**ただし「1 本のまま」という字義は満たしていない**ので、勝手に緑とは書かずここに明示する。

### (b) `PhaseBoundary` / `GateApproved` の doc が裁定 2 で失効した

`phase_boundary.rs` の「**呼出側（ユースケース）が定義から導出して渡す投影材料**であり、
集約は検証せずイベントに載せるだけ」と、`workflow_execution_event.rs` の同趣旨 2 か所を
是正した。裁定 2 の範囲内の変更であり、**実装行は 1 行も変えていない**。

### (c) `repository_error.rs` の `Conflict` doc — 一度書き換えて、戻した

初版ブリーフの「再試行しない」に従って一度是正したが、訂正裁定（doc は正しい・変更しない）を
受けて **origin/main の内容へ完全に戻した**。本 Bolt の diff にこのファイルは含まれない。

### (d) `ReportOutcome` の `clippy::large_enum_variant` allow — **裁定 7 で消滅**

`ReportOutcome` は `Committed` にドメインイベントを丸ごと載せていたため 248 バイト対
40 バイトになり、理由付き `#[allow(clippy::large_enum_variant, reason = "…")]` を置いていた。
裁定 7 で型そのものを削除したので、**この例外も無くなった**。本 Bolt が最終的に持ち込んだ
lint の allow は、`commit_verdict_use_case.rs` のテストモジュールの `clippy::panic` と、
結線テストの file-level `unwrap_used` / `expect_used` / `panic`（既存 integration test と
同じ house style）だけである。

### (e) CQS の逸脱 — **裁定 7 で設計変更により解消**

当初は `execute(&mut self) -> Result<ReportOutcome, _>` とし、「分離すると 2 つ目の呼出が別
トランザクションになる」を理由に `command-query-separation.md` 判定フロー 3 の例外（オーナー
許可が要る）として提示していた。**この提示は 2 度とも誤りで、オーナーに却下された。**

却下の理由が正しい:

- この CLI では `ReadModelUpdater` の catch_up が**同一プロセス内で同期実行される**ので、
  コミット直後の U7 はリードモデルから「何が起きたか」を読める。両側を知ってよいのは合成
  ルートなので境界違反にもならない。私の「リードモデルからは読めない」という前提が誤りだった。
- 「別トランザクションで食い違う」という擁護も成立しない。Q6 が既に「同一クローンの同時実行は
  稀」と裁定済みで、かつ戻り値にしても同じ競合は起きる。
- 集約コマンドが `&mut self -> Result<WorkflowExecutionEvent, _>` である事実は**先例にならない**。
  あれは `store()` へ渡す書込パイプラインの配管であって、Presenter に読ませる読取チャネル
  ではない。私はこの 2 つを「同じ家風」と混同していた。

現在の形は `execute(...) -> Result<(), CommitError>` で、規則が許す Command そのものである。
例外は不要になった。

### (g) 改名 — `ReportUseCase` → `CommitVerdictUseCase`（裁定 7）

`report` は upstream CLI の動詞（コンダクタが結末を報告する = 遷移コミット）であって帳票では
ないが、**プロジェクトを最も知る読者にすら「レポートを作る／読むユースケース」と誤読された**。
オーナー裁定により、更新の意図を先頭に置く名前へ改めた。エラーも `ReportError` →
`CommitError`。CLI 動詞と型の対応は U7 の ROUTES 表が持つので、型名が upstream の綴りに
縛られる必要はない、という整理である。`coding-rules/use-case-rules.md` の適用例の更新は
メインセッションが行った（私は coding-rules 禁止のまま）。

### (f) 止めて相談した点 — 裁定 2 と所有ファイル規律の衝突

`approve_gate` の引数廃止で所有ツリー外の呼出側がコンパイル不能になることを着手直後に報告し、
独断で触れずに回答を待った。結果、**`None,` の削除に限り 5 箇所の編集が承認**され、あわせて
初版ブリーフの `Conflict` 裁定の誤りも判明した（`decisions-1.md` の「訂正」節）。
実際に触ったのはその 5 箇所の `None,` だけである。

**当たり範囲の検証手順について 1 点、順序が指示と違う。** 委任者は「一括置換は当てる前に
dry-run 相当で確認し、当てた後に `git diff` で目視確認する」よう指示したが、私は**当てた後の
`git diff` 確認しか行っていなかった**。事後に origin/main を一時展開して dry-run 相当を取り直し、
結果が一致することを確かめた:

- origin/main 全体の `approve_gate(` 呼出は **18 行**（うち `modules/core/command/domain/src/`
  の 11 行は同一ファイル内のテスト、`domain/tests/` が 2 行、承認 4 ファイルが 5 行）。
- 承認 4 ファイルに限れば、置換パターンに当たるのは**承認された 5 行ちょうど**である。
- `git diff origin/main..HEAD --numstat` の実測も 2 + 1 + 1 + 1 = **5 行**で一致する。
- 承認外ツリー（`modules/app/**`・`modules/core/read-model-updater/**`・`modules/harness/**`）で
  本 Bolt が触れたファイルは、承認された 3 ファイルだけである（`modules/harness/**` は 0 件）。

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
3. **CLI サブコマンドの出力データはクエリユースケース経由で得る**（裁定 7 追補・同訂正）。
   `CommitVerdictUseCase` は成功時に何も返さないので、経路は
   **コマンドユースケース → RMU（リードモデル投影）→ クエリユースケース**になる。
   クエリユースケースは `core/query/*` の新設モジュール群（`cqrs-boundaries.md` が
   「将来のリードモデル読取・クエリ API 層」として既に予約している枠の実体）であり、
   **ドメイン（`core-command-domain`）には絶対依存しない** — 返すのはクエリ側が所有する
   読取用の型で、コマンド側のイベント・集約は現れない。文言は従来どおり出す側
   （U7 Presenter）が組み、クエリユースケースはデータを返すだけである。

   **クエリ向きのリードモデルは `.md` に限らず、SQLite テーブル投影でよい。** `aidlc-state.md`
   と監査シャードは「upstream 互換・人間可読・git 交換用」のリードモデル（バイト逐語が契約）で
   あり、これをクエリユースケースが逆パースする設計にはしない。RMU が同じイベント列から
   別のテーブルへ投影してよく、**既存設計が既に複数投影を予約している** — チェックポイント表は
   `projection` をキーに持ち、投影ごとに独立して前進できる（`ProjectionName` 型も既存）。
   複数投影の駆動（現行 `ReadModelUpdater` は単一投影を駆動する形）と具体的なテーブル設計は
   **U7（FR4）の課題**である。
4. **`report --single` / `--skeleton-stance` / `resume`** は U5 の入力型に含めていない
   （前 2 つはブリーフの非スコープ、`resume` は裁定 7）。いずれも U7 が
   `CommitVerdictUseCase` に到達する前に分岐させる必要がある — upstream も `handleReport` の
   Branch -1 / Branch 0 / `RESUME_RESULTS` の早期分岐で手前分岐している。
5. **`CommitVerdictUseCase` が `Conflict` を再試行するのは 1 回だけ**で、その回数はコードに直書き
   （`match` の 1 段）である。将来 U6 も同じ政策を持つなら、方針の重複を避ける置き場所を
   考える必要がある。
6. **`target-delegate/` と `target-delegate-lint/`** が未追跡のまま残っている（`.gitignore` は
   `/target` しか除外していない）。検証用のビルド生成物なので削除して構わない。

---

## 9. 変更したファイル一覧

新規（`modules/core/command/use-case/src/orchestration/`）:

- `commit_verdict_use_case.rs`（テスト込み）
- `reported_transition.rs`
- `commit_error.rs`
- `test_support.rs`（`#[cfg(test)]`）

新規（`modules/core/command/interface-adapter/tests/`）:

- `commit_verdict_use_case_wiring_test.rs`（契約 C3 ④ の結線テスト 1 本）

裁定 7 で消えたもの（一度作って削除した）:

- `report_outcome.rs`（`ReportOutcome`）— `execute` が値を返さなくなったので不要
- `ReportedVerdict` ラッパー（`Resumed` を持っていた 2 段目）— `ReportedTransition` へ一本化

変更:

- `modules/core/command/domain/src/orchestration/workflow_execution.rs`（裁定 2 の実装 + テスト）
- `modules/core/command/domain/src/orchestration/phase_boundary.rs`（doc のみ）
- `modules/core/command/domain/src/orchestration/workflow_execution_event.rs`（doc のみ）
- `modules/core/command/domain/tests/engine_loop_conformance.rs`（`None,` 削除 1 箇所）
- `modules/core/command/domain/tests/upstream_event_store_conformance.rs`（`None,` 削除 1 箇所）
- `modules/core/command/use-case/Cargo.toml`（`chrono` を dependencies へ）
- `modules/core/command/use-case/src/orchestration/mod.rs`（ファサードの `pub use`）
- `modules/core/command/use-case/src/orchestration/workflow_execution_repository.rs`
  （裁定 3 の doc 更新 + テストダブル一本化）

**例外承認による所有ツリー外の追随**（`None,` 削除のみ・他の行は 1 行も触っていない）:

- `modules/core/read-model-updater/tests/support/mod.rs:194`
- `modules/core/command/interface-adapter/tests/support/contract.rs:40`
- `modules/app/aidlc/tests/crash_reconstruction_test.rs:79, 206`
- `modules/app/aidlc/tests/journal_protocol_conformance.rs:265`

**`tests/golden/**` は 1 バイトも変更していない**（読むだけ）。`docs/` `formal/` `coding-rules/`
`.claude/` `modules/harness/` にも触れていない。
