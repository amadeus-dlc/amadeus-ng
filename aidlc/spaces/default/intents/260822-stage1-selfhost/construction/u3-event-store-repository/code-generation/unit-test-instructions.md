# unit-test-instructions — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> Code Generation（Construction 3.5）のテスト手順（Unit: U3、Bolt: B5）。出典: `code-generation-plan.md` §5、`../nfr-design/logical-components.md` §4、`../functional-design/rules.md`
> BR5.2。すべてリポジトリルートで実行し、本 Unit に関係するクレート / テストに限定する。

## 1. 単体・契約・ITF（Unit スコープ）

```bash
# ドメイン（IntentId UUIDv7 / IntentDirName / WorkflowExecutionState 改名の追随、engine_loop ITF）
cargo test -p core-domain
# ユースケース層（ポート・値・エラー）
cargo test -p core-use-case
# アダプタ（契約テスト両実装・ワイヤ PBT・SQLite 実装・クラッシュ再構成・journal_protocol ITF・既存 WorkflowDefinitionRepository / ゴールデン）
PROPTEST_RNG_SEED=0 cargo test -p core-interface-adapter
# 個別（失敗時の絞り込み）
cargo test -p core-interface-adapter --test workflow_execution_repository_contract
cargo test -p core-interface-adapter --test journal_protocol_conformance
cargo test -p core-interface-adapter --test crash_reconstruction_test
# tools/lint（ルール削除後の自己テスト）
cargo test --manifest-path tools/lint/Cargo.toml
```

## 2. ゲート（受入 BR5.2 — PR 前にローカルで）

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings        # indexing_slicing / panic を含む全ルール deny
cargo lint
cargo test --workspace
bash scripts/quint-gate.sh                                    # journal_protocol: typecheck / invariants / witness
bash scripts/coverage.sh                                      # 絶対 90% 床 + 相対ゲート（TOLERANCE 0.01）
cargo audit                                                   # advisory ジョブ相当（結果を code-summary に）
# 退役の証明
grep -rnE 'WorkspaceLock|FsWorkspaceLock|LockProtocol|LockIdentity|reap_eligible|OwnerStamp|AcquireBudget|LockGuard|LockError|process_alive|ProcessProbe|audit_lock|reap-decision-locality' modules tools scripts formal .github Cargo.toml ; echo "(expect no output)"
grep -rn 'aidlc-lock' modules tools scripts ; echo "(expect no output)"
grep -rn 'Snapshot' modules/core/domain/src/orchestration ; echo "(expect no output)"
```

## 3. 期待カバレッジ・モック・テストデータ

- カバレッジ: ワークスペース 90% 床維持（base は Step 0 で採取）。adapter に除外を足さない。
- テストダブル: `InMemoryEventStore` / `InMemoryWorkflowExecutionRepository`（本番コードの一部 — `memory/`）、`FakeClock`（既存）。SQLite は `tempfile` の一時 dir。
- テストデータ: IntentId は UUIDv7 リテラル（例 `01a02785-1bd8-76eb-aeea-5aa303ebd5b6`）、WorkflowDefinition は既存のゴールデン（`tests/golden/upstream-3c3146cf/`）と
  合成 StageEntry 列（engine_loop ITF と同じ）。ITF fixture は `tests/conformance/fixtures/journal_protocol/*.itf.json`（`#meta` 正規化済み、≥ 6）。
