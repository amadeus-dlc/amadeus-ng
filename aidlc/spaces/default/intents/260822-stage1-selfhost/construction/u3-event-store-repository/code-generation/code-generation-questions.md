# code-generation-questions — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> Code Generation（Construction 3.5）の質問票（Unit: U3、Bolt: B5、規模 L）。出典: `code-generation-plan.md`、`unit-test-instructions.md`、`../functional-design/*.md`
> （+ pending-revision）、`../nfr-requirements/*.md`（+ pending-revision 1〜3）、`../nfr-design/*.md`、`../../../inception/delivery-planning/bolt-plan.md`（B5）。
>
> 質問は 1 点（NFR 要求レビュー所見 2 — lint の機械強制をオーナー裁定で確定）。そのうえで前提 P1〜P4 を確認し、計画承認（Plan Approval）を求める。

## 質問

### Q1. `clippy::indexing_slicing` / `clippy::panic` の workspace lint 昇格（NFR4.3 の機械強制）

現状 `Cargo.toml` の `[workspace.lints.clippy]` は `unwrap_used` / `expect_used` を deny しているが、添字アクセス（`v[i]` — 範囲外で panic）と `panic!` は
人力レビュー任せ。本 Unit は「信頼しない入力からの復号」と「`seq_nr − 1` の減算」を扱うため、機械強制があると安全側:

- A. **昇格する（推奨）** — `indexing_slicing = "deny"` / `panic = "deny"` を `[workspace.lints.clippy]` に追加し、既存コードの該当箇所を B5 で是正する
  （テストコードは `clippy.toml` で許容を検討 — `allow-indexing-slicing-in-tests` は無いため、テスト内は `#[allow]` で個別に）。既存コードの是正量は B5 着手時に
  実測して報告（想定: 集約の `Vec` 索引は `StageIndex` 経由で既に型保証、adapter の parse 周りに数か所）。
- B. **見送る** — NFR4.3 の合格基準を「deny（unwrap / expect）+ レビュー（索引 / panic!）」に改め、B5 ではレビューで担保する。昇格は後続 intent。
- X. Other (please specify)

[Answer]: A

## 前提（確認事項）

- P1. 委任は 5 本（Opus 4 + Sonnet 1）: 委任 1 = 退役 + U2 是正（先行、2 コミット）、委任 2 = ポート / エラー / 値 + InMemory + ワイヤ + 契約テスト（委任 1 の後）、
  委任 3 = SQLite ストア + Repository 実装 + 依存追加（委任 2 の後）、委任 4 = Quint モデル + ITF + quint-gate（委任 2 の後、委任 3 と並行）、委任 5 = 仕様・正本の
  同期（委任 1 の後、委任 3 / 4 と並行）。所有ファイルは重ねない。開発エージェントは計画・検査手順・質問票を書き換えず `developer-report-<n>.md` に報告、`git commit`
  はコンダクタ。
- P2. メメントの改名はメソッドも含む（`state()` / `from_state()` / `WorkflowExecutionState` / `WorkflowExecutionStateBuilder` / `StateError` — B4 統合時の裁定、
  U2 pending-revision 9 の追記）。旧名の再エクスポート・エイリアスは残さない。
- P3. `rusqlite`（bundled）/ `tokio`（rt, macros）は B5 着手時点の最新安定版を固定版で追加（code-summary に記録）。`md5` は除去。`cargo audit` は advisory ジョブ
  （必須チェック外）だが緑を確認する。
- P4. Quint 新モデルの DoD（不変条件ごとの変異・状態遷移レベル・in-module witness・ITF fixture ≥ 6）を満たし、mutation 表を code-summary に残す。U3 FD の
  `entities.md` 末尾の `## Review` は履歴として NOT-READY のまま（本文は是正済み、レビュー予算 1）— ステージゲートで扱う。

## Plan Approval

`code-generation-plan.md`（埋め込みの Testing Contract を含む）と `unit-test-instructions.md` を確認し、実装に進んでよいか。

**再承認（2026-08-24）** — 初回承認（指紋 `sha256:38d7646c…`）以降、オーナー裁定「内部可変性は既定で禁止、`&self` への偽装は禁止」に従い
計画本文の 5 箇所を実態へ同期した（`RefCell<SqliteEventStore>` → `EventStoreImpl<C>` の直接所有、
`RefCell<InMemoryEventStore>` → 直接所有、旧名 `SqliteEventStore` → `EventStoreImpl`）。
実装ステップ・トレーサビリティ・Testing Contract は不変。指紋が動いたため再承認を求める。

[Approval Fingerprint]: sha256:04a8a9e1bfa842839caf26a1c97b0ddfcbbbc939695a772041ef2316bcf39a07

- Approve Plan — 計画どおり実装に進む
- Request Changes — 計画を修正する

[Answer]: Approve Plan
