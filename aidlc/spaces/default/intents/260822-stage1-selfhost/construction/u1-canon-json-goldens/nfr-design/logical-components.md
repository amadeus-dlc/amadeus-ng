# logical-components — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

> NFR Design（Construction 3.3）成果物（Unit: U1、kind: library）。出典: `../nfr-requirements/security-requirements.md`、
> `../nfr-requirements/tech-stack-decisions.md`（クレート配置・依存）、`../functional-design/functional-spec.md`（W1〜W5、
> インターフェイス）、`../../../inception/contract-design/contract-summary.md`（C7 ゴールデン配置）、
> `../../../inception/domain-design/components.md`（CanonJson は依存ゼロの純粋部品）、確認事項 `nfr-design-questions.md`（P2）。
>
> 本 Unit はインフラを持たないため、「論理コンポーネント」= クレート内のモジュール境界とテスト支援の置き場。
> 障害ドメインは「呼出側へ返す `Result`」の 1 つだけで、ブラストラディウスは呼出側の 1 コマンド実行に閉じる。

## 1. コンポーネント一覧

| コンポーネント | 置き場 | 責務 | 公開面 | 依存 |
|---|---|---|---|---|
| `value` | `modules/shared/canon-json/src/value.rs`（private mod） | `JsonValue`（挿入順保持・不変）、`to_value` の変換点 | `pub use` でファサードへ | serde（型付き struct → JsonValue） |
| `profile` | `src/profile.rs`（private） | `SerializationProfile` 3 値と体裁・キー順の属性 | `pub use` | — |
| `writer` | `src/writer.rs`（private） | 体裁（BR1.5）・数値（BR1.3、JS 互換）・文字列エスケープ（BR1.4）・キー順（BR1.1 / BR1.2）の書き出し | `serialize(&JsonValue, SerializationProfile) -> String` | value, profile, canonical |
| `canonical` | `src/canonical.rs`（private） | hash-canonical 用の再帰キーソート | writer から利用（非公開） | value |
| `digest` | `src/digest.rs`（private） | `Digest { family, hex }`、`hash_canonical` / `hash_compact` | `pub use` | writer, sha2 |
| `parse` | `src/parse.rs`（private） | `parse(&str) -> Result<JsonValue, ParseError>`、`ParseError` 変種、深さ上限 | `pub use` | serde_json（preserve_order） |
| ファサード | `src/lib.rs` | 公開 API の列挙（`pub use` のみ。利便再エクスポートは置かない — module-visibility） | 上記 5 つの公開型・関数 | — |
| ゴールデンコーパス | `tests/goldens/{hash-canonical,cli,hooks}/`（リポジトリ root、C7） | 正解データ・正規化規則・来歴・再採取スクリプト | ファイル | — |
| ゴールデン比較器 | `modules/shared/canon-json/tests/` または共有テスト支援（dev-dependency） | コーパス読取・`normalize(<TS>/<CLONE>/<ROOT>/<SESSION>)`・diff・受入表の行ごと比較 | テストのみ | canon-json, コーパス |

## 2. 境界と隔離

- **クレート境界**: `canon-json` はワークスペースの他クレートに依存しない（components.md の依存ゼロ層）。他クレートは
  ファサード経由でのみ使い、`serde_json` の直列化関数を直接呼ばない（clippy disallowed-methods — BR1.7）。
- **モジュール境界**: 6 モジュールはすべて private。公開はファサードの `pub use` 列挙のみ（`unreachable_pub` deny で
  再輸出漏れはビルドエラー）。`canonical` は非公開（writer の内部）。
- **テスト境界**: ゴールデン比較器はプロダクトクレートに混ぜない（dev-dependency / tests 配下）。他 Unit（U6 / U7）の
  テストは同じ比較器とコーパスを共有フィクスチャとして使う。

## 3. 障害ドメインとブラストラディウス

| 障害 | 影響範囲 | 手当て |
|---|---|---|
| `ParseError`（不正 JSON・深さ超過・不正 UTF-8） | 呼出側の 1 コマンド実行（エラー終了） | `Result` で返す。文言はアダプタ層（message-catalog） |
| 直列化の非互換（ゴールデン不一致） | テスト失敗（リリース前に検出） | ゴールデン先行 TDD。実装を直す |
| 依存の脆弱性（sha2 / serde / serde_json） | ビルド全体 | `cargo audit`（CI）、`Cargo.lock` 固定、バージョン更新は PR |
| ゴールデンの劣化（採取環境の混入） | テストの信頼性 | 正規化 + レビュー。更新はピン更新 intent のみ |

共有資源: なし（ファイル I/O も持たない — 読み書きは呼出側が行い、canon-json は文字列/バイト列を扱う）。

## 4. テストの配置（NFR2.x）

| 種別 | 置き場 | 内容 |
|---|---|---|
| ユニット（インライン `#[cfg(test)]`） | 各モジュール | 数値表記・エスケープ・キー順・体裁の境界値、`ParseError` 変種 |
| PBT（proptest） | `src/` 同居 | 決定性（同入力 → 同出力）、parse → serialize の往復、hash の冪等性 |
| ゴールデン（受入） | `tests/`（比較器） | hash-canonical 受入表の全行一致（FR7.3）。cli / hook 族は U6 / U7 が使う |

## 5. Infrastructure Design への橋渡し

infrastructure-design は本 intent でスコープ外（SKIP）。U1 はインフラ資源を持たないため引き渡し事項なし。
CI（U10）側の関係: `cargo audit` と `unsafe_code` forbid の対象にこのクレートが含まれること。
