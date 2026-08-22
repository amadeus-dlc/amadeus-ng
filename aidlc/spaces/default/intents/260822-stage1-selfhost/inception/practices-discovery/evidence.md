# evidence.md — 検査した証拠と推論（確定稿）

> 参加者別に追記。自動検査分（起草エージェント）+ 独立レビュー3件の新規実測 +
> オーナーインタビューの裁定を統合済み。

## 検査した証拠（自動検査分）

### git 履歴 / PR 履歴

- `git log --oneline -30`（実測、リポジトリルート = このワークスペースルート、
  `.git` は worktree gitlink）: 直近30コミットはすべて `main` への
  Merge pull request か、Merge 直前の単発コミット。フィーチャーブランチ名は
  `chore/*`・`feat/*`・`fix/*`・`refactor/*` の prefix 付き短命ブランチ。
- `gh pr list --state merged --limit 25 --json number,title,mergedAt`
  （実測）: PR #1〜#23（欠番あり）。マージ時刻の間隔から、PR は概ね逐次に
  近い順序でマージされている（例: #11〜#18 は 2026-08-21 20:44〜2026-08-22
  00:48 に連続、#19〜#23 は 2026-08-22 01:15〜03:21 に連続）。ただし完全な
  直列（オープン中は1本のみ）だったかは履歴だけからは断定できない
  ——**オーナー明言を第一級証拠として採用**。
- `git rev-parse --short HEAD` = `c4d8d95`（本ドラフト時点・統合時点とも
  同一。統合時点の実測 2026-08-22T05:03:53Z も同じ HEAD で確認済み）。

### `.github/workflows/ci.yml`（実測・全文読了）

- `check` ジョブ: `cargo fmt --all --check` → `cargo clippy --workspace
  --all-targets -- -D warnings` → `cargo lint` → `cargo test --workspace`
  の順で実行。全てブロッキング。
- `quint` ジョブ: `scripts/quint-gate.sh`（Quint 0.32.0、Node 22）。
- `coverage` ジョブ: `scripts/coverage.sh`（`cargo-llvm-cov`）。PR では
  `--base "origin/${{ github.base_ref }}"` を付けて絶対+相対ゲート、
  `workflow_dispatch` では絶対ゲートのみ。
- トリガーは `pull_request`（`main` 向け）+ `workflow_dispatch`。push トリガー
  は無い（PR ベースのゲートのみ。stage-1 スコープには含めない——インタビュー
  Q7 選択肢 D 不採択）。
- `pull_request_target` 等の特権コンテキストは使用していない。シークレットも
  一切参照していない（DevSecOps レビュー新規確認）。

### `Cargo.toml` / `clippy.toml` / `rustfmt.toml` / `.cargo/config.toml`（実測・全文読了）

- `[workspace.lints]` に rust/rustdoc/clippy の deny 一式が定義され、
  コメントに「2026-08-22 オーナー規約のリント一式」と明記——ハード制約の
  出所が人間裁定であることの直接証拠。内訳は **rust 4 + rustdoc 1 +
  clippy 42 = 計47ルール**（開発者レビュー実測。従来の「約50」を訂正）。
- `clippy.toml` はテストコードのみ `unwrap`/`expect` を許可。
- `rustfmt.toml`: `style_edition = "2024"`, `max_width = 100`,
  `newline_style = "Unix"`。
- `.cargo/config.toml`: `cargo lint` エイリアスが `tools/lint`
  （detached クレート）を `--manifest-path` 指定で実行する仕組み。

### `scripts/coverage.sh` / `scripts/quint-gate.sh`（実測・全文読了）

- `coverage.sh`: 絶対しきい値 `ABSOLUTE_THRESHOLD=90.0`、相対ゲートの許容誤差
  `TOLERANCE=0.5`（PBT のランダムケース起因の実測揺れ ±0.4pp に対する較正、
  コメントに実測値 94.87〜95.29% が明記。シード固定後 0.01 への引き締めが
  将来課題として同コメントに記載——インタビュー Q7 選択肢 B で stage-1
  スコープに確定）。除外設定は現状無い。
- `quint-gate.sh`: ADR 0003 決定4 に基づく4ステップ（typecheck / 不変条件run /
  到達性witness の反転判定 / 決定的シナリオ）。品質レビューが
  `quint-gate.sh` から不変条件 run 内訳を実数確認: engine_loop 9 +
  audit_lock 10 + stop_hook 8 = 27。witness は audit_lock 7 + stop_hook 5 =
  12。決定的シナリオは `quint test --match 'r_.*'`。

### RE 成果物（`aidlc/spaces/default/codekb/docs/`、読了分）

- `code-quality-assessment.md`: 品質保証が3層構造（Quint 形式検証 / ITF 準拠
  テスト / PBT+ゴールデンパリティ）であること、リント・CI が3段構え
  （rustfmt / clippy / cargo lint）であること、`tools/lint` が CI に未接続
  という既知の穴（C27）、デプロイパイプラインが「計画済み未着手」（欠落では
  ない）であることを確認。234テスト全緑・カバレッジ94.87〜95.29%の実測値。
- `architecture.md`: クリーンアーキテクチャ + Always Valid Domain Model を
  クレート境界で機械強制する単一 CLI バイナリ構成であること、開発進行が
  inside-out（仕様+Quint → ドメイン層TDD → ポート → Gateway の順）で
  アダプタ層まで完成・ユースケース本体/composition root/CLI が未着手である
  ことを確認。
- `code-structure.md`: インライン `#[cfg(test)]` ファイル数を「48」と記載
  していたが、開発者レビューが同一コミット `c4d8d95` で再実測した結果
  **40**（`modules/` 配下・`tests/` ディレクトリ除く）であり、`code-structure.md`
  側の数値が再現不能であると判明した。`tests/` 配下6本（ITF準拠2 + 統合4）
  を含めても46、`tools/lint/src/check.rs` を含めても47で、どの集計でも48
  にはならない。ITF準拠テスト2本・統合テスト4本の内訳自体は一致。
  `tools/lint` が意図的に workspace 非メンバーである理由（coverage/test
  対象から外すため）も確認。
- `technology-stack.md`: 「`#![forbid(unsafe_code)]` は infra-io を含め
  維持されている」との記述があるが、DevSecOps レビューの実地確認により
  `modules/app/aidlc/src/main.rs`（スタブ）には attribute が無いことが
  判明し、厳密には過大な記述であった。インタビュー Q6 選択肢 C の workspace
  lints 昇格採択により、この記述と実態のずれは stage-1 で解消される。

### 設計監査 `aidlc/spaces/default/knowledge/aidlc-shared/design-audit-2026-08-22.md`（実測・全文読了）

- オーナー全29スキルによる監査5エージェント→33主張→検証12エージェントが
  `main@c4d8d95` に全数照合、29 CONFIRMED / 4 棄却。確定裁定 R1〜R5、
  修正束 A〜E に分類済み。intent 260822-stage1-selfhost の INCEPTION 各
  ステージ（reverse-engineering / domain-design / delivery-planning）の
  入力であることが明記されている。practices-discovery の直接の入力では
  ないが、Testing Posture の「三層品質保証」の裏付けとして参照した。

### コーディング規則正本 `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（README.md 読了、全ファイル確認）

- **6規則 + README**（`domain-equality.md` / `field-visibility.md` /
  `gateway-taxonomy.md` / `module-visibility.md` / `tell-dont-ask.md` /
  `use-case-rules.md` + `README.md`。開発者レビュー指摘により「7ファイル」
  という曖昧な表現を「6規則 + README」へ訂正——規則追加時に数字が乖離
  しないよう区別する）。1ルール1ファイル。裁定日・適用PR・機械強制の有無を
  各ファイルに記す運用。
- `gateway-taxonomy.md` ルール3の禁止語彙リストは Store / Reader / Writer /
  **Source / Provider** の5語であり、team-practices.md 旧稿の部分列挙
  （Store/Reader/Writer のみ）は既に乖離していた（開発者レビュー指摘）。
  確定稿ではこの部分複製をやめ、正本ファイル名の参照に置き換えた。
- 機械化優先順位（型→既存lint→cargo lintカスタムルール）と、カスタムルールの
  赤例テスト必須（Quint ゲートと同じDoD）という運用ルールを確認。
- エラーハンドリング様式（手実装エラー enum、thiserror/anyhow 不使用、
  `fmt::Display` 手実装7ファイル）の裁定ファイルは現状無い。インタビュー
  Q8 選択肢 A により正本への追加が確定した（下記「確定アクション」参照）。

### 新規実測（独立レビューによる追加検査）

- `gh api repos/amadeus-dlc/amadeus-ng/branches/main/protection` →
  **404 "Branch not protected"**、`gh api repos/amadeus-dlc/amadeus-ng/
  rules/branches/main` → **`[]`**（2026-08-22、品質レビュー実測）。CI 全緑
  マージは branch protection / ruleset による機械強制ではなく運用規律で
  あった。インタビュー Q4 で機械強制（required status checks: check /
  quint / coverage）の設定が確定した。
- `Cargo.lock`（メイン workspace）実測: ロック済みパッケージ全60個
  （直接外部依存 serde / serde_json / md5 / nix / libc + dev の proptest /
  tempfile）。`tools/lint` は独立 `Cargo.lock`（5パッケージ）。両方とも
  コミット済み（DevSecOps レビュー実測）。
- `grep` による `unsafe` 使用ゼロ確認、`#![forbid(unsafe_code)]` を
  workspace 9クレートの lib.rs + `tools/lint` main.rs で実地確認
  （`modules/app/aidlc/src/main.rs` のみ欠落、上記のとおり）。
- `rust-toolchain.toml` 不在の実地確認。CI は `dtolnay/rust-toolchain@stable`
  （floating）。GitHub Actions はタグ参照（`@v4`/`@v2`）で SHA ピン留めなし
  （SHA ピン留めは今回 mandate せず、任意事項として見送り）。
- `modules/core/use-case/Cargo.toml` の依存が `core-domain` /
  `audit-events` / `directive-schema` のみで `core-interface-adapter` が
  無いことを確認（開発者レビュー実測）——use-case-rules.md の「実装依存は
  E0432 で物理的に不可能」という機械強制が現物で成立している。
- `cargo test --workspace` 実行、passed 合計 **234** を確認（開発者レビュー
  実測。草案の「234テスト全緑」と一致）。

## 推論（証拠から導いた判断、確定ではない）

- Walking Skeleton を `skeleton: off` とした根拠は、architecture.md の
  「アダプタ層まで完成」という記述と、クリティカルパス最終段（doctor →
  ドッグフード）が縦串疎通を代替する、という推論に基づく。この判断は
  インタビュー Q1（選択肢 A）でオーナーが確認済み。
- TDD サイクルと ITF 準拠テスト／ゴールデンパリティの関係（TDD の外側の
  ゲートと位置づける）は、両者の実行タイミング（前者は実装と同時、後者は
  Quint モデル駆動の別経路）からの推論。インタビュー Q2（選択肢 A）で
  オーナーが確認済み。

## 確定アクション（統合後・後続 Bolt で実施する設定変更）

practices-discovery 自体は設定変更を行わない（発見・裁定ステージ）。以下は
インタビュー裁定に基づき、後続の delivery-planning / construction で
Bolt 化して実施する具体アクションの一覧である。

1. **branch protection 設定**（インタビュー Q4）: `main` に required status
   checks として `check` / `quint` / `coverage` の3ジョブを設定する。
2. **カバレッジ除外設定**（インタビュー Q5）: `scripts/coverage.sh` に
   composition root（`main.rs` の配線部分）のみを除外する設定を追加する。
   それ以外のコードは90%床を維持する。
3. **サプライチェーン/ハードニング**（インタビュー Q6、A/B/C/D 全採用）:
   - `cargo audit` を CI に追加（`tools/lint` の独立 `Cargo.lock` も対象）。
   - `rust-toolchain.toml` を新規作成しツールチェーンを固定する。
   - `unsafe_code = "forbid"` を `[workspace.lints.rust]` へ昇格する。
   - `.github/workflows/ci.yml` に `permissions: contents: read` を追加する。
4. **CI/リンタ整備**（インタビュー Q7、A/B 採用）:
   - `tools/lint` 用の CI ステップ（fmt / clippy / 自己テストの3ステップ）
     を `ci.yml` に追加する（設計監査 C27 の解消）。
   - PBT のシードを固定し、`scripts/coverage.sh` の相対ゲート許容誤差を
     `TOLERANCE=0.5` → `0.01` へ引き締める。
   - （不採択・後続 intent へ）macOS CI ジョブ追加、`main` への push
     トリガー追加。
5. **エラーハンドリング規則の正本追加**（インタビュー Q8）: 現行の実態
   （手実装エラー enum + `fmt::Display` 手実装、thiserror/anyhow 不使用）を
   coding-rules 正本へ1ファイルとして追加する。以下はそのドラフト文面
   （後続 Bolt でオーナー確認のうえ `coding-rules/error-handling.md` 相当
   として正式追加する）:

   > **ルール**: ドメイン層・ユースケース層の失敗はモジュールごとの手実装
   > エラー enum で表現する。`thiserror` / `anyhow` 等のエラーハンドリング
   > 外部クレートには依存しない。各エラー enum は `std::fmt::Display` を
   > 手実装し、利用者向けの説明文を持つ。`missing_errors_doc` clippy lint
   > （deny）に従い、fallible な公開関数には `# Errors` セクションを
   > 付与する。
   >
   > **根拠**: 依存最小化（`Cargo.lock` 60パッケージという極小ツリーの
   > 維持）と、エラー型をドメイン語彙に閉じ込める設計方針（Always Valid
   > Domain Model との一貫性）。
   >
   > **機械強制**: 現状は `missing_errors_doc` deny lint のみ。
   > `thiserror`/`anyhow` の使用禁止自体を機械強制する `cargo lint`
   > カスタムルールは未実装（将来検討、赤例テスト必須の DoD に従う）。

## インタビューで確認（全項目、裁定済み）

1. **Walking Skeleton の裁定**: → **A. 作らない（skeleton: off）**。
   Bolt 1 も通常 Bolt。縦串はクリティカルパス項目6（doctor→ドッグフード）
   で自然に通る。
2. **Testing Posture の Ordering 文言**: → **A. 品質レビュー置換案で確定**
   （Methodology: tdd）。
3. **テストピラミッドの比率定量化**: → **A. 定性のみ**（比率は縛らない）。
4. **main のマージゲート機械強制**: → **A. branch protection
   （required checks: check / quint / coverage）を設定**。
5. **カバレッジ90%床と未テスト層**: → **B. composition root
   （`main.rs` の配線部分）のみカバレッジ除外を許可**、それ以外は床維持。
6. **サプライチェーン整備**: → **A, B, C, D 全採用**（`cargo audit` CI
   追加 / `rust-toolchain.toml` 固定 / `unsafe_code = "forbid"` の workspace
   昇格 / `permissions: contents: read` 明示）。
7. **stage-1 スコープに含める CI/リンタ整備**: → **A, B**（`tools/lint` へ
   の CI 3ステップ追加 / PBT シード固定で相対ゲート 0.5pp → 0.01）。
   macOS CI ジョブ・push トリガーは stage-1 に含めず後続 intent へ。
8. **エラーハンドリング様式の正本追加**: → **A. 追加する**（文面は上記
   「確定アクション」5 に起草済み、後続 Bolt でオーナー確認のうえ正式追加）。

すべての項目が `practices-discovery-questions.md` で回答済み、かつ
Consolidated Summary Confirmation で "Looks correct" が確認されている。
