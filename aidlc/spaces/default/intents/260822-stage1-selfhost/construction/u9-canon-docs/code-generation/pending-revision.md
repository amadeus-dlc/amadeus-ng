# pending-revision — U9 code-generation（ステージゲートの Request Changes で適用する改訂案）

> レビュー受領（READY）後に PR #28 の CodeRabbit 指摘で判明した記録の精度問題。code-summary / unit-test-instructions は凍結（受領・承認指紋）のため
> 本文は据え置き、ゲートで Request Changes を選んだ直後に適用。

1. code-summary §2 の行数: 表の値は統合前の委任報告ベースで、合計 `+148 / −94` は新規ファイル除外の作業ツリー diff。`git diff --numstat origin/main..HEAD` の実測に
   差し替える（レビュー所見反映コミットまで含む）:
   - `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md`: +10 / −9
   - `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md`: +2 / −1
   - `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/error-handling.md`: +24 / −0
   - `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md`: +13 / −8
   - `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/use-case-rules.md`: +1 / −1
   - `docs/specs/01-domain-model.md`: +28 / −12
   - `docs/specs/10-orchestration.md`: +28 / −16
   - `docs/specs/11-workspace.md`: +38 / −21
   - `docs/specs/12-workflow-definition.md`: +30 / −29
   - `docs/specs/deviations.md`: +1 / −0
   - 合計 +175 / −97（新規 error-handling.md を含む、コミット時点の実測）
2. unit-test-instructions §1 表の検査コマンド（`grep -c '^\| \['` の Markdown エスケープ、`| 4 |` のセル）を fenced `bash` ブロックへ移し、そのまま実行できる形にする
   （実行形は `grep -c '^| \['` / `grep -c '^| 4 |' docs/specs/deviations.md`。developer-report-1/2 には実行形を記載済み）。

## 再走（2026-09-07、U9 再走の Bolt）

上記 1 は再走版 `code-summary.md` が `git diff --stat origin/main` の実測（17 files, +817 / −425）を直接書くことで解消、2 は再走版 `unit-test-instructions.md` が
全コマンドを fenced bash に移して解消した。再走で新たに判明し、承認指紋のため本文を据え置いた 2 点:

3. `unit-test-instructions.md` §1 (6) の期待「`decisions.md` の diff は … 削除行（`-`）が無い」は、ブリーフが指示する打消し線（既存行を `~~…~~` で囲む = 行の書き換え）と
   両立しない。「**内容の削除が無い**（`-` 行の内容が `~~` 付きで `+` 側に残る）」に文言を寄せる（code-summary §5 の読み）。
4. `code-generation-plan.md` §1 の「`origin/main` は `e8ca4a5f`（#117）まで進んでいる」は誤り — `origin/main` = `02cacea2`（#118 b51）で gap-measurement の基準そのもの
   （`e8ca4a5f` は `02cacea2` の親）。Step 0 の「基線再実測」は差分ゼロを確認する工程として残す。
5. `unit-test-instructions.md` §1 (1) / (5) / (6) 後半の受入コマンドが `origin/main..HEAD`（コミット間比較）で書かれている（レビュー所見 R-02、Minor）。受入は
   コミット前の作業ツリーで走らせる運用（計画 §5.3 Step 7 → Step 8）なので、`..HEAD` を外して作業ツリー比較 `git diff --stat origin/main -- <path>` にする。
   実際の検査（メイン・レビュアーとも）は作業ツリー比較で行い、結果は code-summary §2 のとおり。文言だけの問題で結果に影響なし。

## レビュー所見の繰延（2026-09-07、advisory レビュー READY・UNIT_COMPLETED 後に記録）

ステージレビューは READY（Critical 0 / Major 1 / Minor 3 / Info 3、`code-generation-plan.md` `## Review`）。レビュー要求（REVIEW_REQUESTED）の記録後は
計画承認ガードが記録ディレクトリ外への書込（`docs/specs/`、`inception/`、scratchpad を含む）をすべて拒否し、さらにレビュー中に成果物（code-summary 等）を
変更すると判定の記録自体が拒否される（「output documents changed outside the reviewer-authored appendix」— 実測。復元して再記録した）。回避スイッチ
（`AIDLC_*`）は使わない。よって次の 2 件は本 Bolt では適用せず、確定した修正文面をここに置く。適用先は (a) code-generation ステージゲートの Request Changes
直後、または (b) U9 の次の Bolt（Build and Test）の冒頭。**教訓**: advisory レビューの所見を同じ Bolt で処理したいなら、REVIEW_REQUESTED の前に全作業を確定させる。

6. **R-01（Major、メインが実測で確認）** `inception/domain-design/traceability.json` の `target` が components.md の改名前の旧名 4 つ（`EngineUseCases` ×12 行 /
   `PersistenceGateways` ×5 / `CanonJson` ×5 / `PublishedLanguage` ×3）を指したまま。計画 §2 の 17 ファイルに入っていなかった本 Unit 起因の参照切れ。
   現行コードの所在（`report` 系 = `core-command-use-case/src/orchestration/commit_verdict_use_case.rs`、`next` / `continue` / Directive / doctor の DAO =
   `core-query-use-case/src/orchestration/`）で写像を確定した。**行ごとの新 `target`**（他の行は不変。JSON の形・キーは変えない）:

   | ID | 旧 target | 新 target | 根拠 |
   |---|---|---|---|
   | FR1 / NFR3 | OrchestrationEngine, PersistenceGateways, ReadModelUpdater | OrchestrationEngine, **CommandGateways**, ReadModelUpdater | 改名（components.md 要約表） |
   | FR1.1 | ReadModelUpdater, PersistenceGateways | ReadModelUpdater, **CommandGateways** | 同上 |
   | FR1.2 | PersistenceGateways | **CommandGateways** | 同上 |
   | FR1.3 | PersistenceGateways, ReadModelUpdater | **CommandGateways**, ReadModelUpdater | 同上 |
   | FR2 / FR2.1 | EngineUseCases, OrchestrationEngine | **CommandUseCases**, OrchestrationEngine | `report` = `commit_verdict_use_case.rs`（コマンド側） |
   | FR2.2 / FR5.1 / FR5.2 / FR5.3 | EngineUseCases | **CommandUseCases** | 書込ユースケース（フックは記録動詞を呼ぶ） |
   | FR3 | OrchestrationEngine, EngineUseCases | OrchestrationEngine, **QueryUseCases** | `next_decision` は OE、Directive 組立 = `find_run_stage_use_case.rs` ほか（クエリ側） |
   | FR3.2 | EngineUseCases, CanonJson, PublishedLanguage | **QueryUseCases, CoreInfrastructure** | `continue_token.rs`（クエリ側）+ 正準 JSON（core-infrastructure） |
   | FR4.2 | CliDispatcher, PublishedLanguage | CliDispatcher, **QueryUseCases** | 文言は出す側（`app/aidlc/src/wording.rs`）、Directive 型は QueryUseCases 所有 |
   | FR5 | CliDispatcher, EngineUseCases | CliDispatcher, **CommandUseCases** | 同 FR2 |
   | FR5.4 | EngineUseCases, ReadModelUpdater | **CommandUseCases**, ReadModelUpdater | 同上 |
   | FR6 / FR6.1 | EngineUseCases, CliDispatcher | **QueryUseCases**, CliDispatcher | doctor の DAO はリードモデル読取（11 号 §3 (2)） |
   | FR7 / FR7.3 | CanonJson | **CoreInfrastructure** | 統合（CanonJson + InfraIo） |
   | FR7.1（N/A 注記文） | 「…受入基準として CanonJson を検収」 | 「…受入基準として **CoreInfrastructure** を検収」 | 同上 |
   | NFR1 | PublishedLanguage, CanonJson, ReadModelUpdater | **WorkspaceModel, CoreInfrastructure**, ReadModelUpdater | 監査語彙 86 語は WorkspaceModel 所有、正準 JSON は CoreInfrastructure |

   適用後の検査: `coverage[].id` の並びが `upstream_ids` と同一、status OK の target を `, ` で割った各名が components.md 要約表の 12 名に含まれる、旧名 4 語 +
   `InfraIo` の出現 0。旧名→新名の履歴は components.md の `~~旧名~~ — 改名` 行が持つ（JSON 側に打消し線は書かない）。
7. **R-03（Minor、確認）** `docs/specs/11-workspace.md` :63（`StateVersion` 行）と :183（W7 行）は `doctor` を現在形で書き、予定表記が無い。追記文面:
   - :63 「（不一致が構造的に不可能）」→「（不一致が構造的に不可能。doctor 側は予定（未実装、クリティカルパス 6））」
   - :183 「（乖離が構造的に不可能）」→「（乖離が構造的に不可能。doctor は予定（未実装、クリティカルパス 6））」
   適用後は受入 (7) の 11 号の件数が 10 → 12 になる（code-summary §2 の値を併せて更新）。
8. **R-04（Minor）** gap-measurement の T1 / T3 / T6 / T7 / T8 は code-summary §7.1 の折り戻し（functional-design ゲート）で処理。R-05 / R-06 / R-07（Info）は追加作業なし。
