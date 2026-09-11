# セルフホスト切替の作業単位

## 分割方針

[要求書](../requirements-analysis/requirements.md)と[確認済みの分割方針](units-generation-questions.md)に基づき、4単位に分ける。既存CLI・SQLiteの永続化・投影構造を維持し、作業の責任範囲を分ける。サービスやデータベースの新設は行わない。[Q1]

`components.md`は承認済み計画で生成を省略している。要求書の変更対象表と実在するコード境界を使用し、架空のコンポーネント一覧を補わない。種類の`service`は既存CLIの実行機能、`packaging`は採取・ビルド・接続の資産を表す。

## 単位一覧

| Unit ID | Directory | 名称 | Kind | 配布形態 | 相対的な複雑さ |
| --- | --- | --- | --- | --- | --- |
| U1 | u1-upstream-acceptance | 本家2.7.1の受入基盤 | packaging | 同じリポジトリ内の採取・比較資産 | M |
| U2 | u2-workflow-authority | 作業進行・監査・承認 | service | 既存aidlcバイナリへ組込み | L |
| U3 | u3-selfhost-doctor | セルフホスト用の自己診断 | service | 同じaidlcバイナリへ組込み | M |
| U4 | u4-selfhost-integration | 配布接続・切替準備 | packaging | 同じリポジトリ内の接続・検証資産 | M |

複雑さは現時点の相対見積りであり、工数・日程の約束ではない。U2は状態・承認・監査の整合を同時に保つ必要があるため大きい。契約差分の実測結果を踏まえ、実行計画で統合可能な変更にまとめる。

## U1: 本家2.7.1の受入基盤

### 責任と成果

- 本家の固定コミット`a277af218f0df7f325d3b8be7b6d90fce2c5bd40`から、必要な配布データとCLI・フック等の実行結果を採取する。
- 採取手順、来歴、環境、非決定値の扱い、比較対象と既知の未検証部分を明示する。採取結果を実装に合わせて編集しない。
- 2.7.1の比較材料と、新旧差分の一覧をU2以降へ渡す。採取の再現性と比較処理そのものを検証する。

主な変更箇所は`scripts/goldens/`、`tests/golden/`と採取・比較処理のテスト。実装の都合で正規化を広げたり、比較項目を削ったりしない。

### 境界と完了条件

U1は受入基盤を所有し、Rustの作業進行・承認処理は所有しない。固定した2.7.1からの採取来歴と比較材料が再現可能で、差分を後続が検証できる状態をU1の完了とする。

**U1完了はFR1完了ではない。** 現行テストの参照先・検証内容と対応実装を2.7.1へ揃える責任はU2へ引き継ぐ。旧期待値の参照を残したまま、互換移行全体を完了扱いしない。CIを成功させるために必要な単位のまとめ方は実行計画で決める。

## U2: 作業進行・監査・承認

### 責任と成果

- 選定したClaude経路のCLI入力・必要な観測を接続し、開始・継続・報告が質問待ちと人間承認を含めて動くようにする。
- 主要4フック、通常の質問・回答、内容確認、引継ぎ、レビュー、実装計画承認の受領と保護を既存Rust経路へ実装する。
- `learnings persist`等を含め、スモークで正本を更新する操作をRustに統一する。追加登録されたフックも、実発火と副作用で対象を判定する。
- U1の比較材料を使用し、既存実装の差分是正と、現行テストの参照先・検証内容の2.7.1移行を行う。

主な変更箇所は`modules/app/aidlc/`、`modules/harness/claude/`、`modules/harness/infrastructure/`、既存の`modules/core/command/`・`query/`・`read-model-updater/`の該当経路とテスト。層を新しく切り直さず、変更する操作に沿って必要な箇所だけを扱う。

### 境界と完了条件

U2が状態・監査・承認受領の更新契約を所有する。自己診断の判定・表示はU3、配布設定からの接続と切替準備はU4へ分ける。

正常入力と拒否入力のCLI/フック境界検証が、本家2.7.1の対象契約と一致すること。旧2.6.40の期待値を現行受入の根拠とする参照・検証内容を移行し、残る観測差がある場合は人間の裁定なしに達成扱いしない。監査行だけで実行許可を作らず、実装計画の対象・内容・セッションとの対応を保つ。

Code Generation段階の検証は実地スモークの代替にはしない。FR2・FR3の統合確認はU4および後続の実地実行でも確認する。

## U3: セルフホスト用の自己診断

### 責任と成果

- `target/release/aidlc --doctor`の入口と、今回使う構成に必要な診断を実装する。
- 対象は、バイナリ入口・必要なClaudeフックの配線、bugfix/featureの配布資産、状態・監査・永続化の整合とする。
- 正常構成と、必要な入口・資産・フックの不足、選定した状態/監査の破損を判定するテストを揃える。

主な変更箇所は既存CLIの診断入口、既存の読取り・投影境界に沿った診断処理とテスト。U2の更新機能を複製せず、診断が正本を勝手に修復・再初期化しない。

### 境界と完了条件

U2が提供する実入口と状態/監査契約を診断の対象として使用する。フック接続を実環境へ配置する責任はU4にある。

本家2.7.1の診断との項目別対応と対象外理由は契約設計で固定する。対象とした正常/異常構成を実際に識別し、未実装項目を成功扱いしないことをU3の完了条件とする。本リポジトリでの最終診断成功は切替工程で再確認する。

## U4: 配布接続・切替準備

### 責任と成果

- 配布済みの手順からU2のRust処理と再利用する補助処理を呼ぶ接続を整え、独自変更を同期パッチで管理する。
- bugfix/featureの必要なステージ・役割・プロトコル・グラフの存在と参照先を検証する。
- 更新しないと分類した補助操作について、依存先を含む副作用と、Rustの投影結果を正しく読めることを確認する。
- releaseバイナリを使うスモークの実行準備、既存CIへの必要最小限の接続、ホスト版の識別と切替・復帰を支える資産を整える。

主な変更箇所は`scripts/aidlc-sync/patches/`、本リポジトリの接続・検証用スクリプト、必要な既存CI設定と関連テスト。一般向けインストーラや配布一般化は追加しない。

### 境界と完了条件

U2の正本更新処理とU3の診断を利用し、状態更新・実行許可の別実装を持たない。補助操作の副作用が見つかった場合、その処理はU2の責任としてRustへ接続し、同じ正本を二重更新しない。

必要な接続と統合検証、スモークの実行方法、ホスト/ターゲットの識別、復帰方法が揃うことをU4の実装完了とする。**U4の準備完了だけでFR7・FR8を達成扱いしない。** 後続の検証・切替工程で、本リポジトリの実フック・人の応答を使ったbugfix一周、自己診断、CI全ジョブ成功、検証済み版への切替を実施して証拠を残す。

## 共通制約と責任の重複防止

- 全単位にNFR1–NFR4を適用する。TDD、カバレッジ床90%と相対ゲート、Quint/ITF、既存CI全ジョブ成功を維持する。
- U2とU3は同じCLIへ組み込む。物理ファイルの共有を並列編集の根拠にせず、操作と役割により責任を区切って直列に統合する。具体的な変更ファイルは各実装計画で固定する。
- U1の採取基盤、U2の更新・互換移行、U3の診断、U4の接続・実行準備が責任境界である。共有箇所の契約・入出力・失敗時の扱いは次の契約設計で定める。
- この分割は作業の内容と依存関係を定める。Boltへのまとめ方、優先順、CI成功を保つ統合単位を決める資料は後続の実行計画とする。1件の [Pull Request](https://github.com/amadeus-dlc/amadeus-ng/pulls) と単位を機械的に1対1対応させない。
- 自律実行、センサー・プラグイン・他ハーネス、独立した先行実装、既存コードの再設計・再文書化は追加しない。コード解析資料は実地スモークで必要な分だけの限定例外。

## Sources

- [scope] Workflow-selected scope: selfhost-stage1、Minimal。
- [Q1] [分割確認](units-generation-questions.md): 4単位への分割と各種類・責任・依存を確認済み。
- [requirements] [承認済み要求書](../requirements-analysis/requirements.md): FR1–FR8、NFR1–NFR4、変更対象表。
- [practices] [開発規則](../practices-discovery/team-practices.md): 直列実行・既存構造の維持・品質基準。

## Assumptions & Open Questions

分割方針の未回答事項はない。U2の具体的な差分量、診断の項目別契約、再利用する補助操作の副作用、各単位をまとめる統合単位は、契約設計と実行計画で実測して確定する。見積りの都合で要求を削除しない。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-07T14:07:37Z
**Iteration:** 1
**Request Challenge:** review:989b954fc1fffe8c17fa8029a353301e

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | aidlc/spaces/default/intents/260907-selfhost-stage1/inception/units-generation/unit-of-work-story-map.md > 対応表、および同ディレクトリ traceability.json > coverage | 対応表にFR1–FR8の主担当とDirectoryは存在し、JSONの対象とも一致するが、指定センサーは「contains no story-to-unit mappings」と返し、8件すべてをinvalid_targets・gapsに分類する。担当の割当漏れではなく、現成果物と検証ツールの間で対応を機械確認できない問題である。原因は検査ソースを読まずに確定できていない。 | FR直接対応を保ったまま、対応表を指定センサーが認識できる形式に揃える。ツール側の制約なら制約と代替確認を明記し、FR1–FR8の割当が検証可能になったことを確認する。架空のUser Story IDを追加して回避しない。 | Resolved |

### 解消確認

R-01の表は初回指摘を保持している。今回、許可された検査ソースを確認し、上流要求の選択はFRに対応していた一方、対応表の抽出がUS固定だった不一致を確認した。`storyAssignments`へ渡す抽出規則を、Units Generationでは上流と同じシナリオ資料の有無で選び、他の呼出しは既定のUSを維持する修正になっている。

正規の同期パッチ` scripts/aidlc-sync/patches/traceability-fr-unit-mapping.patch `と3配布先の変更が対応している。要求の割当内容と単位定義本文は変わっておらず、架空のシナリオ追加や検査の成功固定はない。FRと下位要求の正常対応、割当漏れ・誤った単位の拒否、既存US対応を実CLIの回帰テストで確認した。新たな指摘はない。

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| bun .codex/tools/aidlc-sensor-traceability.ts --output-path aidlc/spaces/default/intents/260907-selfhost-stage1/inception/units-generation/traceability.json | PASS: pass=true、findings_count=0、gaps・invalid_targets等はすべて空 | 現物のFR1–FR8の対応を認識し、R-01の症状が解消した。 |
| bun test ./scripts/aidlc-sync.test.ts ./scripts/aidlc-harness.test.ts ./scripts/aidlc-unit-lifecycle.test.ts ./scripts/aidlc-plan-progress.test.ts ./scripts/aidlc-traceability.test.ts ./.codex/hooks/aidlc-codex-adapter.test.ts | PASS: 6ファイル・56件成功、0件失敗、225 assertions | 追加された3配布先の回帰9件を含む。誤対応の拒否と既存US経路の維持を確認した。CI定義もこの6ファイルを実行対象に含む。 |
| bun scripts/aidlc-sync.ts --check | PASS: コピー0、削除0、保持設定の確認0、同期済み | パッチ適用後の配布物と同期台帳が一致している。 |
| 3配布先のSHA-256比較、およびgit -C vendor/aidlc-workflows status --short | PASS: 検査実装3本は同一ハッシュ、配布元の変更なし | fork本体を変更せず、同期パッチで修正を管理している。 |

### Summary

R-01は解消済みで、未解決の指摘はない。4単位の境界・依存と、FR1の実装適合およびFR7・FR8の実地達成を別途検証する方針を維持したまま、後続の契約設計へ進める。
