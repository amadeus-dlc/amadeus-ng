# developer-report-6 — 委任 6: lint 昇格（`indexing_slicing` / `panic`）と既存コードの是正（U3 / Bolt B5）

> Code Generation（Construction 3.5）委任 6（最後）の報告。所有ファイル: `Cargo.toml`（`[workspace.lints.clippy]` への 2 行追加）、
> `modules/**/src/**` と `modules/**/tests/**`（添字アクセス / panic の是正のみ、挙動不変）。
> 出典: `developer-brief-6.md`（計画 Step 13、code-generation Q1 = A）。

## 1. lint 追加の差分

```diff
 [workspace.lints.clippy]
 unwrap_used = "deny"
 expect_used = "deny"
+# 2026-08-22 オーナー裁定（code-generation Q1 = A）: 添字アクセスと panic! も
+# unwrap/expect と同じ「境界チェックなしで panic しうる操作」に属するため deny へ昇格。
+indexing_slicing = "deny"
+panic = "deny"
 redundant_clone = "deny"
```

`unwrap_used` / `expect_used` の直後に置いた（同じ「境界チェックなしで panic しうる操作」系列としての並び）。

## 2. 是正一覧

### 2.1 src（プロダクトコード、挙動不変・機械的是正）

全 5 ファイル・8 箇所。いずれも「範囲外は型/ループ不変条件で起きない」ことが構造的に分かる箇所で、`unwrap`/`expect`/`panic!` は使わず `Option` を `.get()` + `if let` / `let-else` / `unwrap_or`（安全な既定値。到達しない分岐であることをコメントで明記）で扱った。canon-json はゴールデンテスト（バイト互換）緑を確認済み。

| ファイル | 件数 | 手法 |
|---|---|---|
| `modules/shared/canon-json/src/canonical.rs` | 2 | `member_order`: `(0..len).filter(\|i\| keys[*i]…)` → `keys.iter().enumerate().filter(...)` へイテレータ化（添字を作らない）。`sort_by` 内の `keys[*a]`/`keys[*b]` → `keys.get(*a)`/`keys.get(*b)` を `match` し、両方 `Some` のときだけ比較・それ以外は `Ordering::Equal`（到達しない） |
| `modules/shared/canon-json/src/digest.rs` | 2 | `sha256_hex`: `HEX_DIGITS[nibble]` → `HEX_DIGITS.get(nibble).copied().unwrap_or(b'0')`（nibble は 0..16 保証、既定値は到達しない） |
| `modules/shared/canon-json/src/writer.rs` | 2 | `write_object`: `entries[index]` → `let Some(&(key, value)) = entries.get(index) else { continue };`（`order` は `entries` と同じ `members` 由来で必ず有効）。`write_string`: `HEX_DIGITS[nibble]` → digest.rs と同じ `get().copied().unwrap_or(b'0')` |
| `modules/infra-io/src/append_only.rs` | 1 | `append_all`: `&bytes[written..]` → `let Some(remaining) = bytes.get(written..) else { return Err(io::Error::new(InvalidInput, ...)) };`（ループ不変条件 `written < bytes.len()` により到達しない。同ファイル既存の `io::Error::new` 材料をそのまま流用、新しいエラー型は導入していない） |
| `modules/core/domain/src/workspace/state_writers.rs` | 1 | `set_or_insert_field` の空行巻き戻しループ: `while insert_at > start+1 && lines[insert_at-1]…` → `while insert_at > start+1 { let Some(prev) = lines.get(insert_at-1) else { break }; if !prev…{ break } … }`（`insert_at <= end <= lines.len()` により到達しない） |

### 2.2 tests（`#![allow(...)]` 一覧、file/mod 単位・理由コメント付き）

`clippy::indexing_slicing` は「固定長フィクスチャの添字参照を許容」、`clippy::panic` は「想定外バリアント/ケースの即時失敗という検証用途で使っており、テスト失敗のシグナルとして妥当」を理由コメントとして統一した。3 ファイル（`engine_loop_conformance.rs` / `journal_protocol_conformance.rs` / `workflow_definition_repository_impl_test.rs`）は既存の file-level `#![allow(clippy::unwrap_used, ...)]` に追記する形で拡張した。

| ファイル / スコープ | 追加した allow |
|---|---|
| `canon-json/src/canonical.rs` `mod tests` | `indexing_slicing` |
| `canon-json/src/parse.rs` `mod parse_tests` | `indexing_slicing`, `panic` |
| `canon-json/src/value.rs` `mod to_value_tests` | `indexing_slicing`, `panic` |
| `canon-json/tests/golden_hash_canonical.rs`（file） | `indexing_slicing`, `panic` |
| `canon-json/tests/golden_corpus_read.rs`（file） | `indexing_slicing`, `panic` |
| `canon-json/tests/support/mod.rs`（file） | `indexing_slicing`, `panic` |
| `core-domain/src/workspace/checkbox.rs` `mod tests` | `indexing_slicing` |
| `core-domain/src/workflow_definition/scope_grid.rs` `mod tests` | `indexing_slicing` |
| `core-domain/src/workflow_definition/stage_graph.rs` `mod tests` | `indexing_slicing` |
| `core-domain/src/workflow_definition/stage_node.rs` `mod tests` | `indexing_slicing` |
| `core-domain/src/workflow_definition/workflow_definition.rs` `mod tests` | `indexing_slicing` |
| `core-domain/src/orchestration/workflow_execution.rs` `mod tests` | `indexing_slicing`, `panic` |
| `core-domain/tests/engine_loop_conformance.rs`（file、既存 allow 拡張） | `indexing_slicing`, `panic` |
| `core-use-case/src/orchestration/journal_reader.rs` `mod tests` | `indexing_slicing` |
| `core-interface-adapter/src/orchestration/memory/workflow_definition_repository.rs` `mod tests` | `panic` |
| `core-interface-adapter/src/orchestration/wire/event_wire.rs` `mod tests` | `indexing_slicing`, `panic` |
| `core-interface-adapter/src/orchestration/wire/state_wire.rs` `mod tests` | `panic` |
| `core-interface-adapter/tests/journal_protocol_conformance.rs`（file、既存 allow 拡張） | `indexing_slicing`（`panic` は既存 allow 済み） |
| `core-interface-adapter/tests/workflow_definition_repository_impl_test.rs`（file、既存 allow 拡張） | `indexing_slicing`, `panic` |

19 スコープ（新規 16 + 既存拡張 3）。所有外である `tools/lint`（workspace 非メンバー）は対象外のまま未変更。

## 3. 検査結果

| 検査 | 結果 |
|---|---|
| `cargo clippy --workspace --all-targets -- -D warnings` | 緑（0 warnings/errors） |
| `cargo test --workspace` | 全緑（doctest 含む全ターゲット、失敗 0・golden 系/ITF 準拠テスト含む） |
| `cargo fmt --all --check` | 緑 |
| `cargo lint` | 緑 |

差分は `Cargo.toml` + `modules/**/src/**` + `modules/**/tests/**` の 24 ファイルのみ（`git diff --stat` で確認）。`docs/**`・`formal/**`・`scripts/**`・計画/検査手順/質問票・`tools/lint` はいずれも変更していない。`git add` / `git commit` は実行していない。

## 4. 設計質問

1. **ブリーフの「`panic` は元々 0 件」という想定は、テストコードに関しては外れていた。** `canon-json/src/parse.rs`・`value.rs`、`core-domain/src/orchestration/workflow_execution.rs` ほか複数の `mod tests`、および `canon-json/tests/*.rs` に `other => panic!(...)` / `let ... else { panic!(...) }` の想定外バリアント検出パターンが多数存在した（プロダクトコード側は 0 件で想定どおり）。判断: `clippy::indexing_slicing` と同じ file/mod 単位の `#![allow(...)]`（理由コメント付き）で扱った。根拠は 2 点 — (a) この扱いは委任前から `engine_loop_conformance.rs` / `journal_protocol_conformance.rs` に**既存の先例**があった（両ファイルとも着手前から `#![allow(..., clippy::panic)]` を持っていた）。(b) テストの `panic!` は「テスト失敗」というシグナルそのものであり、production code の panic 回避という lint の意図（呼出側に制御を返さない失敗の禁止）とは無関係。保留はしていない — 全箇所この方針で是正済み。オーナー確認が必要であれば、この判断の当否についてのみご確認いただきたい。

## 5. 未了

なし。ブリーフの作業（Cargo.toml 追加・違反是正・4 検査）はすべて完了した。
