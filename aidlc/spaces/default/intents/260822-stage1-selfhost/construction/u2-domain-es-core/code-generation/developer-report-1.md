# developer-report-1 — U2 ドメイン ES コア（Bolt B3 / 委任 1: 計画 Step 1〜8）

> aidlc-developer-agent の作業報告。承認済み計画 `code-generation-plan.md` §5.1（Step 1〜8）と
> `unit-test-instructions.md` に従う。Testing Contract: tdd / standard / classic / brownfield
> （`sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3`）。
> 計画ファイル 3 本（`code-generation-plan.md` / `unit-test-instructions.md` /
> `code-generation-questions.md`）は読み取り専用として**一切編集していない**。進捗・Red 記録・
> 判断はすべて本ファイルに書く。ブランチ `bolt/b3-u2-domain-es-core`、push / PR は行っていない。

## 0. 着手前の実測（Step 2 — テストランナー確認）

`unit-test-instructions.md` §2 のコマンドが brownfield の現状で走ることを確認した（2026-08-23 実測）。

| 対象 | コマンド | 着手前 | 完了後 |
|---|---|---|---|
| ドメイン（ユニット + PBT） | `PROPTEST_RNG_SEED=20260823 cargo test -p core-domain --lib` | 126 passed | 141 passed |
| ITF 準拠（engine_loop） | `cargo test -p core-domain --test engine_loop_conformance` | 1 passed | 1 passed |
| ITF 準拠（audit_lock） | `cargo test -p core-domain --test audit_lock_conformance` | 1 passed | 1 passed |
| Repository ポート（use-case） | `cargo test -p core-use-case` | 0 passed | 0 passed |
| Repository 実装 | `cargo test -p core-interface-adapter --test workflow_definition_repository_impl_test` | 19 passed | 27 passed |
| ゴールデン | `cargo test -p core-interface-adapter --test golden_parity_test` | 6 passed | 9 passed |
| アダプタ orchestration ユニット | `cargo test -p core-interface-adapter --lib orchestration::` | 15 passed | 18 passed |
| ワークスペース全体 | `cargo test --workspace` | 234 相当 | **368 passed / 0 failed** |

計画の想定（`core-domain` 126 + ITF 2）と一致した。テスト指示のコマンドは**変更なしで確定**とする。

## 1. 変更ファイル一覧

`git diff --stat origin/main..HEAD -- modules tests`（17 ファイル、+1108 / -340）。

**新規**

- `modules/core/domain/src/workflow_definition/workflow_definition_id.rs`（148 行）
- `modules/core/domain/src/workflow_definition/definition_revision.rs`（188 行）
- `tests/golden/upstream-3c3146cf/harness.json`（upstream 実バイト 76 バイト）

**移動**

- `modules/core/domain/src/orchestration/plan_action.rs` → `.../workflow_definition/plan_action.rs`
  （`git mv`、中身は無変更）

**改訂**

- `modules/core/domain/src/workflow_definition/mod.rs`（`plan_action` / `workflow_definition_id` /
  `definition_revision` の `mod` + `pub use` 追加）
- `modules/core/domain/src/orchestration/mod.rs`（`mod plan_action` / `pub use plan_action::PlanAction` 削除）
- `modules/core/domain/src/workflow_definition/workflow_definition.rs`（id / revision 追加、
  `effective_plan_action` / `next_in_scope_stage` 削除、依存テストの書き換え）
- `modules/core/domain/src/workflow_definition/scope_grid.rs` / `stage_graph.rs`（import パス・doc）
- `modules/core/domain/src/orchestration/workflow_execution.rs`（import パス 1 行のみ）
- `modules/core/domain/tests/engine_loop_conformance.rs`（import パスのみ。本体は委任 2）
- `modules/core/use-case/src/orchestration/workflow_definition_repository.rs`
  （`find_by_id`、`NotFound` / `HarnessIdentity` 追加、`find()` 削除）
- `modules/core/interface-adapter/src/orchestration/workflow_definition_repository_impl.rs`
  （`load_harness_identity` / `compute_revision` / `serialize_grid`、生値の取り回し、文言 2 本）
- `modules/core/interface-adapter/src/orchestration/memory/workflow_definition_repository.rs`
- `modules/core/interface-adapter/tests/workflow_definition_repository_impl_test.rs`
- `modules/core/interface-adapter/tests/golden_parity_test.rs`
- `tests/golden/upstream-3c3146cf/README.md`（表に 1 行追加。既存 2 行のバイトは不変）

`modules/core/interface-adapter/Cargo.toml` は**変更不要**だった（`canon-json` は既に
`[dependencies]` に入っている — 16 行目、確認のみ）。

## 2. 各 Red の失敗出力

### Red 1 — Step 3（Data model）

コマンド: `PROPTEST_RNG_SEED=20260823 cargo test -p core-domain --lib`

```
failures:
    workflow_definition::definition_revision::tests::a_bare_hex_digest_is_rejected_because_the_family_is_part_of_the_form
    workflow_definition::definition_revision::tests::a_digest_of_the_wrong_width_is_rejected
    workflow_definition::definition_revision::tests::uppercase_hex_and_non_hex_characters_are_rejected
    workflow_definition::workflow_definition_id::tests::an_empty_or_blank_name_cannot_be_constructed
    workflow_definition::workflow_definition_id::tests::an_interior_control_character_is_rejected
    workflow_definition::workflow_definition_id::tests::surrounding_whitespace_is_trimmed_before_validation
    workflow_definition::workflow_definition_id::tests::the_id_works_as_a_map_and_set_key

test result: FAILED. 133 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s
```

Green 後: `test result: ok. 140 passed; 0 failed;`

### Red 2 — Step 6（Repository）

コマンド 3 本、計 11 本が失敗。

`cargo test -p core-interface-adapter --lib orchestration::`

```
failures:
    orchestration::memory::workflow_definition_repository::tests::a_request_for_another_definition_is_not_found
    orchestration::memory::workflow_definition_repository::tests::the_not_found_error_names_the_provider_as_expected_and_the_request_as_actual
    orchestration::memory::workflow_definition_repository::tests::the_seeded_definition_is_never_mutated_by_a_rejected_request

test result: FAILED. 15 passed; 3 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.00s
```

`cargo test -p core-interface-adapter --test workflow_definition_repository_impl_test`

```
failures:
    a_harness_identity_file_that_is_not_json_or_has_no_name_is_fatal
    a_missing_grid_still_yields_a_revision_derived_from_the_transposed_grid
    a_missing_harness_identity_file_is_fatal
    a_request_for_a_definition_this_harness_does_not_provide_is_not_found
    the_identity_is_checked_before_the_three_inputs_are_read
    the_revision_covers_the_scope_identity_files_as_well_as_the_two_json_inputs
    the_revision_is_stable_for_the_same_inputs_and_changes_with_them

test result: FAILED. 20 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
```

`cargo test -p core-interface-adapter --test golden_parity_test`

```
failures:
    another_harness_name_cannot_open_the_shipped_graph

test result: FAILED. 8 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Green 後: それぞれ 18 / 27 / 9 passed、0 failed。

### Red の作り方について（記録）

Rust では「テストだけ書く」とコンパイルエラーになり**失敗テスト名が出ない**。ブリーフが
「失敗テスト名・要約行」の記録を求めているため、各 Red では

1. テストを先に書き、
2. 型・メソッドの**シグネチャだけ**を持つスタブ（検証なしの `parse`、id を無視する
   `find_by_id`）を置いて、
3. 名前付きの失敗として Red を採取

という順にした。スタブは Green で全面的に置き換えており、`TODO(Step 7 Green)` コメントも
残していない（`grep -rn 'TODO(Step 7' modules` = 0 件）。

## 3. 判断（設計との差分）

以下は計画・設計に明示が無く、実装時に確定した事項。いずれも設計に反する変更ではないが、
レビューで確認してほしい。

1. **`WorkflowDefinitionId` の文法**（設計は「空・不正形は構築できない」のみ）。
   trim + 非空に加えて**制御文字を拒否**する。id は状態ファイルと監査行に 1 行として載るため、
   内部の改行・C0 制御文字は表現できないという理由。内部の空白は許容している（将来の
   ハーネス名を過剰に狭めないため）。

2. **`DefinitionRevision` は小文字 hex のみ受理**。canon-json の `sha256_hex` は小文字を返すので、
   大文字 hex は「別経路で作った値」の印になる。生 hex（非正準族 `hash_compact` の戻り）は
   `MissingPrefix` で拒否 — 2 つのダイジェスト族の取り違えを型で止める（canon-json の
   `DigestFamily` の設計意図と同じ）。

3. **`WorkflowDefinition` の `PartialEq` は derive を維持**（id / revision も等価に参加する）。
   エンティティの identity 比較だけに絞る案も検討したが、この集約は 3 入力から毎回組み立て直す
   読取モデルであり、「同じ系譜の同じ内容」を 1 つの等価関係で表すほうが既存テスト
   （`assert_eq!(first, second)`）の意味が保たれる。id だけの同一性比較が要るのは
   `WorkflowExecution` 側の定義照合（BR2.6）で、そちらは `id()` 同士を突き合わせる。
   この根拠は型の doc コメントに明記した（domain-equality.md の「乖離するならドメインが勝つ」
   に対し、乖離が無いことを記録した形）。

4. **`RevisionInput.scopes` の要素は「読取モデルが保持する frontmatter 値」**。
   ブリーフは「`name` / `depth` / `description` / `keywords` など」と例示するが、
   `ScopeMetadata` は `description` を保持していない（未知キーは寛容に無視するのが 12 §3.3 の
   契約）。そこで実際に保持する 6 値（`name` / `depth` / `keywords` / `skeleton` /
   `review_cap` / `freeform_default`）を `name` 昇順で直列化した。**revision は「この
   Repository が読んだ 3 入力」の内容版であって、ファイルの生バイトの版ではない**という
   位置づけを型の doc に書いた。`stage-graph.json` / `scope-grid.json` については読んだままの
   生値（未知フィールド・キー順込み）を使っているので、この非対称は scope identity だけ。
   → **レビュー確認事項**: 生バイト版にすべきなら後続 Bolt で差し替える。

5. **グリッド欠損時の revision**（ブリーフ指定どおり導出グリッドを直列化）を
   `{ <scope>: { stages: {...} } }` の 2 段構造にした理由を doc 化した — ファイルから読めた
   ときと同じ形にしておくと、「導出グリッドと同じ内容の grid ファイルが置かれた」場合に同じ
   revision になり、内容版が入力の**内容**だけで決まる性質が保たれる。

6. **`load_graph` / `load_grid` の戻り値を `(ドメイン型, 生値)` に変えた**。同じ内容を 2 回
   `serde_json::from_str` するのを避けるため、1 回読んだ文字列から `serde_json::Value` と
   ワイヤ構造体の両方を作る。失敗態度（グラフ fatal / グリッドはフォールバック）は不変。

7. **`harness.json` に env オーバライドを設けなかった**。upstream に対応する env が無く、
   identity はハーネスの配置そのものだから（`AIDLC_STAGE_GRAPH` のような hint 分岐も無い）。

8. **`NotFound` / `HarnessIdentity` の文言は upstream 逐語ではない**。upstream には定義 id の
   概念自体が無いため対応する逐語が存在しない。`malformed` と同じ「診断文言（互換対象外）」
   として `definition_not_found_message` / `harness_identity_message` をアダプタ層に置き、
   その旨を doc に書いた。ポート側（`GraphReadError`）は材料のみを運ぶ。

9. **削除した述語の依存テストの扱い**。`effective_plan_action` / `next_in_scope_stage` を
   検証していたテストのうち、静的グリッドの 3 値契約を見ていたものは `grid().action()` へ、
   文書順を見ていたものは `stages_in_scope` へ書き換えた。実効プランの合成（サフィックスが
   グリッドに勝つ）と checkbox 読み飛ばしを見ていた PBT 2 本
   （`next_in_scope_stage_is_the_first_qualifying_node_in_document_order` /
   `suffixes_beat_the_grid_and_absence_is_none`）は**削除**した — 移設先の
   `WorkflowExecution` 側で再実装するのが正しく（計画 §5.3 / NFR2.2 の PBT (a)〜(f)）、
   定義側に残すと移設が中途半端になるため。
   → **委任 2 への申し送り**: この 2 性質の等価物を集約側 PBT で必ず復活させること。

10. **`stage_graph.rs` の doc の陳腐化を 1 行修正**した（`next_in_scope_stage` への言及を
    `stages_in_scope` と集約側の前進走査に置き換え）。所有ファイル一覧に明記は無いが、
    削除したメソッド名を指す doc を残すと誤読の元になるため。

## 4. 棚卸し I2〜I5

- **I2**（`WorkflowExecution` / `EngineSignal` / `Status` / `PlanAction` の外部利用箇所）:
  - `PlanAction` を含むファイルは **10 ファイル**（実装後の再 grep）。移動前と同数で、
    差分ゼロ（内訳は移動先 `workflow_definition/plan_action.rs`、同 `mod.rs`、`scope_grid.rs`、
    `workflow_definition.rs`、`execution_kind.rs`（doc 言及のみ）、`orchestration/workflow_execution.rs`、
    `domain/tests/engine_loop_conformance.rs`、アダプタの impl と 2 テスト）。
  - `WorkflowExecution` / `EngineSignal` / `Status` の**ドメイン外での利用は doc コメントのみ**
    （`core-use-case` の `workspace/mod.rs`、`core-interface-adapter` の `workspace/mod.rs` /
    `state_file_io.rs` — いずれも「B-2 で設計する `WorkflowExecutionRepository`」への言及）。
    実コードの利用はゼロ。委任 2 の集約全面改訂はドメイン外へ波及しない見込み。
- **I3**（upstream `harness.json` の実バイト）: **取得成功**。
  `curl -fsSL https://raw.githubusercontent.com/awslabs/aidlc-workflows/3c3146cfd7cef33020d48e8d48d4e80d0f8c2820/dist/claude/.claude/tools/data/harness.json`
  → HTTP 200、**76 バイト**、内容 `{ "name": "claude", "harnessDir": ".claude", "rulesSubdir": "rules" }`。
  md5 `4108544495aeb5260fad0fcba21b664d`、sha256
  `85bfdec8f1449f17f164599dbccdb79ffda9af76cdc18588e60dde75e589ace9`。本リポジトリの
  `.claude/tools/data/harness.json` と**実バイト一致**（同 sha256）を確認。README の表に 1 行追加し、
  既存 2 行（`stage-graph.json` / `scope-grid.json`）のバイト・ハッシュは不変。
- **I4**（`DefinitionRevision` の入力順序と JSON 形）: 上記「判断 4・5」で確定。テストは
  `the_revision_is_stable_for_the_same_inputs_and_changes_with_them`（同一入力 2 回で一致、
  `scope-grid.json` の 1 セルを EXECUTE → SKIP に変えて不一致）、
  `the_revision_covers_the_scope_identity_files_as_well_as_the_two_json_inputs`、
  `a_missing_grid_still_yields_a_revision_derived_from_the_transposed_grid`、
  `the_shipped_revision_is_reproducible_from_the_pinned_bytes`（ゴールデン）。
- **I5**（`IntentId` の既存有無）: **存在しない**。`grep -rn 'IntentId\|IntentSlug'
  modules/core/domain/src` = 0 件。`workspace` コンテキストにも該当型は無いので、
  委任 2 で `orchestration` に新設が必要（計画 §2 の想定どおり）。

## 5. 品質ゲートの結果

すべて緑（2026-08-23 実測、最終コミット `9210685` 時点）。

| ゲート | 結果 |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0（warning 0 件） |
| `cargo lint` | exit 0 |
| `cargo test --workspace` | exit 0 / **368 passed, 0 failed** |

**合格 grep**

- FR8.3: `grep -rnE 'enum PlanAction\|pub use .*PlanAction' modules/core/domain/src/orchestration`
  → **0 件**
- FR8.4: `effective_plan_action` / `next_in_scope_stage` のコード定義・呼出 → **0 件**
  （残るのは移設の経緯を書いた doc コメント 3 行のみ）
- C4: `.find()` の呼出 → **0 件**（13 箇所すべて `find_by_id` へ移行）

**受入基準 10 項目の自己照合**

| # | 内容 | 結果 |
|---|---|---|
| 1 | `PlanAction` の再輸出 grep = 0 | OK |
| 2 | 2 述語が不在、残す 6 述語と `grid()` 照会は健在 | OK（6 述語すべて `pub fn` として存在） |
| 3 | `WorkflowDefinitionId` / `DefinitionRevision` の公開、`new` 5 引数 + `id()` / `revision()` | OK |
| 4 | `find_by_id` のみ、`NotFound` / `HarnessIdentity` 追加、rustdoc は材料のみ | OK |
| 5 | `harness.json` の `name` → id、revision = 3 入力の正準ダイジェスト、安定性と変化 | OK（判断 4 の非対称は要確認） |
| 6 | `InMemory…` も同じ識別子契約 | OK |
| 7 | ゴールデン `harness.json` = upstream 実バイト、README 表に 1 行 | OK（HTTP 200 で取得） |
| 8 | Red → Green → Refactor、コンポーネントごと 5〜8 本、`find()` 13 箇所移行 | OK（下表） |
| 9 | 品質ゲート 4 本緑、ITF は import パス修正後も緑 | OK |
| 10 | 本報告ファイル | OK |

**コンポーネント別テスト本数**（standard = 5〜8 本）

| コンポーネント | 本数 |
|---|---|
| `WorkflowDefinitionId` | 7 |
| `DefinitionRevision` | 7 |
| `WorkflowDefinition`（id / revision + 移設後の照会） | 6（新規 5 + 書き換え 1） |
| `WorkflowDefinitionRepositoryImpl`（識別子・内容版） | 8（既存 19 本は維持し `find_by_id` へ移行） |
| `InMemoryWorkflowDefinitionRepository` | 5 |
| ゴールデンパリティ（identity 面） | 3（既存 6 本は維持） |

## 6. コミット一覧

`origin/main`（`0092761`）起点、ブランチ `bolt/b3-u2-domain-es-core`。委任 1 のコミットは
`21dfa8a` より後の 5 本（`21dfa8a` はコンダクタの aidlc 記録コミット）。

| SHA | メッセージ（1 行目） |
|---|---|
| `6cda871` | `refactor(workflow-definition): move PlanAction out of orchestration (FR8.3/FR8.4)` |
| `3e44965` | `feat(workflow-definition): add WorkflowDefinitionId / DefinitionRevision (ADR-008)` |
| `0b333a9` | `test(golden): pin the upstream harness.json bytes at 3c3146cf` |
| `6c924e6` | `feat(workflow-definition): identify the definition and switch the port to find_by_id (ADR-008, C4)` |
| `9210685` | `test(workflow-definition): characterise the phase column of stages_in_scope` |

`aidlc/` 配下は 1 つもコミットしていない（本報告ファイル含む — コンダクタが行う）。
`git add -A` は使わず、毎回変更ファイルを明示して add した。push / PR は行っていない。

## 7. 委任 2 への申し送り

1. 削除した PBT 2 性質（実効プランの合成、checkbox 読み飛ばしの最小性）の等価物を、
   移設先の `WorkflowExecution` 側 PBT で必ず復活させること（計画 §5.3 / NFR2.2 (a)〜(f)）。
2. `IntentId` は既存に無い（I5）。`orchestration` へ新設が要る。
3. `engine_loop_conformance.rs` は **import パスのみ**を直した状態で緑。本体（旧 API →
   `start_with_entries` / `decide` / `apply_event`）の書き換えは未着手。
4. `WorkflowDefinition::grid().action(scope, slug)` と `stages_in_scope(scope)` が、集約側で
   実効プランを合成するための材料。`stages_in_scope` は全ステージを文書順で
   `(slug, phase, Option<PlanAction>)` として返し、索引 0 のみ initialization であることを
   `stages_in_scope_reports_the_phase_of_every_stage_alongside_the_action` が固定している。
