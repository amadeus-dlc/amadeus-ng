# カバレッジ向上 報告書 — 担当 `u2_cov_domain_uc`（`core-command-domain` / `core-command-use-case`）

2026-09-11。対象一覧は [targets-domain-uc.md](targets-domain-uc.md)（workspace 計測で未カバー 1,088 行・107 ファイル）。プロダクトコードは変更していない（テスト追加のみ）。ログは [u2_cov_domain_uc/](u2_cov_domain_uc/)。

## 結果の要約

| 項目 | 値 |
| --- | --- |
| 対象ファイルの未カバー行（workspace 計測、着手前） | 1,066 行（私の行導出で数えた値。一覧表の 1,088 との差は集計方法による） |
| 同・着地後（crate 単独計測との共通部分で上界を推定） | **297 行（72.1% 減）** |
| 内訳: `#[cfg(test)]` 内の行（テストの `let … else { panic!() }` 腕など） | 70 行 |
| 内訳: `use-case/src/orchestration/test_support.rs`（テスト用フィクスチャ） | 16 行 |
| 内訳: プロダクトコード | 211 行（うち到達不能の dead code 候補は下記） |
| crate 単独の行カバレッジ（2 crate 合算、`cargo llvm-cov -p … --tests`） | 着手前 87.95%（34,758 行中 30,571）→ 着地後 **91.59%**（34,850 行中 31,919） |
| 着地後・crate 別 | domain 93.50%（28,683 行）、use-case 82.68%（6,167 行） |
| `cargo test -p core-command-domain -p core-command-use-case` | 0 失敗（[06-green-both-crates.log](u2_cov_domain_uc/06-green-both-crates.log)） |
| `cargo fmt --all --check` / `cargo clippy … --all-targets -- -D warnings` / `cargo lint` | すべて成功（[07-fmt-clippy-lint.log](u2_cov_domain_uc/07-fmt-clippy-lint.log)） |

完了条件 1（85% 減）には届いていない（72.1% 減）。残り 297 行のうち 86 行はテストコード自身、プロダクト側の残りも `?` による伝播行・防御的な到達不能分岐・Markdown パーサの深い枝が大半である（後述）。

注記: 着地後の「残り」は **workspace 計測の未カバー集合 ∩ crate 単独計測の未カバー集合** で求めた上界である。workspace 計測は上位層のテストも実行するので、実際の残りはこれ以下になる。着手前の crate 単独計測 JSON は共有スクラッチ領域で他担当に上書きされたため、着手前の数値は計測直後に記録した集計値（87.95%）を採用した。

## 追加したテスト（合計 104 件、すべて TDD の red → green を経て着地）

### `core-command-domain`（`tests/` の新規 5 ファイル + 既存 4 ファイルへ追記）

| ファイル | 件数 | 検証した契約 |
| --- | ---: | --- |
| `tests/rejection_material_contract.rs`（新規） | 12 | 拒否型の `Display` が**材料のみ**を綴ること（`AnswerError` / `ReviewEvidenceError` / `PlanRuntimeError` 全 18 変種・文言の重複なし / `HookHealthError` / `SessionAuditError` / `ContinuationError` / `CommandError` の pipeline・Plan Approval 変種 / 単一変種 parse 拒否 7 型 / `ReportCommitError` の原因連鎖と `Unwired` の無連鎖 / `PipelineLinkError` の Debug 材料） |
| `tests/plan_approval_rejection_contract.rs`（新規） | 24 | `PlanTarget::for_unit`（空・65 文字超・非 ASCII 成分の逐語拒否）、`PlanSession`（キー正規化・96 文字上限・空白/記号のみ拒否）、`PlanApprovalOperationId` / `PlanApprovalEventId`（正準 UUIDv7 のみ受理・`generate` の往復）、`PlanOfferedOptions`（2 択厳守）、`HookHealthTarget::parse`（相対領域の往復と 5 形の拒否）、`CodeGenerationAuthority::resolve` の全拒否経路（state 欠落・指示欠落/stale・境界欠落・別 stage・load-steering/error 種別・Unit 不一致・ゼロ Unit 要求）、`PlanApprovalEvidence::verify_for_file` の全拒否経路（ゴールデン `plan-readiness.json` の `approved` 観測を材料に: source floor 不一致・非正規 questions file・空文書・stale 契約・指紋不一致・回答不一致 `(blank)`）、`PlanApprovalRuntime::request_generation` の「受領未保持」拒否 |
| `tests/review_evidence_contract.rs`（新規） | 10 | `ReviewDocuments` の受理判断: 不安定 snapshot・必須成果物の欠落/非ファイル → `ArtifactsUnavailable`、任意成果物の欠落は指紋に載る、追記先より前の改変 → `ArtifactsChanged`、source 変化 → `SourceChanged`、残置された旧 Review 節 → `StaleAppendix`、書き直しに `Request Challenge` 行が無い/不要なのに有る → `InvalidAppendix`（逐語）、`## Review` 以外の見出し・後続 H2・verdict 不一致の逐語、CRLF/タブの先頭空白は証跡に含めない、再試行の NOT-READY だけ空追記を許す、既存 Review 節の pin 位置 |
| `tests/value_object_rejection_contract.rs`（新規） | 25 | `MemoryEntry::parse` の退化（記号なし・空白なしダッシュ）、`ContinuationRequest::new`（上限 0・リセット履歴の拒否）と環境上限の解釈（負/0/非数は既定、巨大値は `u64::MAX`）、`DecisionPrompt` の rationale/options、`CodeGenerationRunFloor::new` の不整合拒否、`ReviewerScopeBlock` の材料と `Blocked` 判定、`SessionAudit::new` / `HookHealth::new` の再構成拒否、`SummaryQuestions` の構造拒否（字下げ見出し・重複 Q<n>・重複 Assumption Confirmation・除外ダイジェスト・コンテナ内の `[Answer]:` 不採用・setext/HTML 見出しは節を開かない・角括弧リンク/インラインコード内の `<h1>`・コメント/raw HTML/フェンスの不可視化）、`TryFrom<String>`（`StageSlug` / `WorkflowDefinitionId` / `DefinitionRevision`）、識別子の表示と拒否（`AnswerId` / `SessionAuditEventId` / `ArtifactAuditEventId` / `SessionAuditId` / `SpaceName`）、`SummaryEvidence` の digest 検査、`PlanQuestions::default` / `TransitionSteps::default`、`StageNode::produced_artifact_files` |
| `tests/intent_execution_guard_contract.rs`（新規） | 9 | `IntentExecution` の入口ガード: pipeline 系 4 コマンドが別 intent を `IntentMismatch` で拒み状態を動かさない、完了根拠不要/pipeline 無効の report は通る、計画承認 3 クエリの別 intent 拒否と自 intent でも指示未発行なら解決不能、`verify_plan_answer` の別実行拒否、`plan_fingerprint` の逐語 4 形（回答済み・計画空・指示空・契約なし）、`synchronize_task` の未知 stage、`record_single_stage_run` の未開始/未知 stage、完全コンストラクタ `IntentExecution::new` の再構成拒否 3 形（別実行の対話状態・進捗通番の範囲外・別実行の pipeline 履歴） |
| `tests/collection_contract_test.rs`（追記） | 1 | 一級コレクション 10 型（`WriteTargets` / `ExemptPaths` / `InspectedCommand` / `InspectedCommandStep` / `ReviewerScopeCandidates` / `LearningObservations` / `CapturedLearnings` / `MemoryEntries` / `MemoryJournalSurvey` / `EmptyMemoryStages`）の走査契約と `Default` の同値 |
| `tests/plan_authority_contract.rs`（追記） | 4 | `PlanApprovalRuntime` の保留ガードが**どの変種**で拒むか（`PendingResponse` / `PendingInvalidation` / `PendingAnswer` / `ResponseTargetMismatch` / `NoPreparedResponse` / `UnknownInvalidation` / `InvalidationTargetMismatch` / `AnswerTargetMismatch` / `InvalidState`）、`PlanGeneration::new` と `PlanAnswer::new` の受領不一致拒否、別実行への応答記録拒否（`PlanResponseTargetMismatch` / `PlanResponseUnavailable`）、回答記録/開始要求の保留待ちと修正要求回答の受領なし |
| `tests/workflow_continuation_contract.rs`（追記） | 1 | `guard_signature` が実効 guard の進捗を指す |
| `tests/testing_posture_contract.rs`（追記） | 3 | 空の `Methodology:` は宣言ではない、`custom` は本文を順序として保つ、フェンス/インラインコード/HTML コメント内の宣言は分類対象外（4 空白字下げ・短い閉じ記号・バッククォート付き info 文字列・エスケープの各境界） |

### `core-command-use-case`（`src` の `#[cfg(test)]`）

| ファイル | 件数 | 検証した契約 |
| --- | ---: | --- |
| `src/orchestration/command_error_material_tests.rs`（新規、`mod.rs` に `#[cfg(test)] mod` を 1 行追加） | 10 | 失敗封筒 15 型の `Display` 材料と `From` 変換、`RepositoryError::Corrupt` の原因連鎖を封筒が切らないこと（`JumpError` / `PlanApprovalCommandError` / `InteractionCommandError` / `ContinuationCommandError` / `ArtifactAuditCommandError` / `SessionAuditCommandError` / `HookHealthCommandError` / `TaskSynchronizationError` / `MemoryJournalError` / `LearningCaptureError` / `HealthCheckError` / `ReviewFreezeError` / `PipelineLinkCommandError` / `ReviewerScopeError` + `ReviewerScopeCause` / `CommitError` の Pipeline・ReportResult） |
| `src/orchestration/begin_single_stage_run_use_case.rs`（追記） | 4 | 開始の 1 件コミット、既に開いている試行は再コミットせず成功、initialization の拒否を封筒へ写す、競合の 1 回再試行と 2 回目の伝播 |
| `src/orchestration/record_single_stage_run_use_case.rs`（追記） | 1 | 開始未記録の試行は集約の `SingleStageAttemptNotOpen` として拒否される |

`test_support.rs` の既存フィクスチャ（`genesis` / `InMemoryIntentExecutionRepository::holding` / `holding_behind_concurrent_writes` / `InMemoryIntentRepository::holding`）を再利用した。

## Dead code 候補（到達不能と判断した行。削除はしていない）

| ファイル:行 | 内容 | 根拠 |
| --- | --- | --- |
| `domain/src/orchestration/intent_execution.rs:1336-1340` | `plan_fingerprint` の `embedded.ok_or_else(…)` | 直前の `embedded.as_ref().is_none_or(…)` ガードで `None` は既に `Err` 返却済み。到達時は必ず `Some` |
| `domain/src/orchestration/intent_execution.rs:2021-2022` | `recompose` の「Execute でも Skip でもない反転」拒否 | `effective_plan` が `None` になるのは範囲外のみで、範囲外は直前の `out_of_reach` で拒否済み |
| `domain/src/orchestration/plan_approval_runtime.rs:233-242` | `record_answer` の `offered` / `response` の `ok_or_else` 2 箇所 | 直前の `valid` 判定が `offered.is_some_and(… response().is_some_and(…))` を要求するため、到達時は両方 `Some` |
| `domain/src/orchestration/plan_approval_runtime.rs:518-519` | `resolve_invalidation` の `UnknownInvalidation` | 唯一の呼出し元 `resolve_for_publication`（同 412-428）が `invalidations` の存在を先に検査する（`grep -n "resolve_invalidation(" plan_approval_runtime.rs` → 427 の 1 箇所） |
| `domain/src/orchestration/testing_posture/classification.rs:87-88` | 構造化 Methodology を `components` へ追加する枝 | `scan` は `method` 自身を含むので、受理される 4 語（tdd/bdd/atdd/test-after）は常に語句照合で `components` に入る |
| `domain/src/orchestration/testing_posture/classification.rs:129-131` | `default_ordering("custom")` | `custom` は呼出し側で本文を順序に使うため `default_ordering` に `"custom"` が渡らない |
| `domain/src/orchestration/testing_posture/classification.rs:116-118` | 正規表現のコンパイル失敗 | パターンは定数リテラル |
| `use-case/src/orchestration/record_single_stage_run_use_case.rs:152-160` | `record_single_stage_run` の拒否を封筒へ写す `map_err` | 直前の `require_pipeline_single` が同じ intent・同じ stage で `UnknownStage` / `SingleStageAttemptNotOpen` を先に拒むため、集約側の同じ拒否には到達しない（別 intent は同一 UseCase 内で発生しない） |
| `domain/src/orchestration/summary_questions.rs:125-127` | `missing required H2 section` | `parse` は `in_summary` を先に検査して `InvalidAnswer` を返すため、`confirmed_lines` に summary 無しで到達しない |

## 残った未カバー行と理由（297 行）

| 分類 | 行数 | 例 |
| --- | ---: | --- |
| テストの `let … else { panic!(…) }` 腕・`panic!` 補助関数 | 70 | `intent_execution.rs` 3899〜9046（38 行）、`commit_verdict_use_case.rs` 389〜733、`guard_reviewer_scope_use_case.rs` 176/180/236、`workflow_definition.rs` 727/759/844 など。成功するテストでは構造的に実行されない |
| テスト用フィクスチャの未使用経路 | 16 | `use-case/src/orchestration/test_support.rs` 205, 428, 438-445, 711-716（`reviewer_max_iterations` / `CompiledDefinition` の `store` / `find_for_approval_origin` スタブ）。フィクスチャ自身を呼ぶだけの無意味な呼出しは避けた |
| 上記 dead code 候補 | 約 25 | 表のとおり |
| `?` による失敗伝播行（呼出し先が失敗しない不変条件下） | 約 30 | `intent_execution.rs` 1383/2693/2702、`workflow_continuation.rs` 84/93/127/164、`session_audit.rs` 70/79/106、`hook_health.rs` 155/185、`plan_approval_runtime.rs` 250/262、`code_generation_run_floor.rs` 103-106（通番枯渇）など。検査済みの値からの再構成で失敗させるには不変条件を破る材料が要り、公開 API からは作れない |
| 再生（replay）時の履歴破損 `panic!` | 4 | `plan_approval_runtime.rs:662`、`workflow_continuation.rs:207`、`hook_health.rs:83`、`session_audit` 系。壊れた履歴は書込時に検査済みという裁定（2026-08-30）で回復不能扱い |
| Markdown パーサの深い枝 | 約 25 | `summary_questions/visibility.rs`（raw HTML 継続・setext 下線の走査位置など）、`headings.rs`（リンク参照定義のエスケープ判定）、`containers.rs`（タブ幅計算）、`testing_posture/markdown.rs`（エスケープされたバッククォート走査）。本家の観測（ゴールデン）に無い入力を契約として固定することを避けた |
| UseCase の error 写し・監査失敗経路 | 約 20 | `commit_verdict_use_case.rs` 228-232/241/255-256/277/298-300（`ReportCommitError` の変種別写し・`Definition` 無しの経路・2 回目競合）、`guard_review_freeze_use_case.rs` 18-20/141/151/153、`guard_reviewer_scope_use_case.rs` 101/111/113（監査保存の失敗経路）。既存の in-memory 監査リポジトリに `failing_on_store` はあるが、凍結側の失敗注入は追加フィクスチャが要る |
| その他の小片（`?`・アクセサ・`Display`） | 残り | `reviewer_scope.rs` 232/235/255/351、`reviewer_scope_paths.rs` 69/73/74/145（グロブ照合のバックトラック）、`intent_execution.rs` 880/885/902（review freeze の早期 `Allowed` / 欠落 slot）、`pending_summary_decisions.rs` 22-28（内容確認提示付き `DecisionRecorded` の復元）など |

## 所見（プロダクトコードの問題として親へ渡す。修正はしていない）

- `domain/src/orchestration/pipeline_link_error.rs:89-98` の `Display` 実装は、書式文字列の中に doc コメント行 `/// 指定されたlink。` と改行が混入している（`"pipeline \n/// 指定されたlink。\nlink: {self:?}"`）。テストでは先頭 `pipeline` と末尾の `Debug` 材料だけを固定し、混入部分は契約として固定していない。文言の正本は出す側（`wording`）にあるので観測互換には影響しないが、コードとしては誤りに見える。
- `record_single_stage_run_use_case.rs:152-160` と `summary_questions.rs:125-127` は防御的な二重検査で、上流の検査と同じ材料を別の封筒へ写している（dead code 候補の表を参照）。

## ログ

| ログ | 内容 |
| --- | --- |
| [01-baseline-crate-coverage.log](u2_cov_domain_uc/01-baseline-crate-coverage.log) | 着手前の crate 計測（テスト結果。集計値 87.95% は計測直後の出力から記録） |
| [02-red-first-run.log](u2_cov_domain_uc/02-red-first-run.log) | 新規テストの初回実行（`plan_fingerprint` の質問文書フィクスチャが red） |
| [03-green-guard-and-runtime.log](u2_cov_domain_uc/03-green-guard-and-runtime.log) | 同フィクスチャ修正後の green |
| [04-mid-crate-coverage.log](u2_cov_domain_uc/04-mid-crate-coverage.log) | 中間計測（`review_evidence_contract` の red を検出） |
| [05-green-second-run.log](u2_cov_domain_uc/05-green-second-run.log) | 2 回目の全実行（コンテナ内 `[Answer]:` の契約が red → 観測結果へ書き直し） |
| [06-green-both-crates.log](u2_cov_domain_uc/06-green-both-crates.log) | 両 crate の `cargo test` 0 失敗 |
| [07-fmt-clippy-lint.log](u2_cov_domain_uc/07-fmt-clippy-lint.log) | `cargo fmt --all --check` / clippy `-D warnings` / `cargo lint` 成功 |
| [08-final-crate-coverage.log](u2_cov_domain_uc/08-final-crate-coverage.log) | 着地後の crate 計測 |
