//! `report` が運ぶ引数（upstream `parseReportFlags` — `aidlc-orchestrate.ts:4825`）。

/// `report` が運ぶ引数（upstream `parseReportFlags` — `aidlc-orchestrate.ts:4825`）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportArgs {
    result: Option<String>,
    stage: Option<String>,
    user_input: Option<String>,
    reason: Option<String>,
    skeleton_stance: Option<String>,
    single: bool,
}

impl ReportArgs {
    /// 報告された結末の生値（`--result`）。
    #[must_use]
    pub fn result(&self) -> Option<&str> {
        self.result.as_deref()
    }
    /// 明示されたステージ（`--stage`）。**有無それ自体が契約**なので `Option` で運ぶ。
    #[must_use]
    pub fn stage(&self) -> Option<&str> {
        self.stage.as_deref()
    }
    /// 承認時の人間入力（`--user-input`）。
    #[must_use]
    pub fn user_input(&self) -> Option<&str> {
        self.user_input.as_deref()
    }
    /// 読み飛ばし理由（`--reason`）。
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    /// skeleton stance の報告（`--skeleton-stance`）。
    #[must_use]
    pub fn skeleton_stance(&self) -> Option<&str> {
        self.skeleton_stance.as_deref()
    }
    /// 単独ステージ実行の報告（`--single`）。
    #[must_use]
    pub const fn is_single(&self) -> bool {
        self.single
    }
}

/// `report` のフラグを [`ReportArgs`] へ畳む。
///
/// `ReportArgs` のフィールドはすべて private なので、フィールド単位で組み立てる本関数は
/// 同じファイル（同じモジュール）に置く（`coding-rules/field-visibility.md`）。`cli::request`
/// の `parse` から呼ばれるため `pub(super)`（`cli` とその子孫まで可視）に上げる。
pub(super) fn parse_report(args: &[String]) -> ReportArgs {
    let mut flags = ReportArgs::default();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        let value = args.get(index + 1).cloned();
        let mut took_value = true;
        match arg.as_str() {
            "--result" => flags.result = value,
            "--user-input" => flags.user_input = value,
            "--reason" => flags.reason = value,
            "--skeleton-stance" => flags.skeleton_stance = value,
            "--stage" => flags.stage = value,
            "--single" => {
                flags.single = true;
                took_value = false;
            }
            _ => took_value = false,
        }
        if took_value {
            index += 1;
        }
        index += 1;
    }
    flags
}
