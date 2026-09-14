# U3: セルフホスト用の自己診断の実装計画

## 対象と完了条件

FR5・FR6（NFR1–NFR4）を、承認済み契約 C5（自己診断へ渡す根拠）・C7（自己診断 CLI、D1–D5、DC1–DC10）に沿って実装する。比較基準は U1 が採取した本家 2.7.1 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の doctor 観測（`tests/golden/upstream-a277af21/doctor/cases.json`、28 ケース = 準備 14 + 観測 14）。U2 が着地した状態・監査・受領の正本、`RecordHealthCheckUseCase` と `HEALTH_CHECKED` 投影、`HookHealth` 読取りモデルを利用し、診断が正本を修復・再初期化しない。

完了条件は unit-of-work U3 のとおり: `target/release/aidlc --doctor` の入口があり、対象とした正常/異常構成（DC1–DC10）を実際に識別し、未実装・対象外の項目を成功行として数えない。本リポジトリでの最終診断成功（切替条件）は後続の切替工程で再確認する。B2 は U3 単独の [Pull Request](https://github.com/amadeus-dlc/amadeus-ng/pulls) とし、全 CI とレビュー収束後に main へ squash-merge する。

## 現コードで確かめた差分

- `main` は B1 マージ後の `33b61cdb`（U1 + U2 を含む）。workspace 検査 3,599 件成功、カバレッジ 98.55%（絶対床 90% のみ。相対ゲートは 2026-09-11 の裁定で廃止）。
- `cli/request.rs:254` は `next --doctor` を `ReadOnlyVerb::Doctor` として受け、`turn.rs` の pre-guard が本家どおり `bun .claude/tools/aidlc-utility.ts doctor` を名指す print directive を返す。**`aidlc --doctor`（Orchestrate 面の先頭引数）は `UnknownOrchestrateVerb`** であり、C7 の入口はまだ無い。
- U2 が用意済み: `RecordHealthCheckUseCase`（`core-command-use-case`）、`HealthCheckResult`（domain）、`HealthChecked` イベントと `HEALTH_CHECKED` 監査行の RMU 投影、`HookHealthView` / `HookHealthDao`（heartbeat・drops）、`StateVersionClassification`（domain、版分類器）、`diagnostic_record_contract.rs`（記録 API の SQLite 結合）。
- 環境検査（Bun、settings.json のフック参照、disableAllHooks の優先順、managed-only、workspace shell、scope 検証、循環・孤児、schema・参照）は本家 `aidlc-utility.ts` の該当分岐（C7 の表の行番号）にあり、本 build には Query 側の観測入力として未実装。
- 本家 doctor の 50 行のうち、C7 が採用するのは D1.a・D2.a–d・D2.f・D3.a–d・D4.a と、対応行の無い独自の必須診断 D1.b・D2.e・D4.b・D4.c・D5.a・D5.b。プラグイン・センサー・intent registry・worktree・branch・practices staleness・MERGE_DISPATCH・rule drift・workspace records・`Workflow diagnosis (advisory)` 欄は対象外で行を出さない（doctor 全文とのバイト一致は主張しない）。

## 責任と配置

CQS の例外は設けない。診断は読取り専用の Query として組み、診断実施の記録だけを U2 の更新コマンド経由で行う。

1. **観測入力（Query interface-adapter）**: Bun の所在、`.claude/settings.json` とその優先順の設定群、フックファイルの存在、`.claude/` と `aidlc/spaces/default/memory/` の配置、`stage-graph.json` / `scope-grid.json` / scope ファイル、状態ファイルの版、イベントストアの開閉と schema、投影ファイルの存在。ファイル・環境の読取りは DAO 実装に置き、観測元を識別して返す（C5 の `observed_configuration`）。
2. **判定（Query use-case）**: D1–D5 の各検査を `DoctorCheck`（ID・ラベル・結果・修復案/原因）として評価し、`DoctorReport`（行の並び・passed/failed・終了コード）へ集約する。初回状態では D4–D5 を非適用として行を出さない。本家対応行のラベル・fix は corpus のバイトを正とし、独自行は固定ラベル + ` — <原因>`。ドメイン判断（状態版の分類、投影整合）は U2 の読取りモデル・分類器を使い、Query で再実装しない。
3. **入口と描画（app）**: `aidlc --doctor`（Orchestrate 面、追加引数なし）を `Request::Doctor` として受け、`DoctorReport` を C7 の書式（ヘッダ、`─`×37、`✓  ` / `✗  `、集計行、LF）で stdout へ描画し、`failed=0` なら 0、それ以外は 1 で終了する。stderr は通常空。
4. **実行記録（app → U2 コマンド）**: 起動時に監査シャードが存在する記録がある場合だけ、`RecordHealthCheckUseCase` へ `HealthCheckResult(passed, failed)` を渡し、RMU が `HEALTH_CHECKED`（`Request: /aidlc --doctor`、`Details: <passed> passed, <failed> failed`）を投影する。記録失敗は C2 の共通エラー経路で終了 1（診断 stdout は残る）。監査なしでは状態・監査・ストアを作らない。

## テストする公開境界

| 境界 | 検証すること | 主なテスト |
| --- | --- | --- |
| 観測 DAO | Bun 不在、settings 不読/参照なし/JSON 不正、フック欠落、workspace shell 不足、scope/graph/schema 不正、状態版の欠損・過去・未来、ストア不在/不読/schema 不正、投影欠落を「成功に丸めず」観測元付きで返す | 新規 `doctor_observation_contract`（query interface-adapter） |
| 判定 use case | D1–D5 の各行の成否・ラベル・fix、初回の非適用、advisory を失敗にしない、対象外項目を出さない、集計と終了コード | 新規 `doctor_report_contract`（query use-case） |
| CLI | `aidlc --doctor` の入口、追加引数の拒否、書式、終了コード、監査あり/なしの副作用、記録失敗の終了 1 | 新規 `doctor_contract`（app、子プロセス実行） |
| 本家対応 | 採用行を corpus 14 観測の該当行・終了値と突き合わせ（採用行のバイト一致と、非採用行を出さないこと） | 同 `doctor_contract` の corpus 駆動ケース |
| release | 同じ契約を `--release` バイナリで確認 | `cargo test -p aidlc --release --test doctor_contract` |

Standard の各コンポーネント 5–8 件を基準とし、正常・拒否・評価不能・境界（heartbeat 300000ms）を含める。件数合わせの同義テストは作らない。

## 実装手順

各項目を Red（失敗の実行確認）→ Green（最小実装）→ Refactor（成功維持）で進める。

- [ ] **Step 1 — 入力と実行基盤の確認**（FR6、NFR2–NFR4）
  - C7 の表・DC1–DC10・corpus 28 ケースを読み、採用行ごとに本家のラベル・fix・適用条件を対応表へ写す。U2 の `RecordHealthCheckUseCase` / `HookHealthDao` / 版分類器の公開 API を確認する。
  - 既存ランナーを確認する（`cargo test -p aidlc --test diagnostic_record_contract`）。新テスト target の追加時は未検出・ビルド不足を Red に数えない。
- [ ] **Step 2 — 観測 DAO を TDD で追加**（FR5・FR6、NFR1）
  - Red: Bun の所在、settings の読取（不読/参照なし/JSON 不正/managed-only/disableAllHooks の優先順）、フックファイル存在、workspace shell、scope・graph・schema、状態版、ストア、投影の各観測が「不在・不読・不正」を区別して返す契約。
  - Green: Query interface-adapter に観測 DAO（1 ファイル 1 公開型、フィールド非公開）を追加。ドメインへ serde・SQL を持ち込まない。
- [ ] **Step 3 — 判定 use case を TDD で追加**（FR6、NFR1）
  - Red: D1.a〜D5.b の各行の成否・ラベル・原因、初回状態の非適用、advisory を失敗にしない、対象外行を出さない、`passed/failed` と終了コード。
  - Green: `DoctorCheck` / `DoctorReport` と評価 use case。本家対応行の文言は corpus を正とする。
  - Refactor: 行の順序（D1.a から表順、フック別は名前順）と集計を 1 箇所に置く。
- [ ] **Step 4 — CLI 入口と描画を TDD で接続**（FR6、NFR1・NFR2）
  - Red: `aidlc --doctor` が現在 `UnknownOrchestrateVerb` であること、追加引数の拒否、書式（ヘッダ・区切り・集計・LF）、終了コード。
  - Green: `Request::Doctor` → 判定 use case → 描画。`next --doctor` の既存 print directive は変えない。
- [ ] **Step 5 — 実行記録の副作用を TDD で接続**（FR6、NFR1）
  - Red: 監査なしでファイル・イベントを作らない（DC1）、監査ありで `HEALTH_CHECKED` を 1 件（DC2）、記録失敗で終了 1（DC10）。
  - Green: 起動時の監査存在判定 → `RecordHealthCheckUseCase` → RMU 投影。診断のために既存の状態・監査を修復しない。
- [ ] **Step 6 — 異常注入と本家対応の受入**（FR5・FR6、NFR1・NFR3）
  - corpus の 14 観測（cold / initialized / missing-bun / missing-hook / disabled-hooks / missing-settings / no-hooks / invalid-settings / managed-only / version-missing / past / future / heartbeat-boundary / stale / unreadable / missing-stage / invalid-stage / invalid-reference / cyclic-graph / audit-locked）を DC1–DC10 へ対応付け、採用行のバイト一致と終了値を固定する。独自診断（D1.b・D2.e・D4.b・D4.c・D5.a・D5.b）は異常注入（入口欠落、接続不一致、状態不読、識別不整合、ストア不読、投影欠落）で失敗を確認する。
  - 差の意味が C7 で決まらない場合は入力・両出力・該当ソースを提示して裁定を求める。
- [ ] **Step 7 — B2 の統合検証と引継ぎ**（FR5・FR6、NFR1–NFR4）
  - fmt / clippy / `cargo lint` / `tools/lint`、workspace 全通し、Quint ゲート、カバレッジ絶対床 90%、`cargo audit`、release バイナリでの `doctor_contract`、CI の bun テスト群。
  - `source-manifest.json` / `traceability.json` / `code-summary.md` を揃え、独立レビュー（adversarial、最大 2 反復）を受ける。B2 の PR を作り、CI 全ジョブ成功とレビュー収束後に squash-merge する。
  - 本リポジトリでの doctor 成功と切替は U4・切替工程の達成条件として残す。

## 変更範囲

`modules/core/query/use-case/`（診断の判定・View）、`modules/core/query/interface-adapter/`（観測 DAO）、`modules/app/aidlc/`（`Request::Doctor`、描画、記録の呼出し）、対応するテスト。U2 の更新処理・RMU・ドメインは変更せず利用する（必要な読取りモデルの欠落が見つかった場合は U2 の責任として最小の RMU 追加を行い、code-summary に理由を残す）。配布資産（`.claude/` 配下）は変更しない。

## 品質と比較の限界

workspace 行カバレッジ絶対床 90.0%、seed 20260823、除外 `main.rs` を維持する。閾値・アサートを下げない。doctor 全文とのバイト一致は主張せず、採用した本家項目と独自項目を C7 の契約で比較する。macOS で採取した corpus と Linux CI の差（パス接頭辞、ロック診断文言）は U2 で学んだとおりテスト側の正規化で吸収し、期待値を手修正しない。

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

- [要求書](../../../inception/requirements-analysis/requirements.md) FR5・FR6、[契約書](../../../inception/contract-design/contract-summary.md) C5・C7、[単位定義](../../../inception/units-generation/unit-of-work.md) U3。
- 本家採取 `tests/golden/upstream-a277af21/doctor/cases.json`、U1 の `scripts/goldens/capture-doctor.ts`。
- U2 の `record_health_check_use_case.rs`、`health_check_result.rs`、`hook_health_view.rs`、`state_version_classification.rs`、`diagnostic_record_contract.rs`、[U2 code-summary](../../u2-workflow-authority/code-generation/code-summary.md)。

## Assumptions & Open Questions

- 本家の環境検査（scope 検証・schema・参照）の判定規則は、本家 `aidlc-utility.ts` の該当分岐を読んで Rust へ写す。対象は bugfix / feature の 2 スコープに限定し、本家の「11 scopes valid」とは集計が異なることを C7 どおり許容する。
- 独自診断 D2.e（Native hook bindings）の「U4 の接続定義」はまだ無いため、U3 では `.claude/settings.json` の各フック登録が本バイナリの面（`aidlc hook <name>` または配布 `.ts`）のどちらへ結ばれているかを識別し、未接続・混在を失敗にする形で実装する。U4 が接続定義を確定したら判定基準を追従する。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-12T08:57:57Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | aidlc/spaces/default/intents/260907-selfhost-stage1/construction/u3-selfhost-doctor/code-generation/traceability.json > `NFR4` の coverage 行 | `NFR4`（調査・変更の限定 — 差分の限定、日本語の会話・成果物、術語の初出注釈、観測契約の固定文字列の逐語保持）の target が `modules/core/command/domain/src/workspace/doctor_checks/environment_checks.rs` 1 本になっている。同ファイルは本家ラベル（`bun installed (required for CLI tools and hooks)` 等）を逐語で保持する点では NFR4 の一部を裏付けるが、「必須経路で使う差分だけを変更する」という NFR4 の主眼（変更範囲の限定）はこの 1 ファイルからは検証できない。`source-manifest.json`（175 経路の申告そのもの）や `code-summary.md` の「変更範囲」節など、範囲限定を直接裏付ける成果物を target に含める。 | New |
| R-02 | Minor | aidlc/spaces/default/intents/260907-selfhost-stage1/construction/u3-selfhost-doctor/code-generation/relocation-brief.md > 「背景と裁定」「目標の配線」節 | `relocation-brief.md`（`code-summary.md` の Sources が引く作業指示）は Q3 裁定時点の「記録があるときは既存ストアへ追記し、`HEALTH_CHECKED` も従来どおり投影する」という設計を現在の配線として記す。この記述は同日の Q4 再裁定（`doctor-placement-questions.md`）で「実装できないことが判明」し撤回され、確定仕様は `code-summary.md`「主要な実装判断 2」「承認済み計画からの逸脱 2」の毎回一時ストア方式に置き換わっている。本ファイルは code-generation の `produces` 対象外の作業メモであり実装自体に矛盾は無いが、Q4 の帰結を追記するか失効の注記を残さないと、後続の読者が撤回済みの配線を現行設計と誤認する。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `cargo test -p core-command-domain --test workspace_doctor_contract` | PASS (10/10) | 集約 `WorkspaceDoctor` の判定（D1.a〜D5.b、初回の非適用、イベント再生）を確認。 |
| `cargo test -p core-command-interface-adapter --test workspace_doctor_repository_contract` | PASS (8/8) | 一時ストア (`open_ephemeral`) を含むリポジトリ契約を確認。裁定 Q4 の帰結と一致。 |
| `cargo test -p core-read-model-updater --test workspace_doctor_projection_contract` | PASS (8/8) | RMU が `passed`/`failed`/終了コードまで焼き込み、クエリ側が数え直さないことを確認。 |
| `cargo test -p core-query-interface-adapter --test doctor_observation_contract` | PASS (8/8) | 観測 DAO が不在・不読・不正を区別して返すことを確認。 |
| `cargo test -p core-query-interface-adapter --test doctor_report_dao_contract` | PASS (6/6) | DAO が「1 表 1 引当」であることを確認。 |
| `cargo test -p core-query-use-case --test doctor_report_contract` | PASS (7/7) | `DoctorReportUseCase` が `dao.find → View` のみで判断を持たないことを確認。 |
| `cargo test -p aidlc --test doctor_contract` | PASS (10/10) | 入口・書式・終了コード・DC1/DC2/DC10・corpus 19 観測のバイト一致を確認。 |
| `cargo fmt --all --check` | PASS | 差分なし。 |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | 警告なし。 |
| `cargo lint` | PASS (exit 0) | 独自 lint（`dao-single-table` 等）を含め成功。 |
| `git status --porcelain -uall`（`aidlc/` 除外）と `source-manifest.json`（175 経路） の突合 | 完全一致（差分ゼロ） | 未申告の変更経路・過大申告のいずれも無い。 |

### Summary

C7 の D1.a〜D5.b の対応表・表示順・集計・終了コード、DC1/DC2/DC10 の副作用契約、Q1〜Q4 の利用者裁定（対象集合、判定の置き場所、一時ストア化）はいずれも実装・テストの双方で一致を確認できた。CQRS 境界（判定は集約、集計は RMU の非正規化、クエリ側は `dao.find → View` のみ、DAO は 1 表 1 引当）にも違反は無い。`source-manifest.json` は実際の差分と 175 経路すべてで一致し、対象外の変更や無申告の変更は無い。指摘 2 件はいずれも Minor（トレーサビリティの target 選定の弱さ、失効済み設計を記した作業メモの追記漏れ）であり、実装の正しさやレビュー対象成果物の整合を損なわない。
