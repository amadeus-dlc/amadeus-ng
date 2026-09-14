//! `CodekbTokenError` — compare-and-swap の合言葉を組めなかった理由。

/// 呼び手が渡した合言葉が合言葉として成立しない。
///
/// **綴りの妥当性は見ない** — upstream は `--expect-store` / `--expect-source` の中身を検査
/// せず文字列として突き合わせるだけなので、こちらも形を検査しない。空だけは upstream も
/// `!expectedStore` で使い方の誤りとして落とすので、ここで拒否する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodekbTokenError {
    /// 空の合言葉。
    Empty,
}

impl std::fmt::Display for CodekbTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("empty codekb compare-and-swap token")
    }
}

impl std::error::Error for CodekbTokenError {}
