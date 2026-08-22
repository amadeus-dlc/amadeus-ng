# discovered-rules.md — 発見された確定ルール（確定稿）

> 人間が明言したハード制約、および既に機械強制されている規約のうち
> 人間裁定に由来するもののみを記す。推測による規則化は行わない。
> `team.md` / `project.md` への昇格は practices-discovery の統合ステップで行う。
> 独立レビュー3件とオーナーインタビュー（全8問回答済み）を統合済み。

## Mandated

- ALWAYS テストは t_wada 提唱の red-green-refactor（TDD）で書く。新規
  プロダクションコードはレイヤーごとに red-green-refactor（失敗するテストを
  先に書く）で実装する。Quint モデル検査・ITF 準拠テスト・ゴールデンパリティ
  は TDD サイクルの外側の受け入れゲートとして維持し、TDD の red を代替
  しない。テストピラミッド（ユニット層を厚く、結合・E2E層を薄く）を意識
  した配分（定性のみ、比率は定めない）にする（オーナー明言 2026-08-22、
  インタビュー Q1〜Q3 で確定）。
- ALWAYS PR は Bolt 単位で出す。Bolt ブランチは `main` へ squash-merge し、
  コミット名は Bolt slug とする。PR は直列運用とし、オープンな PR は常に
  一度に1本のみとする（オーナー明言 2026-08-22）。
- ALWAYS GitHub Issue をそのまま intent とする（1 Issue = 1 intent）。
  Issue のスコープを縮めない（オーナー明言 2026-08-22）。
- ALWAYS コード・仕様・レビューを書く前に、コーディング規則の正本
  `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（オーナー裁定、
  1ルール1ファイル、インデックスは同ディレクトリの README.md）を読んで
  従う。規則はレビューと `cargo lint` で強制される
  （project.md ## Mandated に既に登録済み、affirmed 2026-08-22）。
- ALWAYS 会話および人間可読成果物は日本語で書く（コード識別子・固定トークンは
  英語のまま）（オーナー明言 2026-08-22、org.md/project.md 既定の適用）。
- ALWAYS マージ前に CI 3ジョブを全緑にする — check（`cargo fmt --all --check`
  → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo lint`
  → `cargo test --workspace`）、quint（`scripts/quint-gate.sh`）、coverage
  （`scripts/coverage.sh`、絶対90%床 + PR 相対ゲート）（`.github/workflows/
  ci.yml` 実測）。**この3ジョブは branch protection の required status
  checks として機械強制する**（インタビュー Q4、選択肢 A——`gh api` 実測で
  `main` に branch protection / ruleset が未設定であることが判明したため、
  従来「ブロッキングゲートとして実行する」としていた文言を、実態（CI は
  走るが赤でもマージ可能）に合わせて修正し、機械強制の設定自体をオーナー
  裁定として確定した。設定作業は `evidence.md` の確定アクションを参照）。
- ALWAYS プロダクトコードでは `unwrap`/`expect` を使わない。テストコードのみ
  `clippy.toml`（`allow-unwrap-in-tests` / `allow-expect-in-tests`）で許容する
  （`Cargo.toml` workspace lints、オーナー規約）。
- ALWAYS 新規カスタム `cargo lint` ルールには検出力を証明する赤例テストを
  添える（Quint ゲートと同じ Definition of Done。coding-rules/README.md
  に明記、オーナー裁定）。
- ALWAYS `unsafe_code = "forbid"` を `[workspace.lints.rust]` として
  workspace 全体に適用する（従来はクレート個別 attribute のみで app スタブ
  に漏れがあった。インタビュー Q6、選択肢 C で workspace lints への昇格を
  確定）。
- ALWAYS `.github/workflows/ci.yml` に `permissions: contents: read` を
  明示する（least privilege。インタビュー Q6、選択肢 D で確定）。
- ALWAYS 依存追加・更新時は `cargo audit`（RustSec advisory DB）を CI で
  実行する。対象には `tools/lint` の独立 `Cargo.lock` も含める
  （インタビュー Q6、選択肢 A で確定）。
- ALWAYS ツールチェーンバージョンは `rust-toolchain.toml` で固定する
  （floating stable による CI 突然赤リスクの解消。インタビュー Q6、
  選択肢 B で確定）。

## Forbidden

- NEVER 複数の PR を同時にオープンにしない（PR は直列運用、オーナー明言
  2026-08-22。新規発見——実測の PR 履歴だけでは直列を断定できないが
  オーナー明言を第一級証拠として採用した。org.md 既定の trunk-based /
  squash-merge 一般則の再掲は当セクションに含めない——それらは org 層で
  既にロードされ機械強制の裏取りもないため、二重記載を避ける）。
- NEVER フィールドを既定で公開にしない（デフォルト private、公開はアクセサ
  経由。`cargo lint` no-public-fields ルールで機械強制、正本は
  `coding-rules/field-visibility.md`）。
- NEVER モジュールを既定で公開にしない（デフォルト private、公開は
  ファサードの `pub use` 経由。現状は既存の `unreachable_pub` deny lint
  （私有 mod 化により実効化）で機械強制されており、`cargo lint` への
  ルール化は未実施・予定である——開発者レビュー指摘により、
  no-public-fields（フィールド専用）とは別の強制手段として書き分けた。
  正本は `coding-rules/module-visibility.md`）。

## Corrections

（このステージでは未記入。）
