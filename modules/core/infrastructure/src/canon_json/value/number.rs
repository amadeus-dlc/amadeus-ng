//! `JsonValue::Number` が運ぶ数値表現。

/// JSON 数値の表現。非負は `PosInt` を優先し、負の整数は `NegInt`、小数・非有限は `Float`。
///
/// 表現の違いは同値関係に含まれる — `PosInt(1)` と `Float(1.0)` は等しくない。
/// 直列化結果は同じ `1` でも、往復 (parse → serialize) で表現が保たれることを保証したいため。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    /// 非負整数 (u64 の範囲)。
    PosInt(u64),
    /// 負整数 (i64 の範囲)。
    NegInt(i64),
    /// 浮動小数。非有限 (NaN / ±Infinity) も保持でき、直列化時に `null` へ落ちる (BR1.3)。
    Float(f64),
}
