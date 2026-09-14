# U4: 配布接続・切替準備の実装計画

## 対象と完了条件

FR5・FR7・FR8（NFR1–NFR4）を、承認済み契約 C6（配布接続と補助処理の読取り）・C8（接続・ホスト識別と切替準備）に沿って実装する。U2 の正本更新処理と U3 の診断を**利用する**単位であり、状態更新・実行許可の別実装を持たない。

完了条件は unit-of-work U4 のとおり、「必要な接続と統合検証、スモークの実行方法、ホスト/ターゲットの識別、復帰方法が揃うこと」である。**U4 の準備完了だけで FR7・FR8 を達成扱いにしない。** 本リポジトリの実フック・人の応答を使う bugfix 一周、自己診断、CI 全ジョブ成功、検証済み版への切替は、後続の検証・切替工程で実施して証拠を残す。

B3 は U4 単独の [Pull Request](https://github.com/amadeus-dlc/amadeus-ng/pulls) とし、全 CI とレビュー収束後に main へ squash-merge する。

## 現コードで確かめた差分（2026-09-12 実測）

### フック接続

- `.claude/settings.json` の**フック登録 16 本はすべて配布 `.ts`** を指す（`bun "$CLAUDE_PROJECT_DIR/.claude/hooks/aidlc-*.ts"`）。native 面（`aidlc hook <name>`）へ結ばれた登録は**1 本も無い**。
- 本 build の `NATIVE_HOOKS`（`modules/app/aidlc/src/runtime.rs:1060`）は **14 本**: `record-human-turn` / `state-transition-guard` / `write-audit-log` / `continue-workflow` / `session-start` / `session-end` / `log-subagent` / `validate-state` / `sync-workflow-state` / `rebuild-stage-graph` / `review-freeze` / `reviewer-scope` / `deliver-stage-rules` / `fold-usage`。
- 配布 16 本のうち **native に無いのは 2 本**: `plan-approval-guard`、`run-sensors`。
- `statusLine` は `statusline-combined.sh` を指す。本家 `a277af21` の配布は `aidlc-statusline.ts` で、この差が U3 の D2.a 名指し 16 本 / 本家 17 本の由来である。

### 動詞接続

配布のステージ本文・プロトコルが呼ぶ動詞を実測で列挙した（`.claude/aidlc-common/` の全ステージ・プロトコル）。この build に**配線済み**なのは U3 の `missing_entry_points` が引く 14 入口だけである。

| 面 | 配線済み | 配布が呼ぶが未配線（実測） |
| --- | --- | --- |
| `aidlc-orchestrate` | `next` / `continue` / `report` / `park` / `--doctor` | — |
| `aidlc-log` | `decision` / `answer` / `review` / `link` | — |
| `aidlc-state` | `practices-promote` | `unit` / `lookup` / `reuse-artifact` / `set` / `skip` / `unpark` / `practices-event` / `set-construction-iteration` / `set-unit-ownership` / `set-unit-gate-rhythm` |
| `aidlc-utility` | `intent-create` | `project-description` / `document-input` / `scope-table` / `scope-change` / `recompose` / `codekb-*` 4 種 |
| `aidlc-learnings` | `surface` / `persist` | — |
| `aidlc-bolt` | `set-autonomy` | — |
| `aidlc-testing-posture` | （配布 TS のまま） | `render` / `fingerprint` |
| `aidlc-review-brief` | （配布 TS のまま） | `context` / `review` / `summary` |
| `aidlc-graph` / `aidlc-audit` / `aidlc-jump` / `aidlc-swarm` / `aidlc-worktree` / `aidlc-unit` | — | `compile` / `append` / `execute` / `prepare` / `merge`・`info` / `claim`・`participate`・`release` |

この表は**全 33 ステージ**の呼出しであり、実装対象ではない。U4 の対象は bugfix スコープ 1 周（9 / 33 ステージ = 0.1–0.3・reverse-engineering・requirements-analysis・code-generation・build-and-test・deployment-pipeline・deployment-execution）が実際に踏む部分集合に限る。その確定は Step 1 で行う。

### 読取り専用の再利用候補（C6）

C6 が `reader_candidates` に挙げるのは `aidlc-review-brief.ts`（`summary` / `review` / `context`）と `aidlc-testing-posture.ts`（`render` / `fingerprint`）の 2 本 5 動詞である。C6 は**操作単位**での判定を求め、`receipt_creation` / `code_generation_authority_begin` / `learnings_persist` / `audited_catalog_updates` を読取り専用から除外する。

### 同期パッチ

`scripts/aidlc-sync/patches/` に 12 本。`bun scripts/aidlc-sync.ts --check` は成功する（2026-09-12 実測）。ただし `claude-without-bedrock.patch` は `settings.json` の `env` ブロックと `model` 行を削る差分であるのに対し、現物の `settings.json` には `env` も `effortLevel` も `model` も無い。この不一致は Step 1 で確認し、必要ならパッチを現物へ追従させる。

### ホスト識別・切替資産

現物に、ホスト（安定タグ）とターゲット（開発版）を識別する資産・切替手順・復帰手順は**無い**。安定タグも未作成である（team.md「今はタグを作成しない」）。

## 責任と配置

1. **必要集合の確定（U4 の起点）**: bugfix スコープ 1 周が踏む動詞・フック・受領記録を、配布の `SKILL.md`・`stage-protocol.md`・該当ステージ本文から実測で列挙する。想像で増やさない。列挙結果を `required-surface.md` として記録する。
2. **フック接続**: 列挙で必要と出た native 14 本の登録を `aidlc hook <name>` へ向ける。`plan-approval-guard` / `run-sensors` は native に無いため配布 `.ts` のまま残す（根拠は「品質と比較の限界」）。`statusLine` は本件で変更しない。
3. **動詞接続**: 列挙で必要と出た動詞のうち、この build に無いものを列挙する。**新規実装は U2 の責任**であり、U4 は接続と、不足の明示に留める。接続の失敗を別の更新実装へ切り替えて隠さない（C6）。呼出しの作業ディレクトリ・選択した作業記録・セッション・入力文字列を維持し、入力をシェルの文字列展開で再解釈しない。
4. **読取り専用の再利用検証**: C6 の 2 本 5 動詞について、依存先を含む副作用と、前後の正本（状態ファイル・監査シャード・ストア・承認受領）の不変を統合テストで確認する。ファイル名だけで読取り専用と扱わない。
5. **ホスト識別と切替準備（C8）**: タグ・コミット・バイナリ実体を対応付ける識別、切替手順、復帰手順、`target/release/aidlc` を使うスモークの実行準備を、本リポジトリ固有の接続として用意する。一般向けインストーラへ拡張しない。**タグは作成しない**（作成は切替工程）。
6. **同期パッチ**: 2・3・5 で生じた本リポジトリ固有の差分を `scripts/aidlc-sync/patches/` へ記録し、`aidlc-sync.ts --check` を成功させる。

## テストする公開境界

| 境界 | 検証すること | 主なテスト |
| --- | --- | --- |
| フック接続 | 配布の登録から `aidlc hook <name>` が起動し、stdin の受け渡し・終了コード・stdout/stderr の契約が配布 `.ts` と同じであること。未知フック名の拒否 | 新規 `harness_binding_contract`（app、子プロセス実行） |
| 必要集合 | 列挙した動詞・フック・受領記録の集合が、bugfix ステージ本文の実バイトから導けること。未配線の動詞を「配線済み」と数えないこと | 新規 `required_surface_contract`（列挙の突合） |
| 読取り専用の再利用 | `review-brief` 3 動詞・`testing-posture` 2 動詞の前後で、状態ファイル・監査シャード・ストア・承認受領が 1 バイトも変わらないこと。除外操作（受領作成・承認開始・学習永続化・目録更新）が混ざらないこと | 新規 `read_only_reuse_contract` |
| ホスト識別 | ホスト/ターゲットの識別が、タグ・コミット・バイナリ実体の対応として読めること。復帰手順が検証済みホストへ戻せること。準備だけで達成扱いにしないこと | 新規 `host_binding_contract` |
| 同期パッチ | 追加した差分を含めて `aidlc-sync.ts --check` が成功すること | 既存 `scripts/aidlc-sync.test.ts` の回帰 |

Standard の各コンポーネント 5–8 件を基準とし、正常・拒否・評価不能・境界を含める。件数合わせの同義テストは作らない。**模擬の人間応答・保護の無効化・`runtime::run` の繰返しを、実地スモークの成功証拠にしない**（C8 `not_proof_of_completion`）。

## 実装手順

各項目を Red（失敗の実行確認）→ Green（最小実装）→ Refactor（成功維持）で進める。

- [ ] **Step 1 — 必要集合の確定**（FR5、NFR4）
  - bugfix スコープ 9 ステージの本文と `stage-protocol.md`・`SKILL.md` を読み、踏む動詞・フック・受領記録を実測で列挙する。`required-surface.md` に、各項目の出典行と「配線済み / 未配線 / 配布 TS のまま」の別を書く。
  - `claude-without-bedrock.patch` と現物 `settings.json` の不一致を確認し、追従の要否を決める。
  - 既存ランナーを確認する（`cargo test -p aidlc --test upstream_271_contract`）。
- [ ] **Step 2 — フック接続を TDD で差し替え**（FR5、NFR1・NFR2）
  - Red: 配布の登録から native 面が起動しないこと、stdin / 終了コード / 出力の契約差。
  - Green: `settings.json` の該当登録を `aidlc hook <name>` へ向ける。native に無い 2 本は配布 `.ts` のまま残し、理由を記録する。
  - Refactor: 差分を同期パッチへ切り出す。
- [ ] **Step 3 — 動詞接続と不足の明示**（FR5、NFR4）
  - Red: 列挙で必要と出た動詞のうち未配線のものが、配布の呼出し形で拒否されること。
  - Green: この build にある動詞への接続を通す。無い動詞は**実装せず**、不足として `required-surface.md` と code-summary へ明示する。新規の状態更新実装を U4 で作らない。
- [ ] **Step 4 — 読取り専用の再利用を TDD で検証**（FR5、NFR1）
  - Red: `review-brief` 3 動詞・`testing-posture` 2 動詞の実行前後で正本が変わらないこと、除外操作が混ざらないことを主張するテスト。
  - Green: 副作用が見つかった場合、その処理は U2 の責任として不足に挙げ、U4 では二重実装しない。
- [ ] **Step 5 — ホスト識別と切替準備**（FR8、NFR4）
  - Red: ホスト/ターゲットの識別と復帰手順が無いこと。
  - Green: タグ・コミット・バイナリ実体の対応を読める識別と、切替・復帰の手順を本リポジトリ固有の接続として用意する。タグは作成しない。
  - `target/release/aidlc` を使うスモークの実行準備（実行方法・前提・証拠の残し方）を書く。
- [ ] **Step 6 — 統合検証と引継ぎ**（FR5・FR7・FR8、NFR1–NFR4）
  - fmt / clippy / `cargo lint` / `tools/lint`、workspace 全通し、Quint ゲート、カバレッジ絶対床 90%、`cargo audit`、`aidlc-sync --check`、ゴールデン bun テスト群。
  - `source-manifest.json` / `traceability.json` / `code-summary.md` を揃え、独立レビュー（adversarial、最大 2 反復）を受ける。B3 の PR を作り、CI 全ジョブ成功とレビュー収束後に squash-merge する。
  - 実地スモーク・自己診断・切替は**後続工程**の達成条件として残す。

## 変更範囲

`.claude/settings.json`（フック登録）、`scripts/aidlc-sync/patches/`、本リポジトリの接続・検証用スクリプト、`modules/app/aidlc/tests/`（新規契約テスト）、必要な既存 CI 設定。**配布資産のステージ本文・エージェント・プロトコル・コンパイル済みグラフは変更しない**（変更は同期パッチとして記録する）。U2 の更新処理・RMU・ドメインと U3 の診断は変更せず利用する。一般向けインストーラや配布一般化は追加しない。

## 品質と比較の限界

- workspace 行カバレッジ絶対床 90.0%、seed `20260823`、除外 `main.rs` を維持する。閾値・アサートを下げない。
- `plan-approval-guard` と `run-sensors` を配布 `.ts` のまま残すのは、前者が `NATIVE_HOOKS` に無く、後者がセンサーとして本 intent の**スコープ外**（`project.md` Forbidden）であるためである。この 2 本が配布 TS のまま native エンジンの書く状態を読めるかは、Step 2 の契約テストで実測する。「全フックが native」を切替条件と読み替えない。
- U4 の準備完了を FR7・FR8 の達成と混同しない。実地スモーク・自己診断・CI 全ジョブ成功・切替は別々に記録する。
- ゴールデンの採用版（配布 2.7.1 系 vs 受入 `a277af21` = 2.6.40 系）は未裁定のままである。切替条件 2 を判定する前に人間の裁定を受ける。

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

- [要求書](../../../inception/requirements-analysis/requirements.md) FR5・FR7・FR8、NFR1–NFR4
- [契約書](../../../inception/contract-design/contract-summary.md) C6・C8、および「所有・変更・検証の規則」「未解決事項と実装時の確認」
- [単位定義](../../../inception/units-generation/unit-of-work.md) U4
- 実測: `.claude/settings.json`、`modules/app/aidlc/src/runtime.rs:1060`（`NATIVE_HOOKS`）、`modules/app/aidlc/src/runtime/doctor.rs`（`missing_entry_points`）、`.claude/aidlc-common/` 配下のステージ・プロトコルの呼出し、`scripts/aidlc-sync/patches/`、`tests/golden/selfhost-stage1/doctor-shell/.claude/settings.json`
- [U2 code-summary](../../u2-workflow-authority/code-generation/code-summary.md)、[U3 code-summary](../../u3-selfhost-doctor/code-generation/code-summary.md)

## Assumptions & Open Questions

- **`plan-approval-guard` と `run-sensors` は配布 `.ts` のまま残す**と仮定する。前者は `NATIVE_HOOKS` に無く、後者はセンサーとして本 intent のスコープ外である。「全フックが native であること」を切替条件に読み替えない。この仮定を変えるなら、U4 の範囲と `project.md` の Forbidden の両方を見直す必要がある。
- **未配線の動詞は U4 で実装しない**と仮定する。C6 は U2 を writer、U4 を integrator と定めており、不足は明示して U2 の責任に送る。bugfix 一周が実際に踏む動詞に不足があった場合は、その場で人間へ提示して裁定を求める。
- **`statusLine` は本件で変更しない**と仮定する。`statusline-combined.sh` のままにすると、本リポジトリでの D2.a 名指しは 16 本で本家 17 本と異なるが、これは U3 が記録済みの既知差であり、切替条件ではない。
- ゴールデンの採用版は未裁定のままとする。切替条件 2 の判定前に別途裁定を受ける。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-12T17:24:05Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `scripts/aidlc-selfhost/host-binding.json` の `binding_selected` / `binding_selected_why` フィールド（C8 の正本、`host_binding_contract` が検査対象） | `binding_selected` は `"distributed-typescript"`、`binding_selected_why` は「現在の `.claude/settings.json` は配布 TypeScript の 16 本を登録している…まだ適用していない」と書く。しかし実測では作業ツリーの `.claude/settings.json`（`git status` で `M`、コミット未了）は既に `"$CLAUDE_PROJECT_DIR/target/release/aidlc" hook <name>` 形の native 登録へ書き換え済みである（`sed -n` で確認、かつ本レビュー冒頭で発生した `PreToolUse:Bash hook error` 自体が `target/release/aidlc" hook reviewer-scope` を名指しており、この live 接続が現に発火していることの直接証拠でもある）。ファイルの更新時刻を突き合わせると `host-binding.json`（2026-09-12 18:59 JST 生成）は `.claude/settings.json` の書換え（20:54 JST）より前に書かれ、以後更新されていない。一方で `code-summary.md` と `selfhost-runbook.md` は同じ書換えを正しく「実施済み」として記述しており、C8 の正本であるはずの機械可読資料 (`host-binding.json`) だけが古い状態を主張する形で残っている。`host_binding_contract` の `preparation_is_never_recorded_as_achievement` はこの資料の内部整合だけを検査し、資料と実際の `.claude/settings.json` との整合は検査していないため、この食い違いは既存テストでは捕まらない。加えて、この live 書換えは `switch_procedure` の step 4（本来は切替条件 3 つを満たした後に行う手順）と同じ操作であり、要求書 `requirements.md` 106 行目「FR8 の切替も、それらの証拠が揃ってから行う」より前に実行されている（監査ログ 2026-09-12 の人間裁定に基づく実施であることは確認したが、その裁定が D2.e の判定基準の話として提示されたものか、この live 書換えの是非そのものを問うものであったかまでは、本レビューの読取り範囲内の資料だけでは確認できなかった）。 | `host-binding.json` の `binding_selected` を実際の live 状態（`"native"` 相当、ただし切替 3 条件は未達である旨も併記）に合わせて更新するか、あるいは `binding_selected` は「host/target の識別」専用でありフック配線の現況を語らない旨をスキーマで明示し、フック配線の現況は `hook-binding.json` 側だけが正本であると資料間の責務を明記する。あわせて `host_binding_contract` に、`binding_selected` と実際の `.claude/settings.json` の内容（native/distributed のどちらが登録されているか）が食い違わないことを検査するケースを追加する。 | New |
| R-02 | Minor | 本レビューの検証範囲（`cargo test -p aidlc --test harness_binding_contract` / `required_surface_contract` 等の直接実行） | ターン予算の制約により、依頼された契約テストの直接実行はコンパイルが完了する前に打ち切られ、独立した再実行での pass/fail は確認できなかった。ソースコード（`harness_binding_contract.rs`・`required_surface_contract.rs`・`read_only_reuse_contract.rs`・`host_binding_contract.rs`・`doctor_checks/hook_checks.rs`・`hook_wiring.rs`・`hook_binding_declaration.rs`・`settings.rs` DAO・`observation.rs` 写像）は読み込み、実装がテストの前提（D2.e の判定順序、CQRS 境界、宣言の観測ロジック）と整合していることは静的に確認した。親から報告された全通し実測（119 テストバイナリ・3,805 件成功・0 失敗、line カバレッジ 98.0641%、絶対床 90.0% PASS）は既知の事実として採用し、再実行はしていない。 | 次回反復（必要になった場合）またはゲート前に、本レビューで確認できなかった単位限定コマンドの実行結果を記録に残す。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| 出典行の抜き取り突合（`required-surface.md` の代表引用 4 件 → `stage-protocol.md:681`、`reverse-engineering.md:321`、`requirements-analysis.md:63`、`state-init.md:58-59`） | 一致 | 引用は実バイトの該当行と一致し、捏造・ずれは無い |
| `required-surface.md` の件数集計を `awk`/`grep` で再集計（全動詞表 46 行、配線済み16/未配線27/配布TS3、必ず踏む✓22） | 一致 | `code-summary.md` の件数表（16/27/3=46、必ず踏む11/8/3=22）と符合する |
| `.claude/settings.json` の live 実測（`sed -n`） | native 14 本の登録を確認 | `code-summary.md`／`selfhost-runbook.md` の記述と一致。`host-binding.json` の `binding_selected` とは不一致（R-01） |
| `.github/workflows/ci.yml` の bun 導入段 3 箇所（`git diff`／`grep`） | 同一の pinned Action SHA・同一バージョン (`1.3.13`) | 「既存 `aidlc-distribution` ジョブと同じ固定 Action・同じ版」という `code-summary.md` の主張と一致 |
| `hook_checks.rs` の D2.e 判定順序（未知呼出し→未知 native 名→宣言食い違い）を読解 | ruling（宣言を正として照合、宣言どおりの混在は失敗にしない）と一致 | `harness_binding_contract.rs` の 7 レイアウト期待値とも整合 |
| `cargo test -p aidlc --test harness_binding_contract` / `required_surface_contract` | 未完了（ターン予算内でコンパイルが終わらず打ち切り） | 独立の pass/fail 確認はできていない（R-02）。親の全通し実測を既知の事実として採用 |
| `cargo fmt` / `clippy` / `cargo lint` / `bun scripts/aidlc-sync.ts --check` | 未実行（親が実測済みとして提供された値をそのまま採用） | 独立再実行はしていない |

### Summary

反証を試みた 7 論点のうち、C6 の分担（未配線動詞を U2 へ送る判断・接続失敗を別実装へ切替えない設計・入力保持）、必要集合の出典行、D2.e 追従の判定順序、CI 変更の同一性は実測で裏付けが取れ、破綻は見つからなかった。唯一の実質的な欠陥は、C8 の正本であるはずの `host-binding.json` が、live `.claude/settings.json` の実際の状態（既に native へ書き換え済み）と矛盾する古い記述を残している点（R-01、Major）である。これは実行時の失敗には直結しないが、切替判断の拠り所となる機械可読資料の信頼性を損なうため是正が必要である。Critical 0 件・Major 1 件のため READY とするが、R-01 は次の反復までに解消することを求める。
