# tech-stack-decisions — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

> NFR Requirements（Construction 3.2）成果物（Unit: U1、kind: library）。出典: `../functional-design/functional-spec.md`、
> `../functional-design/rules.md`、`../../../inception/requirements-analysis/requirements.md`（NFR1 / NFR4、制約 C1〜C2）、
> `../../../inception/contract-design/contract-summary.md`（C7）、`aidlc/spaces/default/codekb/docs/technology-stack.md`
> （既存: Rust edition 2024、serde 1.0.229、serde_json 1.0.151、proptest 1.11.0、`canon-json` はスタブ）、
> `docs/adr/0001-canonical-json-serializer.md`、確認事項 `nfr-requirements-questions.md`（前提 P1）。

## 1. 選定

| 領域 | 選定 | 理由 | 代替案（不採用の理由） |
|---|---|---|---|
| クレート配置 | 既存スタブ `modules/shared/canon-json`（`canon-json`）を実体化。依存ゼロ層（components.md CanonJson） | ADR 0001 決定 1（単一クレート）。層 = クレートで直接呼び出し禁止を機械化しやすい | 新クレート名: 既存スタブと ADR 0005 の構成を変えない |
| JSON 読取 | `serde` + `serde_json`（既存依存。`preserve_order` フィーチャをワークスペース全体で有効） | 挿入順保持が JS 互換の前提（ADR 0001 決定 3）。既存のワイヤ構造体と共有 | 自前パーサ: 保守コストに見合わない。`json` / `simd-json`: 依存追加と互換リスク |
| 書き出し | canon-json 内の**自前ライタ**（3 プロファイル、キー順・体裁・数値・エスケープを BR1.1〜BR1.5 で実装） | serde_json の既定フォーマッタは `1.0` / 指数表記 / 非有限の扱いが JS と異なる（ADR 0001 決定 4） | `serde_json::to_string*` の直接利用: 禁止（BR1.7、clippy disallowed-methods）。`ryu` 既定: JS の閾値・`e+` 書式と不一致 |
| 正準化 | 再帰キーソート（UTF-16 コード単位順 = ASCII ではバイト順）を hash-canonical プロファイルの直列化時に適用 | hashObject 互換（ADR 0001 決定 2） | RFC 8785（JCS）クレート（`serde_jcs` 等）: upstream が JCS ではない（ADR 0001 代替案で不採用） |
| ハッシュ | `sha2`（SHA-256） | pure Rust、広く監査済み、`cargo audit` 既知問題なし、依存木が小さい | `ring`: C/asm を含み依存が重い。`openssl`: システム依存 |
| 数値表現 | 契約型の数値は i64/u64。浮動小数が現れる箇所のみ ECMA-262 `Number::toString` 互換ライタ（指数閾値 1e21 / 1e-6、`e+`、`-0` → `0`、非有限 → `null`） | ADR 0001 決定 4。符号判別は「非負は u64 優先、それ以外は i64」（functional-design レビュー Minor 3 の回答） | f64 経由の一律表現: `"1.0"` 化で互換が壊れる |
| PBT | `proptest`（既存） | 往復・決定性・冪等性の性質検証 | — |
| ゴールデン採取 | 再採取スクリプト（シェル + bun、upstream ピン `3c3146cf` の dist ツールを使い捨てワークスペースで実行）。bun は開発時ツールでプロダクト依存にしない（D1） | 前提 A3（bun は本リポジトリで動く）。来歴をコーパスに記録（BR2.1） | 手動採取: 再現性がない |
| ゴールデン配置 | `tests/goldens/{hash-canonical,cli,hooks}/...`（contract-summary C7 の layout。hash-canonical は `{ input, expected_output, expected_sha256 }`） | C7 契約どおり。他 Unit のテストが共有フィクスチャとして読む | テストコードへの埋め込み: 更新・レビューが難しい |
| 機械強制 | clippy `disallowed-methods` で `serde_json::to_string` / `to_string_pretty` / `to_vec` / `to_vec_pretty` / `to_writer` / `to_writer_pretty` と契約経路の `to_value` を canon-json 以外で拒否 | ADR 0001 決定 5。型（E1）→ 既存 lint → `cargo lint` の優先順に従い既存 lint で実現 | `cargo lint` 新ルール: 既存 lint で足りる |

## 2. 依存の差分（予定）

| クレート | 追加先 | 種別 | 備考 |
|---|---|---|---|
| `sha2` | `modules/shared/canon-json` | runtime | 新規 |
| `serde` / `serde_json`（`preserve_order`） | `modules/shared/canon-json`（+ ワークスペース features） | runtime | 既存依存の適用範囲拡大。`preserve_order` は Cargo のフィーチャ統合で全体に効く |
| `proptest` | dev-dependency | dev | 既存 |

`cargo audit`（U10）で clean を維持し、`rust-toolchain.toml` 固定（U10）後も同じバージョンでビルドできること。

## 3. 未決（後続で確定）

- `preserve_order` 有効化による既存 `serde_json::Value` 利用箇所（core-interface-adapter のワイヤ構造体）への
  影響確認 — 型付き struct 中心のため影響は小さい見込み。code-generation の計画で棚卸し。
- bun のバージョン固定（再採取スクリプト内に記録）。
