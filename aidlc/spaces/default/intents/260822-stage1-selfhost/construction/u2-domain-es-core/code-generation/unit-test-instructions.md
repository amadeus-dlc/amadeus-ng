# unit-test-instructions — U2 ドメイン ES コア（FCC 化と `next_decision` の ID 照合、Bolt b51）

> Code Generation（Construction 3.5）のユニットテスト指示（Unit: U2、kind: library）。**2026-09-07 再走（Modify）** — 旧版
> （2026-08-23、Bolt B3）は `unit-test-instructions-history-2026-08-23.md` に保存した。Testing Contract: tdd / standard / classic /
> brownfield（`code-generation-plan.md` の `## Testing Contract`、`contract_sha256` = `sha256:303d9bb7…`）。方針の正本は
> `aidlc/spaces/default/memory/team.md` Testing Posture。

## 1. テストフレームワークと設定

- Rust 標準テストハーネス（`cargo test`）+ proptest（PBT、`core-command-domain` の dev-dependency — 既存）+ serde_json（dev、ITF の
  JSON 読取）。**新規依存なし**（NFR4.1）。ツールチェーンは `rust-toolchain.toml`（1.95.0）。
- PBT のシードは固定: `PROPTEST_RNG_SEED=20260823`（`scripts/coverage.sh` / CI と同値）。性質試験・カバレッジ計測は必ずこの環境変数
  付きで走らせる。
- lint: `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`（workspace lints 50）、`cargo lint`
  （`tools/lint`）。テストコードでは `clippy.toml` により `unwrap` / `expect` を使ってよい（統合テストは file-level
  `#![allow(clippy::unwrap_used)]` — 既存どおり）。プロダクトコードでは禁止。
- 共通契約のハーネスは `modules/core/command/domain/tests/collection_contract_test.rs`（`check(&collection, expected_len)`）。
  infrastructure 側の汎用型は `modules/core/infrastructure/tests/collections_test.rs`（本 Unit では触らない）。

## 2. 本 Unit のテストの走らせ方（Unit 限定コマンド — Step 0 で実走を確認してから最初の Red へ進む）

| 対象 | コマンド |
|---|---|
| ドメイン（ユニット + PBT、FCC 11 型を含む） | `PROPTEST_RNG_SEED=20260823 cargo test -p core-command-domain --lib` |
| ドメイン（新設 FCC だけを絞る例） | `PROPTEST_RNG_SEED=20260823 cargo test -p core-command-domain --lib -- orchestration::stage_slots orchestration::stage_index_set orchestration::stage_slug_set orchestration::stage_entries orchestration::artifact_paths orchestration::transition_steps orchestration::review_closures orchestration::pending_iterations workspace::promoted_sections workspace::rule_lines` |
| 共通契約（FCC の横展開漏れ） | `cargo test -p core-command-domain --test collection_contract_test` |
| ITF 準拠（engine_loop、受け入れゲート） | `cargo test -p core-command-domain --test engine_loop_conformance` |
| command interface-adapter の DTO 往復・Repository 実装・契約 | `cargo test -p core-command-interface-adapter --lib orchestration::dto` と `cargo test -p core-command-interface-adapter --test intent_execution_repository_impl_test --test commit_verdict_use_case_wiring_test --test upstream_event_store_conformance` |
| read-model-updater の DTO・行生成・投影・ゴールデン | `cargo test -p core-read-model-updater --lib orchestration::dto` と `cargo test -p core-read-model-updater --lib read_tables workspace::resolved_plan workspace::projection` と `cargo test -p core-read-model-updater --test read_tables_test --test projection_golden_test --test read_model_updater_test --test journal_reader_impl_test` |
| command use-case（報告適用・昇格） | `cargo test -p core-command-use-case --lib orchestration::commit_verdict_use_case orchestration::promote_practices_use_case` |
| app（scaffold・ジャーナル準拠・クラッシュ再構成） | `cargo test -p aidlc --lib scaffold` と `cargo test -p aidlc --test journal_protocol_conformance --test crash_reconstruction_test` |
| Quint ゲート（受け入れゲート、モデル不変） | `bash scripts/quint-gate.sh` |
| BR4.1 の判定式（0 件で合格）と検出力の裏取り（1 件以上） | 下のコードブロック |
| カバレッジ（クレート全体の基準値 98.66% と orchestration 単独値） | 下のコードブロック |
| ワークスペース全体（品質ゲート、Step 3 末尾と Step 4 でのみ） | `PROPTEST_RNG_SEED=20260823 cargo test --workspace` |

```sh
rg -n -e 'enum PlanAction' -e 'pub use .*PlanAction' modules/core/command/domain/src/orchestration          # 0 件が合格
rg -n -e 'enum PlanAction' -e 'pub use .*PlanAction' modules/core/command/domain/src/workflow_definition    # 1 件以上で検出力を確認
PROPTEST_RNG_SEED=20260823 cargo llvm-cov --package core-command-domain --summary-only
PROPTEST_RNG_SEED=20260823 cargo llvm-cov --package core-command-domain --summary-only \
  --ignore-filename-regex 'modules/core/command/domain/src/(workflow_definition|workspace)/'
bash scripts/coverage.sh   # 同一条件で 2 回、差 0.00 と絶対床 90% の PASS を記録
```

Build and Test は各 Unit のコマンドを実行するため、ワークスペース全体の `cargo test --workspace` は品質ゲートでのみ使う。

## 3. テスト範囲と量（standard: コンポーネントごと 5〜8 本）

| コンポーネント | テスト（代表 — Red で先に書く） |
|---|---|
| `StageEntries` | `new` の 4 種の Err（空 / initialization が SKIP / initialization が条件付き / slug 重複）、`at` の範囲外 `None`、`position_of`、`first_of`、`fold_left` の文書順、`filter` が `Collection<StageEntry>` |
| `StageSlot` / `StageSlots` | `genesis` が全 Pending・未承認・0・空会計、`new` の `Empty` / `DuplicateSlug`、`at(StageIndex)`、位置指定コマンド（`mark` / `record_approval` / `invalidate_approval` / `bump_revision` / `override_plan` / `record_review_request` / `record_review_verdict` / `affirm_practices` / `reset_attempt`）、一括コマンド（`mark_all` / `invalidate_approvals` / `reset_attempts_all`）、`fold_left` で 7 列へ展開し `new` で畳むと同値 |
| `StageIndexSet` | `range` / `singleton` / `contains` / `at` の昇順、proptest: 結合法則・左右単位元・冪等・交換、`A \ A = ∅`、`A \ ∅ = A`、`(A ∪ B) \ B ⊆ A` |
| `StageSlugSet` | 辞書順・重複なし、proptest: Monoid 則・差集合則、`StageEntries::slugs_at(&StageIndexSet)` の写像 |
| `ArtifactPaths` / `RuleLines` | 素通し（順序・重複保持、空可）、`at` / `fold_left` / `filter`、`empty()` |
| `TransitionSteps` | `new` の `Duplicate`、`single`、`contains`、`apply_report` の段分岐に使う名前付きクエリ |
| `ReviewClosures` / `PendingIterations` | `record` の記録順、`has_terminal(policy)`、`with` / `without`、`contains`、`at` |
| `PromotedSections` | `new` の `DuplicateHeading`、順序保持、`fold_left` で見出し列 |
| 共通契約 | `collection_contract_test.rs` に新設型（空 / 非空）を登録し `len` / `is_empty` / `at` / `fold_left` / `filter` の契約が通る |
| `IntentExecution`（切替後） | `next_decision` の `IntentMismatch`（新規）と一致時 `Ok`、`slots()` / `stage_key()`、`open_gate(ArtifactPaths)`、`recompose(StageIndexSet)`（複数件・空 → `InvalidTarget`）、`apply_report(&TransitionSteps)`、jump の読み飛ばし・巻き戻し・承認無効化が `StageIndexSet` で同じ観測、既存 PBT（decide = 旧 + apply / replay = 通常実行 / 通番単調 / Quint 不変条件 / Err 無副作用 / DTO 往復）が緑 |
| `Intent` / イベント / `PracticesPromotion` | `stages() -> &StageEntries`、`Created` / `Started` / `GateOpened` / `Recomposed` / `PracticesAffirmed` のペイロードアクセサが FCC、`PracticesPromotion::plan` の列が FCC |
| ITF 準拠 | 8 fixture 全緑 + アクション網羅アサート + `EngineSignal` 照合（既存）を改修後 API で維持 |
| DTO 境界（interface-adapter / RMU） | 往復 `to_domain(to_dto(agg)) == agg`、列の長さ不一致 → `DtoDecodeError::InvariantViolation`、ゴールデン（バイト不変）、`Recomposed` の投影順序が文書順のまま |
| use-case / app | `commit_verdict_use_case` の Approve 判定が `TransitionSteps` で同じ結果、`promote_practices_use_case` の昇格、`scaffold` の EXECUTE / SKIP 列挙が同じ出力、`next_answer_row` の Err 経路 1 本 |

## 4. カバレッジ目標

- ワークスペース絶対床 90%（`scripts/coverage.sh`、除外は `modules/app/aidlc/src/main.rs` のみ — U2 のコードに除外を足さない）。
  PR 相対ゲート（TOLERANCE 0.01）を base に対して下回らない。
- `core-command-domain` 単独の行カバレッジは基準値 **98.66%**（2026-09-06 実測）を下回らない。`orchestration/` 単独値は希釈を避ける
  参考値として Step 0 と Step 4 で記録する（NFR2.3）。

## 5. モック / スタブ

- ドメインは I/O を持たないためモック不要。集約のテストは合成の `Intent`（固定 ID・合成計画）と `StageEntries` で組む。
- DTO 境界のテストは既存フィクスチャ（`tests/support/`）を使い、FCC への切替後も同じ入力データで往復を確認する。
- ITF 準拠テストは Quint の plan / conditional から合成した `StageEntries`（索引 0 = initialization）で集約を作る（既存の合成手順）。

## 6. テストデータ

- Quint トレース fixture: `tests/conformance/fixtures/engine_loop/*.itf.json`（8 本、不変）。
- ゴールデン: `tests/golden/`（upstream 実バイト、不変）。RMU の `projection_golden_test.rs` が監査行の逐語一致を固定する。
- 各テストは自前でデータを組み立て、共有の可変状態を持たない。性質試験の生成器は `StageIndex` の範囲を stage_count 内に閉じる。
