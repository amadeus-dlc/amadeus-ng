# code-generation-questions — U2 ドメイン ES コア（`u2-domain-es-core`）

> Code Generation（Construction 3.5）の質問票（Unit: U2、Bolt: B3、規模 L）。出典: `code-generation-plan.md`、`unit-test-instructions.md`、
> `../functional-design/*.md`、`../nfr-requirements/*.md`、`../nfr-design/*.md`、`../../../inception/domain-design/decisions.md`（ADR-008）、
> `../../../inception/delivery-planning/bolt-plan.md`（B3）。
>
> **質問なし。** ブランチ / PR / 記録コミットの運用は B1・B2 で確定済み（`origin/main` から Bolt ブランチ、記録コミット → コードコミット、
> PR は 1 本直列、squash-merge）。前提 P1 を確認のうえ、計画承認（Plan Approval）を求める。

## 前提（確認事項）

- P1. `WorkflowDefinitionId` の値 = `<harnessRoot>/tools/data/harness.json` の `name`（`claude`。upstream ピン `3c3146cf` の dist にも同一
  ファイルがあることを HTTP 200 で実測）。framework 名の接頭辞（`aidlc:claude`）は付けない — 識別子の出所をデータ（harness.json）1 つに
  保つ。`DefinitionRevision` = 3 入力の正準 JSON（`{ "stage_graph", "scope_grid", "scopes" }`）の `sha256:`（canon-json、アダプタ層で計算）。
- P2. 委任は 2 回直列（委任 1 = workflow_definition / Repository 側、委任 2 = orchestration 側）。開発エージェントは計画ファイルを書き換えず、
  進捗は `developer-report-<n>.md` に書く（B1 で計画チェックボックス編集により承認ガードに止められた教訓）。
- P3. 後方互換の旧 API（`report_forward` / `gate_start` / `reject` / `revise` / `report_skipped` / `recompose_flip` / `next` / `find()` 等）は
  残さず削除する（オーナー裁定 2026-08-23）。

## Plan Approval

`code-generation-plan.md`（埋め込みの Testing Contract を含む）と `unit-test-instructions.md` を確認し、実装に進んでよいか。

[Approval Fingerprint]: sha256:d2b66e0b979801ba3a4ad0849901ed290035871a4c4a3d749e6a223324a1bbd9

- Approve Plan — 計画どおり実装に進む
- Request Changes — 計画を修正する

[Answer]: Approve Plan
