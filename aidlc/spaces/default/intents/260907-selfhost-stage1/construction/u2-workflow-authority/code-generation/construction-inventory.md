# 集約識別子・構築経路・setter是正の棚卸し

## 判定基準と走査範囲

利用者の2026-09-08指示と `knowledge/aidlc-shared/coding-rules/factory-naming.md`、`ubiquitous-language.md`、`tell-dont-ask.md` を適用した。consuming `with_*` はファクトリでありsetterに分類しない。完成型の構築は全フィールドと定義済み初期値を同時に初期化する口へ集める。集合のinsert/removeと、保存済みイベントの適用も、名前だけでsetterとはしない。

`modules/`、`tools/lint/src/`、`tests/support/` のRustソースをsyn構文木で走査した。構造体、フィールド、impl/traitメソッド、Default derive、構造体/タプル構築を一覧化し、その後コードを読んで分類した。マクロを展開せず、deriveによるClone/Deserializeの内部までは走査していない。DTOのDeserializeは輸送の復号であり、ドメイン再構成では検査付きの構築口へ渡す。enum変種は構造体の二重構築と数えない。

## 集約と識別子

| 集約 | idの型 | 確認結果 |
| --- | --- | --- |
| Intent | IntentId | 一致。Createdから全フィールドを構築。 |
| IntentExecution | IntentExecutionId | 一致。検査付きnewへ復元材料を渡す。 |
| WorkflowDefinition | WorkflowDefinitionId | 一致。Definedから構築し、保存通番/versionは消費型ファクトリで復元。 |
| CompiledDefinition | CompiledDefinitionId | 一致。Compiledの内容から構築しrevisionを導出。 |
| PlanApprovalRuntime | PlanApprovalRuntimeId | 一致。ワークスペース全体の承認状態を管理する集約。 |

`PlanApprovalOperationId` は回答・開始・配送操作の相関識別子であり集約識別子ではない。PlanApprovalRuntimeRepositoryのキーはPlanApprovalRuntimeIdで、イベントのaggregate_idも同型である。イベント自身は別のEventIdを持つ。適合している集約識別子は改名していない。

## 構築経路の是正対象

以下は初回AST走査で、実構造体に複数リテラルまたはDefault deriveと別リテラルが確認された型。型ごとの公開入力検査・既定値・順序を維持し、構築口へ集約した。検査済み部分集合を包むprivateな構築口は外部へ公開しない。

| 型 | ファイル | 初回の構築箇所 |
| --- | --- | --- |
| TestingContractRow | `modules/core/read-model-updater/src/read_tables/testing_contract_row.rs` | literal:of, literal:of |
| MemoryRules | `modules/core/read-model-updater/src/read_tables/memory_rules.rs` | derived-default:, literal:new |
| PublicationFile | `modules/core/read-model-updater/src/orchestration/publication_file.rs` | literal:replacement, literal:audit, literal:restored |
| Head | `modules/core/read-model-updater/src/orchestration/shared_projection.rs` | literal:read, literal:record |
| PublicationBatch | `modules/core/read-model-updater/src/orchestration/publication_batch.rs` | literal:new, literal:restored |
| GlobalSeqNr | `modules/core/read-model-updater/src/orchestration/global_seq_nr.rs` | tuple-literal:, tuple-literal:new, tuple-literal:from |
| InMemoryWorkflowDefinitionRepository | `modules/core/command/use-case/src/orchestration/test_support.rs` | literal:new, literal:corrupt |
| InMemoryCompiledDefinitionRepository | `modules/core/command/use-case/src/orchestration/test_support.rs` | literal:serving, literal:unreadable |
| InteractionStateDto | `modules/core/command/interface-adapter/src/orchestration/dto/intent_execution_dto.rs` | derived-default:, literal:of |
| ReviewAttemptDto | `modules/core/command/interface-adapter/src/orchestration/dto/intent_execution_dto.rs` | derived-default:, literal:of |
| IntentRepositoryImpl | `modules/core/command/interface-adapter/src/orchestration/intent_repository_impl.rs` | literal:open, literal:in_memory, literal:reopened |
| WorkflowDefinitionRepositoryImpl | `modules/core/command/interface-adapter/src/orchestration/workflow_definition_repository_impl.rs` | literal:open, literal:in_memory, literal:reopened |
| IntentExecutionRepositoryImpl | `modules/core/command/interface-adapter/src/orchestration/intent_execution_repository_impl.rs` | literal:open, literal:in_memory, literal:reopened |
| StageGraph | `modules/core/command/domain/src/workflow_definition/stage_graph.rs` | literal:filter, literal:new, literal:with_plugin_selection |
| ScopeGrid | `modules/core/command/domain/src/workflow_definition/scope_grid.rs` | derived-default:, literal:new, literal:from_graph |
| WorkflowDefinitionEventId | `modules/core/command/domain/src/workflow_definition/workflow_definition_event_id.rs` | tuple-literal:parse, tuple-literal:generate |
| CompiledDefinitionEventId | `modules/core/command/domain/src/workflow_definition/compiled_definition_event_id.rs` | tuple-literal:parse, tuple-literal:generate |
| PracticesPromotion | `modules/core/command/domain/src/workspace/practices_promotion.rs` | derived-default:, literal:plan |
| HumanTurns | `modules/core/command/domain/src/workspace/human_turns.rs` | derived-default:, literal:find_in |
| OrderedAuditEvents | `modules/core/command/domain/src/workspace/ordered_audit_events.rs` | tuple-literal:filter, tuple-literal:find_in |
| PromotedSections | `modules/core/command/domain/src/workspace/promoted_sections.rs` | derived-default:, literal:new, literal:filter |
| RuleLines | `modules/core/command/domain/src/workspace/rule_lines.rs` | derived-default:, literal:empty, literal:new |
| StateVersionClassification | `modules/core/command/domain/src/workspace/state_version_classification.rs` | literal:classify, literal:of |
| SpaceName | `modules/core/command/domain/src/workspace/space_name.rs` | tuple-literal:default, tuple-literal:parse |
| BoltRefs | `modules/core/command/domain/src/workspace/bolt_refs.rs` | derived-default:, tuple-literal:combine, tuple-literal:divide, tuple-literal:filter, tuple-literal:map, tuple-literal:parse |
| Checkboxes | `modules/core/command/domain/src/workspace/checkboxes.rs` | tuple-literal:parse, tuple-literal:filter, tuple-literal:map |
| AuditFields | `modules/core/command/domain/src/workspace/audit_fields.rs` | derived-default:, tuple-literal:new |
| StorePath | `modules/core/command/domain/src/workspace/store_path.rs` | literal:for_runtime, literal:for_space |
| Containers | `modules/core/command/domain/src/orchestration/summary_questions/containers.rs` | derived-default:, tuple-literal:new |
| PendingSummaryDecisions | `modules/core/command/domain/src/orchestration/pending_summary_decisions.rs` | derived-default:, literal:new |
| IntentExecutionEventId | `modules/core/command/domain/src/orchestration/intent_execution_event_id.rs` | tuple-literal:parse, tuple-literal:generate |
| InteractionState | `modules/core/command/domain/src/orchestration/interaction_state.rs` | derived-default:, literal:new |
| PendingIterations | `modules/core/command/domain/src/orchestration/pending_iterations.rs` | derived-default:, literal:empty, literal:filter |
| PlanReceipts | `modules/core/command/domain/src/orchestration/plan_receipts.rs` | derived-default:, literal:new |
| PlanGenerations | `modules/core/command/domain/src/orchestration/plan_generations.rs` | derived-default:, literal:new |
| TransitionSteps | `modules/core/command/domain/src/orchestration/transition_steps.rs` | derived-default:, literal:new, literal:single, literal:recovered_approval, literal:filter |
| ReviewAttempt | `modules/core/command/domain/src/orchestration/review_attempt.rs` | derived-default:, literal:restored |
| PlanInvalidations | `modules/core/command/domain/src/orchestration/plan_invalidations.rs` | derived-default:, literal:new |
| StageSlots | `modules/core/command/domain/src/orchestration/stage_slots.rs` | literal:new, literal:genesis |
| PlanChallenges | `modules/core/command/domain/src/orchestration/plan_challenges.rs` | derived-default:, literal:new |
| PlanAnswers | `modules/core/command/domain/src/orchestration/plan_answers.rs` | derived-default:, literal:new |
| StageSlugSet | `modules/core/command/domain/src/orchestration/stage_slug_set.rs` | derived-default:, literal:empty, literal:new, literal:filter, literal:combine, literal:divide |
| PendingDecisions | `modules/core/command/domain/src/orchestration/pending_decisions.rs` | derived-default:, literal:new |
| ReviewClosures | `modules/core/command/domain/src/orchestration/review_closures.rs` | derived-default:, literal:empty, literal:new, literal:filter |
| PlanPendingResponses | `modules/core/command/domain/src/orchestration/plan_pending_responses.rs` | derived-default:, literal:new |
| StageSlot | `modules/core/command/domain/src/orchestration/stage_slot.rs` | literal:genesis, literal:new |
| PlanTarget | `modules/core/command/domain/src/orchestration/plan_target.rs` | literal:stage_level, literal:for_unit |
| ArtifactPaths | `modules/core/command/domain/src/orchestration/artifact_paths.rs` | derived-default:, literal:empty, literal:new |
| IntentEventId | `modules/core/command/domain/src/orchestration/intent_event_id.rs` | tuple-literal:parse, tuple-literal:generate |
| StageIndexSet | `modules/core/command/domain/src/orchestration/stage_index_set.rs` | derived-default:, literal:empty, literal:singleton, literal:new, literal:filter, literal:combine, literal:divide |
| Classification | `modules/core/command/domain/src/orchestration/testing_posture/classification.rs` | literal:fallback, literal:parse |
| PlanQuestions | `modules/core/command/domain/src/orchestration/plan_questions.rs` | derived-default:, literal:parse |
| PlanAppliedOperations | `modules/core/command/domain/src/orchestration/plan_applied_operations.rs` | derived-default:, literal:new |
| CodeGenerationRunFloor | `modules/core/command/domain/src/orchestration/code_generation_run_floor.rs` | derived-default:, literal:new |
| ObjectMembers | `modules/core/infrastructure/src/canon_json/value/object_members.rs` | derived-default:, literal:new |
| PartIndex | `modules/core/query/use-case/src/orchestration/part_index.rs` | tuple-literal:, tuple-literal:next, tuple-literal:from_raw |
| TokenVersion | `modules/core/query/use-case/src/orchestration/token_version.rs` | tuple-literal:, tuple-literal:from_raw |
| Definition | `tools/lint/src/domain_getter/index.rs` | derived-default:, literal:declare |
| Index | `tools/lint/src/domain_getter/index.rs` | derived-default:, literal:build |

## 任意状態代入の是正と残り

- `PlanGeneration::set_state` / `PlanGenerations::set_state` は除去。操作IDの一致とPending状態を確認し、GenerationCertified/GenerationRevokedの保存事実を適用する。任意のenum値を受け取らない。
- `ReadModel::set_active_directive` は除去。DirectiveIssuedから描画するイベント適用へ変更。
- `RecordPlanAnswerUseCase` の関連取得はRepositoryのfind_for_approval_originへ、応答の照合はIntentExecutionのverify_plan_answerへ移動。ドメインのsource hashをUseCaseが取り出さない。
- `RecordPlanDecisionUseCase` のoptions/values取得はPlanChallenge::from_promptへ移動。監査先行の順序は維持する。
- `StageSlot` / `StageSlots` のoverride_plan系は除去。Recomposedで名指されたslugだけへ反転を適用。
- gate/Reportedのcheckbox変更と完了後の次ステージ開始をイベント適用へ移動。質問のclearを保持する。
- jumpのmark/mark_allを除去し、記録済みJumpedと既存カーソル/計画から適用する。Startedの初期進捗も完全構築で確立する。
- ReadModelのreplace_state/replace_memory、MemoryFacesのreplaceも除去。RMUの描画責務は維持し、完全コンストラクタを通る完成値のファクトリへ変更した。memory不在時のno-op・dirty・監査追記は回帰で確認した。
- **是正完了**: 2026-09-08、下記の検査が成功。FCC要素の全体移行は利用者承認の後続 [Issue #123](https://github.com/amadeus-dlc/amadeus-ng/issues/123) であり、この是正へ含めない。U2の残作業は継続する。

## 是正途中の検証履歴

以下は途中時点の検出と検査結果である。是正完了時の結果は後段に記録する。

- setter-methodルール: 真のRed 4件失敗後にGreen。tools/lintは98テスト成功。親の独立CLI7ケースも成功。
- getter4件と3setter是正後のcargo lintは成功。構築是正途中のlintも成功。
- 完全コンストラクタ往復2件、planに一致する30件、adapter全100件成功。
- 再構成の新しい名前付きステージ境界はRed→Green、既存再構成を含め9件成功。
- 広いdomain検査は703成功・監査語彙の旧期待3件失敗（構築是正前は701成功・同3件と往復2件失敗）。全体Greenではない。2.7.1レジストリは91語、nativeは87語で、旧86件期待への帳尻合わせはしない。差の4語はStep8で移行する。
- 構築是正途中のclippyでconst不足7箇所とRepositoryのstrategy材料漏れ2箇所を検出し、是正後のclippyは成功。後続イベント適用変更後の最終検査は未実施。

更新: 固定本家91語の受入へ移行後、domain全711件成功（`/tmp/amadeus-u2-domain-corrections-green.log`）。旧監査期待の3失敗は解消した。

## 是正完了時の検証

- domain全711件、adapter全100件、upstream_271_contract全53件、plan_runtime_contract全4件、RMU全307件、engine_loop_conformanceの全保存ITFトレース、lintツール全98件成功。
- cargo lint、cargo clippy -p aidlc --all-targets -- -D warnings、独立lintツールのclippy --all-targets -- -D warnings、workspaceとlintツールのfmt --check、git diff --check成功。
- 構築AST再走査で、対象ソースの実structに複数リテラル/Defaultとの二経路が残らないことを確認。走査結果は `test-evidence/constructor-scan-results.json`。マクロ展開や派生Clone/Deserializeまで検査したという主張ではない。
- 生ログは `test-evidence/` のsetter/constructor/corrections/projection-factories系列に保存。`started-construction-green.log` は最初のassert誤記による失敗で、正式Greenは `started-construction-green-corrected.log`。
- U2全体のCI・カバレッジ・全フック完成とは区別する。是正完了後はwrite-audit-log実装へ復帰する。

追加確認: RMUのファイル結合は空見出しfixture2件を本家の実体判定に基づき修正し、全31件成功。BOM/NELを含む固定本家10観測もGreen。詳細はtdd-evidence.mdの追記を参照する。

HookHealthの初回集約・イベント・SQLite Repository・専用RMU・CLIheartbeat接続を追加。保存の成功範囲はStarted/HeartbeatObserved/AuditDroppedの3事実とRMU read_hook_health投影、CLIの2回発火であり、artifact監査本文とdrop履歴投影は未完了。PlanApprovalのown streamフィルタは実際の交差行でRed→Greenを検証した。

ArtifactAudit専用RMU段階のGreen（同一spaceDBのArtifact event→監査/Query）を保存。通常ReadModelUpdater PublicationBatch統合は未完了。

**2026-09-09再開時の訂正**: 直前の専用RMU段階という記述は古い中断点である。2026-09-08T18:36:59Zの監査記録と現物を確認し、ArtifactSavedの通常JournalReader/ReadModelUpdater/PublicationBatchへの統合、専用ArtifactAuditReadModelUpdaterの除去を確認した。現在地はnext入力の観測を進めた段階。今回のworkspace名詞21入力のRed/Greenと残作業は `tdd-evidence.md` 最終節に記録する。U2全体の完了とは扱わない。
