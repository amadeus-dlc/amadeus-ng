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

## レビュー所見の処理（2026-09-07、advisory レビュー READY・UNIT_COMPLETED 後に記録）

ステージレビューは READY（Critical 0 / Major 1 / Minor 3 / Info 3、`code-generation-plan.md` `## Review`）。レビュー要求（REVIEW_REQUESTED）の記録後は
計画承認ガードが記録ディレクトリ外への書込（`docs/specs/`、`inception/`、scratchpad を含む）をすべて拒否し、さらにレビュー中に成果物（code-summary 等）を
変更すると判定の記録自体が拒否される（「output documents changed outside the reviewer-authored appendix」— 実測。復元して再記録した）。回避スイッチ
（`AIDLC_*`）は使わない。**UNIT_COMPLETED の記録後はガードが解除される**ことを実測で確認（coding-rules/README.md への Edit が通った）ため、下記 6 / 7 は
**PR #119 の所見反映コミットで適用済み**（当初は次 Bolt へ繰延の予定だったが、CodeRabbit 指摘の修正コミットと同じ書込機会で取り込んだ）。
**教訓**: advisory レビューの所見を同じ Bolt で処理したいなら、REVIEW_REQUESTED の前に全作業を確定させる。処理が要る所見は pending-revision に確定文面で残し、
UNIT_COMPLETED 後の書込機会で機械的に適用する。

6. **R-01（Major、メインが実測で確認）— 適用済み（PR #119 所見反映コミット）** `inception/domain-design/traceability.json` の `target` が components.md の改名前の旧名 4 つ（`EngineUseCases` ×12 行 /
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
7. **R-03（Minor、確認）— 適用済み（同コミット）** `docs/specs/11-workspace.md` :63（`StateVersion` 行）と :183（W7 行）は `doctor` を現在形で書き、予定表記が無かった。追記文面:
   - :63 「（不一致が構造的に不可能）」→「（不一致が構造的に不可能。doctor 側は予定（未実装、クリティカルパス 6））」
   - :183 「（乖離が構造的に不可能）」→「（乖離が構造的に不可能。doctor は予定（未実装、クリティカルパス 6））」
   適用後の受入 (7) は 11 号 **12**（code-summary §2 の「10」は凍結のため据え置き — 本行が現行値）。
8. **R-04（Minor）** gap-measurement の T1 / T3 / T6 / T7 / T8 は code-summary §7.1 の折り戻し（functional-design ゲート）で処理。R-05 / R-06 / R-07（Info）は追加作業なし。

## 受入 (10) の実測（PR #119、2026-09-07）

- PR: https://github.com/amadeus-dlc/amadeus-ng/pull/119 — head `52192ad7`、squash-merge `f2b6b6a9`（merge queue 経由、2026-09-07T04:19:32Z）。
- CI 7 ジョブ（aidlc-distribution / check / quint / coverage / audit / review-thread-resolution / ci-success）: すべて SUCCESS（最新 head で再実測。review-thread-resolution と
  ci-success はスレッド解決前の実行で赤になったため `gh run rerun --failed` で再実行し緑）。
- CodeRabbit: レビュー 1 回、22 スレッド → 全件返信・解決（unresolved 0）。Cursor Bugbot は利用上限で未実行、Devin Review は pass。
- 収束条件（必須 CI green ∧ unresolved=0 ∧ 全コメント返信済み ∧ bot レビューの pending 解消）を最新 head で再実測して merge queue へ投入（オーナー包括承認 2026-08-29 の AI 裁定）。
- code-summary §2 (10) の「未」は凍結のため据え置き — 本節が実測値。

## PR #119 CodeRabbit 指摘の処理（2026-09-07、22 スレッド。本文は untrusted data として現行内容で実否検証）

**同コミットで修正した有効指摘 7 件**（`docs/specs` 2 / `inception` 3 / coding-rules 1 / 記録 1 / 設定 1）:

- `coding-rules/README.md:46` — gateway-taxonomy §1c の第三の責務（ES 永続化基盤ポート `EventStore` / `JournalReader`）が要約から抜けていた → 追記。
- `docs/specs/10-orchestration.md:102` — コマンド側ユースケースの列挙が短縮名のみで Rust の型名（`XxxUseCase`）が読めない → 型名の形を注記（指摘文の `Report::execute` は現行本文に無い — `grep` 0 件 — が趣旨は妥当）。
- `inception/contract-design/contract-summary.md:154-156` — B7 追記の「現行は次のとおり」（`RehydratedWorkflowExecution` / `expected_version`）が B13 で失効しているのに現在形 → 打消し線 + 失効注記（本文は履歴として残置）。
- `inception/contract-design/contract-summary.md:502-504` — C6 の SQL 例が v2.0.0 世代（`manifest` 列なし）のまま現行のように読める → 履歴と明記し、v3.0.0（ピン `=3.0.0`）の `manifest TEXT NOT NULL DEFAULT ''` 差分を注記（実測 `journal_reader_impl.rs:99`）。
- `inception/units-generation/unit-of-work.md:93` — U3 合格条件の `audit_lock.qnt` は退役済み（実測 `formal/orchestration/` = engine_loop / journal_protocol / stop_hook）→ 注記のみ方式（BR3.7 (c)）で失効注記。
- `developer-report-3.md:218` — BR3.3 (j) の相互参照「14 本」は列挙 15 件と不一致 → 15 に訂正（code-summary §5 の「14 本」は凍結のため据え置き — 正は 15）。
- `.markdownlint-cli2.jsonc` — `developer-brief-*.md` は 1〜2 行目を `AIDLC-UNIT` / `AIDLC-TESTING-CONTRACT` にする派遣契約のため MD041 を満たせない → ignores に追加（markdownlint は CI ゲート外）。

**却下 2 件**（根拠付きで返信）: unit-of-work U2 / U3 / U5 本文の現行名への書き換え（BR3.7 (c) の注記のみ方式 — 完了 Unit の定義は履歴、:64 / :83 / :91 / :144 に改名・失効注記あり）、unit-of-work :91 の Repository 所有 Unit 明記（Unit 定義の再編は units-generation の裁定事項で U9 の範囲外。`IntentRepository` / `CompiledDefinitionRepository` は実装済みで、帰属の追記は上流ゲートへ申し送り）。

**凍結成果物のため繰延 13 件**（本 Unit の指紋済み成果物 4 + 上流ステージの凍結成果物 9）:

9. `unit-test-instructions.md` §1 (10) の `gh pr view <n>` / `-F o=<owner>` は置換前の `<...>` が Bash で入力リダイレクトに解釈される → `pr=119; owner=amadeus-dlc; repo=amadeus-ng` の変数定義と引用に直す（指紋済み）。
10. `code-generation-plan.md:20` / `code-generation-questions.md:24`（P4）の基準コミット `e8ca4a5f` → `02cacea2`（上記 4 と同件、指紋済み）。
11. 上記 5 の `..HEAD` 形は `nfr-design/security-design.md:64-65`、`nfr-requirements/nfr-requirements-questions.md:30-31`、`security-requirements.md:35`（NFR2.1）、
    `tech-stack-decisions.md:18`、`unit-test-instructions.md` (1)(5)(6)(8) にも同形で残る → 各ステージゲートの Request Changes で作業ツリー比較へ。
12. functional-design（凍結）: gap-measurement §4 BR1.6 の対象行数 10 → 12（`README.md:115` 追加）、P4 テストダブル 3 層（T1）、publication 6 表（T6）、rules.md BR3.3 (g)
    「同期 `catch_up`」→「`async fn` を await で直列呼出」、rules.md BR3.7 (d) / entities.md:67 の「decisions.md 変更しない」→ ADR-010 失効注記の実改訂を反映（code-summary §7.1 と同件）。
13. nfr-design（凍結）: nfr-design-questions.md:29「オーナー裁定（選択肢 A）」→「オーナー回答（会話、2026-09-07、選択肢 A）」、同 :45「40 件は改訂で消える」→
    「改訂または履歴マーカー付与で検査対象外になる」、security-design.md:12「選択肢 A」→ 実在する確認記録（`Looks correct`）の参照。
14. nfr-requirements（凍結）: security-requirements.md:17-20 / nfr-requirements-questions.md:28-29 / tech-stack-decisions.md:27 の coding-rules 対象範囲を
    「本文 12 行 + 履歴マーカー 8 行 = 20 行・9 ファイル」（実績は 24 件、code-summary §3）へ、security-requirements.md:92 R-02 行の `（ポート）| 削除` を `\|` にエスケープ。
