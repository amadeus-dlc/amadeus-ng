# code-generation-questions — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

> Code Generation（Construction 3.5）の質問票（Unit: U1、kind: library、Bolt: B1）。出典: `../functional-design/`
> （functional-spec / rules / entities）、`../nfr-requirements/tech-stack-decisions.md`、`../nfr-design/`
> （security-design / logical-components）、`../../../inception/contract-design/contract-summary.md`（C7）、
> `../../../inception/delivery-planning/bolt-plan.md`（B1 = U1、base/target = `main`、squash-merge）、
> `aidlc/spaces/default/memory/team.md`（Bolt = PR、PR 直列運用）、実地確認（`git worktree list`・`tests/` 配下・
> upstream ピン `3c3146cf` の raw 取得可否）。
>
> 計画（`code-generation-plan.md`）の形を左右する 2 点だけを先に問う。それ以外（モジュール分割・依存・TDD 順序・
> clippy 設定・棚卸し項目）は上流成果物から一意に決まるため質問しない。

## 以前の質問（2026-08-22の記録）

Q1のブランチ名と初回実装の手順は履歴として保持する。現在は既存の実装を確認する再作業であり、旧ブランチ作成や初回採取を再実行しない。今回の対象は末尾のPlan Approvalに示す。

### Q1. Bolt B1 のブランチと aidlc 記録のコミット方法

現在の作業ブランチは `main-sync`（`origin/main` と同一コミット `c4d8d95`）で、AI-DLC の記録（`aidlc/spaces/default/`
配下の intents / codekb / memory / coding-rules 追補）が**未コミット**のまま作業ツリーにあります。Bolt のブランチを
`main` から切ると、この記録がブランチに含まれず、フレームワークの状態ファイルも見えなくなります。

- A. `main-sync` 上で Bolt ブランチ `bolt/b1-u1-canon-json-goldens` を切り、**最初のコミットとして aidlc 記録を含める**
  （B1 の PR に記録が同乗する。PR は 1 本のまま、直列運用を維持）— 推奨
- B. 先に aidlc 記録だけの PR（例: `chore(aidlc): inception と U1 設計の記録`）を `main` へマージし、その後 `main` から
  Bolt ブランチを切る（PR が 1 本増えるが、B1 の PR はコードだけになる）
- C. ブランチを切らず `main-sync` でそのまま作業し、B1 完了時に `main-sync` を PR にする
- X. Other (please specify)

[Answer]: A

### Q2. ゴールデン（正解データ）の配置

契約 C7 は `tests/goldens/{hash-canonical,cli,hooks}/` を定めていますが、リポジトリには既に upstream 配布実バイトの
`tests/golden/upstream-3c3146cf/`（README が「ピン留めコミットごとのディレクトリ、バイト不変」と規定）があります。
`golden/` と `goldens/` の兄弟並立は構造が読みにくいため、裁定を仰ぎます。

- A. `tests/golden/upstream-3c3146cf/{hash-canonical,cli,hooks}/` に統合する（ピン単位の 1 ルート。C7 の layout 行を
  同時に改訂し、既存の dist 実バイト `stage-graph.json` / `scope-grid.json` は同ディレクトリ直下のまま不変）— 推奨
- B. C7 どおり `tests/goldens/{hash-canonical,cli,hooks}/` を新設する（既存 `tests/golden/` はそのまま）
- X. Other (please specify)

[Answer]: A

## Plan Approval

2026-09-06の対象: `code-generation-plan.md`のStep 1〜6と、そのTesting Contract、および`unit-test-instructions.md`。

現行のcore-infrastructure::canon_jsonを再利用し、mod.rs・parse.rsの説明コメントを更新する。既存の単体・PBT・ゴールデン・rustdoc試験を現行パスで確認し、code-summary・traceability・source-manifestを現行実装に合わせる。振る舞い・公開API・固定データ・依存・品質基準は変更しない。機能欠陥が判明した場合は、再現試験を先に置く変更案を返して計画を改訂する。

計画準備時のUnit限定コマンドは87件＋16件＋rustdoc 1件の計104件が成功した。これは全体CI・性能・最新依存検査の成功を意味しない。

[Approval Fingerprint]: sha256:dc02047c5e496d6aed8c870f4daa314b48d9dcd33d50544e5cf0fa90144de28b

- Approve Plan — この計画で実コード生成に進む
- Request Changes — 計画・テスト手順を修正する

[Answer]: Approve Plan
