# developer-brief-6 — 委任 6: lint 昇格（`indexing_slicing` / `panic`）と既存コードの是正（U3 / Bolt B5）

Conversation language: 日本語（コメント・報告はすべて日本語）。

## 役割と範囲

あなたは aidlc-developer-agent。Unit **u3-event-store-repository**（Bolt B5）の委任 6（最後）。リポジトリルート `/Users/j5ik2o/orca/workspaces/amadeus-ng/docs`、ブランチ
`bolt/b5-u3-event-store-repository`（委任 1〜5 はコミット済み）。オーナー裁定（code-generation Q1 = A）: `clippy::indexing_slicing` / `clippy::panic` を workspace lint の deny に
昇格し、既存コードを是正する。**コーディング規則の正本 `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（README + 7 ルール）を読む。**

所有ファイル: `Cargo.toml`（`[workspace.lints.clippy]` への 2 行追加のみ）、`modules/**/src/**` と `modules/**/tests/**`（添字アクセス / panic の是正に限る — 挙動を変えない）、
`clippy.toml`（必要なら）、報告 `developer-report-6.md`（新規）。`tools/lint` は workspace 非メンバー（lints 非適用）— 触らない。
触らないもの: 計画・検査手順・質問票、`docs/**`、`formal/**`、`scripts/**`。`git add` / `git commit` はしない。`.claude/` のツールは実行しない。

## 作業（計画 Step 13）

1. `Cargo.toml` の `[workspace.lints.clippy]` に `indexing_slicing = "deny"` と `panic = "deny"` を追加（既存の並び・コメント様式に合わせる）。
2. `cargo clippy --workspace --all-targets -- -D warnings` を実行し、違反を是正:
   - **プロダクトコード（`src/`）**: 添字 `v[i]` / `s[a..b]` を `get(i)` / `get(a..b)` / イテレータ / `split_at_checked` / `chars().nth` 等へ。範囲外が「起きない」ことが型で
     保証されている箇所でも、`Option` を `?` / `ok_or` で既存のエラー型（材料のみ）に写すか、`if let` で分岐する。`unwrap` / `expect` / `panic!` は使わない。挙動
     （戻り値・エラー種別・バイト出力）は変えない — 既存テスト緑のまま。canon-json（`parse.rs` / `value.rs` / `writer.rs` / `canonical.rs` / `digest.rs`）は
     バイト互換のゴールデンテストがあるので、それを頼りに慎重に。
   - **テストコード（`#[cfg(test)] mod tests` / `tests/*.rs`）**: file / mod 単位で `#![allow(clippy::indexing_slicing)]`（理由コメント 1 行: 「テストは固定長フィクスチャの
     添字参照を許容（clippy.toml に相当設定が無いため file 単位で allow）」）。`panic` は元々 0 件。
3. 検査: `cargo clippy --workspace --all-targets -- -D warnings` 緑、`cargo test --workspace` 全緑、`cargo fmt --all --check` 緑、`cargo lint` 緑。
   是正件数（src: ファイル別件数 / tests: allow を付けたファイル一覧）を報告に。

## 作法

- 機械的・挙動不変。迷う箇所（`Option` の扱いで新しいエラー変種が要りそう等）は報告の「設計質問」へ書き、その箇所だけ保留して他を進める。

## 報告（`developer-report-6.md`）

「lint 追加の差分」「是正一覧（src: ファイル / 件数 / 手法、tests: allow 一覧）」「検査結果」「設計質問」「未了」。最終応答は要約（日本語、10 行以内）。
