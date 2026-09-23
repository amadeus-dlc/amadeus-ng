//! `AbsentConsume` — run-stage 指示の `consumes_absent` の 1 項目。

/// 解決した入力のうち、ディスクに無い**必須**の 1 件（2.8.2 `consumes_absent`）。
///
/// `expected` は欠落が想定内か — 生産するステージが実効計画の経路に無い（scope が
/// 走らせない）なら真、経路にあるのに出力が無いなら偽（回復の手順が扱う本物の欠落）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsentConsume {
    path: String,
    expected: bool,
}

impl AbsentConsume {
    /// 解決したパスと、欠落が想定内かを束ねる。
    #[must_use]
    pub fn new(path: impl Into<String>, expected: bool) -> AbsentConsume {
        AbsentConsume {
            path: path.into(),
            expected,
        }
    }

    /// ワークスペース相対のパス。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 欠落が想定内か。
    #[must_use]
    pub const fn is_expected(&self) -> bool {
        self.expected
    }
}
