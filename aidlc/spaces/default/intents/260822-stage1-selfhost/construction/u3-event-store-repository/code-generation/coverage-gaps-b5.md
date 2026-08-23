# coverage-gaps-b5 — 未カバー行（`cargo llvm-cov --workspace --lcov`、PROPTEST_RNG_SEED=20260823、コミット 328efc9 時点）

> 相対ゲート: head 96.81% < base 97.39% − 0.01（`scripts/coverage.sh --base origin/main`）。目標: ワークスペース総行数 ≈ 10,539 行のうち **+70 行以上**を新規テストでカバー。

- `modules/core/interface-adapter/src/orchestration/workflow_definition_repository_impl.rs` — 62 行: 107-108, 110, 119-124, 129-131, 488-491, 498-499, 563, 596-597, 599-602, 613-618, 620, 628-630, 632, 634-636, 638, 645-647, 649, 661-666, 668, 674-676, 678, 685-687, 689, 749, 785, 788
- `modules/core/interface-adapter/src/orchestration/event_store_impl.rs` — 25 行: 113, 223-227, 355, 388, 423-426, 428, 492-495, 516, 550-554, 572-573
- `modules/core/interface-adapter/src/orchestration/memory/workflow_execution_repository.rs` — 24 行: 32-34, 36, 39, 44-46, 58-60, 101-105, 114-121
- `modules/core/interface-adapter/src/orchestration/memory/in_memory_event_store.rs` — 19 行: 76-82, 119-122, 124, 162-165, 196, 209, 212
- `modules/core/interface-adapter/src/orchestration/wire/mod.rs` — 11 行: 93, 126, 143, 171, 188, 196, 204, 214, 263, 320, 357
- `modules/core/interface-adapter/src/orchestration/wire/state_wire.rs` — 7 行: 96, 101, 154, 158, 162, 257, 273
- `modules/core/interface-adapter/src/orchestration/workflow_execution_repository_impl.rs` — 4 行: 45, 61-63
- `modules/core/interface-adapter/src/orchestration/wire/event_wire.rs` — 3 行: 286, 425, 510
- `modules/core/use-case/src/orchestration/event_store.rs` — 3 行: 95-97
- `modules/core/use-case/src/orchestration/projection_name.rs` — 3 行: 78-80
