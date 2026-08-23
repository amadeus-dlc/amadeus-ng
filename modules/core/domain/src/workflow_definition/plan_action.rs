//! `PlanAction` — EXECUTE / SKIP (01 §3.1)。scope grid の列値であり、recompose オーバレイと
//! 合成した実効プラン (`effectivePlanAction` — 裁定 B1) の要素。

/// grid 1 マスの 2 値。「コンパイル済みの全ステージが EXECUTE か SKIP のどちらかを明示する」
/// ため、未指定・未知は表現不能 (upstream 01 §5.4)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanAction {
    /// このスコープで実施する。実効プランが EXECUTE のステージだけが
    /// `next` のルーティング対象になり、run-stage を受け取れる (I2)。
    Execute,
    /// このスコープでは実施しない。`next` は読み飛ばし、run-stage を
    /// emit しない (I2)。`execution: CONDITIONAL` とは別軸で、ALWAYS のステージでも
    /// スコープによっては SKIP になる。
    Skip,
}

impl PlanAction {
    /// scope grid・状態ファイルのサフィックスに現れる正準綴り (常に大文字)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            PlanAction::Execute => "EXECUTE",
            PlanAction::Skip => "SKIP",
        }
    }

    /// 正準綴りからの厳密パース (大文字 2 語のみ・正規化なし)。`None` は grid が
    /// EXECUTE / SKIP 以外を載せている状態であり、既定値へフォールバックさせない。
    #[must_use]
    pub fn parse(s: &str) -> Option<PlanAction> {
        match s {
            "EXECUTE" => Some(PlanAction::Execute),
            "SKIP" => Some(PlanAction::Skip),
            _ => None,
        }
    }

    /// 2 値の反転 — recompose の flip 1 回分。適用先は不変の grid ではなくオーバレイ
    /// (裁定 B1)。自己逆元なので 2 回適用すると元に戻る。
    #[must_use]
    pub const fn flipped(self) -> PlanAction {
        match self {
            PlanAction::Execute => PlanAction::Skip,
            PlanAction::Skip => PlanAction::Execute,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_rejects_lowercase() {
        assert_eq!(PlanAction::parse("EXECUTE"), Some(PlanAction::Execute));
        assert_eq!(PlanAction::parse("SKIP"), Some(PlanAction::Skip));
        assert_eq!(PlanAction::parse("execute"), None);
    }
}
