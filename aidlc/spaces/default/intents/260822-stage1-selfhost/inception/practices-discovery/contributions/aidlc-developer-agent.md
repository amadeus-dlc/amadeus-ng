**Collaborator:** aidlc-developer-agent

## Contribution

開発者観点からの独立検査。リード草案の記述を一次証拠（coding-rules 正本7ファイル全読、`Cargo.toml` / `clippy.toml` / `rustfmt.toml` / `.cargo/config.toml` / `.github/workflows/ci.yml` / `scripts/coverage.sh` 実測、`tools/lint` ソース、実ドメインコード `modules/core/domain/src/` サンプル、`cargo test --workspace` 実行）と突き合わせた。検査コミットは草案と同一の `c4d8d95`。

### 裏取りできた記述（コマンド実測で一致）

- **リンタ3段構えと CI 順序**: `ci.yml` の `check` ジョブは `cargo fmt --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo lint` → `cargo test --workspace` の順で、全ステップがブロッキング。草案の記述と完全一致。
- **workspace lints の人間裁定由来**: `Cargo.toml` に「2026-08-22 オーナー規約のリント一式」コメントが実在。deny の内訳は rust 4 + rustdoc 1 + clippy 42 = **計47ルール**（後述の精度指摘あり）。草案が例示する `unwrap_used` / `expect_used` / `missing_docs` / `unreachable_pub` / `todo` / `unimplemented` / `print_stdout` / `dbg_macro` / `needless_pass_by_value` はすべて実在。
- **テスト限定の unwrap/expect 許可**: `clippy.toml` は `allow-unwrap-in-tests` / `allow-expect-in-tests` のみ。プロダクトコード deny との書き分けは草案どおり。
- **rustfmt 設定**: `style_edition = "2024"` / `max_width = 100` / `newline_style = "Unix"` — 一致。
- **`cargo lint` の実装実態**: `.cargo/config.toml` の alias が `tools/lint`（workspace 非メンバー detached クレート）を `--manifest-path` 起動。実装済みルールは `checkbox-vocabulary` / `reap-decision-locality` / `no-public-fields` の**3本**（`check.rs` の定数で確認）、`#[test]` は**31本** — 草案の「ルール3本・赤例テスト31本」と一致。
- **層境界のクレート強制**: `modules/core/use-case/Cargo.toml` の依存は `core-domain` / `audit-events` / `directive-schema` のみで `core-interface-adapter` が無い — use-case-rules.md の「実装依存は E0432 で物理的に不可能」という機械強制が現物で成立している。
- **実コードの正本準拠**: `workflow_definition/mod.rs` は private mod + ファサード `pub use` 列挙（module-visibility.md どおり、doc コメントで正本を明示参照）。`stage_slug.rs` は Always Valid（private ctor + `parse`）、`as_str` アクセサ（`get_` 接頭辞なし）、PBT 同居 — field-visibility.md / code-structure.md のパターン記述と一貫。ドメイン層に無制限 `pub` フィールドは grep で検出ゼロ。
- **エラーハンドリング様式（草案の抑制が正しい）**: 実測では thiserror / anyhow への依存ゼロ、ドメイン層はモジュールごとの手実装エラー enum + `fmt::Display` 手実装（7ファイル）+ `missing_errors_doc` deny という様式が観察できるが、coding-rules 正本にエラーハンドリングの裁定ファイルは無い。草案がこれを**規則化していない**のは「推測の規則化を行わない」宣言に忠実であり正しい。将来オーナー裁定を得たら正本へ1ファイル追加すべき候補として記録に値する。
- **テスト総数**: `cargo test --workspace` を実行し passed 合計 **234** を確認 — 草案の「234テスト全緑」と一致。
- **カバレッジゲート**: `coverage.sh` の `ABSOLUTE_THRESHOLD=90.0` / `TOLERANCE=0.5` — 一致。
- **discovered-rules の出所監査**: 「ALWAYS 新規カスタム `cargo lint` ルールには赤例テスト」は README.md に明記（オーナー裁定文書）、「ALWAYS プロダクトコードで unwrap/expect 禁止」は `Cargo.toml` + `clippy.toml` の現物、「ALWAYS リンタ違反はマージ前に解消」は `ci.yml` 現物に由来し、いずれも人間裁定・機械強制由来という宣言に適合。推測の規則化は検出しなかった（ただし org.md 既定の再掲問題を Positions で指摘）。

### 発見した差異・修正提案

1. **インラインテストファイル数「実測48」は再現不能**（team-practices.md Testing Posture、出所は codekb `code-structure.md`）。同一コミット `c4d8d95` での実測は、インライン `#[cfg(test)]`（`modules/` 配下、`tests/` ディレクトリ除く）= **40**。`tests/` 配下6本（ITF 2 + 統合4）を足して46、`tools/lint/src/check.rs` を足しても47で、どの集計でも48にならない。「実測」を名乗る数値は集計方法込みで40に訂正するか、集計範囲を明記すべき。codekb 側にも同じ誤りが伝播している。
2. **可視性 NEVER ルールの強制手段の誤帰属**（discovered-rules.md）。「NEVER フィールド／モジュールを既定で公開にしない（`cargo lint` no-public-fields ルールで機械強制）」とあるが、`no-public-fields` が検出するのは**フィールドのみ**。モジュール既定 private の機械強制は正本 module-visibility.md のとおり「既存 `unreachable_pub`（deny、私有 mod 化で実効化）+ `cargo lint` ルール化**予定**」であり、書き分けが必要。昇格後の `team.md` に誤った強制根拠が固定化するのを防ぐため統合ステップで修正すべき。
3. **正本の部分複製による乖離の実例**（team-practices.md ## Code Style 命名規則）。「`Store`/`Reader`/`Writer` 造語…を禁止」は正本 gateway-taxonomy.md ルール3の禁止リスト（Store / Reader / Writer / **Source / Provider**）の部分列挙で、既に乖離が発生している。「等」でヘッジされてはいるが、これは brief の言う「規則本文の複製は正本と乖離する」の実証例。規則内容の列挙をやめ、ルールファイル名（`gateway-taxonomy.md` / `field-visibility.md` / `module-visibility.md` / `domain-equality.md`）の参照に置き換えることを推奨。
4. **org.md 既定の再掲エントリ**（discovered-rules.md ## Forbidden）。「NEVER 長命ブランチ」「NEVER Bolt の中間コミットを `main` に残さない」は org.md 既定の再掲であり、ヘッダの「人間が明言したハード制約、および機械強制されている規約のうち人間裁定に由来するもののみ」という自己宣言と整合しない。ルール解決チェーン（org → team → project）は org 層を常時ロードするため、昇格すると同一規則の二重記載となり将来の乖離リスクになる。オーナー明言 #2 由来の「PR 直列運用」だけが新規発見であり、org 既定分は「実績で裏付け済み・昇格不要」と明示するか削除すべき。
5. **「clippy 約50ルール deny」の精度**(discovered-rules.md)。実測は clippy 42 / rust 4 / rustdoc 1 = 計47。team-practices.md の「workspace lints 約50」は概算として許容だが、discovered-rules は昇格候補の規則文面なので「workspace lints 計47（rust 4 + rustdoc 1 + clippy 42）」と正確に書くべき。
6. **軽微**: 「正本…の7ファイル」は README（インデックス）込みの数。「6規則 + README」と書く方が、規則追加時に数字が乖離しにくい。

## Positions

- AGREE: ## Code Style の構成（正本パスを名指しして参照し、規則の詳細は正本に委ねる形）— DRY の方向性として正しく、フォーマッタ・リンタ3段構え・機械化優先順（型 → 既存 lint → `cargo lint`）の記述はすべて実測・正本と一致する
- AGREE: Testing Posture の TDD（red-green-refactor）と三層品質保証（Quint / ITF / ゴールデンパリティ）を区別し、ITF・ゴールデンを TDD サイクルの外側のゲートと位置づけた整理 — 実測のテスト配置（インライン + `domain/tests/` 2本 + `interface-adapter/tests/` 4本）と実行経路の実態に合致し、かつ推論であることを evidence.md に明示している
- AGREE: エラーハンドリング様式（手実装エラー enum、thiserror/anyhow 不使用）を規則化しなかった判断 — 正本に裁定ファイルが無い以上、規則化は「推測の規則化」になる。抑制が正しい
- AGREE: `tools/lint` の CI 未接続（C27）をスコープ注記としてインタビュー確認に回した判断 — 実測（`ci.yml` の fmt/clippy/test が detached クレートに届かない）と一致し、勝手に完了条件へ格上げしていない
- OBJECT: team-practices.md の「インライン `#[cfg(test)]` 実測48ファイル」— 同一コミット `c4d8d95` での再現値は40（最大集計でも47）。「実測」表記の数値が再現不能なままの昇格は証拠基準を毀損する。40への訂正または集計範囲の明記が必要
- OBJECT: discovered-rules.md の「NEVER フィールド／モジュール既定公開（`cargo lint` no-public-fields で機械強制）」の強制手段記述 — `no-public-fields` はフィールド専用で、モジュールは `unreachable_pub` + ルール化予定。正本どおりの書き分けに修正しないと、誤った強制根拠が `team.md` に固定化する
- OBJECT: discovered-rules.md ## Forbidden の org.md 既定再掲（長命ブランチ禁止・中間コミット禁止）— ヘッダの収載基準（人間明言・機械強制由来のみ）に反し、昇格すれば org 層との二重記載で乖離リスクを生む。新規発見はオーナー明言 #2 の「PR 直列」のみであり、org 既定分は昇格対象から外すべき
- OBJECT: team-practices.md 命名規則の禁止造語列挙（Store/Reader/Writer）— 正本 gateway-taxonomy.md は Source / Provider も禁止しており部分列挙が既に乖離している。規則本文の複製をやめ正本ファイル名参照へ置き換えるべき（DRY 違反の実証例）
