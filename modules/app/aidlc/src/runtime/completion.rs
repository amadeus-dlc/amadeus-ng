//! `Completion` — 1 回の起動の結末 (stdout の 1 行・stderr の診断・終了コード)。

/// 1 回の起動の結末。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    line: Option<String>,
    diagnostic: Option<String>,
    code: u8,
}

impl Completion {
    const fn new(line: Option<String>, diagnostic: Option<String>, code: u8) -> Self {
        Self {
            line,
            diagnostic,
            code,
        }
    }
    /// ツール実行を停止するフックの拒否。
    #[must_use]
    pub const fn hook_denied(diagnostic: String) -> Self {
        Self::new(None, Some(diagnostic), 2)
    }

    /// stdout/stderrを出さず成功するフック応答。
    #[must_use]
    pub const fn silent() -> Self {
        Self::new(None, None, 0)
    }

    /// directive を 1 つ出して正常終了する（ビジネス拒否の `error` directive もこちら）。
    #[must_use]
    pub const fn emitted(line: String) -> Completion {
        Self::new(Some(line), None, 0)
    }

    /// 何も stdout へ出さず、stderr へ逐語を出して失敗する（自己防衛拒否）。
    #[must_use]
    pub const fn refused(diagnostic: String) -> Completion {
        Self::new(None, Some(diagnostic), 1)
    }

    /// 報告書を stdout へ出し、報告書自身が決めた終了コードで終わる (`--doctor` — C7)。
    /// stderr は空である。
    #[must_use]
    pub const fn reported(text: String, code: u8) -> Completion {
        Self::new(Some(text), None, code)
    }

    /// 報告書を stdout へ出したうえで、後続の記録に失敗して stderr へ診断を出し 1 で終わる
    /// (C7 DC10 — 診断出力は残り、記録の成功は捏造しない)。
    #[must_use]
    pub const fn reported_then_refused(text: String, diagnostic: String) -> Completion {
        Self::new(Some(text), Some(diagnostic), 1)
    }

    /// 何も stdout へ出さず、stderr へ警告を出して成功 (0) で終わる (PreCompact の状態検査)。
    #[must_use]
    pub const fn warned(diagnostic: String) -> Completion {
        Self::new(None, Some(diagnostic), 0)
    }

    /// 何も stdout へ出さず、stderr へ助言を出して**止めずに**終わる。
    ///
    /// 終了コード 3 は PreToolUse の拒否 (2) でも成功 (0) でもない第 3 の観測であり、
    /// 規則を自前で先読みするハーネスへ「今回は付けられなかった」とだけ伝える
    /// (upstream `hooks/aidlc-deliver-stage-rules.ts:337-345`)。
    #[must_use]
    pub const fn hook_advisory(diagnostic: String) -> Completion {
        Self::new(None, Some(diagnostic), 3)
    }

    /// stdout へ出す 1 行（改行は書く側が付ける）。
    #[must_use]
    pub fn line(&self) -> Option<&str> {
        self.line.as_deref()
    }

    /// stderr へ出す診断。
    #[must_use]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }

    /// 終了コード。
    #[must_use]
    pub const fn code(&self) -> u8 {
        self.code
    }
}
