# reverse-engineering-timestamp — 実施記録

## 実施記録

- **実施日**: 2026-08-22
- **コミット**: `c4d8d95`（full: `c4d8d950e6b29562dbd1d897b1d3e35b815ae845`、2026-08-22 12:21:48 +0900）
- **ブランチ**: スキャン時（第 1 リンク）は `docs`。統合時（第 2 リンク）のチェックアウトは同一コミットの `main-sync`（実地確認）
- **intent**: `260822-stage1-selfhost`（GitHub Issue #7 — stage-1 セルフホスト切替への最短経路）
- **パイプライン**: 第 1 リンク = developer-agent のコードスキャン、第 2 リンク（本成果物群）= architect-agent の統合。既存 codekb ストアは無し（NO_STORE — 初回作成）
- **実行検証（スキャン時実測）**: `cargo fmt --all --check` PASS / `cargo lint` PASS（所見 0）/ `cargo test --workspace` PASS（234 テスト全緑）

## 深度の注記（YAML ブロックの但し書き）

下記 `analyzed.paths` はディレクトリ粒度で記載しているが、次のファイルは部分読みである（公開契約は把握済み、本体後半は未読）: `workspace/lock_protocol.rs`（冒頭 140 行）、`fs_workspace_lock.rs`（1〜440 行）、`workflow_definition_repository_impl.rs`（1〜310 行）、`state_file_io.rs`（冒頭 60 行）、`tools/lint/src/check.rs`（冒頭 100 行）、infra-io の 4 モジュール本体（契約 doc 各冒頭 40 行のみ）、core-domain の Domain Primitive 17 ファイル（モジュール doc ヘッダ精読 + 公開シグネチャ全数抽出のみ）。`target/` は対象外。

## Scope of Analysis

```yaml
scope_version: 1
kind: partial
intent: 260822-stage1-selfhost
fingerprint: unknown
analyzed:
  paths:
    - .cargo/config.toml
    - .github/workflows/ci.yml
    - Cargo.toml
    - Cargo.lock
    - clippy.toml
    - rustfmt.toml
    - modules/core/domain/src/
    - modules/core/use-case/
    - modules/core/interface-adapter/src/
    - modules/shared/
    - modules/infra-io/src/
    - modules/app/aidlc/
    - modules/harness/claude/
    - tools/lint/
    - scripts/
    - docs/specs/00-policy.md
    - docs/specs/deviations.md
    - aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/
    - aidlc/spaces/default/knowledge/aidlc-shared/design-audit-2026-08-22.md
  components:
    - core-domain
    - core-use-case
    - core-interface-adapter
    - audit-events
    - directive-schema
    - message-catalog
    - canon-json
    - infra-io
    - aidlc
    - harness-claude
    - amadeus-lint
    - scripts
shallow:
  paths:
    - docs/specs/01-domain-model.md
    - docs/specs/10-orchestration.md
    - docs/specs/11-workspace.md
    - docs/specs/12-workflow-definition.md
    - docs/specs/research/
    - docs/upstream/specs/
    - docs/adr/
    - formal/
    - modules/core/domain/tests/
    - modules/core/interface-adapter/tests/
    - tests/
    - .claude/
    - aidlc/spaces/default/memory/
    - aidlc/spaces/default/intents/
    - aidlc/spaces/default/knowledge/
```
