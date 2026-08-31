//! `PlanActionView` — EXECUTE / SKIP (01 §3.1)。scope grid の列値。

/// grid 1 マスの 2 値。「コンパイル済みの全ステージが EXECUTE か SKIP のどちらかを明示する」
/// ため、未指定・未知は表現不能 (upstream 01 §5.4)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanActionView {
    /// このスコープで実施する。
    Execute,
    /// このスコープでは実施しない。`execution: CONDITIONAL` とは別軸である。
    Skip,
}

impl PlanActionView {
    /// scope grid に現れる正準綴り (常に大文字)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            PlanActionView::Execute => "EXECUTE",
            PlanActionView::Skip => "SKIP",
        }
    }

    /// 正準綴りからの厳密パース (大文字 2 語のみ・正規化なし)。
    ///
    /// `None` は grid が EXECUTE / SKIP 以外を載せている状態であり、既定値へ
    /// フォールバックさせない。
    #[must_use]
    pub fn parse(s: &str) -> Option<PlanActionView> {
        match s {
            "EXECUTE" => Some(PlanActionView::Execute),
            "SKIP" => Some(PlanActionView::Skip),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_rejects_lowercase() {
        assert_eq!(
            PlanActionView::parse("EXECUTE"),
            Some(PlanActionView::Execute)
        );
        assert_eq!(PlanActionView::parse("SKIP"), Some(PlanActionView::Skip));
        assert_eq!(PlanActionView::parse("execute"), None);
        assert_eq!(PlanActionView::parse("MAYBE"), None);
        assert_eq!(PlanActionView::Execute.as_str(), "EXECUTE");
        assert_eq!(PlanActionView::Skip.as_str(), "SKIP");
    }
}
