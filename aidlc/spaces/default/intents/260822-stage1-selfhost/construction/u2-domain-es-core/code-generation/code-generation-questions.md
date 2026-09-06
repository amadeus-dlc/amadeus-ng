# code-generation-questions — U2 ドメイン ES コア（FCC 化と `next_decision` の ID 照合、Bolt b51）

> Code Generation（Construction 3.5）の質問票（Unit: U2、kind: library、規模 L）。**2026-09-07 再走（Modify）** — 旧版（2026-08-23、
> Bolt B3、Approve Plan 済み）は `code-generation-questions-history-2026-08-23.md` に保存した。出典: `code-generation-plan.md`、
> `unit-test-instructions.md`、`../functional-design/*.md`（§9 引継ぎ、レビュー R-01〜R-10）、`../nfr-requirements/security-requirements.md`
> （レビュー R-01〜R-08）、`../nfr-design/{security-design,logical-components}.md`（レビュー R-01〜R-07）。
>
> 用語: **FCC** = ファーストクラスコレクション（配列を不変条件と操作を持つ専用型で包んだもの）。**DTO** = アダプタ層の保存・復元用の写し。
> **RMU** = read-model-updater（イベントからリードモデルを投影するクレート）。

## 前提（確認事項）

- P1. **Artifact Re-use = Modify**（2026-09-07 裁定）: 旧計画・旧テスト指示・旧成果要約・旧トレーサビリティ・旧質問票は
  `*-history-2026-08-23.*` として保存し、本計画を新しく書いた。旧 Bolt B3 の実装（`WorkflowExecution` の ES 化）は完了済みで、
  本 Bolt は functional-spec §9 #1〜#4 の差分（FCC 化・`next_decision` の Result 化・doc 是正・兄弟クレートの追随）だけを実装する。
- P2. **委任は 2 回直列**（委任 1 = FCC 11 型の新設（追加のみ）、委任 2 = 集約・境界の一斉切替 + 受入）。開発エージェントは計画
  ファイルを書き換えず、進捗と各 Red の失敗出力を `developer-report-3.md` / `developer-report-4.md` に書く（B1 で計画チェックボックス
  編集により承認ガードに止められた教訓、旧 P2 の継承）。チェックボックスはコンダクタが検証後に付ける。
- P3. **ブランチと PR**: 本ワークツリーのブランチ `stage1-selfhost`（`origin/main` `e8ca4a5f` から intent 記録 4 コミット先行、未 push）で
  作業し、Bolt 完了後に親セッションが push して PR 1 本（直列、タイトル = Bolt slug `b51: …`、squash-merge）を開く。開発エージェントは
  push / PR を行わない。
- P4. **後方互換の旧 API は残さない**（no-backward-compatibility）: `stage_keys()`、`&[StageEntry]` / `&[String]` / `&[StageSlug]` /
  `&[PromotedSection]` / `&[ReviewClosure]` を返す旧アクセサ、`StageEntry::check_plan` / `Intent::check_plan` は削除し、エイリアスや
  `#[deprecated]` を置かない。
- P5. **DTO の列表現（正準 JSON のバイト）は変えない**: `IntentExecutionDto` の 7 列はそのまま、`StageSlots` との相互変換は
  `fold_left`（展開）と `StageSlots::new`（畳み込み）で行う。ゴールデン・往復テストが緑であることを DTO 不変の証跡にする。
  `Recomposed` の投影順序（文書順）は RMU 側で `plan` の位置により並べ直して維持する。

## 裁定済みの質問（2026-09-07、計画作成前に確認）

- Q1. FCC の `combine`（和集合・連結）/ `divide`（差集合）を今回どの型に実装するか。
  [Answer]: A. 集合型 2 つだけに実装（`StageIndexSet` / `StageSlugSet`、Monoid 則・差集合則を性質試験）。列型は用途が出た時点で追加。
  共通 trait への一律化（オーナーの最終方針、Q4a）は引き続き積み残し。
- Q2. `ReviewAttempt` の内部列 2 つ（`pending` / `closed`）をどう FCC 化するか。
  [Answer]: A. 両方を FCC 化（`closed` は公開型 `ReviewClosures`、`pending` はクレート内型 `PendingIterations`）。

## Plan Approval

`code-generation-plan.md`（埋め込みの Testing Contract `sha256:303d9bb7…` を含む）と `unit-test-instructions.md` を確認し、実装に
進んでよいか。計画の要点: FCC 11 型の新設（§2 の不変条件・操作・エラー型を本計画で確定し functional-design ゲートへ折り戻す）、
`IntentExecution` の 7 並列列 → `StageSlots` 統合、`next_decision` の Result 化（`IntentMismatch`）、冒頭 doc の是正、兄弟クレート
4 つ（interface-adapter / RMU / use-case / app）と各テストの追随、契約試験・性質試験・ITF の追随、受入（CI 4 ステップ + Quint +
audit、カバレッジ 2 回同値と 98.66% 床、BR4.1 判定式と裏取り、`# Panics` 3 か所、`Cargo.lock` 不変）。

[Approval Fingerprint]: sha256:dd1170c1a75b16e30a351f34d9f4ff57164bcbe65482361e94e6909de7f0634d

- Approve Plan — 計画どおり実装に進む
- Request Changes — 計画を修正する

[Answer]: Approve Plan
