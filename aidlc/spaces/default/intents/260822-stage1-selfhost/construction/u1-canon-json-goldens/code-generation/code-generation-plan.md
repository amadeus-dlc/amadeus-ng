# code-generation-plan — U1 正準JSONの実装記録の是正

> Unit: u1-canon-json-goldens。2026-09-06の再確認計画。
> 出典: `../functional-design/functional-spec.md`・`rules.md`・`entities.md`、
> `../nfr-requirements/security-requirements.md`・`tech-stack-decisions.md`、
> `../nfr-design/security-design.md`・`logical-components.md`、
> `../../../inception/contract-design/contract-summary.md`、
> `../../../inception/units-generation/unit-of-work.md`、
> `../../../inception/requirements-analysis/requirements.md`。

## 1. 目的と変更範囲

既存のcore-infrastructure::canon_jsonと採取済みコーパスを、更新済み設計・品質要件に照らして確認する。実装は旧スタブから作り直さず、API別名・後方互換ラッパーを追加しない。期待出力・固定ピン・依存・CI基準を変更しない。

ソースの予定変更は `modules/core/infrastructure/src/canon_json/mod.rs` と `parse.rs` の説明コメントだけである。「全入力がparseを通る」「契約JSONには孤立サロゲートが現れないので実害なし」などの過大な保証を、確認済みの経路・値域・127/128段境界に合わせる。エラーメッセージを含む実行時の振る舞いは変更しない。

実装との不一致が新たに見つかった場合は、問題を再現する試験と変更案を報告し、計画の変更を受けてから扱う。本計画を根拠に他Unitや凍結済みの設計成果物まで変更しない。機能設計R-08（重複キー動作の記載不足）は最終の機能設計承認時の所見として残っており、今回の計画でその承認を代行しない。

## 2. 所有するファイルと保持する成果

| 区分 | 対象 | 扱い |
|---|---|---|
| ソースコメント | modules/core/infrastructure/src/canon_json/mod.rs、parse.rs | 入力経路・深さ・文字列・依存境界の説明を修正 |
| 実装と既存試験 | modules/core/infrastructure/src/canon_json/、同クレートtests/golden_hash_canonical.rs・golden_corpus_read.rs・support/mod.rs | 読取確認と試験。実装や試験を同じ内容で再生成しない |
| 固定データと採取手順 | tests/golden/upstream-3c3146cf/、scripts/goldens/ | C7の配置・来歴・未採取記録・入力測定との対応を確認。再採取・期待値更新なし |
| Unit記録 | code-summary.md、traceability.json、source-manifest.json | 現行ファイルへの対応と今回実行した検証を記録 |
| 計画と試験手順 | このファイル、unit-test-instructions.md | この計画承認の対象。完了チェック以外の変更が必要なら承認を更新 |

既存code-summaryの歴史的な作成パス・Red/Green記録は、今回の事実と混ぜず保存する。過去の実装を今回新規に作ったと記載しない。source-manifestには実際に作成・変更・削除したアプリケーション側パスを列挙する。今回変更しない既存コードはcode-summaryの再利用欄で示し、変更済みと偽らない。

## 3. 実行ステップ

- [x] Step 1. ランナーと設定を確認する。既存Cargo設定・preserve_order/float_roundtrip・固定シードを確認し、unit-test-instructionsのUnit限定3コマンドが実行できることを記録する。実行済みログを再利用する場合は日時と対象を明示する。
- [x] Step 2. 現行実装・既存試験・採取済み32行を、13件のBRと11件の詳細NFRに対応付ける。parse/parse_bytes/to_value、全プロファイルの整数形式キー優先、大整数丸め、UTF-8拒否境界、ハッシュ族を確認する。入力88ファイルの実測と、CLI/フックの未採取記録も確認する。
- [x] Step 3. mod.rsとparse.rsの説明コメントを是正する。旧クレート単位の説明をモジュール境界へ合わせ、直接構築値の深さ検査や巨大入力の保護を過大に保証しない。rustdoc例の振る舞いは維持する。
- [x] Step 4. Unit限定の単体・PBT・ゴールデン・rustdoc試験を実行し、件数と結果を記録する。コメントだけの修正に形式的な新規テストは追加しない。機能欠陥が判明した場合は本計画の範囲を超えて直さず、再現試験・Red→Green→Refactorの変更案を提示する。
- [x] Step 5. code-summaryを現在の事実に更新し、過去のTDD証跡を歴史として保存する。traceabilityの旧shared/canon-jsonパスを実在する現行ファイルへ改め、FR7/FR7.1〜FR7.3・全BR・全詳細NFRの対応を記録する。性能の数値目標を作らず、NFR5.1は観測時の測定手順への対応を明示する。source-manifestへ実際の変更パスを記録する。
- [x] Step 6. 差分を点検し、実行コード・期待値・依存・品質閾値に変更がないことを確認する。独立レビューへ引き渡す。親セッションがレビュー・Unit完了・次工程を処理する。

## 4. Testing Contractの適用

本Unitはbrownfieldで、既存の値モデル・変換・公開APIの実装と試験がある。今回は説明と検証記録の是正で、新規プロダクションコードはない。DB・Repository・業務判断・HTTP API・フロントエンドの新設もないため、それらの実装用ステップを架空に実行しない。

埋め込み契約のTDD方針は維持する。今後振る舞いの変更を行う場合、対象レイヤーで実行可能な試験を先に用意し、失敗出力を記録してから最小修正し、成功中に整理する。既存成功ログから過去のRedを推定しない。

既存のStandard相当の境界試験・性質検証・統合試験を保持する。必須CI、カバレッジ90%床、相対差0.01ポイント、固定シード20260823を維持する。全体の品質ゲートは統合時の検証であり、Unit試験の成功を全体CI・最新依存検査・性能測定の成功に読み替えない。

## 5. 要求からステップへの対応

| 要求・規則 | Step | 確認対象 |
|---|---|---|
| FR7.1、BR2.1・BR2.3・BR2.5 | 2・4・5 | 採取済み32行、来歴、固定ピン、全行比較 |
| FR7.2、BR2.2・BR2.4、NFR1.3・NFR4.4 | 2・4・5 | CLI/フックコーパス、比較器、正規化、未採取記録 |
| FR7.3、BR1.1〜BR1.6、NFR1.1・NFR1.2 | 2〜5 | キー順・数値・文字列・体裁・ハッシュ族 |
| BR1.7・BR1.8、NFR4.1・NFR4.2 | 1〜3・5 | モジュール境界、Cargo機能、局所許可、依存検査設定 |
| NFR4.3 | 2〜5 | parse/parse_bytes/to_value、127/128段、変換エラー |
| NFR2.1〜NFR2.3 | 1・4〜6 | Testing Contract、既存試験、品質閾値、履歴 |
| NFR5.1 | 5 | 劣化を観測した場合の入力・環境・比較条件・測定記録 |

## 6. 作業の進め方

計画承認後、開発担当が上記ファイルの範囲で実行する。他者の変更を戻さず、commit・push・外部投稿は親セッションに任せる。現在の作業履歴と監査を保持し、過去のmain-syncや旧Boltブランチの作成手順を再実行しない。親セッションは全差分と検証を確認し、監査を含む作業ツリー全体を回収する。

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "新規プロダクションコードはレイヤーごとに red-green-refactor",
  "scope": "classic",
  "test_strategy": "standard",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: tdd\n- **Ordering**: 新規プロダクションコードはレイヤーごとに red-green-refactor\n  （失敗するテストを先に書く）で実装する。Quint モデル検査・ITF 準拠テスト・\n  ゴールデンパリティは TDD サイクルの外側の受け入れゲートとして維持し、\n  TDD の red を代替しない。（インタビュー Q2、選択肢 A で確定——品質レビュー\n  の自己完結化置換案どおり）\n\nテストピラミッド（ユニット層を厚く、結合・E2E層を薄く）を意識した配分とする\n（オーナー明言）。比率は**定性のみ**とし、数値目標は定めない（インタビュー\nQ3、選択肢 A）: 単体テスト優位・統合テストは境界ごと・E2E は最小、という\n配置規則で充足する。\n\nこのプロジェクトは TDD の上に **3層の品質保証** を重ねている点が特徴的で、\nそれぞれ役割が異なる（`code-quality-assessment.md` §品質保証の全体像より）:\n\n1. **Quint 形式検証**（毎 PR）— 決定論コアの状態機械契約そのものを検証。\n   不変条件 run 27本・到達性 witness 12本の反転判定・決定的シナリオ。\n   モデルの検査力自体も mutation テストで証明済み（engine_loop 3/3、\n   audit_lock 10/10 + witness 7/7、stop_hook 7/7）。\n2. **ITF 準拠テスト**（`modules/core/domain/tests/`、engine_loop / audit_lock\n   の2モデル・2ファイル）— Quint モデルのトレースを集約に再生し状態射影を\n   突き合わせることで、モデルと実装の乖離を検出。TDD の「テストを先に書く」\n   対象は実装コードだが、契約の正本は Quint 側にあるため、ITF 準拠テストは\n   実装後に契約適合を機械確認する位置づけ（TDD サイクルの red-green-refactor\n   そのものではなく、その外側のゲート）。なお stop_hook は ITF 準拠テストが\n   未整備（既知の穴、`evidence.md` インタビュー未確定事項 (e) 参照）。\n3. **PBT（proptest）+ ゴールデンパリティ**— upstream 配布実バイト33ノードの\n   全数 load パリティを固定し、upstream 互換の逸脱を検出。\n\nしたがって TDD サイクルは主にユニットテスト層（インライン `#[cfg(test)]`、\n実測**40ファイル**——集計方法: `modules/` 配下・`tests/` ディレクトリを除いた\nインライン `#[cfg(test)]` 数。`tests/` 配下6本（ITF準拠2 + 統合4）を含めると\n46、`tools/lint/src/check.rs` を含めても47であり、いずれの集計でも48には\nならない。開発者レビュー指摘どおり40へ訂正した）に適用し、ITF 準拠テスト・\nゴールデンパリティはレイヤー横断の受け入れ確認として TDD サイクルの外側に\n位置づける。\n\n- **カバレッジ**: 絶対ゲート90%床 + PR 相対ゲート（head が base を下回ったら\n  fail、許容誤差 0.5pp。PBT のシード非固定に起因するノイズ較正値であり、\n  stage-1 スコープで**シード固定により 0.01 へ引き締める**——インタビュー\n  Q7、選択肢 A/B。除外設定は現状無いが、**composition root（`main.rs` の\n  配線部分）のみカバレッジ除外を許可**し、それ以外は床を維持する\n  （インタビュー Q5、選択肢 B。除外設定は `scripts/coverage.sh` への確定\n  アクション、`evidence.md` 参照）。実測 94.87〜95.29%（`scripts/coverage.sh`）。\n- **ツーリング**: `cargo test --workspace`（234テスト全緑、実測）、\n  `cargo-llvm-cov`、Quint 0.32.0（Node 22 経由）。\n- **テスト種別**: ユニット（インライン `#[cfg(test)]`）、PBT（proptest、集約\n  本体同居）、ITF 準拠（`modules/core/domain/tests/` 2本）、統合（\n  `modules/core/interface-adapter/tests/` 4本 — ゴールデンパリティ・FS ロック・\n  Repository 実装・シンボリックリンク防御）。\n- **CI ゲート**（`main` へのマージ条件、実測）: `check` ジョブ（`cargo fmt\n  --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` →\n  `cargo lint` → `cargo test --workspace`）、`quint` ジョブ\n  （`scripts/quint-gate.sh`）、`coverage` ジョブ（`scripts/coverage.sh`、\n  絶対90%床 + PR 相対ゲート）の3ジョブすべてを緑にする。この3ジョブは\n  **stage-1 スコープで branch protection の required status checks として\n  機械強制する**（インタビュー Q4、選択肢 A——現状は運用規律のみで機械強制が\n  無いという品質レビューの重大指摘を受けての裁定。設定作業は\n  `evidence.md` の確定アクションに記載）。\n- **スコープ注記**: `tools/lint`（`cargo lint` の実装クレート）は workspace\n  非メンバーの detached クレートであり、CI の fmt/clippy/test がまだ届いて\n  いない（設計監査 C27）。**stage-1 スコープに含める**: `tools/lint` への\n  CI 3ステップ（fmt/clippy/自己テスト）追加（インタビュー Q7、選択肢 A）。\n  macOS CI ジョブ追加・`main` への push トリガー追加は本 intent には\n  含めず、後続 intent へ繰り延べる（インタビュー Q7、選択肢 E 相当の一部\n  不採択）。"
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
  "input_sha256": "sha256:e4f36aa113753d3604df570f5ec3a0cb465d4b29d82a17a16efbb2ea8b993111",
  "contract_sha256": "sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3"
}
```


## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-06T02:33:19Z
**Iteration:** 1

### Findings

今回のコメント2ファイルと実装記録の是正を対象とした、1回のADVISORYレビューである。新規所見はない。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|

機能設計のR-08（重複キー規則の本文記載不足）は同設計の所見として保持する。旧code-summaryのReviewも履歴のまま保持し、過去のTDD証跡・公開面の承認や、欠落一覧の非空アサート・広いCLONE正規化・フック区分の可読性を、今回の成功から解消済みとは判定しない。今回のcode-summary第6・7節は、この境界と未検証範囲を明記している。

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc-sensor-required-sections（stage=code-generation、code-generation-plan.md / unit-test-instructions.md / code-summary.md） | PASS、findings_count=0、追記前H2数7 / 5 / 8 | 指定3文書の構造検査を実行した。 |
| aidlc-sensor-traceability（同stage、traceability.json） | FAIL、findings_count=39 | missing_from_upstream_idsは他UnitのFR1〜FR6・FR8・FR9親子34件と共有NFR1〜NFR5。gaps / orphans / missing_from_table / invalid_entries / invalid_targetsはすべて空。センサー成功とは読み替えない。 |
| U1割当・詳細要求の手動照合 | 一致、28件 | unit-of-workのU1とrequirementsのFR7親子4件、functional-designのBR13件、security-requirementsの詳細NFR11件を列挙済み。共有NFRのU1への適用は枝番で表現され、NFR3は永続化・投影を持たないため対象外。NFR2.1・NFR5.1のN/Aには今回の範囲と以後の手順がある。OKは実装先の対応であり、未実行の全体品質検査成功を意味しない。 |
| git diff（modules/core/infrastructure/src/canon_json/mod.rs / parse.rs）と非コメント行・rustdoc例の機械比較 | コメントのみ、比較一致 | 実行コード・公開面・実行時文言・rustdoc例は不変。source-manifestの2パスと一致する。 |
| 入力経路と設計の照合（mod.rs / parse.rs / value/json_value.rs） | 一致 | parseの深さ事前走査、127段受理・128段拒否、parse_bytesのUTF-8拒否からの委譲、to_valueの変換失敗を確認。直接構築値や巨大入力まで保護するとは保証していない。 |
| 修正後の既存実行ログ（/tmp/u1-code-unit-after.log、/tmp/u1-code-golden-after.log、/tmp/u1-code-doc-after.log） | 87 + 16 + 1 = 104 passed、失敗・ignoredとも0 | 準備時の3ログと対象件数が一致。重複キー、孤立サロゲート、深さ境界、型付き変換失敗を含む。今回レビューでは再実行せず同セッションのログを確認した。別修正の47件は含めない。 |
| golden_hash_canonical.rsとC7・受入表・来歴・欠落記録 | 一致 | 32行の3プロファイル・2族の全行比較を確認。固定ピン・CLI28件・フック14件と理由付き未採取2+1件は要約と一致する。コーパス読取は後続Unitの全実行経路比較の証明ではない。 |
| nfr-input-measurements.jsonの独立再計算 | 全88ファイル一致、最大深さ7 | パス・バイト数・コンテナ深さをPythonで再計算し記録と一致。将来入力の上限保証とは扱わない。 |
| Cargo・clippy・ツールチェーン・CI・coverage設定 | 記載と一致 | serde_jsonの2機能、モジュール依存3種、局所allow、unsafe forbid継承、Rust固定、CI権限と両ロックファイルのaudit設定、90%床・相対差0.01・固定シードを確認。全体CI・coverage・最新依存検査・性能は未実行。 |
| linter / type-check | 対象外 | 対象成果物にTypeScript/JavaScriptのソース片はなく、アプリ変更はRustコメントのみ。 |
| 計画本文のバイト境界 | 先頭17,450バイト不変 | 追記前SHA-256はa99ef337ed73ce4101bc9912f6d7549993dfee058cbf420bc39694ab4352df7d。本文を編集せず本Reviewだけを末尾に追加する。 |

### Summary

入力検証・互換範囲の説明が現在の設計と実装に揃い、今回の変更と再利用・歴史・未検証事項が区別されている。実行時の変更はなく、今回の限定範囲に新たな修正要求はない。
