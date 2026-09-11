# カバレッジ相対ゲートの未達（Step 9）

2026-09-11。`bash scripts/coverage.sh --base main` の結果。絶対床は満たすが、相対ゲートを満たさない。

| ゲート | 値 | 判定 |
| --- | --- | --- |
| 絶対床 90.0% | head 94.77% | PASS |
| 相対 `head >= base - 0.01` | base（main）99.15% / head 94.77% | **FAIL**（差 4.39 ポイント） |

同じ計測を通すには未カバー行を 4,532 行から約 745 行以下へ減らす必要がある（約 3,800 行分のテスト駆動）。全文ログは [step9-logs/s9-coverage.log](step9-logs/s9-coverage.log)、head の per-file JSON は [step9-logs/s9-coverage-head-per-file.json](step9-logs/s9-coverage-head-per-file.json)。

計測前に llvm-cov 計測下でのみ落ちる契約テスト（計測済み子プロセスが cwd に `.profraw` を書く副産物）を `tests/support/coverage_profile_env.rs` の引き継ぎで是正した（閾値・アサートは無変更）。

## 未カバー行の多いファイル

| 未カバー行 | 総行 | 率 | ファイル |
| ---: | ---: | ---: | --- |
| 234 | 3519 | 93.4% | `modules/app/aidlc/src/runtime.rs` |
| 116 | 312 | 62.8% | `modules/core/read-model-updater/src/orchestration/hook_health_reader.rs` |
| 109 | 228 | 52.2% | `modules/app/aidlc/src/runtime/session_start.rs` |
| 105 | 6097 | 98.3% | `modules/core/command/domain/src/orchestration/intent_execution.rs` |
| 96 | 469 | 79.5% | `modules/core/read-model-updater/src/orchestration/runtime_graph_read_model_updater.rs` |
| 89 | 152 | 41.4% | `modules/core/command/interface-adapter/src/orchestration/session_audit_repository_impl.rs` |
| 84 | 630 | 86.7% | `modules/app/aidlc/src/runtime/plan_approval.rs` |
| 80 | 2412 | 96.7% | `modules/app/aidlc/src/turn.rs` |
| 75 | 480 | 84.4% | `modules/app/aidlc/src/runtime/continuation.rs` |
| 68 | 164 | 58.5% | `modules/core/command/interface-adapter/src/orchestration/plan_approval_runtime_repository_impl.rs` |
| 66 | 549 | 88.0% | `modules/harness/infrastructure/src/shell_write_targets.rs` |
| 65 | 158 | 58.9% | `modules/core/command/interface-adapter/src/orchestration/workflow_continuation_repository_impl.rs` |
| 63 | 2729 | 97.7% | `modules/core/read-model-updater/src/workspace/projection.rs` |
| 57 | 416 | 86.3% | `modules/app/aidlc/src/runtime/learnings.rs` |
| 53 | 511 | 89.6% | `modules/harness/infrastructure/src/shell_mutation.rs` |
| 52 | 217 | 76.0% | `modules/app/aidlc/src/runtime/jump.rs` |
| 52 | 106 | 50.9% | `modules/core/command/interface-adapter/src/orchestration/dto/hook_health_event_dto.rs` |
| 50 | 194 | 74.2% | `modules/app/aidlc/src/runtime/testing_posture.rs` |
| 49 | 497 | 90.1% | `modules/app/aidlc/src/source_baseline.rs` |
| 48 | 329 | 85.4% | `modules/harness/claude/src/delegated_lifecycle.rs` |
| 48 | 196 | 75.5% | `modules/app/aidlc/src/session_navigation.rs` |
| 42 | 242 | 82.6% | `modules/app/aidlc/src/runtime/pipeline_link.rs` |
| 42 | 74 | 43.2% | `modules/core/read-model-updater/src/orchestration/dto/learnings_captured_dto.rs` |
| 40 | 113 | 64.6% | `modules/core/command/interface-adapter/src/orchestration/dto/plan_approval_event_dto.rs` |
| 39 | 714 | 94.5% | `modules/core/command/domain/src/orchestration/plan_approval_runtime.rs` |
| 38 | 2013 | 98.1% | `modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs` |
| 37 | 141 | 73.8% | `modules/core/command/domain/src/orchestration/learning_observations.rs` |
| 36 | 78 | 53.8% | `modules/core/read-model-updater/src/orchestration/dto/active_directive_dto.rs` |
| 33 | 759 | 95.7% | `modules/app/aidlc/src/wording.rs` |
| 33 | 441 | 92.5% | `modules/core/read-model-updater/src/orchestration/read_model_updater.rs` |
| 30 | 164 | 81.7% | `modules/core/read-model-updater/src/orchestration/dto/intent_execution_event_dto.rs` |
| 29 | 309 | 90.6% | `modules/core/command/interface-adapter/src/orchestration/dto/intent_execution_event_dto.rs` |
| 29 | 53 | 45.3% | `modules/core/read-model-updater/src/orchestration/dto/memory_journals_observed_dto.rs` |
| 27 | 190 | 85.8% | `modules/harness/infrastructure/src/reviewer_scope_segments.rs` |
| 27 | 74 | 63.5% | `modules/core/read-model-updater/src/orchestration/plan_approval_files.rs` |
| 27 | 62 | 56.5% | `modules/core/command/domain/src/orchestration/write_targets.rs` |
| 26 | 152 | 82.9% | `modules/harness/infrastructure/src/shell_substitutions.rs` |
| 26 | 39 | 33.3% | `modules/core/command/domain/src/orchestration/plan_target.rs` |
| 25 | 204 | 87.7% | `modules/core/read-model-updater/src/orchestration/workflow_continuation_read_model_updater.rs` |
| 25 | 149 | 83.2% | `modules/core/command/interface-adapter/src/orchestration/artifact_audit_repository_impl.rs` |
| 25 | 147 | 83.0% | `modules/app/aidlc/src/runtime/review_documents.rs` |
| 25 | 143 | 82.5% | `modules/core/command/interface-adapter/src/orchestration/hook_health_repository_impl.rs` |
| 25 | 34 | 26.5% | `modules/core/command/domain/src/orchestration/reviewer_scope_candidates.rs` |
| 25 | 34 | 26.5% | `modules/core/command/domain/src/orchestration/inspected_command.rs` |
| 25 | 25 | 0.0% | `modules/core/command/use-case/src/orchestration/jump_error.rs` |
| 24 | 249 | 90.4% | `modules/app/aidlc/src/runtime/review_guards.rs` |
| 24 | 137 | 82.5% | `modules/core/read-model-updater/src/orchestration/dto/reported_dto.rs` |
| 24 | 120 | 80.0% | `modules/core/command/domain/src/orchestration/memory_entries.rs` |
| 24 | 49 | 51.0% | `modules/core/read-model-updater/src/orchestration/dto/plan_approval_evidence_dto.rs` |
| 24 | 47 | 48.9% | `modules/core/read-model-updater/src/orchestration/dto/answer_recorded_dto.rs` |
| 24 | 33 | 27.3% | `modules/harness/infrastructure/src/reviewer_scope_words.rs` |
| 23 | 910 | 97.5% | `modules/core/command/use-case/src/orchestration/commit_verdict_use_case.rs` |
| 23 | 237 | 90.3% | `modules/app/aidlc/src/runtime/session_hooks.rs` |
| 23 | 186 | 87.6% | `modules/harness/infrastructure/src/redirection_free_words.rs` |
| 22 | 801 | 97.3% | `modules/core/read-model-updater/src/read_tables/sql.rs` |
| 22 | 265 | 91.7% | `modules/harness/infrastructure/src/shell_invocation.rs` |
| 22 | 137 | 83.9% | `modules/core/command/interface-adapter/src/orchestration/dto/reported_dto.rs` |
| 21 | 191 | 89.0% | `modules/core/command/domain/src/orchestration/code_generation_authority.rs` |
| 21 | 149 | 85.9% | `modules/core/command/domain/src/orchestration/plan_approval_evidence.rs` |
| 21 | 133 | 84.2% | `modules/core/command/domain/src/orchestration/review_appendix.rs` |
| 21 | 61 | 65.6% | `modules/core/read-model-updater/src/orchestration/dto/plan_approval_runtime_dto.rs` |
| 21 | 52 | 59.6% | `modules/core/read-model-updater/src/orchestration/dto/decision_recorded_dto.rs` |
| 21 | 30 | 30.0% | `modules/core/command/domain/src/orchestration/inspected_command_step.rs` |
| 20 | 173 | 88.4% | `modules/app/aidlc/src/session_processes.rs` |
| 20 | 139 | 85.6% | `modules/core/command/domain/src/orchestration/review_documents.rs` |
| 20 | 112 | 82.1% | `modules/harness/infrastructure/src/shell_words.rs` |
| 20 | 103 | 80.6% | `modules/core/command/domain/src/orchestration/memory_journal_survey.rs` |
| 18 | 685 | 97.4% | `modules/core/command/use-case/src/orchestration/test_support.rs` |
| 18 | 306 | 94.1% | `modules/core/command/interface-adapter/src/orchestration/dto/intent_execution_dto.rs` |
| 18 | 133 | 86.5% | `modules/core/read-model-updater/src/orchestration/plan_approval_journal_reader_impl.rs` |
| 18 | 40 | 55.0% | `modules/core/read-model-updater/src/orchestration/dto/pipeline_link_completed_dto.rs` |
| 18 | 30 | 40.0% | `modules/core/command/domain/src/orchestration/exempt_paths.rs` |
| 17 | 566 | 97.0% | `modules/core/command/interface-adapter/src/orchestration/workflow_definition_repository_impl.rs` |
| 17 | 155 | 89.0% | `modules/core/read-model-updater/src/orchestration/dto/intent_dto.rs` |
| 17 | 36 | 52.8% | `modules/core/command/use-case/src/orchestration/plan_approval_command_error.rs` |
| 16 | 441 | 96.4% | `modules/harness/claude/src/dispatch_rules_envelope.rs` |
| 16 | 215 | 92.6% | `modules/core/read-model-updater/src/read_tables/plan_approval_tables.rs` |
| 16 | 129 | 87.6% | `modules/app/aidlc/src/source_fingerprint.rs` |
| 16 | 45 | 64.4% | `modules/app/aidlc/src/summary_questions_input.rs` |
| 16 | 42 | 61.9% | `modules/app/aidlc/src/runtime/task_sync.rs` |
| 16 | 21 | 23.8% | `modules/core/command/domain/src/orchestration/plan_runtime_error.rs` |
| 15 | 339 | 95.6% | `modules/core/command/domain/src/orchestration/summary_questions/visibility.rs` |
| 15 | 206 | 92.7% | `modules/core/command/domain/src/orchestration/summary_questions/headings.rs` |
| 15 | 94 | 84.0% | `modules/core/command/domain/src/orchestration/captured_learnings.rs` |
| 15 | 77 | 80.5% | `modules/core/command/domain/src/orchestration/empty_memory_stages.rs` |
| 15 | 74 | 79.7% | `modules/core/command/interface-adapter/src/orchestration/dto/learnings_captured_dto.rs` |
| 15 | 65 | 76.9% | `modules/core/read-model-updater/src/orchestration/plan_source.rs` |
| 15 | 40 | 62.5% | `modules/core/query/use-case/src/orchestration/port/artifact_audit_view.rs` |
| 15 | 23 | 34.8% | `modules/core/command/interface-adapter/src/orchestration/dto/jump_scope_dto.rs` |
| 15 | 15 | 0.0% | `modules/core/command/domain/src/orchestration/answer_error.rs` |
| 14 | 882 | 98.4% | `modules/core/command/interface-adapter/src/orchestration/compiled_definition_repository_impl.rs` |
| 14 | 371 | 96.2% | `modules/app/aidlc/src/layout.rs` |
| 14 | 172 | 91.9% | `modules/harness/infrastructure/src/shell_text.rs` |
| 14 | 147 | 90.5% | `modules/app/aidlc/src/runtime/runtime_graph.rs` |
| 14 | 109 | 87.2% | `modules/core/read-model-updater/src/orchestration/intent_registry.rs` |
| 14 | 30 | 53.3% | `modules/core/read-model-updater/src/orchestration/dto/jump_observation_dto.rs` |
| 13 | 293 | 95.6% | `modules/app/aidlc/src/validation_basis.rs` |
| 13 | 95 | 86.3% | `modules/core/command/domain/src/orchestration/summary_questions.rs` |
| 13 | 32 | 59.4% | `modules/core/read-model-updater/src/orchestration/dto/prompt_observed_dto.rs` |
| 13 | 25 | 48.0% | `modules/core/read-model-updater/src/orchestration/dto/plan_answer_input_dto.rs` |
| 13 | 13 | 0.0% | `modules/harness/infrastructure/src/shell_parse_error.rs` |
| 12 | 129 | 90.7% | `modules/core/read-model-updater/src/read_tables/jump_result_row.rs` |
| 12 | 95 | 87.4% | `modules/core/command/domain/src/orchestration/summary_questions/containers.rs` |
| 12 | 84 | 85.7% | `modules/harness/infrastructure/src/shell_segments.rs` |
| 12 | 21 | 42.9% | `modules/core/command/interface-adapter/src/orchestration/dto/session_audit_event_dto.rs` |
| 12 | 12 | 0.0% | `modules/core/command/use-case/src/orchestration/interaction_command_error.rs` |
| 12 | 12 | 0.0% | `modules/core/command/domain/src/orchestration/review_evidence_error.rs` |
| 11 | 270 | 95.9% | `modules/core/command/use-case/src/orchestration/record_single_stage_run_use_case.rs` |
| 11 | 120 | 90.8% | `modules/harness/claude/src/stop_transcript.rs` |
| 11 | 19 | 42.1% | `modules/core/read-model-updater/src/orchestration/dto/command_failed_dto.rs` |
| 11 | 18 | 38.9% | `modules/core/command/use-case/src/orchestration/continuation_command_error.rs` |
| 11 | 11 | 0.0% | `modules/core/command/use-case/src/orchestration/artifact_audit_command_error.rs` |
| 10 | 449 | 97.8% | `modules/core/command/interface-adapter/src/orchestration/intent_repository_impl.rs` |
| 10 | 438 | 97.7% | `modules/app/aidlc/src/cli/request.rs` |
| 10 | 208 | 95.2% | `modules/core/infrastructure/src/canon_json/parse.rs` |
| 10 | 122 | 91.8% | `modules/core/command/domain/src/orchestration/command_error.rs` |
| 10 | 63 | 84.1% | `modules/app/aidlc/src/usage_ledger/stage_bucket.rs` |
| 10 | 41 | 75.6% | `modules/core/command/domain/src/orchestration/pending_summary_decisions.rs` |
| 10 | 19 | 47.4% | `modules/core/read-model-updater/src/orchestration/dto/plan_answer_logged_dto.rs` |
| 10 | 18 | 44.4% | `modules/core/read-model-updater/src/orchestration/dto/health_checked_dto.rs` |
| 10 | 10 | 0.0% | `modules/core/read-model-updater/src/orchestration/journal_reader.rs` |
| 10 | 10 | 0.0% | `modules/core/command/use-case/src/orchestration/task_synchronization_error.rs` |
| 10 | 10 | 0.0% | `modules/core/command/use-case/src/orchestration/memory_journal_error.rs` |
| 10 | 10 | 0.0% | `modules/core/command/use-case/src/orchestration/learning_capture_error.rs` |
| 10 | 10 | 0.0% | `modules/core/command/use-case/src/orchestration/health_check_error.rs` |

（未カバー 10 行以上のファイルのみ。全体: 82046/86578 行、未カバー 4532 行、944 ファイル）
