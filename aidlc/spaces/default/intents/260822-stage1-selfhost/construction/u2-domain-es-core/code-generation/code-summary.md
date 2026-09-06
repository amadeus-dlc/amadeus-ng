# code-summary — U2 ドメイン ES コア（`u2-domain-es-core`）、Bolt b51

> Unit: `u2-domain-es-core`（kind: library）。2026-09-07 の再走（Artifact Re-use = Modify）。
> 対象コミット: `3edf320d`（委任 1 — FCC 11 型の新設）と `dd20266a`（委任 2 — 集約・イベント・境界の一斉切替）。
> ブランチ `stage1-selfhost`、基底 `origin/main` = `e8ca4a5f`。時刻はすべて UTC。
> 出典: 承認済み `code-generation-plan.md`（承認指紋 `sha256:dd1170c1a75b16e30a351f34d9f4ff57164bcbe65482361e94e6909de7f0634d`、
> Testing Contract `sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3`）、`unit-test-instructions.md`、
> `developer-report-3.md` / `developer-report-4.md`、`../functional-design/functional-spec.md` §9、`../nfr-design/security-design.md`。
> 2026-08-23 の旧版（B3 実装時）は `code-summary-history-2026-08-23.md` に全文保存してある。

## 1. 今回の作業範囲

functional-spec §9 #1〜#4 の差分を実装した。旧世界（7 並列ベクトル・`Vec` / `&[..]` の公開・`next_decision` の非可謬）から、
コーディング規則 `first-class-collections.md`（2026-09-06）に従う世界へ切り替えている。

1. **ファーストクラスコレクション（FCC）11 型の新設**（委任 1、`3edf320d`）: `StageEntries` / `StageSlot` / `StageSlots` /
   `StageIndexSet` / `StageSlugSet` / `ArtifactPaths` / `TransitionSteps` / `ReviewClosures` / `PendingIterations`（クレート内） /
   `PromotedSections` / `RuleLines`。すべて `FirstClassCollection` 契約を実装し、公開 9 型は `tests/collection_contract_test.rs` の
   契約ハーネスへ登録した。集合型 2 つ（`StageIndexSet` / `StageSlugSet`）だけが `combine` / `divide` を持ち、Monoid 則・差集合則を
   proptest（シード固定 `PROPTEST_RNG_SEED=20260823`）で確かめている（裁定 Q1 = A）。
2. **集約・イベントの一斉切替**（委任 2、`dd20266a`）: `IntentExecution` の 7 並列列を `slots: StageSlots` に統合し、`new` の
   引数を 16 → 12 に減らした。`Intent` / `Created` / `Started` は `StageEntries`、`GateOpened` は `ArtifactPaths`、`Recomposed` は
   `StageSlugSet`、`PracticesAffirmed` / `PracticesPromotion` は `PromotedSections` + `RuleLines`、`ReportDecision::Commit` は
   `TransitionSteps`、`ReviewAttempt` の内部列は `ReviewClosures` / `PendingIterations`（裁定 Q2 = A）。`StageEntry::check_plan` /
   `Intent::check_plan` は削除し、計画の検査は `StageEntries::new` の構築時 1 か所へ移した（BR5.5）。
3. **`next_decision` の取り違えガード**: `Result<NextDecision, CommandError>` にし、別 intent には `IntentMismatch` を返す（BR2.6 / BR3.1）。
4. **冒頭 doc の是正**: `intent_execution.rs`（decide 16 コマンド、版トークンの意味、失敗境界の二層と `# Panics` 3 か所、memento
   の旧記述の削除）と `orchestration/mod.rs`（最新スナップショット + 差分、`next_decision` の所有、`recompose(StageIndexSet)`）。
5. **兄弟クレートの追随**: `core-command-interface-adapter`（DTO 境界）、`core-read-model-updater`（DTO・投影・読取表）、
   `core-command-use-case`、`aidlc`（app）、`core-query-interface-adapter`（tests）。DTO のバイト表現（7 列、JSON 形）は不変。

作っていないもの: 新規依存、Quint モデル・ITF fixture・ゴールデンの変更、`Cargo.toml` / `Cargo.lock` / `scripts/**` / `.github/**` の
変更、後方互換の旧 API（`stage_keys()` / `check_plan` / `&[..]` 返却 / `#[deprecated]` はいずれも 0 件）。

## 2. 変更ファイル

`git diff --name-only origin/main..HEAD -- modules` = **78 パス**（新規 14、変更 64）。全件を `source-manifest.json` に列挙した。

| クレート | 新規 | 変更 | 主な内容 |
|---|---|---|---|
| `core-command-domain`（src） | 14 | 20 | FCC 11 型 + エラー型 3 + `mod.rs` 2、集約・イベント・`ReviewAttempt` / `PracticesPromotion` の切替、`stage_entry.rs` から検査を移設、`PromotionPlanError::DuplicateSection` 新設 |
| `core-command-domain`（tests） | 0 | 2 | 契約ハーネス登録、ITF 準拠テストの追随（fixture 不変） |
| `core-command-interface-adapter` | 0 | 12 | `IntentExecutionDto` の 7 列 ⇄ `StageSlots` 相互変換（内部の `SlotColumns` で 1 走査）、イベント DTO の `fold_left` 化、tests |
| `core-read-model-updater` | 0 | 19 | DTO（読む側）、`NextAnswerRow::of` の可謬化、`Recomposed` の文書順投影（`in_document_order`）、`read_tables.rs` の走査、tests |
| `core-command-use-case` | 0 | 4 | `CommitOutcome::Committed.steps: TransitionSteps`、`contains(TransitionStep)`、tests |
| `aidlc`（app） | 0 | 5 | `committed_transition` を名前付きクエリへ、`scaffold.rs` を `filter` / `fold_left` へ、`DuplicateSection` の文言配線、tests |
| `core-query-interface-adapter` | 0 | 1 | tests/support の追随 |

## 3. 主要な実装判断（計画 §2 からの逸脱を含む）

| # | 判断 | 理由 |
|---|---|---|
| 1 | `recompose` の引数を `&StageIndexSet`（計画は値渡し） | `clippy::needless_pass_by_value` が deny。集約は集合を消費しない |
| 2 | `TransitionSteps::recovered_approval()` を新設 | `new` が `Result` を返し、プロダクトコードで `unwrap` できないため。2 段は異なるので重複は構造的に起き得ない |
| 3 | `StageSlots::override_plan_all(&StageIndexSet, PlanAction)` を新設 | `Recomposed` 適用の一括書込先（`mark_all` と同じ「列に在る位置だけ」の集合演算） |
| 4 | `PromotionPlanError::DuplicateSection(String)` + `From<PromotedSectionsError>` を新設 | `PromotedSections::new` の `Result` を握り潰さない。app 側の文言配線も追加 |
| 5 | `ReviewAttempt::restored(requests, Vec<u32>, ReviewClosures)` — 判定待ちだけ生の並びを受ける | `PendingIterations` は `pub(crate)` で DTO から組めない。DTO 境界の例外として doc に理由を明記 |
| 6 | `ReviewAttempt::pending()` → `pending_iterations() -> Vec<u32>` | FCC を返さない読取用アクセサであることを名前で示す |
| 7 | `NextAnswerRow::of` を可謬化し、`Err` を既存の `ReadTablesError::IntentUnavailable` へ写す | 新変種を足さない。意味は「材料が揃わない」 |
| 8 | `Recomposed` の投影は `in_document_order(plan, set)` で文書順へ並べ直す | `StageSlugSet` は辞書順。監査行 `**Stages skipped**` と行末トークンの逐語一致（NFR1）を守る。型側の順序は変えない |
| 9 | `StageSlotsError::OutOfRange` は `ApplyError::InvariantViolation` へ写し、`apply_event` の panic 経路へ流す | 適用は `resolve` 済み位置で呼ぶので起きないが、起きたら壊れた歴史として無言の no-op にしない |
| 10 | 構造的に不能になったテスト（列長不一致、破れた計画の `should_panic` 4 本）は新しい検査点で同じ拒否を観測する形へ置換 | 検査点が `StageEntries::new` / `StageSlots::new` へ移り、旧経路は構成不能。テスト数は減らしていない |
| 11 | 1 コミットにまとめた（計画は意味単位の分割を許容） | ドメインのみをステージした状態で `cargo check --workspace` が 42 件の error（exit 101）で落ちることを実測 |

裁定が要る設計上の問い（ドメインサービス新設・4 種以外のドメインオブジェクト・`StageIndex::new` の公開構築口）は発生しなかった。

## 4. テスト

- **TDD**: ドメイン層は時系列の Red（切替着手時のビルドエラー約 94 件、`next_refuses_to_answer_for_a_foreign_intent` はガード実装前に
  `Ok(RunStage)` で落ちるのを確認）→ Green → Refactor（私有ヘルパ 6 本）。RMU の新振る舞い 2 件（`NextAnswerRow::of` の Err 経路、
  `Recomposed` の文書順投影）は実装が先に入っていたため**時系列の Red ではなく**、テストを書いたうえで実装を反転させて落ちることを
  確認した（失敗出力は `developer-report-4.md` §3.3 に記録）。
- **件数**: `core-command-domain` lib 591 → 699（委任 1 で +103、委任 2 で +5 と置換）、契約試験 1 → 2、ITF 1、doc 3。
  ワークスペース全体 2,354 passed / 0 failed。
- **ゴールデン / ITF**: `engine_loop_conformance` 1、`journal_protocol_conformance` 5、`upstream_event_store_conformance` 10、
  `golden_parity_test` 11、`projection_golden_test` 18、`audit_block_golden_test` 1、`cli_golden_test` 5、`golden_corpus_read` 14、
  `golden_hash_canonical` 7 — すべて緑、fixture・Quint モデル・スクリプトは不変（`git diff --stat origin/main..HEAD -- tests formal scripts .github` 空）。
- **カバレッジ**: 委任 2 の切替直後は 98.78% で床（98.87%）を 0.09pt 下回ったため、未到達行を洗い出しテスト 6 本を足して 98.90% へ
  戻した（床の調整はしていない）。

## 5. 受入の実測（開発者報告と、コンダクタの独立再測）

| 項目 | 開発者（`developer-report-4.md`） | コンダクタ独立再測（2026-09-06T18:50Z〜） |
|---|---|---|
| (a) `cargo fmt --all --check` / `clippy -D warnings` / `cargo lint` | exit 0 | exit 0 |
| (a) `PROPTEST_RNG_SEED=20260823 cargo test --workspace` | 2,354 passed / 0 failed | 全バイナリ緑（domain lib 699、契約 2、ITF 1 ほか）、exit 0 |
| (a) `bash scripts/quint-gate.sh` | PASS | `[PASS] quint gate: all steps green` |
| (a) `cargo audit` × 2（workspace / `tools/lint/Cargo.lock`） | 脆弱性なし | 再測なし（依存不変を (f) で確認） |
| (b) `bash scripts/coverage.sh` × 2 | 99.15169660678644% × 2、差 0.00、床 90% PASS | 99.15169660678644%、PASS（同値） |
| (c) `cargo llvm-cov -p core-command-domain` 行 | 98.90%（床 98.87% 以上）、orchestration 単独 99.38% | 98.90%（TOTAL 行 15,466 / 未到達 170） |
| (d) `PlanAction` の所有 | orchestration 0 件 / workflow_definition 2 件 | 同左（報告の rg 出力を確認） |
| (e) `# Panics` | ヘッダ doc を除き `intent_execution.rs` 3 + `workflow_definition.rs` 1、不増 | 同左 |
| (f) `Cargo.toml` / `Cargo.lock` | 差分ゼロ | 同左 |
| (g) `&[..]` 公開 / `to_vec()` | orchestration・workspace で 0 件、ドメイン全体で `to_vec()` 0 件 | 同左。RMU / query に FCC 型のフィールド・戻り値型なし（DTO 復号境界・投影の即時読取・テストのみ） |

## 6. センサー

| センサー | 結果 | 備考 |
|---|---|---|
| `required-sections`（`code-generation-plan.md`） | pass、H2 7 | — |
| `required-sections`（`unit-test-instructions.md`） | pass、H2 6 | — |
| `required-sections`（本ファイル） | pass、H2 8 | — |
| `traceability`（`traceability.json`、49 行） | `gaps` / `orphans` / `missing_from_table` / `invalid_entries` / `invalid_targets` すべて 0 件。`missing_from_upstream_ids` 37 件（FR1〜FR9、NFR1〜NFR5 の親 ID と他 Unit の ID） | 既知のノイズ（ステージ定義が `upstream-coverage` を code-generation にインポートしていないため per-unit の狭い `upstream_ids` と突合できない）。U1 / U10 と同じ扱い。センサー成功とは読み替えない |
| `source-manifest.json` | 78 パス（strict schema、`repo` なし） | エンジンの記録時検証に委ねる |

## 7. 申し送り（functional-design ゲートの Request Changes で本文へ折り戻す確定事項）

1. `StageSlugSet` の辞書順は業務順ではない。表示・監査行・upstream 逐語一致が要る場所は計画の文書順へ並べ直す。現状その責務は
   RMU 投影の `in_document_order` 1 か所。第 3 の消費者が現れたら置き場所の裁定が要る。
2. `PendingIterations` が `pub(crate)` であることの帰結として `ReviewAttempt::restored` が `Vec<u32>` を受ける。公開するか DTO 側に
   専用の構築経路を作るかは未決。
3. `StageIndex::new` は `pub(crate)` のまま。クレート外から位置集合を組む公開経路は `stage_index(usize)` / `position_of` の 2 系統で足りた。
4. 計画 §2 の 11 型の確定事項（不変条件・操作・`Filtered`・エラー型）と、本ファイル §3 の判断 1〜9 を `entities.md` / `rules.md` /
   `functional-spec.md` へ折り戻す。
5. カバレッジの余裕は床まで 0.03pt。次の Bolt で新規コードを足すときは同じ Bolt 内でテストも足す。
6. 上流 `components.md` / contract-summary C3 の「ジャーナル全再生」注記は本 Bolt の doc 是正（最新スナップショット + 差分）と食い違う
   ままなので、同期は別途。

## 8. 記録

- 委任: `developer-brief-3.md` → `developer-report-3.md`（Opus、Step 0〜1）、`developer-brief-4.md` → `developer-report-4.md`（Opus、Step 2〜4）。
- コンダクタの diff レビュー: 67 + 16 ファイルを全件読了。計画 §2 と一致、逸脱は §3 の 11 件で妥当と判定。留意点: `IntentExecutionDto::to_domain`
  の列長検査が走査ごとに重複して走る（正しいが冗長）、`scaffold.rs::first_post_initialization` が `Option<StageEntry>` を clone で返す
  （`filter` が所有コレクションを返すため）。いずれも機能・契約に影響せず、差し戻し対象にしない。
- PR: 本 Bolt の完了後に `stage1-selfhost` を push し、`b51` として 1 本だけ開く（直列運用）。
