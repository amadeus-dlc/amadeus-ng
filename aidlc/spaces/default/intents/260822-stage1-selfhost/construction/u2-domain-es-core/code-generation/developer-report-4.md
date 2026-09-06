# 開発者報告 4 — u2-domain-es-core / Bolt b51 委任 2（ステップ 2〜4）

- **計画**: `code-generation-plan.md`（fingerprint `sha256:dd1170c1a75b16e30a351f34d9f4ff57164bcbe65482361e94e6909de7f0634d`）
- **テスト契約**: `sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3`
- **ワークツリー**: `/Users/j5ik2o/orca/workspaces/amadeus-ng/stage1-selfhost`（ブランチ `stage1-selfhost`）
- **前提コミット**: `3edf320d feat(domain): FCC 11 型を新設し契約試験へ登録`（委任 1）

## 1. 結論

委任 1 が起こしたファーストクラスコレクション（以下 FCC）11 型へ、集約・イベント・
境界・兄弟クレートを一斉に載せ替えた。ワークスペースは全ゲート緑である。後方互換 API
は残していない。DTO のバイト表現は変えていない。

コミットは 1 本のみ:

| SHA | 件名 | `cargo check --workspace` |
| --- | --- | --- |
| `dd20266a` | `refactor: switch the aggregates and boundaries onto the first-class collections` | 通る |

規模は 67 ファイル、2329 挿入 / 1893 削除。

## 2. なぜ 1 コミットなのか（実測付き）

計画は「意味単位で分けてよい」としていたが、**分けられなかった**。ドメインの公開 API
（`stage_keys()` の削除、`IntentExecution::new` の引数変更、`StageEntry::check_plan` の
移設、イベント payload の型変更）は 4 つの兄弟クレートが同時に消費しているため、
ドメインだけをコミットした状態はコンパイルできない。

これは推測ではなく実測した。ドメインのみをステージし、兄弟クレートの変更を一時退避
した状態で:

```
$ cargo check --workspace
error[E0308]: mismatched types
error[E0599]: no method named `to_vec` found for reference `&ArtifactPaths` in the current scope
error[E0599]: no method named `iter` found for reference `&StageEntries` in the current scope
error[E0599]: no function or associated item named `check_plan` found for struct `StageEntry` in the current scope
error[E0599]: no method named `iter` found for reference `&PromotedSections` in the current scope
error[E0599]: no method named `to_vec` found for reference `&RuleLines` in the current scope
（計 42 件）
exit=101
```

退避は名前付き（`git stash push -u -k -m`）で行い、SHA を控えて `apply` で戻し、
タグで再検索して `drop` した。裸の `git stash` / `git stash pop` は使っていない。

## 3. TDD の記録（レイヤーごと）

### 3.1 ドメイン層（`core-command-domain`）

**Red（時系列どおり）**。FCC への切替を先に書いた集約テストと、新しい振る舞いの
テストが、切替前の実装に対して落ちた。切替着手時点のドメインテストのビルドエラーは
約 94 件で、先頭は次の形である。

```
error[E0599]: no method named `stage_keys` found for reference `&IntentExecution` in the current scope
error[E0061]: this function takes 11 arguments but 9 arguments were supplied
error[E0308]: mismatched types
   expected `&StageIndexSet`, found `Vec<StageIndex>`
```

新しい振る舞いのうち `next_decision` の取り違えガードは、ガードを書く前にテスト
（`next_refuses_to_answer_for_a_foreign_intent`）を置き、`Ok(RunStage { .. })` が返る
ことで落ちるのを確認してから実装した。

**Green**。695 → 699 テスト全緑（本報告時点。契約試験 2、ITF 準拠 1、doc-test 3 は別勘定）。

**Refactor**。集約の内部に 6 つの私有ヘルパを切り出した（`all_positions` /
`positions_after` / `mark_stage` / `record_approval` / `reset_attempt` /
`override_all`）。いずれも `StageSlots` への同型の書き込みを 1 か所へ寄せたもので、
外向きの振る舞いは変えていない。

### 3.2 interface-adapter 層（DTO）

**Red**。`IntentExecutionDto` の 7 列展開が `stage_keys()` を失って落ちた。

```
error[E0599]: no method named `stage_keys` found for reference `&IntentExecution` in the current scope
```

**Green**。私有の `SlotColumns` を畳み込み先に置き、`StageSlots` を 1 回の
`fold_left` で 7 列へ展開する形にした。`to_domain` は列長の不一致を
`DtoDecodeError::InvariantViolation` で拒み、`StageSlot` を位置ごとに組み直してから
`StageSlots::new` → `IntentExecution::new` を通す。

**Refactor**。列ごとに `Vec` を回していた 7 本の走査を 1 本の畳み込みへ統合した。

### 3.3 read-model-updater 層

**Red（新振る舞い 2 件は反転で検出力を確認）**。この 2 件は実装が先に入っていたため、
時系列の Red ではない。テストを書いたうえで**実装を反転させて落ちることを確認**し、
戻して緑に復した。誤って「Red 先行」と記録しないためにそのまま書く。

1. `NextAnswerRow::of` の `Err` 経路 — テスト
   `a_foreign_intent_yields_no_answer_row_but_a_missing_material_error`。
   `next_decision` の取り違えガードを外すと落ちる。

   ```
   thread 'a_foreign_intent_yields_no_answer_row_but_a_missing_material_error' panicked at
   modules/core/read-model-updater/tests/read_tables_test.rs:1202:10:
   取り違えは行にならない: NextAnswerRow { id: "f0e43640...", execution_id: "0190aaaa-...",
   request_kind: "bare", decision_kind: "run-stage", stage_index: Some(2), ... }
   ```

2. `Recomposed` の投影順 — テスト
   `the_recomposed_spelling_follows_the_document_order_not_the_alphabet`。
   文書順の並べ直しを辞書順の畳み込みへ戻すと落ちる。

   ```
   thread '...the_recomposed_spelling_follows_the_document_order_not_the_alphabet' panicked at
   modules/core/read-model-updater/src/workspace/projection.rs:2920:9:
   実際:
   ## Plan Recomposed
   **Stages skipped**: alpha, zulu
   ```

   期待は `zulu, alpha`（計画の文書順）。テストは文書順と辞書順が逆になる計画
   （`zulu` が `alpha` より前）を専用に組んでいるので、順序以外の理由では落ちない。

**Green / Refactor**。`in_document_order(plan, slugs) -> Vec<String>` を投影に置き、
監査行と行末トークンの両方をそこから作る。`stage_list` は `&[String]` を取る形へ
狭めた。

### 3.4 use-case / app / query テスト

**Red**。`CommitOutcome::Committed.steps` の型変更と `TransitionSteps` の畳み込み
API 化により、切替着手時点で兄弟クレートに 89 件のビルドエラーが出た。

**Green**。`runtime.rs` の `committed_transition` を `is_single` / `fold_left` で書き
直し、`scaffold.rs` の走査を `slugs_of` / `first_post_initialization` に置き換えた。

## 4. ステップ 4 検収（a)〜(g)

### (a) ゲート一式

| コマンド | 結果 |
| --- | --- |
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo lint` | exit 0 |
| `PROPTEST_RNG_SEED=20260823 cargo test --workspace` | exit 0 / 2354 passed / 0 failed |
| `bash scripts/quint-gate.sh` | exit 0（`[PASS] quint gate: all steps green`） |
| `cargo audit` | exit 0（125 crate 依存を走査、脆弱性なし） |
| `cargo audit --file tools/lint/Cargo.lock` | exit 0（5 crate 依存を走査、脆弱性なし） |

`cargo-audit` は導入済み（0.22.2）である。Quint は不変条件 run 3 本、witness 19 本、
`quint test --match 'r_.*'` がすべて PASS で、モデル（`formal/**`）には触れていない。

### (b) `scripts/coverage.sh` を同一条件で 2 回

| 実行 | head line coverage | 絶対床 90% |
| --- | --- | --- |
| 1 回目 | 99.15169660678644% | PASS |
| 2 回目 | 99.15169660678644% | PASS |
| 差 | 0.00 | — |

シードは `scripts/coverage.sh` が固定しており、2 回のバイト一致で PBT のシード非固定に
よる揺れが無いことを確認した。PR 相対ゲートは base を必要とするためローカルでは
発火しない（CI 側の判定に委ねる）。

### (c) `core-command-domain` の行カバレッジ

| 対象 | 行カバレッジ | 床 |
| --- | --- | --- |
| パッケージ全体 | 98.90%（15466 行中 170 未到達） | 98.87% を上回る |
| `orchestration` のみ（`workflow_definition` / `workspace` を除外） | 99.38%（9185 行中 57 未到達） | — |

**測定の途中経過を明記する。** 切替直後の初回測定は **98.78%** で床を 0.09pt 下回った。
未到達行を洗い出したところ、私の変更が作った穴が特定できたので、次の 6 本のテストを
足して 98.90% へ戻した。床を下げる調整は一切していない。

- `workspace/promotion_plan_error.rs`（7 行）— 新設した `DuplicateSection` 変種の
  `Display` と `From<PromotedSectionsError>`、および `source()` が `None` であること。
- `orchestration/intent.rs`（6 行）— `review()` と `created_at()` のアクセサ。
  `should_panic` の再構成テスト 4 本を差し替えた際に呼出が消えていた。
- `orchestration/stage_entries.rs`（1 行）— 先頭以外の initialization ステージが
  EXECUTE でない場合の拒否。`check_plan` から移設した検査のうち、走査側の口が
  未到達だった。
- `orchestration/command_error.rs`（1 行）— `IntentMismatch` の `Display`。
- `orchestration/next_decision.rs`（1 行）— 網羅 match の `RunStage` 腕。
  「8 決定を網羅」と名乗るテストが `RunStage` だけ呼んでいなかった（既存の穴）。
- `intent_execution_event/review_completed.rs`（3 行）— `reviewer()` アクセサ。

### (d) `PlanAction` の所有

```
$ rg -n -e 'enum PlanAction' -e 'pub use .*PlanAction' modules/core/command/domain/src/orchestration
（0 hits）

$ rg -n -e 'enum PlanAction' -e 'pub use .*PlanAction' modules/core/command/domain/src/workflow_definition
workflow_definition/mod.rs:97:pub use plan_action::PlanAction;
workflow_definition/plan_action.rs:7:pub enum PlanAction {
```

`orchestration` は 0 件、`workflow_definition` が 2 件で所有している。PASS。

### (e) `# Panics` の総数

```
$ rg -n '# Panics' modules/core/command/domain/src
orchestration/intent_execution.rs:40:  （ヘッダ doc の本文 — 検収対象外）
orchestration/intent_execution.rs:347
orchestration/intent_execution.rs:1506
orchestration/intent_execution.rs:2364
workflow_definition/workflow_definition.rs:213
```

ヘッダ doc の 1 件を除くと `intent_execution.rs` に 3 件、`workflow_definition.rs` に
1 件。委任前と同数で増えていない。PASS。

### (f) 依存の不変（NFR4.1）

```
$ git diff --stat origin/main..HEAD -- Cargo.lock Cargo.toml modules/core/command/domain/Cargo.toml
（空）

$ git status --porcelain -- Cargo.lock Cargo.toml modules/core/command/domain/Cargo.toml
（空）
```

コミット済み差分にも作業ツリーにも一切の変更が無い。`tests/`・`formal/`・`scripts/`・
`.github/` も `git diff --stat origin/main..HEAD` が空であることを確認した。PASS。

### (g) スライス返しと `to_vec`、読取側の FCC 保持

```
$ rg -n 'pub fn .*-> &\[' modules/core/command/domain/src/orchestration modules/core/command/domain/src/workspace
（0 hits）

$ rg -n 'to_vec\(\)' modules/core/command/domain/
（0 hits）
```

`orchestration` / `workspace` にスライス返しのアクセサは無く、ドメイン全体（テストを
含む）に `to_vec()` は 1 件も残っていない。**残存理由を書く対象がそもそも無い。**

`workflow_definition` には `&[..]` を返すアクセサが 14 本あるが（`stage_node.rs` の
`produces` / `consumes` など）、これは静的定義グラフ側であり本委任の射程外である。

読取側（`core-read-model-updater` / `core-query-*`）の FCC 型の出現は、次のいずれか
にしか無いことを目視と機械検索の両方で確認した。

- **DTO の復号境界** — `StageEntries::new(..)` / `StageSlugSet::new(..)` /
  `ArtifactPaths::new(..)` / `PromotedSections::new(..)` / `RuleLines::new(..)` を
  呼んでドメインのイベントへ渡す。ドメイン型を組んで手放すだけで、保持しない。
- **DTO の符号化境界** — `slug_column(&StageSlugSet) -> Vec<String>` /
  `rule_column(&RuleLines) -> Vec<String>`。参照で受けて平坦な列へ写す。
- **投影の即時読取** — `in_document_order(&ResolvedPlan, &StageSlugSet) -> Vec<String>`。
- **テストのフィクスチャ**。

構造体フィールドと戻り値型の機械検索は次のとおりで、製品コードの保持はゼロである。

```
$ rg -n ':\s*(StageEntries|StageSlots|StageSlot|StageIndexSet|StageSlugSet|ArtifactPaths|TransitionSteps|ReviewClosures|PendingIterations|PromotedSections|RuleLines)\s*,?$' \
    modules/core/read-model-updater/src modules/core/query
（0 hits）

$ rg -n -e '-> *&? *(StageEntries|...|RuleLines)\b' modules/core/read-model-updater/src modules/core/query
read-model-updater/src/orchestration/dto/tests.rs:66:  fn stages() -> StageEntries   （テストのフィクスチャ）
query/interface-adapter/tests/support/mod.rs:181: fn stages() -> StageEntries   （テストのフィクスチャ）
```

`-> Result<StageSlugSet, DtoDecodeError>` が DTO 復号に 1 件あるが、これは復号境界が
ドメイン型を組んで返す経路であり、読取側が保持する型ではない。

### 後方互換の不在

```
stage_keys      … 3 件すべて別物（CLI ゴールデンのテスト名、`stage_slot.rs` の旧実装への
                  言及コメント、DTO テストの関数名）。API は無い。
check_plan      … 0 件（DTO テストに残っていた 2 件の古い言及コメントは
                  `StageEntries::new` へ書き改めた）。
#[deprecated]   … 0 件。
pub use .. as   … 2 件。いずれも RMU の自由関数の別名（`read as read_state_file` /
                  `write_atomic as write_state_file`）で、FCC とは無関係の既存物。
```

製品コードの `unwrap` / `expect` は `clippy` の `unwrap_used` / `expect_used` = deny が
機械強制しており、`-D warnings` が exit 0 であることが証拠である。

## 5. ゴールデン / ITF の観測

すべて緑で、フィクスチャは 1 バイトも変えていない。

| テスト対象 | 結果 |
| --- | --- |
| `engine_loop_conformance`（ITF 準拠） | 1 passed |
| `journal_protocol_conformance`（ITF 準拠） | 5 passed |
| `upstream_event_store_conformance` | 10 passed |
| `golden_parity_test` | 11 passed |
| `projection_golden_test` | 18 passed |
| `audit_block_golden_test` | 1 passed |
| `cli_golden_test` | 5 passed |
| `golden_corpus_read` | 14 passed |
| `golden_hash_canonical` | 7 passed |

**P5（DTO バイト不変）の担保**。`IntentExecutionDto` の 7 列と各イベント DTO の JSON
形は同一である。危なかったのは `Recomposed` の 1 か所だけで、そこは `StageSlugSet` の
**辞書順を変えずに**投影側で文書順へ並べ直して解いた。型の意味には手を付けていない。

## 6. 計画 §2 からの逸脱

いずれも実装上の必要から生じたもので、外向きの契約は変えていない。

1. **`recompose` の引数を `&StageIndexSet` にした**（計画は値渡し）。
   `clippy::needless_pass_by_value` が deny のため、値で受けると `-D warnings` を
   通せない。集約は集合を消費しないので参照で十分である。

2. **`TransitionSteps::recovered_approval()` を新設した**。`TransitionSteps::new` は
   `Result` を返すが、製品コードで `unwrap` できないため、
   `[GateStartRecovered, Approve]` の 2 段を返す全域構築子を足した。委任 1 の型への
   追加であり、既存 API は変えていない。

3. **`StageSlots::override_plan_all(&mut self, &StageIndexSet, PlanAction)` を新設した**。
   `override_all` ヘルパの書き込み先として要る。

4. **`PromotionPlanError::DuplicateSection(String)` と
   `From<PromotedSectionsError>` を新設した**。`PracticesPromotion::plan` の中で
   `PromotedSections::new` が `Result` を返すため、握り潰さずに写す先が要る。

5. **`ReviewAttempt::restored` は pending を `Vec<u32>` で受ける**。`PendingIterations`
   が `pub(crate)` であり、DTO からは構築できないため。境界の例外として doc に明記した。
   closed は `ReviewClosures` を受ける。

6. **`ReviewAttempt::pending()` を `pending_iterations() -> Vec<u32>` に改名した**。
   FCC を返さない読取用アクセサであることを名前で示すため。

7. **`CommitOutcome::Committed.steps` を `TransitionSteps` にした**。

8. **`NextAnswerRow::of` を可謬にした**。`next_decision` の `Err` を握り潰さず、
   既存の `ReadTablesError::IntentUnavailable`（`execution_id` / `intent_id` を運ぶ）
   へ写す。**新しい変種は足していない** — この経路の意味は「材料が揃わない」であり、
   既存変種がそれを正確に言うためである。

9. **構造的に不能になったテストを書き換えた**。列長の不一致、`Intent::from` /
   `Started::new` に破れた計画を渡す `should_panic` の 4 本などは、検査点が
   `StageEntries::new` / `StageSlots::new` へ移ったことで**そもそも到達できなく
   なった**。同じ拒否を新しい検査点で観測する形に置き換えるか、壊れたバイトを
   DTO 復号へ直に流す形にした。テストを消して数を減らしてはいない。

10. **委任 1 の型に手を入れた箇所**は上記 2・3 の 2 件のみ（追加であり変更ではない）。
    加えて `PendingIterations` の `#[cfg_attr(not(test), expect(dead_code, ..))]` を
    外し、`is_empty` を `FirstClassCollection` の実装経由で生かした。

11. **`clippy` 由来の付随修正**。`dto/tests.rs` に
    `#![allow(clippy::indexing_slicing, reason = ..)]` を足した（固定長フィクスチャを
    位置で読むテスト。ハウススタイルの `clippy::panic` 許可と同形）。
    `journal_reader_impl.rs` の `#[expect(clippy::disallowed_methods)]` は、テストが
    生バイト構築に変わって不要になったので外した（`unfulfilled_lint_expectations`）。
    `stage_lookup.rs` の doc が削除済みの `stage_keys` を指していたので `stage_key`
    へ直した。

## 7. 裁定が要る設計上の問い

**無し。** 停止条件（新規ドメインサービス、4 種以外のドメインオブジェクト、
`StageIndex::new` の公開構築経路）はいずれも発生しなかった。追加したのは既存の
FCC 型への全域構築子とエラー変種、および集約内部の私有ヘルパだけである。

## 8. 申し送り（`functional-design` ゲート向け）

1. **`StageSlugSet` の辞書順は業務順ではない**。表示・監査行・upstream 逐語一致が
   要る場所では、必ず計画の文書順へ並べ直してから使う。現状その責務は投影の
   `in_document_order` 1 か所にある。第 3 の消費者が現れたら、置き場所を
   見直すべきか裁定が要る。

2. **`PendingIterations` が `pub(crate)` であることの帰結**。`ReviewAttempt::restored`
   が DTO 境界のためだけに `Vec<u32>` を受けている。公開するか、DTO 側に専用の
   構築経路を作るかは未決である。

3. **`stop_hook` の ITF 準拠テストは依然として未整備**（`team.md` に既知の穴として
   記録済み）。本委任では触れていない。

4. **`workflow_definition` にはスライス返しのアクセサが 14 本残る**。FCC 化の射程外
   としたが、同じ規律を当てるかは別途の判断が要る。

5. **カバレッジの床は現在 98.90% まで戻っている**（床 98.87%）。余裕が 0.03pt しか
   ないので、次の Bolt で新規コードを足すときは同じ Bolt 内でテストも足さないと
   床を割る。
