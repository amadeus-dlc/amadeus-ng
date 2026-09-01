//! `Digest` が属するダイジェストの族 (BR1.6)。

/// ダイジェストの族。どのプロファイルのバイト列から計算し、どう表記するかを型で固定する。
///
/// 族を型で持つのは取り違えを防ぐため — 正準族と非正準族は同じ 64 桁 hex でも
/// 入力バイト列が違うので、混ぜると静かに不一致になる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DigestFamily {
    /// `hashObject` 互換。hash-canonical 出力の sha256 を `sha256:` 接頭辞付きで表記する。
    /// `contract_sha256`・approval fingerprint がこの族。
    CanonicalPrefixed,
    /// contract-compact 出力の sha256 を生 hex で表記する。
    /// bundle hash・`directiveHash`・route hash・ルール配送の冪等 digest がこの族。
    CompactRaw,
}
