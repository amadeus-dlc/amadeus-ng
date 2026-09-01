//! 正準 JSON シリアライザ (ADR 0001 / A2) — 3 プロファイル。純粋部品: 全層から依存可。
//!
//! 2026-08-29 オーナー裁定により独立クレート `modules/shared/canon-json` から本モジュールへ移設 —
//! 直列化の力学は**言語拡張（infrastructure）**であり、ドメインも相手方システムの契約も知らない
//! （upstream 互換のバイト挙動は「JSON の書き方」の仕様であって、プロトコル結合ではない）。
//!
//! upstream (JS の `JSON.stringify` / `hashObject`) と**バイト一致**する JSON の読み書きと
//! ダイジェスト計算を提供する。契約 JSON の読み書きはすべてこのクレートを経由し、
//! 呼出側は `serde_json` の直列化関数と `to_value` を直接呼ばない (BR1.7 — `clippy.toml` の
//! `disallowed-methods` で機械強制。読取の `from_str` は禁止対象外だが、深さ上限を持つ
//! [`parse`] を通すこと)。
//!
//! # 3 つの直列化プロファイル (ADR 0001 決定 2)
//!
//! | プロファイル | 体裁 | キー順 | 用途 |
//! |---|---|---|---|
//! | [`SerializationProfile::ContractPretty`] | 2 スペース + 末尾改行 | 宣言順 / 挿入順 | ディスク成果物・Markdown 埋め込み |
//! | [`SerializationProfile::ContractCompact`] | 空白なし | 宣言順 / 挿入順 | stdout の 1 行 JSON・非正準ハッシュ族の入力 |
//! | [`SerializationProfile::HashCanonical`] | 空白なし | 再帰ソート | `hashObject` 互換のハッシュ入力 |
//!
//! どのプロファイルでも **integer-like キー** (0〜2^32-2 の正準十進表記) は挿入順に関係なく
//! 数値昇順で先頭に並ぶ (ECMAScript の所有プロパティ順序 — BR1.2)。`HashCanonical` は
//! さらに残りのキーを **UTF-16 コード単位順**で整列する (BR1.1)。
//!
//! # 2 つのダイジェスト族 (BR1.6)
//!
//! | 族 | 関数 | 表記 | 用途 |
//! |---|---|---|---|
//! | [`DigestFamily::CanonicalPrefixed`] | [`hash_canonical`] | `sha256:` + hex | `contract_sha256`・approval fingerprint |
//! | [`DigestFamily::CompactRaw`] | [`hash_compact`] | 生 hex | bundle hash・`directiveHash`・route hash・ルール配送の冪等 digest |
//!
//! 族は [`Digest`] が型として持つ。同じ 64 桁 hex でも入力バイト列が違うため、
//! 取り違えると静かに不一致になるからである。
//!
//! # 数値と文字列 (BR1.3 / BR1.4)
//!
//! - 整数は十進。ただし絶対値が 2^53 を超える整数は JS 側が f64 に丸めてから表記するため、
//!   こちらも f64 経路で書く。
//! - 浮動小数は ECMA-262 `Number::toString` 互換 (非指数表記は `1e-6 ≤ |x| < 1e21`、
//!   指数が正なら `e+`)。`-0` は `0`、非有限 (NaN / ±Infinity) は `null`。
//! - エスケープは `"` `\` と U+0000〜U+001F のみ。`/`・U+007F・非 ASCII・U+2028 / U+2029 は
//!   生出力する。
//!
//! # 読取の境界と深さ上限 (NFR4.3)
//!
//! 入力検証は [`parse`] / [`parse_bytes`] の 1 か所に集約する。ネストは
//! [`MAX_DEPTH`] `- 1` = 127 段までを受け入れ、それ以上は [`ParseError::TooDeep`] として
//! 決定的に拒否する (`serde_json` の再帰エラーが表に出る前に弾く — スタック枯渇の防止)。
//!
//! # 既知の非対称
//!
//! JS の `JSON.stringify` は孤立サロゲート (対にならない U+D800〜U+DFFF) を `\udXXX` として
//! 書けるが、Rust の `String` は UTF-8 の不変条件によりそれを保持できない。`"\ud800"` は
//! 読取段階で [`ParseError::Syntax`] になる。契約 JSON には現れない形なので実害はない。
//!
//! # 例
//!
//! ```
//! use core_infrastructure::canon_json::{SerializationProfile, hash_canonical, parse, serialize, to_value};
//!
//! // 動的な JSON テキストは parse で読む (挿入順を保つ)。
//! let value = parse(r#"{"z":1,"a":[1.0,2]}"#)?;
//!
//! // contract-compact は挿入順のまま、hash-canonical は整列する。
//! assert_eq!(serialize(&value, SerializationProfile::ContractCompact), r#"{"z":1,"a":[1,2]}"#);
//! assert_eq!(serialize(&value, SerializationProfile::HashCanonical), r#"{"a":[1,2],"z":1}"#);
//!
//! // contract-pretty は 2 スペース + 末尾改行。
//! assert_eq!(
//!     serialize(&value, SerializationProfile::ContractPretty),
//!     "{\n  \"z\": 1,\n  \"a\": [\n    1,\n    2\n  ]\n}\n"
//! );
//!
//! // 正準族のダイジェストは `sha256:` 接頭辞付き。
//! assert!(hash_canonical(&value).rendered().starts_with("sha256:"));
//!
//! // 型付き契約型はフィールド宣言順で JsonValue になる。
//! #[derive(serde::Serialize)]
//! struct Directive { kind: &'static str, stage: &'static str }
//! let directive = to_value(&Directive { kind: "run-stage", stage: "domain-design" })?;
//! assert_eq!(
//!     serialize(&directive, SerializationProfile::ContractCompact),
//!     r#"{"kind":"run-stage","stage":"domain-design"}"#
//! );
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

// 型ファイルの mod は private。公開 API は下の `pub use` 列挙が唯一の宣言であり、
// 消費側のパスは `canon_json::<型>` で安定する。利便性のための再エクスポートは置かない
// (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。
mod canonical;
mod digest;
mod digest_family;
mod parse;
mod profile;
mod value;
mod writer;

pub use digest::{Digest, hash_canonical, hash_compact};
pub use digest_family::DigestFamily;
pub use parse::{MAX_DEPTH, ParseError, parse, parse_bytes};
pub use profile::{Indent, KeyOrder, SerializationProfile};
pub use value::{JsonValue, Number, ObjectMembers, ToValueError, to_value};
pub use writer::serialize;

#[cfg(test)]
mod facade_tests {
    /// 設計 (`logical-components.md` §1、`code-generation-plan.md` §2) が定める公開面。
    /// ここに無い名前を `pub use` へ足すのは公開 API の拡大なので、設計側の更新を伴う。
    const DECLARED_SURFACE: &[&str] = &[
        "Digest",
        "DigestFamily",
        "hash_canonical",
        "hash_compact",
        "MAX_DEPTH",
        "ParseError",
        "parse",
        "parse_bytes",
        "Indent",
        "KeyOrder",
        "SerializationProfile",
        "JsonValue",
        "Number",
        "ObjectMembers",
        "ToValueError",
        "to_value",
        "serialize",
    ];

    /// `mod.rs` の `pub use` 行から再輸出名を取り出す。
    fn reexported_names() -> Vec<String> {
        let source = include_str!("mod.rs");
        let mut names = Vec::new();
        for line in source.lines() {
            let Some(rest) = line.trim().strip_prefix("pub use ") else {
                continue;
            };
            let Some((_, items)) = rest.trim_end_matches(';').split_once("::") else {
                continue;
            };
            let items = items.trim_start_matches('{').trim_end_matches('}');
            for item in items.split(',') {
                let item = item.trim();
                if !item.is_empty() {
                    names.push(item.to_string());
                }
            }
        }
        names
    }

    #[test]
    fn the_facade_publishes_exactly_the_declared_surface() {
        let mut actual = reexported_names();
        actual.sort();
        let mut declared: Vec<String> = DECLARED_SURFACE.iter().map(|s| (*s).to_string()).collect();
        declared.sort();

        assert_eq!(actual, declared, "公開面が設計の列挙と食い違っている");
    }

    #[test]
    fn every_module_declaration_is_private() {
        let source = include_str!("mod.rs");

        for line in source.lines() {
            assert!(
                !line.trim_start().starts_with("pub mod "),
                "モジュールは private のまま公開はファサード経由: {line}"
            );
        }
    }
}
