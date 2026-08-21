//! `StageNumber` — `"<phaseIndex>.<seq>"` の**文字列**。エンジン割当のステージ番号で、
//! 順序は phase → seq の 2 段整数比較 (`numericStageOrder`。`"1.10" > "1.9"`)。
//!
//! 生表現を逐語で保持する (**数値正規化禁止** — `"3.10"` を `3.1` にしてはならない。
//! レポート §5.1-5 の観測可能契約)。

use std::cmp::Ordering;
use std::fmt;

/// パース済みのステージ番号 (Always Valid)。`as_str()` は入力の生表現を逐語で返す。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StageNumber {
    raw: String,
    phase_index: u32,
    seq: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageNumberError {
    Empty,
    /// `.` がちょうど 1 個でない。
    MalformedSeparator {
        dot_count: usize,
    },
    /// `.` の左側 (`<phaseIndex>`) が空。
    EmptyPhaseIndex,
    /// `.` の右側 (`<seq>`) が空。
    EmptySeq,
    /// ASCII 数字以外を含む (符号・空白を含む)。
    NonDigit(char),
    /// `u32` に収まらない。
    Overflow,
}

impl StageNumber {
    /// # Errors
    ///
    /// 空文字列・`.` の個数違反・空セグメント・非数字・`u32` 溢れを拒否する。
    pub fn parse(s: &str) -> Result<StageNumber, StageNumberError> {
        if s.is_empty() {
            return Err(StageNumberError::Empty);
        }
        let dot_count = s.matches('.').count();
        if dot_count != 1 {
            return Err(StageNumberError::MalformedSeparator { dot_count });
        }
        let Some((phase_part, seq_part)) = s.split_once('.') else {
            return Err(StageNumberError::MalformedSeparator { dot_count });
        };
        let phase_index = parse_segment(phase_part, StageNumberError::EmptyPhaseIndex)?;
        let seq = parse_segment(seq_part, StageNumberError::EmptySeq)?;
        Ok(StageNumber {
            raw: s.to_string(),
            phase_index,
            seq,
        })
    }

    /// 生表現 (正規化なし)。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    #[must_use]
    pub const fn phase_index(&self) -> u32 {
        self.phase_index
    }

    #[must_use]
    pub const fn seq(&self) -> u32 {
        self.seq
    }
}

/// `<phaseIndex>` / `<seq>` セグメントの数値化。前置ゼロは受理する (`parseInt` 相当)。
fn parse_segment(segment: &str, when_empty: StageNumberError) -> Result<u32, StageNumberError> {
    if segment.is_empty() {
        return Err(when_empty);
    }
    let mut acc: u32 = 0;
    for c in segment.chars() {
        let Some(digit) = c.to_digit(10) else {
            return Err(StageNumberError::NonDigit(c));
        };
        acc = acc
            .checked_mul(10)
            .and_then(|v| v.checked_add(digit))
            .ok_or(StageNumberError::Overflow)?;
    }
    Ok(acc)
}

impl Ord for StageNumber {
    /// `numericStageOrder` — phase → seq の整数比較。
    ///
    /// 前置ゼロ違い (`"1.01"` と `"1.1"`) は整数として同値なので、`Ord` / `Eq` の
    /// 整合性を保つために最後に生表現で決定的に決着させる。
    fn cmp(&self, other: &StageNumber) -> Ordering {
        (self.phase_index, self.seq)
            .cmp(&(other.phase_index, other.seq))
            .then_with(|| self.raw.cmp(&other.raw))
    }
}

impl PartialOrd for StageNumber {
    fn partial_cmp(&self, other: &StageNumber) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for StageNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn n(s: &str) -> StageNumber {
        StageNumber::parse(s).unwrap()
    }

    #[test]
    fn the_second_segment_is_compared_as_an_integer_not_lexically() {
        assert!(n("1.10") > n("1.9"));
        assert!(n("3.7") < n("4.1"));
        assert!(n("0.1") < n("0.2"));
    }

    #[test]
    fn the_raw_representation_is_never_normalized() {
        let v = n("3.10");
        assert_eq!(v.as_str(), "3.10");
        assert_eq!(v.to_string(), "3.10");
        assert_eq!((v.phase_index(), v.seq()), (3, 10));
        // 前置ゼロも逐語保存され、整数値としては同値だが別の値として扱う
        assert_eq!(n("1.01").seq(), 1);
        assert_ne!(n("1.01"), n("1.1"));
    }

    #[test]
    fn rejects_shapes_that_are_not_phase_dot_seq() {
        assert_eq!(StageNumber::parse(""), Err(StageNumberError::Empty));
        assert_eq!(
            StageNumber::parse("12"),
            Err(StageNumberError::MalformedSeparator { dot_count: 0 })
        );
        assert_eq!(
            StageNumber::parse("1.2.3"),
            Err(StageNumberError::MalformedSeparator { dot_count: 2 })
        );
        assert_eq!(
            StageNumber::parse(".2"),
            Err(StageNumberError::EmptyPhaseIndex)
        );
        assert_eq!(StageNumber::parse("1."), Err(StageNumberError::EmptySeq));
        assert_eq!(
            StageNumber::parse("-1.2"),
            Err(StageNumberError::NonDigit('-'))
        );
        assert_eq!(
            StageNumber::parse("1.2a"),
            Err(StageNumberError::NonDigit('a'))
        );
        assert_eq!(
            StageNumber::parse("4294967296.0"),
            Err(StageNumberError::Overflow)
        );
    }

    proptest! {
        /// 正準表現 (前置ゼロなし) の範囲では `Ord` は `(phaseIndex, seq)` の整数比較と一致する。
        #[test]
        fn ord_agrees_with_integer_pair_comparison(
            p1 in 0u32..6, s1 in 0u32..40,
            p2 in 0u32..6, s2 in 0u32..40,
        ) {
            let a = n(&format!("{p1}.{s1}"));
            let b = n(&format!("{p2}.{s2}"));
            prop_assert_eq!(a.cmp(&b), (p1, s1).cmp(&(p2, s2)));
        }

        /// 正準表現ならパースは往復し、セグメントは逐語で読み出せる。
        #[test]
        fn parse_round_trips_canonical_numbers(p in 0u32..1000, s in 0u32..1000) {
            let raw = format!("{p}.{s}");
            let v = n(&raw);
            prop_assert_eq!(v.as_str(), raw.as_str());
            prop_assert_eq!((v.phase_index(), v.seq()), (p, s));
        }

        /// `Ord` と `Eq` は整合する (Equal ⟺ 等値)。前置ゼロ違いを含めても崩れない。
        #[test]
        fn ord_is_consistent_with_eq(
            a in "0*[0-9]{1,3}\\.0*[0-9]{1,3}",
            b in "0*[0-9]{1,3}\\.0*[0-9]{1,3}",
        ) {
            let x = n(&a);
            let y = n(&b);
            prop_assert_eq!(x.cmp(&y) == Ordering::Equal, x == y);
        }
    }
}
