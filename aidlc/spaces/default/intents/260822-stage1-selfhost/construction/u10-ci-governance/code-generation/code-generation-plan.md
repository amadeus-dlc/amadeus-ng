# code-generation-plan — U10 CI ガバナンス（`u10-ci-governance`）

> Code Generation（Construction 3.5）の計画（Unit: U10、kind: packaging、Bolt: B2、規模 M）。出典:
> `../nfr-requirements/security-requirements.md`（NFR2.1〜2.5 / NFR4.1〜4.5）、`../nfr-requirements/tech-stack-decisions.md`、
> `../nfr-design/security-design.md`（CI 4 ジョブ・ruleset 手順・ワークスペース設定・障害ドメイン、レビュー Minor 2 件）、
> `../../../inception/requirements-analysis/requirements.md`（FR9.1〜9.5、NFR2 / NFR4）、`../../../inception/units-generation/
> unit-of-work.md`（U10 の責務・境界・合格）、`../../../inception/contract-design/contract-summary.md`（U10 は契約面を
> 持たない）、`../../../inception/delivery-planning/bolt-plan.md`（B2 = U10、2026-08-23 改訂）、`aidlc/spaces/default/knowledge/
> aidlc-shared/coding-rules/`、`code-generation-questions.md`（Q1、Plan Approval）。
>
> 実装はワークスペースルート（`.github/workflows/ci.yml`、`Cargo.toml`、`tools/lint/Cargo.toml`、`rust-toolchain.toml`、
> `scripts/coverage.sh`、`scripts/governance/`）に書く。記録ディレクトリにはコードを置かない。**プロダクトコードは触らない**
> （`unsafe_code` 昇格で赤になるクレートがあれば U7 — 現状 unsafe 使用ゼロ）。

## 1. 前提と範囲

- **作るもの**: FR9.2（toolchain 固定・`unsafe_code` forbid 昇格・`permissions`・`cargo audit`）→ FR9.3（`tools/lint` の CI 3 ステップ）
  → FR9.4（PBT シード固定・`TOLERANCE` 0.01）→ FR9.5（カバレッジ除外）→ FR9.1（ruleset に required checks + `merge_group`
  トリガ）の順（story map の U10 実装順）。加えて検証スクリプト `scripts/governance/verify-ci-governance.sh`（packaging の
  「テスト」— 設定の事実を機械検査する）。
- **作らないもの**: FR9.6（エラーハンドリング規則の正本化 — U9）、Dependabot / SHA ピン留め（後続 intent）、`audit` の required 化
  （運用後に再判断）、macOS CI / push トリガ（後続 intent）。
- **ブランチ（Q1）**: PR は直列運用（team.md）。B2 のブランチ `bolt/b2-u10-ci-governance` は **PR #24（B1）が `main` に
  squash-merge された後に `main` から切る**（Q1 = A）。#24 マージ前は計画承認まで進め、実装の委任はマージ後に行う。
- **GitHub 設定の実行**: ruleset 変更はオーナー権限が要る。開発エージェントは手順スクリプトを書き `--dry-run` と `jq` 検証まで行い、
  **実行はオーナー**（または `gh auth` がオーナー権限ならコンダクタが実行を提案して承認後に実行）。実行結果（前後 JSON）は
  `<record>/construction/u10-ci-governance/code-generation/ruleset/` に保存する。
- **受入**（unit-of-work U10）: CI 3 ジョブ緑、`cargo audit` clean、`gh api` で required checks が確認できる。正常系: 緑 PR が
  merge queue を通って squash-merge 完走（本 Bolt の PR 自身で確認）。

## 2. 変更の一覧（設計の写し — 実装の契約）

| 対象 | 変更 | 要求 |
|---|---|---|
| `rust-toolchain.toml`（新規） | `[toolchain] channel = "1.95.0"`、`components = ["rustfmt", "clippy", "llvm-tools"]`、`profile = "minimal"` | NFR4.2 |
| `Cargo.toml` | `[workspace.lints.rust]` に `unsafe_code = "forbid"` | NFR4.3 |
| `tools/lint/Cargo.toml` | `[lints.rust]` に `unsafe_code = "forbid"` | NFR4.3 |
| `.github/workflows/ci.yml` | `on:` に `merge_group: {}`; 直下に `permissions: contents: read`; toolchain を `dtolnay/rust-toolchain@master`（`components:` 撤去、`rust-toolchain.toml` 駆動）; `check` に `tools/lint` の fmt / clippy / test 3 ステップ; `coverage` は `pull_request` 時のみ `--base`、それ以外は絶対; 新規 `audit` ジョブ（`taiki-e/install-action@v2` `tool: cargo-audit` → `cargo audit` → `cargo audit --file tools/lint/Cargo.lock`） | NFR2.2 / 2.3 / 4.1 / 4.2 / 4.4 |
| `scripts/coverage.sh` | `cargo llvm-cov` に `--ignore-filename-regex '^modules/app/aidlc/src/main\.rs$'`（相対パス基準 — tech-stack-decisions の表記を正本とする、nfr-design レビュー Minor 1）; `TOLERANCE=0.01`; `PROPTEST_RNG_SEED`（固定値、例 `20260823`）を計測時に export; コメントの較正根拠を更新 | NFR2.4 / 2.5 |
| `.github/workflows/ci.yml`（`check` / `coverage`） | `env: PROPTEST_RNG_SEED: "20260823"` を `cargo test` / coverage 実行に適用（ローカルと CI で同じシード） | NFR2.4 |
| `scripts/governance/ruleset-required-checks.sh`（新規） | `gh api` で ruleset「main」の前 JSON 取得 → `required_status_checks`（check / quint / coverage、strict）が**期待どおりのコンテキスト集合で**無ければ追加 / 補正した JSON を `PUT`（既存規則・`bypass_actors` 維持）→ 後 JSON 取得 → `jq` 検証。`--dry-run`（PUT しない）と `--out-dir <dir>`（前後 JSON の保存先）。冪等判定はコンテキスト集合の一致（nfr-design レビュー Minor 2） | NFR2.1 / 4.5 |
| `scripts/governance/verify-ci-governance.sh`（新規） | packaging の「テスト」: (1) `rust-toolchain.toml` の channel / components、(2) `Cargo.toml` と `tools/lint/Cargo.toml` の `unsafe_code = "forbid"`、(3) `ci.yml` の `merge_group` / `permissions` / `audit` ジョブ / `tools/lint` 3 ステップ / `@master` toolchain、(4) `scripts/coverage.sh` の `TOLERANCE=0.01` / 除外 regex / `PROPTEST_RNG_SEED`、(5) `--with-ruleset` 指定時のみ `gh api` で required checks の存在（ネットワーク要）を検査し、失敗項目を列挙して非 0 終了 | 全要求の機械検査 |

## 3. テスト戦略（Testing Contract: tdd / standard の packaging への適用）

packaging Unit にはプロダクトコードの「層」が無い。Testing Contract の `plan_profile` を次のように写す（方法論は変えない）:

- **ランナー**: bash（`scripts/governance/verify-ci-governance.sh`）+ `cargo test --manifest-path tools/lint/Cargo.toml`（既存 31 本）+
  `bash -n` 構文検査。最初の Red の前にランナーが走ることを確認（brownfield: `bash -n scripts/coverage.sh` は現時点で exit 0）。
- **Red**: `verify-ci-governance.sh` を先に書き、現在のツリーに対して実行して**失敗項目一覧**（toolchain ファイル無し・
  `unsafe_code` 未昇格・`merge_group` 無し・`permissions` 無し・`audit` 無し・`tools/lint` ステップ無し・`TOLERANCE=0.5`・除外無し・
  `PROPTEST_RNG_SEED` 無し）を記録する。
- **Green**: 変更を順に入れ、失敗項目が 1 つずつ消えることを記録。全項目 PASS で Green。
- **Refactor**: スクリプトの重複整理、メッセージの日本語化、`shellcheck`（導入済みなら）。
- **受入（Bolt の外側のゲート）**: CI 3 ジョブ + `audit` の実行結果、`scripts/coverage.sh` 2 回実行で差 0.00pp（NFR2.4）、
  ruleset 変更後の正常系 PR 完走。
- Standard 戦略の「コンポーネントごと 5〜8 本」は、`verify-ci-governance.sh` の検査項目（対象ファイルごとに 2〜5 項目、
  合計 15 項目以上）と `tools/lint` 既存テスト 31 本で満たす。

## 4. 実装ステップ（番号順）

### 4.0 コンダクタ（承認後・委任前）

- [ ] Step 0. PR #24 のマージを確認 → `git switch main && git pull` → `bun .claude/tools/aidlc-bolt.ts start --name B2 --batch 1` →
      `git switch -c bolt/b2-u10-ci-governance`。

### 4.1 骨格とランナー（開発エージェント）

- [ ] Step 1. `scripts/governance/` を作り、`verify-ci-governance.sh` の骨格（検査関数の枠、`--with-ruleset` オプション、
      結果一覧と終了コード）を書く。`bash -n` で構文確認。`cargo test --manifest-path tools/lint/Cargo.toml` が走ることを確認
      （既存 31 本緑）。
- [ ] Step 2. **Red**: `verify-ci-governance.sh` の検査項目（§2 の (1)〜(4)）を実装し、現在のツリーで実行 → 失敗項目一覧を記録
      （期待: toolchain / unsafe / merge_group / permissions / audit / tools-lint-steps / tolerance / ignore-regex / proptest-seed
      の 9 項目が FAIL）。

### 4.2 FR9.2 サプライチェーン（Green 1）

- [ ] Step 3. `rust-toolchain.toml` 新規、`Cargo.toml` と `tools/lint/Cargo.toml` に `unsafe_code = "forbid"`、`ci.yml` に
      `permissions: contents: read` と toolchain の `@master` 化、`audit` ジョブ追加。`cargo build --workspace` と
      `cargo clippy --workspace --all-targets -- -D warnings` が緑（`tools/lint` も `--manifest-path` で確認）。検査を再実行し
      該当 5 項目が PASS に変わることを記録。

### 4.3 FR9.3 tools/lint の CI（Green 2）

- [ ] Step 4. `check` ジョブに `tools/lint` の fmt / clippy / test 3 ステップを追加。ローカルで同じ 3 コマンドを実行して緑。
      検査 1 項目 PASS。

### 4.4 FR9.4 / FR9.5 カバレッジ（Green 3）

- [ ] Step 5. `scripts/coverage.sh`: `PROPTEST_RNG_SEED` の export、`--ignore-filename-regex`、`TOLERANCE=0.01`、コメント更新。
      `ci.yml` の `check` / `coverage` に `PROPTEST_RNG_SEED` env。ローカルで `bash scripts/coverage.sh` を 2 回実行し `head`
      値が一致（差 0.00pp）することを記録（NFR2.4 の受入）。検査 3 項目 PASS。

### 4.5 FR9.1 ruleset と merge queue（Green 4）

- [ ] Step 6. `ci.yml` に `merge_group: {}` と coverage の条件分岐（`pull_request` のみ `--base`）。検査 1 項目 PASS。
- [ ] Step 7. `scripts/governance/ruleset-required-checks.sh` を書く（`--dry-run` / `--out-dir`、コンテキスト集合で冪等判定、
      前後 JSON 保存、`jq` 検証）。`--dry-run` で組み立て JSON を確認（PUT はしない）。`verify-ci-governance.sh --with-ruleset` は
      この時点では FAIL のまま（実行前）— 記録。
- [ ] Step 8. **Refactor**: スクリプト整理、`bash -n`、`shellcheck`（あれば）。全検査（ruleset 以外）PASS を再確認。

### 4.6 品質ゲートとコミット

- [ ] Step 9. `cargo fmt --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo lint` → `cargo test --workspace`
      → `tools/lint` 3 コマンド → `bash scripts/coverage.sh`（絶対ゲート）。意味単位でコミット（`chore(ci): …` / `chore(toolchain): …` /
      `chore(coverage): …` / `chore(governance): …`）。push / PR はしない。

### 4.7 コンダクタ / オーナー（委任後）

- [ ] Step 10. ruleset 変更の実行: `scripts/governance/ruleset-required-checks.sh --out-dir <record>/construction/u10-ci-governance/
      code-generation/ruleset/` をオーナー権限で実行（コンダクタが `gh auth status` でオーナー権限を確認できれば承認後に実行）。
      `verify-ci-governance.sh --with-ruleset` PASS を記録。
- [ ] Step 11. Bolt ゲート → PR 作成（`bolt/b2-u10-ci-governance` → `main`）→ PR の CI で `merge_group` を含む 4 ジョブの実行を確認 →
      オーナーが merge queue でマージ（正常系の受入）。

## 5. トレーサビリティ（要求 → ステップ）

| 要求 | ステップ | 主な成果物 |
|---|---|---|
| FR9.1 / NFR2.1 / NFR4.5 | 6, 7, 10, 11 | `scripts/governance/ruleset-required-checks.sh`、`ci.yml`（merge_group）、ruleset 前後 JSON |
| FR9.2 / NFR4.1 / NFR4.2 / NFR4.3 / NFR4.4 | 3 | `rust-toolchain.toml`、`Cargo.toml`、`tools/lint/Cargo.toml`、`ci.yml`（permissions / audit / toolchain） |
| FR9.3 / NFR2.3 | 4 | `ci.yml`（check ジョブ） |
| FR9.4 / NFR2.4 | 5 | `scripts/coverage.sh`（TOLERANCE / PROPTEST_RNG_SEED）、`ci.yml`（env） |
| FR9.5 / NFR2.5 | 5 | `scripts/coverage.sh`（ignore regex） |
| NFR2.2 | 6 | `ci.yml`（merge_group + coverage 分岐） |
| 全要求の機械検査 | 1, 2, 8 | `scripts/governance/verify-ci-governance.sh` |

## 6. 委任の形

- 1 回の委任（Step 1〜9）を aidlc-developer-agent へ。冒頭行 `AIDLC-UNIT: u10-ci-governance` と `AIDLC-TESTING-CONTRACT: <contract_sha256>`。
  委任は PR #24 のマージ後（Q1 = A）。計画・質問票・本ファイルのバイト列は承認後に変更しない（指紋）。
- ruleset の実行（Step 10）はオーナー権限の操作のため委任しない。

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

