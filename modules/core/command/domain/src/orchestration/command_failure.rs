//! 拒否されたコマンドの診断。監査の逐語内容は出力境界で決まる。

/// コマンド失敗の観測。許可や工程の遷移を表す値ではない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandFailure {
    tool: String,
    command: String,
    error: String,
}

impl CommandFailure {
    /// 出力境界で所在を秘匿した診断から組む唯一のコンストラクタ。
    #[must_use]
    pub const fn new(tool: String, command: String, error: String) -> Self {
        Self {
            tool,
            command,
            error,
        }
    }
    /// 実行したツール名。
    #[must_use]
    pub fn tool(&self) -> &str {
        &self.tool
    }
    /// 拒否された呼出し。
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }
    /// 利用者へ返した失敗内容。
    #[must_use]
    pub fn error(&self) -> &str {
        &self.error
    }
}
