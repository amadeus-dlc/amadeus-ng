//! 正準 JSON シリアライザ (ADR 0001 / A2) — 3 プロファイル。純粋部品: 全層から依存可。
//!
//! upstream (JS の `JSON.stringify` / `hashObject`) とバイト一致する JSON の読み書きと
//! ダイジェスト計算を提供する。契約 JSON の読み書きはすべてこのクレートを経由し、
//! 呼出側は `serde_json` の直列化関数を直接呼ばない (BR1.7 — `clippy.toml` の
//! `disallowed-methods` で機械強制)。
//!
//! # 3 つの直列化プロファイル (ADR 0001 決定 2)
//!
//! | プロファイル | 体裁 | キー順 | 用途 |
//! |---|---|---|---|
//! | `ContractPretty` | 2 スペース + 末尾改行 | 宣言順 / 挿入順 | ディスク成果物・Markdown 埋め込み |
//! | `ContractCompact` | 空白なし | 宣言順 / 挿入順 | stdout の 1 行 JSON・非正準ハッシュ族の入力 |
//! | `HashCanonical` | 空白なし | 再帰ソート | `hashObject` 互換のハッシュ入力 |
//!
//! # 2 つのダイジェスト族 (BR1.6)
//!
//! - 正準族 (`CanonicalPrefixed`): `hash_canonical` — `sha256:` + hex。`contract_sha256`・
//!   approval fingerprint がこの族。
//! - 非正準族 (`CompactRaw`): `hash_compact` — 生 hex。bundle hash・`directiveHash`・
//!   route hash・ルール配送の冪等 digest がこの族。
//!
//! # 読取の境界と深さ上限 (NFR4.3)
//!
//! 入力検証は [`parse`] / [`parse_bytes`] の 1 か所に集約する。ネスト深さの上限は
//! 128 段で、超過は [`ParseError::TooDeep`] として決定的に拒否する (スタック枯渇の防止)。

#![forbid(unsafe_code)]

// 型ファイルの mod は private。公開 API は下の `pub use` 列挙が唯一の宣言であり、
// 消費側のパスは `canon_json::<型>` で安定する。利便性のための再エクスポートは置かない
// (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。
mod canonical;
mod digest;
mod parse;
mod profile;
mod value;
mod writer;
