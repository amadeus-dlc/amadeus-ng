# 業務概要 — amadeus-ng

## このシステムは何か

amadeus-ng は、AI-DLC（AI-Driven Development Life Cycle）のワークフローエンジンを Rust で作り直したものである。元の実装は upstream の TypeScript 配布物（`.claude/tools/*.ts`、Bun で動く）で、本リポジトリはその**観測できる振る舞いを互換に保ったまま**置き換えることを目的にしている。互換の正本は次の 2 つで、設計規則よりも優先される（`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md` の衝突優先順 1）。

- ゴールデン `tests/golden/upstream-a277af21/`（ピン `a277af21` = v2.7.1 の配布実バイト）
- 配布元 submodule `vendor/aidlc-workflows/`

利用者は Claude Code などのハーネス上の AI エージェントと、それを見守る人間である。エージェントは `aidlc` のマルチコールバイナリ（`aidlc-orchestrate` / `aidlc-log` / `aidlc-utility` など）を呼び、ワークフローの進行・記録・承認ゲートを扱う。

## 業務ドメイン

| 領域 | 中身 |
| --- | --- |
| ワークフロー進行（orchestration） | intent（仕事の単位）の開始、ステージの進行・完了報告・ジャンプ、承認ゲート、レビュー、pipeline link の受領 |
| ワークフロー定義（workflow_definition） | ステージグラフ・スコープグリッド・スコープ定義の配布束。コンパイル済み定義の取込と改訂 |
| ワークスペース（workspace） | スペース・intent 記録の配置、状態ファイル `aidlc-state.md`、監査シャード、コード知識ベース |
| ハーネス結線 | Claude Code のフック（規則配送、書込監査、人手ターン記録など） |

ドメインの中心は集約 `IntentExecution`（ワークフロー実行）、`Intent`、`WorkflowDefinition`、`CompiledDefinition` である。コンポーネントごとの責務は `component-inventory.md` を参照。

## 主な機能

- **次の指示の算出**（`next` / `continue`）: ステージグラフと実行状態から、エージェントが次にすべきことを directive として返す。読取専用で、RMU が作った構造化リードモデルを読む。
- **更新動詞**（`report` / `log review` / `log decision` / `log answer` / `log link` / `jump` など）: 集約にコマンドを送り、ドメインイベントをイベントストアへ記録する。記録後に投影を走らせ、状態ファイル・監査シャード・`read_*` 表を最新にしてから成功を返す。
- **pipeline link の受領**（`aidlc-log link`）: pipeline モードのステージで、リンクが宣言順に完了したことを受け取り、監査に `PIPELINE_LINK_COMPLETED` を 1 件だけ残す。ステージの承認はこの受領を条件にする。
- **診断・補助**: `doctor`、ステータスライン、学びの儀式（`aidlc-learnings`）、テスト方針（testing posture）など。

## 今回の intent との関係

Issue #134 は、**同じ pipeline link の完了報告が並行して 2 本届いた**ときの振る舞いに関する不具合である。期待される結果は「記録に成功した 1 本は成功を返し、重複した 1 本は `already completed` で拒否される」こと。実際には、記録に成功した側も後段の投影で `WouldBlock`（SQLite のロック競合）に当たり、終了コード 1 で失敗として返る。

流れは `architecture.md` の Interaction Diagrams、原因の候補・修正の論点・業務上の影響（merge queue の停止、stage-1 実地スモークとの関係）は `code-quality-assessment.md` に書いた。
