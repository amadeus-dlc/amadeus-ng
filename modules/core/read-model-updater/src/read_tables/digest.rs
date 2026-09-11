//! 行に載る**ダイジェスト**の素材づくり (公開型ゼロの内部モジュール)。
//!
//! 素材は名前付き構造の正準 JSON を手で組み、`ContractCompact` 族 ([`hash_compact`]) で
//! ハッシュする。`Debug` 表現への依存は derive 変更で黙って値が変わる時限爆弾であり、
//! 区切り文字連結は区切り文字注入を許す (オーナー裁定 2026-08-30)。`serde_json::to_string*`
//! は `clippy.toml` の `disallowed-methods` で禁じられているので使わない。
//!
//! **挿入順が素材バイトの一部である** — `ObjectMembers` は宣言順を保つので、キーの並びを
//! 変えると値が変わる。ここに在るのは写像とハッシュだけで、判断は 1 つも含まない
//! (`coding-rules/cqrs-boundaries.md` 規則 3 の 2026-09-02 追記)。

use core_command_domain::workflow_definition::StageSlug;
use core_infrastructure::canon_json::{JsonValue, hash_compact};

use super::rule_content::RuleContent;

/// 挿入順を保持する素材オブジェクト。
fn object<const N: usize>(members: [(&str, JsonValue); N]) -> JsonValue {
    JsonValue::Object(
        members
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

/// 規則の 1 片を `{path, text}` の素材にする。
fn piece(content: &RuleContent) -> JsonValue {
    object([
        ("path", JsonValue::String(content.path().to_string())),
        ("text", JsonValue::String(content.text().to_string())),
    ])
}

/// `route_digest` — 対象ステージと、その scope の in-scope ステージ列。
///
/// 素材の正本はドメインの [`StageRoute`] である (集約のクエリ `stage_route` の答え)。
///
/// [`StageRoute`]: core_command_domain::workflow_definition::StageRoute
pub(crate) fn route(stage: &StageSlug, stages_in_scope: &[StageSlug]) -> String {
    let material = object([
        ("stage", JsonValue::String(stage.as_str().to_string())),
        (
            "stages",
            JsonValue::Array(
                stages_in_scope
                    .iter()
                    .map(|slug| JsonValue::String(slug.as_str().to_string()))
                    .collect(),
            ),
        ),
    ]);
    hash_compact(&material).rendered()
}

/// `directive_digest` — 届けようとしている run-stage の**環境由来**のキー項目。
///
/// pins (`gate` / `unit` / `single`) は素材に**含めない**。pins は HMAC 封筒の中で token
/// 自身が主張する値であり、環境がドリフトしたかどうかの素材ではない (設計 §1)。したがって
/// 行は定義 × scope × ステージだけで決まり、要求フラグに依らない。
pub(crate) fn directive(
    stage: &StageSlug,
    stage_file: &str,
    memory_path: &str,
    next_stage: Option<&str>,
) -> String {
    let material = object([
        ("stage", JsonValue::String(stage.as_str().to_string())),
        ("stage_file", JsonValue::String(stage_file.to_string())),
        ("memory_path", JsonValue::String(memory_path.to_string())),
        (
            "next_stage",
            next_stage.map_or(JsonValue::Null, |name| JsonValue::String(name.to_string())),
        ),
    ]);
    hash_compact(&material).rendered()
}

/// `bundle_digest` — 規則ファイル 1 つずつの `[{path, text}]` (読み順、分割前) を素材に、
/// `sha256:` を前置した値。
///
/// upstream `transportRunStage` の `` `sha256:${sha256(JSON.stringify(loaded.content))}` ``
/// (`aidlc-orchestrate.ts:3686` @a277af21) の写しである。`loaded.content` は分割前のファイル列
/// なので、素材はチャンクの入れ子ではなく平坦なファイル列である — この値は `load-steering` の
/// `bundle` としてそのまま外に出る Published Language であり (`cli/next/start/stdout.json`)、
/// continue の照合 (`b`) もこの値で行う。分割境界は素材に含まれない (upstream と同じ):
/// 分割は内容とパック規則から決定的に導かれるので、内容が同じなら分割も同じである。
pub(crate) fn bundle(files: &[RuleContent]) -> String {
    let material = JsonValue::Array(files.iter().map(piece).collect());
    format!("sha256:{}", hash_compact(&material).rendered())
}

/// `source_digest` — 参照入力の規則ファイル群 (path + text、読み順)。
///
/// 取得ループはこの値だけを見て「参照入力が変わったか」を決める。1 バイトでも違えば
/// 再パックが走り、同じなら steering の行に触らない (設計 §3)。
pub(crate) fn source<'a>(files: impl IntoIterator<Item = &'a RuleContent>) -> String {
    let material = JsonValue::Array(files.into_iter().map(piece).collect());
    hash_compact(&material).rendered()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slug(value: &str) -> StageSlug {
        StageSlug::parse(value).expect("テストの slug は文法内")
    }

    fn content(path: &str, text: &str) -> RuleContent {
        RuleContent::new(path.to_string(), text.to_string())
    }

    #[test]
    fn every_digest_is_a_bare_sixty_four_digit_hex() {
        // CompactRaw 族なので `sha256:` 接頭辞は付かない (canon_json の族の約束)。外に出る
        // `bundle` だけは upstream の綴りに合わせて接頭辞を持つ (下のテスト)。
        for value in [
            route(&slug("state-init"), &[slug("state-init")]),
            directive(&slug("state-init"), "a.md", "b.md", None),
            source([]),
        ] {
            assert_eq!(value.len(), 64, "実際: {value}");
            assert!(
                value
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
            );
        }
    }

    #[test]
    fn the_route_material_covers_the_stage_and_the_whole_scope_membership() {
        let base = route(&slug("state-init"), &[slug("state-init")]);
        assert_eq!(base, route(&slug("state-init"), &[slug("state-init")]));
        assert_ne!(
            base,
            route(&slug("intent-capture"), &[slug("state-init")]),
            "対象ステージが違えば別の route"
        );
        assert_ne!(
            base,
            route(
                &slug("state-init"),
                &[slug("state-init"), slug("intent-capture")]
            ),
            "scope の顔ぶれが動けば別の route"
        );
    }

    #[test]
    fn the_directive_material_covers_the_four_environment_keys() {
        let base = directive(&slug("state-init"), "a.md", "b.md", Some("Next"));
        assert_eq!(
            base,
            directive(&slug("state-init"), "a.md", "b.md", Some("Next"))
        );
        assert_ne!(
            base,
            directive(&slug("intent-capture"), "a.md", "b.md", Some("Next"))
        );
        assert_ne!(
            base,
            directive(&slug("state-init"), "z.md", "b.md", Some("Next"))
        );
        assert_ne!(
            base,
            directive(&slug("state-init"), "a.md", "z.md", Some("Next"))
        );
        assert_ne!(
            base,
            directive(&slug("state-init"), "a.md", "b.md", None),
            "終端かどうかは素材の一部"
        );
    }

    /// `bundle` は upstream `` `sha256:${sha256(JSON.stringify(loaded.content))}` `` の写し —
    /// 固定本家 2.7.1 の memory 層 (org.md + phases/inception.md) から採取した実バイト
    /// `cli/next/start/stdout.json` の `bundle` と同じ計算である (全文一致は
    /// `classic_corpus_contract.rs` が固定する)。
    #[test]
    fn the_bundle_digest_is_prefixed_and_covers_the_files_in_reading_order() {
        let a = content("a.md", "# A\n");
        let b = content("b.md", "# B\n");
        let value = bundle(&[a.clone(), b.clone()]);
        let hex = value.strip_prefix("sha256:").expect("upstream の綴り");
        assert_eq!(hex.len(), 64);
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
        );
        assert_eq!(value, bundle(&[a.clone(), b.clone()]), "決定的である");
        assert_ne!(value, bundle(&[b, a]), "読み順は素材の一部");
        assert_ne!(
            value,
            bundle(&[content("a.md", "# A!\n"), content("b.md", "# B\n")]),
            "本文の 1 バイトも素材の一部"
        );
        // 素材は `JSON.stringify([{path, text}, …])` そのもの。
        let expected = format!(
            "sha256:{}",
            hash_compact(&JsonValue::Array(vec![
                piece(&content("a.md", "# A\n")),
                piece(&content("b.md", "# B\n")),
            ]))
            .rendered()
        );
        assert_eq!(
            bundle(&[content("a.md", "# A\n"), content("b.md", "# B\n")]),
            expected
        );
    }

    #[test]
    fn the_source_material_changes_with_any_byte_of_any_file() {
        let files = [content("org.md", "# Org\n")];
        let base = source(&files);
        assert_eq!(base, source(&files));
        assert_ne!(base, source(&[content("org.md", "# Org!\n")]), "本文の変化");
        assert_ne!(base, source(&[content("team.md", "# Org\n")]), "パスの変化");
        assert_ne!(
            base,
            source(&[content("org.md", "# Org\n"), content("team.md", "")]),
            "ファイルが増えた"
        );
    }
}
