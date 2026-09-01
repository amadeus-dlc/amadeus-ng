//! `SkeletonStance` — on / off / scope-dependent (10 §2.2)。エンジンが計算できない唯一の
//! ゲート値で、conductor の分類が `report --skeleton-stance` で返る。

use super::unknown_stance::UnknownStance;

/// 分類結果の 3 値。チームの自由記述 `## Walking Skeleton` を conductor が読んで決め、
/// 状態ファイルの `Skeleton Stance` フィールドに載る。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkeletonStance {
    /// walking skeleton を常に採る分類 (「always」「every greenfield feature」相当)。
    /// 最初の Construction Bolt に always-gate を強制する。
    On,
    /// walking skeleton を採らない分類 (「never」相当)。最初の Bolt は通常の Bolt として
    /// 走るが、autonomy 未設定は gated 扱いなのでバッチゲート自体は提示される。
    Off,
    /// 判断をスコープに委ねる分類 (明示の「scope-dependent」に加え、未記載・空も**ここ**に
    /// 落ちる)。解決はアクティブなスコープの `skeleton:` 既定へフォールバックする。
    ScopeDependent,
}

impl SkeletonStance {
    /// # Errors
    ///
    /// 3 値以外は `UnknownStance` で拒否する。
    pub fn parse(s: &str) -> Result<SkeletonStance, UnknownStance> {
        Ok(match s {
            "on" => SkeletonStance::On,
            "off" => SkeletonStance::Off,
            "scope-dependent" => SkeletonStance::ScopeDependent,
            other => return Err(UnknownStance::new(other)),
        })
    }

    /// `report --skeleton-stance <…>` と状態フィールドの正準綴り (`parse` の逆写像)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            SkeletonStance::On => "on",
            SkeletonStance::Off => "off",
            SkeletonStance::ScopeDependent => "scope-dependent",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_values_round_trip_and_unknown_is_rejected() {
        for s in ["on", "off", "scope-dependent"] {
            assert_eq!(SkeletonStance::parse(s).unwrap().as_str(), s);
        }
        // 3 値以外は生値を逐語で持ち帰る
        let rejected = SkeletonStance::parse("maybe").unwrap_err();
        assert_eq!(rejected.as_str(), "maybe");
        assert_eq!(rejected, UnknownStance::new("maybe"));
    }
}
