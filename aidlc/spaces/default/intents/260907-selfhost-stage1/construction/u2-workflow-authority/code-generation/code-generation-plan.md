# U2: 作業進行・監査・承認の実装計画

## 対象と完了条件

FR1–FR4とNFR1–NFR4を、承認済み契約C1–C6、およびC7の診断実施記録に沿って実装する。U1が採取した本家2.7.1 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の観測を比較基準にする。既存のRust構成、SQLiteイベントストア、RMU（イベントから読取りモデルを構築する処理）を使う。

U1の独立レビュー第2回はREADY、R-01〜R-03はResolvedで、UNIT_COMPLETEDを記録済み。U2では実装と受入検査を2.7.1へ移行する。B1はU1とU2をまとめた [Pull Request](https://github.com/amadeus-dlc/amadeus-ng/pulls) とし、全CIとレビュー収束後にmainへsquash-mergeする。今回の実装計画承認は、未検証の統合やセルフホスト切替を承認済みとするものではない。

## 現コードで確かめた差分

- HEAD、ローカルmain、origin/mainの記録はいずれも `1dc727e00a26c27258d916cb3a5e0592c6c48c1c`。U1ではRustを変更しておらず、開始時のworkspace検査2,354件は成功している。
- `CommitVerdictUseCase::execute` は `Result<CommitOutcome, CommitError>` を返す。no-opは保存せず戻り値を返し、成功した遷移の表示材料も返す。これを正当化するコメントが残っている。
- `modules/app/aidlc/src/runtime.rs:313–329` はその戻り値を保持し、投影後に `committed_directive` へ渡す。報告識別子に対応する結果のクエリはない。
- `IntentExecutionEvent` は現在16変種で、Reportedはない。集約の `report_dispatch` と `apply_report` が判断と遷移の既存境界である。
- クエリ側にはDAOとViewの独立したクレートがあり、`read_next_answer` 等を読む。報告結果の読取りモデルをこの構成へ追加する。
- `cli/request.rs` のdecision/answer/linkはLogNotWired。parse_nextには必要な入口観測の不足がある。harness-claudeとharness-infrastructureはまだ実装本体を持たない。
- U1の142プロセス観測には、Brownfieldのbugfix 9段開始、受領・保護・補助操作、診断20シナリオがある。採取と実際のRust適合は区別する。

## 報告処理の責任と順序

CQS（更新と読取りの分離）の例外は設けない。

1. 呼出側で報告識別子を用意し、実行識別子・報告要求とともに更新ユースケースへ渡す。外部CLIへreport-idフラグを追加しない。
2. 集約が受理・拒否・遷移・no-opを判断し、報告事実を単一イベントとして生成する。集約全体や輸送用封筒のコピーを結果にしない。
3. Repositoryが既存SQLiteストアへ追記する。更新ユースケースの成功は `Result<(), E>`。表示材料を返す別経路を残さない。
4. RMUが報告事実と遷移を投影し、report_idを自然キーとする結果行とチェックポイントを整合して更新する。
5. クエリAPIがreport_idで結果Viewを取得し、出力側が2.7.1の文言へ描画する。クエリ側はドメインへ依存せず、現在状態から報告結果を判断し直さない。

適用段階・適用操作・no-op理由、表示に必要な報告時のscope等を事実として保持する。後続の報告で現在状態が変わっても、別の報告結果と取り違えない。成功するno-opも内部の報告事実を追記するが、本家にない公開監査イベントや遷移を作らない。拒否は型付きエラーで返す。既存の1回だけの楽観競合再試行では、report_idと最初の対象を保持する。

## テストする公開境界

| 境界 | 検証すること | 主なテスト |
| --- | --- | --- |
| 集約・更新ユースケース | 報告の単一イベント、拒否、no-op、競合時の対象固定、成功戻り値がunit | 既存commit_verdict_use_case内テスト、ドメインの報告テスト |
| Repository → RMU → Query | 保存した報告をIDで取得、取り違え防止、投影失敗/再開、結果欠落の明示 | 新規report_result_contract / report_result_projection_contract / report_result_dao_contract |
| CLI | 開始・next/continue/report、質問/回答/引継ぎ/レビュー、実装計画承認 | 新規stage1_authority_contract / upstream_271_contract、既存CLI境界テスト |
| Claudeフック | JSON封筒、正常・拒否・無関係入力、実際の人間応答との対応 | 新規harness-claudeのhook_contract |
| 補助更新と投影ファイル | session・保存・規則・診断記録の更新、状態/監査の逐語、非適用時の無変更 | 上記CLI/フック契約と既存projection/publication契約 |

Standardの各コンポーネント5–8件を基準にする。主要4フックはそれぞれ正常・不正入力・適用外・状態/セッション境界を検証する。受領・永続化等で契約が求める追加ケースも含め、件数を合わせるための同義テストは作らない。実SQLiteと公開API/プロセスを使い、実装内部の呼出し回数だけを検査しない。

## 実装手順

各項目内の振る舞いを1件ずつRed（失敗の実行確認）→Green（最小実装）→Refactor（成功を維持した整理）で進める。全テストを先に書いてから実装する方式は採らない。

報告イベントが保存DTOとRMUへの入力契約になるため、Step 2でその振る舞いを固定し、Step 3で保存・投影を接続する。これは既存のイベントソーシングの依存順によるもので、テスト先行の順序を変えない。

- [x] **Step 1 — 入力と実行基盤の確認**（FR1、NFR2–NFR4）
  - U1の採取証跡・ケース引用・旧参照一覧を読み、必要操作と既存入口を対応付ける。コードを変える前に既存のreportテストを実行し、単位限定のランナーを確認する。
  - 現在のRust検査基準とB1統合時のmain差分を確認する。新テストファイルを追加したときは、未検出・ビルド環境不足をRedに数えない。
  - データベース・Repository・ドメイン・APIの既存配置と依存方向を維持し、対象機能がない層を形式のために増やさない。
- [x] **Step 2 — 報告事実をTDDで実装**（FR1・FR2、NFR1・NFR2）
  - Red: 成功する報告と3種類のno-opについて、呼出側が指定した報告識別子に対応する単一の報告イベントが得られることを検証する。拒否では成功結果を作らない。
  - Green: 既存集約の報告判断・適用へ必要な差分を加え、Reportedの事実を生成する。報告による遷移と結果を同じ保存単位にする。
  - Red→Greenを繰り返し、楽観競合の有限再試行と対象固定を検証する。競合相手が前進しても次の段階へ誤って報告しない。
  - Refactor: CommitOutcomeの公開成功戻り値と例外の正当化コメントを削除する。別名や旧APIを並立させず、呼出側を合わせて変更する。
- [x] **Step 3 — 保存・投影・結果クエリをTDDで接続**（FR1・FR2、NFR1・NFR2）
  - Repositoryの書込DTOとRMUの読込DTOをそれぞれ追加し、ドメインへserdeやSQLを持ち込まない。未知/破損した事実を成功に丸めない。
  - Red: SQLiteへ追記した報告を投影してIDで取得する契約、未取得、異なるreport_id、再投影、途中失敗と復旧を検証する。
  - Green: 報告結果のread表、行モデル、Query側の独立View/DAO/取得ユースケースを追加する。結果とチェックポイントを同時に確定し、RMU以外が読取りモデルを構築しない。
  - 出力側を、更新成功→RMU→report_idのクエリ→逐語描画へ切り替える。投影/取得の失敗時に旧戻り値や「最新の結果」で代用しない。
  - Refactor: 既存publication/crash-recovery契約を保ち、報告結果のために別ストアや別の状態正本を作らない。
- [x] **Step 4 — 作業開始と進行のCLIを2.7.1へ合わせる**（FR1・FR2、NFR1・NFR2）
  - Red: U1の同じ入力・初期ファイル・環境を使い、開始・next/continue/reportの値と状態/監査の違いを検出する。
  - Green: 必要なCLI観測を接続する。ワークスペース走査は本リポジトリのBrownfield判定・bugfix 9段の開始に必要な実走査を行い、空の既定値で成功させない。
  - 継続トークンを不透明な入力として渡し、古い/不正/別対象のトークンを拒否する。narrationやconductor_persona等の対象出力を、既知欠落を許す検査のまま残さない。
  - Refactor: 必要な分岐と文言を既存の入力・出力境界へ置く。Kiro専用や自律実行の未接続分岐を一律に実装しない。
- [x] **Step 5 — 質問・回答・承認受領をTDDで実装**（FR2・FR3・FR4、NFR1・NFR2）
  - decision/answer/link/review、内容確認、実装計画承認の必要な入力・受理・拒否を接続する。
  - 実際の人間応答の記録と、CLIが意味上の回答を保存する操作を分離する。CLIや監査行の手書きだけで実行許可が発行されないようにする。
  - Red→Greenで回答前・別セッション・古い質問・対象/内容変更・受領再利用・レビュー確定後の変更を検証する。状態を使う判断は集約へ置き、Queryで再判定しない。
  - 番号回答は、利用者が是正を指示した入力保持と既存の意味への変換を保つ。承認対象・セッション・真正な応答の照合は弱めない。
  - Refactor: 受領を使う入口を同じRustの更新経路へ統一し、TypeScriptへの更新フォールバックを作らない。
- [x] **Step 6 — 主要4フックをTDDで実装**（FR2・FR3、NFR1・NFR2）
  - 停止制御、人間応答記録、状態遷移保護、ファイル保存監査について、それぞれ本家の正常・拒否・無視を先に失敗させてから実装する。
  - `harness-claude`がJSON入力/出力とClaude固有の接続を、`harness-infrastructure`が必要な汎用機構を扱う。状態・監査・許可の更新はコマンド→イベント→SQLite→RMUを通す。
  - バイナリに `aidlc hook <name>` の接続入口を設け、stdinの封筒とフック別のstdout/stderr/exitを保持する。配布設定へ実際に結び付ける工程はU4。
  - 正当な質問・承認待ち、停止フックの再入、進捗なし反復、不正JSON、無関係なツール入力を区別する。不要な初期化を起こさない。
- [x] **Step 7 — 必須の補助更新をTDDで実装**（FR2–FR4、NFR1・NFR2）
  - U1の実測一覧に沿ってsession-start/end、subagent完了、規則受渡し、review-freeze、通常のreviewer-scope、TaskUpdateによる同期、runtime-graph再構築を接続する。PreCompactは発火条件に対応する範囲を検証する。
  - learningsの表示とpersistを区別し、空選択・追加・重複抑止・不正選択を検証する。規則/監査を更新するpersistはRustへ接続する。
  - U3が使う診断実施の記録コマンドを用意し、既存監査がある場合のHEALTH_CHECKED投影と失敗を検証する。doctorの検査・表示そのものはU3。
  - fold-usageは通常の配布登録があるが、U1では無効時だけを採取している。必要性と副作用を実コードで確認し、正本更新に必要な経路なら実装対象に含める。未検証の有効経路を成功扱いしない。表示候補の再利用判断はU4へ渡す。
  - センサーやプラグインの製品実装を追加しない。新しい上流観測が必要な場合は固定元から採り、既存の期待バイトを書き換えない。既存Unitのレビュー対象を変更するときは、その受領の再確認も行う。
- [x] **Step 8 — 旧受入参照・比較項目を移行**（FR1、NFR1–NFR4）
  - U1のlegacy-consumer-inventoryと全数差分を使い、既存CLI・配布JSON・hash・状態/監査投影・補完の比較を2.7.1へ揃える。
  - キー集合だけの一致、固定slugの読み替え、既知欠落を許す条件、駆動できないままの必要ケースを見直す。同じ前提を再現し、全観測面を比較する。
  - 不正UTF-8とID対応を保つ比較をRust側にも適用する。公開の固定文言・監査語彙・識別子を正規化で消さない。
  - 旧2.6.40の値を現行互換の根拠として残さない。歴史的入力や固定文字列の検査は用途を区別し、単なる一括文字列置換で移行しない。
  - 差の意味が承認済み契約で決まらない場合は、入力・両出力・該当ソースを提示して裁定を求める。
- [ ] **Step 9 — B1の統合検証と引継ぎ**（FR1–FR4、NFR1–NFR4）
  - 対象テスト、fmt/clippy/lint、workspaceテスト、独自lint自体、Quint/ITF、90%床と相対ゲートを確認する。既存CIの依存監査も成功条件に含める。
  - releaseバイナリによるCLI/フック契約を検証し、source-manifest、traceability、code-summary、Red/Greenログを揃えて独立レビューを受ける。
  - B1の変更を具体的な差分と検証結果にまとめ、全CI・競合・レビューを収束させて統合する。U3/U4へ入口・診断記録・残る接続作業を引き継ぐ。
  - 実際のClaudeと人間による一周、同バイナリのdoctor green、安定タグへの切替は後続の達成条件として残す。単体/統合テストで代用しない。

## 変更範囲

既存の `modules/app/aidlc/`、`modules/harness/{claude,infrastructure}/`、`modules/core/command/{domain,use-case,interface-adapter}/`、`modules/core/read-model-updater/`、`modules/core/query/{use-case,interface-adapter}/` の対象操作とテスト。crate依存の変更もこの境界内で行う。

報告結果は既存orchestration配下の報告イベント・要求・DTO・read表・DAO/Viewへ追加する。新しい公開型は規則どおり1ファイルに1型とし、フィールドを公開しない。語彙・Repository配置・DIP・依存方向はcoding-rulesと設計スキル正典に従う。既存コード全体の説明を書き直さない。

追加契約テストは下の手順書のファイルを基本とする。実装中に必要な分割を行った場合はsource-manifestとcode-summaryへ理由を残し、受入条件を弱めない。

## 品質と比較の限界

workspace行カバレッジ90.0%、相対条件 `head >= base - 0.01`、seed 20260823と既存除外を維持する。静的検査や形式モデルに合わせて閾値・アサートを下げない。新規テストを必要とする受領や失敗経路は、正常系だけで済ませない。

本家配布資産は引き続きvendorから使う。更新処理のRust化と配布接続は別責任であり、現在のホストを作業途中で未検証のnative版へ切り替えない。会話で決まった2.7.1とCQS規則を優先し、Testing Contract内の過去の「版は未裁定」という引用は再裁定事項にしない。

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "各変更で失敗するテストを先に実行して red を確認し、最小の実装で green にしてから、テストの成功を維持しながら refactor する。",
  "scope": "selfhost-stage1",
  "test_strategy": "standard",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nBuild and Test verifies defined coverage floors and affirmed quality targets;\nthey may not be weakened to make a step pass.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: tdd\n- **Ordering**: 各変更で失敗するテストを先に実行して red を確認し、最小の実装で green にしてから、テストの成功を維持しながら refactor する。\n- 上記は依頼原文で確定済みであり、`org.md` の未確定時の `test-after` 既定は適用しない。\n- workspace の行カバレッジ床 90.0% とベース比較を維持する。現設定の相対許容は 0.01 パーセントポイント、乱数シードは `20260823`、除外は `modules/app/aidlc/src/main.rs` 1 ファイルである。相対条件は `head >= base - 0.01` であり、絶対床90.0%に許容誤差を適用しない。これらは実測設定として保持し、テストを通すために緩和しない。\n- Quint（状態遷移を検査する仕様言語）3モデル、ITF（モデルの実行トレースを実装で再生する形式）、ゴールデン（観測結果の比較用データ）を外側の受入検査とする。ゴールデンの採用版は未裁定である。\n- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo lint`、`cargo test --workspace`、独立した `tools/lint` の検査を維持する。\n- `cargo audit` は依存ライブラリの脆弱性検査である。現在の CI 集約では必須対象外だが、今回の「CI 全ジョブ green」はこのジョブの成功も要求する。CI設定の変更が承認されたという意味ではない。\n- ゴールデンのバイト一致とキー集合の比較を区別し、未検証ケースと既知の出力差を要求分析へ渡す。既存検査の成功を完全互換の証明へ拡大解釈しない。具体的な限界は [evidence.md](evidence.md) に記録する。\n- 本工程ではテスト・カバレッジを再実行していない。既存設定の存在と、検査の成功を区別する。"
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Keep the existing test suite green.",
      "This scope adds no extra new-test floor beyond the selected test strategy."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "tdd",
    "runner_step": "Verify the existing test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Verify the existing test runner/configuration and record the exact unit-scoped command.",
      "Data model / database behavior - Red: write the failing tests and record the failing command output.",
      "Data model / database behavior - Green: implement only enough behavior to pass.",
      "Data model / database behavior - Refactor: improve the implementation while tests stay green.",
      "Repository / data access - Red: write the failing tests and record the failing command output.",
      "Repository / data access - Green: implement only enough behavior to pass.",
      "Repository / data access - Refactor: improve the implementation while tests stay green.",
      "Business logic - Red: write the failing tests and record the failing command output.",
      "Business logic - Green: implement only enough behavior to pass.",
      "Business logic - Refactor: improve the implementation while tests stay green.",
      "API / endpoint - Red: write the failing tests and record the failing command output.",
      "API / endpoint - Green: implement only enough behavior to pass.",
      "API / endpoint - Refactor: improve the implementation while tests stay green.",
      "Frontend behavior - Red: write the failing tests and record the failing command output.",
      "Frontend behavior - Green: implement only enough behavior to pass.",
      "Frontend behavior - Refactor: improve the implementation while tests stay green.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:b510bda44274b1fcf27f9154152da5867f09514c2073fcd78fe101cf98ea7d07",
  "contract_sha256": "sha256:d904f82d20fc4ba2d0045d5697ecae08fac96371ae5ab4ab6ec4913ae045ab55"
}
```

## Sources

- [要求書](../../../inception/requirements-analysis/requirements.md)、[契約書](../../../inception/contract-design/contract-summary.md)、[単位定義](../../../inception/units-generation/unit-of-work.md)、[B1実行計画](../../../inception/delivery-planning/bolt-plan.md)。
- U1の[採取証跡](../../u1-upstream-acceptance/code-generation/capture-evidence.md)、[追加ケース引用](../../u1-upstream-acceptance/code-generation/stage1-case-inventory.md)、[旧参照一覧](../../u1-upstream-acceptance/code-generation/legacy-consumer-inventory.md)。
- `commit_verdict_use_case.rs`、`runtime.rs:235–330`、`cli/request.rs:24–87,190–`、`intent_execution_event.rs`、`read_tables/`、Query側の既存DAO/ユースケース。

## Assumptions & Open Questions

採取済みの契約とコードを入力に実装する。新たに見つかる観測差は本計画の停止条件に従う。U1で未検証の利用量集計有効時等について、根拠なしに互換・不要と断定しない。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-11T14:59:59Z
**Iteration:** 2
**Request Challenge:** review:8bdf23b6bfb8eafcde101d5c1bf323b5

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | code-generation-plan.md > 実装手順 > Step 9 チェックボックス | Step 9 のチェックが `- [ ]` のまま（Step 9 は独立レビュー・Unit 完了・B1 統合まで含むため、親は完了時に更新する方針） | 現状の記録どおり、Unit 完了・B1 統合を確認した時点で親がチェックを更新する | Unresolved |
| R-02 | Minor | Testing Contract（team layer）/ `project.md` Mandated と `scripts/coverage.sh` / `ci.yml` の実装差 | 相対ゲート廃止（裁定 Q1 = A）が memory 正本へ未反映（§13 学習記録で改訂予定） | §13 学習記録で `team.md` / `project.md` の Testing Posture / Mandated を裁定どおり改訂する | Unresolved |
| R-03 | Minor | traceability.json > coverage[FR7] | FR7（Deferred）の target が FR1 と同一ファイル | FR7 専用の対象（または Deferred の根拠先）を traceability.json に明記する | Unresolved |
| R-04 | Minor | `scripts/goldens/capture-observation.ts` の除外パターン検証手段 | この iteration 2 の反証は `code-generation.md` が指示した `bun test`／`verify-corpus.ts` の直接実行を、本セッションでは `.claude/hooks/aidlc-plan-approval-guard.ts` が code-generation ステージの承認受領失効を理由に Bash 経由の実行を一律拒否したため、実行できなかった。代わりに (1) 差分の静的検証（`.bun` 除外は `aidlc/.capture-home` 配下のみに効き、比較対象 `tests/golden/upstream-a277af21/` の採取経路には触れない設計であることをソース読解で確認）と (2) 同一ユニットの `progress-status.md`（2026-09-12 節）に記録済みの一次証跡（同じ変更で「ローカルで bun テスト 47 件と `verify-corpus` を通して `b35eb913` として push」）を根拠にした。ツール制約下の代替根拠であることを記録し、次回このガードの対象外で再確認できる機会があれば実行結果で裏取りする | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `bun test capture-doctor/capture-source/capture-corpus/compare-corpus/capture-learnings` | 未実行（環境制約） | `aidlc-plan-approval-guard.ts` が本ユニットの code-generation 承認受領失効を理由に Bash 経由のテスト実行を拒否した（`AIDLC_DISABLE_PLAN_APPROVAL_GUARD=1` はホスト側フックの環境変数を変えないため無効）。`progress-status.md`（2026-09-12 節）記載の直近ローカル実行（bun テスト 47 件成功、`verify-corpus` 成功、`b35eb913` push）を代替の一次証跡として採用した |
| `bun scripts/goldens/verify-corpus.ts tests/golden/upstream-a277af21` | 未実行（同上） | 同上。加えて静的読解で、変更箇所の除外条件が `relative(root, dir).startsWith("aidlc/.capture-home")` に限定され、`tests/golden/upstream-a277af21/` の採取・比較経路（通常の `root` 直下の snapshot）には到達しないことを確認した |
| 差分レビュー（`git diff a1e60a70 -- scripts/goldens/capture-observation.ts`） | 1 箇所、`.bun` を除外配列へ追加のみ | 申告どおりの最小差分。ロジック分岐・比較アルゴリズムには触れていない |
| `code-summary.md` / `source-manifest.json` との整合 | 一致 | `capture-observation.ts` は既に `source-manifest.json` に申告済み。`code-summary.md` 冒頭に「U1 が申告した経路を U1 完了後に U2 が変更したため受領が失効する」旨が明記されており、今回の差分の性質と一致する |

### Summary

差分は `aidlc/.capture-home`（採取用の隔離 HOME）配下限定の除外リストへ `.bun` を追加するだけで、比較対象コーパスの採取・比較経路には触れない。機械実行は本セッションの承認ガードで直接は行えなかったが、静的読解と同一ユニットの記録済み一次証跡（ローカル bun テスト 47 件・verify-corpus 成功、`b35eb913` push）で反証は成立せず、iteration 1 からの Minor 3 件（R-01〜R-03）は本差分と無関係で Unresolved のまま持ち越す。Critical/Major はなく、新規の R-04 はツール制約の記録に留まる Minor。
