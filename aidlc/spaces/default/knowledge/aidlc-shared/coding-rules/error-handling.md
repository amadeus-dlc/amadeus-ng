# エラーハンドリング様式 — モジュールごとの手実装エラー enum

**裁定日**: 2026-08-23（オーナー、FD Q1 = A）
**適用例**: Bolt B1 / B3 のエラー型（core-domain `CommandError` / `ApplyError` / `StartError` / `SnapshotError`、~~core-use-case `GraphReadError`~~ → 廃止 2026-08-31・b26 段階2。下記「Repository エラーはジェネリック 1 本」を参照）
**機械強制**: `missing_errors_doc` / `missing_panics_doc` / `unwrap_used` / `expect_used` deny（`Cargo.toml` workspace lints）。`thiserror` / `anyhow` 禁止は `cargo lint` ルール候補（赤例テスト必須）

## ルール

- ドメイン層・ユースケース層の失敗はモジュールごとの**手実装エラー enum** で表現する。
- `thiserror` / `anyhow` 等のエラーハンドリング外部クレートには依存しない。
- 各エラー enum は `std::fmt::Display` と `std::error::Error` を手実装する。
- `Display` は**材料**（ID・索引・状態・原因）だけを描く開発者向けの診断表示であり、利用者向けの逐語文言（upstream 互換面）は**出す側の `wording` モジュール**（アダプタ層・RMU の投影ライタ — 2026-08-29 の message-catalog 解体後の形）が組み立てる — ドメイン層に文言を持ち込まない。
- 変種フィールドは材料のみ（`stage`, `actual`, `expected`, `path`, `cause` など）で、`String` の文言を運ぶ変種を作らない。
- fallible な公開関数には `# Errors` セクションを付ける（`missing_errors_doc` deny）。
- `# Panics` を要する公開関数は作らない（範囲は型で保証 — `StageIndex` 等）。

## 根拠

依存最小化と、エラー型をドメイン語彙に閉じ込める方針（Always Valid、設計監査 R4）。文言をドメインのエラー型に載せると、upstream 互換の逐語文言がドメイン層の変更理由になり、逐語契約の所在が二重化する。材料だけを運べば、文言の組み立て（および将来の多言語化・書式変更）はアダプタ層に閉じる。

## Repository エラーはジェネリック 1 本（オーナー裁定 2026-08-30）

- Repository ポートの失敗は **`RepositoryError<Id>` 1 本**で表す。リポジトリごとに ID 型だけが
  違うエラー型（旧 `IntentRepositoryError`）を複製しない。読取専用ポートでも `Conflict` が型上
  構成可能になるが、「構成不能を型で語る」精密さより統一が勝る（裁定）。
- `Corrupt` の**分類はポート契約に載せない**（裁定 6 — エラーは契約の一部であり、内部実装が
  バレる情報を含めない）。原因はアダプタ私有の型を `Error::source` 連鎖で運び、契約は
  「壊れていた」としか約束しない。代償として `PartialEq` を失う — テストは `matches!` +
  `source` の文字列確認で判定する（受容済み）。
- **適用（2026-08-31 オーナー裁定、b26 段階2）**: `WorkflowDefinitionRepository` も本則へ
  収束した — ポート専用エラー `GraphReadError`（6 変種）を**廃止**し、
  `RepositoryError<WorkflowDefinitionId>` 1 本にした（リポジトリにビジネスロジックエラーを
  扱わせない）。upstream 逐語文言（`docs/specs/12-workflow-definition.md` §4/§6 が規範）の
  所有は**クエリ側へ移った** — 「文言は出す側が持つ」の帰結であり、コマンド側のポートは
  材料すら持たず「壊れていた」としか言わない。

## 再構成は失敗を返さない（オーナー裁定 2026-08-30 — 「# Panics を作らない」の例外）

集約の `replay` / `apply_event` は `Result` を返さない。壊れた歴史はクラッシュが正であり、
そのための `expect` / `panic!` は**この経路に限って**容認する（allow に理由を書き、`# Panics`
を明記する）。「`# Panics` を要する公開関数は作らない」の一般則はそれ以外で従来どおり。
詳細は [aggregate-commands.md](aggregate-commands.md)「再構成の形」。

## 対象外

- **境界の文言モジュール（各出し手の `wording`）**: 利用者向けの逐語文言そのものを保持するのは**それを出す側**（アダプタ層・RMU の投影ライタ）の責務であり、本ルールの「文言を持ち込まない」対象外。~~独立クレート message-catalog~~ は 2026-08-29 に解体 — 「純粋部品だから全層依存可」を免罪符に domain が完成文言を運んでいた依存方向違反を是正し、文言は出す側に同居させた。ドメインが返すのは材料（例: `InvalidModeArg::given`）だけである。
- **テストコードの `unwrap` / `expect`**: `clippy.toml`（`allow-unwrap-in-tests` / `allow-expect-in-tests`）で許容する。

## 射程の明確化 2026-08-26 — 本ルールは「我々が書くエラー型」に限る

`thiserror` / `anyhow` 不使用は**我々が定義するエラー型**の規則である。外部依存
（event-store-adapter-rs v2.0.0 が `thiserror` を使う）の**推移依存として入ることは
違反ではない**（Conformist — ADR-010。相手のエラー設計は相手のドメイン）。
`cargo audit` の対象には含まれ続ける。我々のコードが `thiserror` を**直接** use したら
従来どおり違反。

