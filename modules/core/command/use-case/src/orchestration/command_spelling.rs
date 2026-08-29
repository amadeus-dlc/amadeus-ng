//! コマンド綴りのカタログ — directive 文言中のコマンド参照はここだけが組む。
//!
//! upstream 3 形 (bun 直接形 / 素のマルチコール形 / ディスパッチャ形) のうち、self-host の
//! 正準として**素のマルチコール形** (例 `aidlc-utility status` — `07-hooks.md:260` に実在し、
//! バイナリが busybox 式マルチコールで受ける — ADR 0002 決定 3) を使う。ディスパッチャ語彙の
//! 完全 ROUTES 写し (30 経路 + SLASH_FLAG_ALIASES) は U7 / A1 で表として実体化し、差し替えは
//! 本モジュール 1 点で行う (逸脱台帳 #1)。

/// 読み取り専用ユーティリティの起動綴り。
pub(crate) fn utility(subcommand: &str) -> String {
    format!("aidlc-utility {subcommand}")
}

/// 名詞トークン列 (workspace / plugin / knowledge) の逐語通し。
pub(crate) fn utility_tokens(tokens: &[String]) -> String {
    format!("aidlc-utility {}", tokens.join(" "))
}

/// park 解除の起動綴り。
pub(crate) fn state_unpark() -> String {
    "aidlc-state unpark".to_string()
}

/// jump の純読み取り解決。
pub(crate) fn jump_resolve(stage: &str) -> String {
    format!("aidlc-jump resolve --stage {stage}")
}

/// intent の鋳造 (birth — `next` は自身で実行しない)。ラベルは conductor が置換する
/// プレースホルダ付き。
pub(crate) fn intent_create(scope: &str) -> String {
    format!("aidlc-utility intent-create --scope {scope} --label \"<2-3 word kebab essence>\"")
}

/// scope 変更の名指し。
pub(crate) fn scope_change(scope: &str) -> String {
    format!("aidlc-utility scope-change --scope {scope}")
}

/// depth / test-strategy / review の設定変更の名指し。
pub(crate) fn config_change(field: &str, value: &str) -> String {
    format!("aidlc-utility config-change --{field} {value}")
}

/// composer ディスパッチの名指し。
pub(crate) fn composer_dispatch() -> String {
    "aidlc-composer detect".to_string()
}
