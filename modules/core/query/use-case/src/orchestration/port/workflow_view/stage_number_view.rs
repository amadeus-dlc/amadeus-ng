//! `StageNumberView` — `"<phaseIndex>.<seq>"` の**文字列**。順序は phase → seq の 2 段整数
//! 比較 (`numericStageOrder`。`"1.10" > "1.9"`)。
//!
//! 生表現を逐語で保持する (**数値正規化禁止** — `"3.10"` を `3.1` にしてはならない。
//! 12 §6.1-5 の観測可能契約)。

use std::cmp::Ordering;

use super::stage_number_error::StageNumberError;

/// パース済みのステージ番号。`as_str()` は入力の生表現を逐語で返す。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StageNumberView {
    raw: String,
    phase_index: u32,
    seq: u32,
}

impl StageNumberView {
    /// # Errors
    ///
    /// 空文字列・`.` の個数違反・空セグメント・非数字・`u32` 溢れを拒否する。
    pub fn parse(s: &str) -> Result<StageNumberView, StageNumberError> {
        if s.is_empty() {
            return Err(StageNumberError::Empty);
        }
        let dot_count = s.matches('.').count();
        let Some((phase_part, seq_part)) = s.split_once('.').filter(|_| dot_count == 1) else {
            return Err(StageNumberError::MalformedSeparator { dot_count });
        };
        let phase_index = parse_segment(phase_part, StageNumberError::EmptyPhaseIndex)?;
        let seq = parse_segment(seq_part, StageNumberError::EmptySeq)?;
        Ok(StageNumberView {
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

    /// `.` の左側の整数値 (前置ゼロは落ちる)。
    #[must_use]
    pub const fn phase_index(&self) -> u32 {
        self.phase_index
    }

    /// `.` の右側の整数値 — フェーズ内の連番 (前置ゼロは落ちる)。
    #[must_use]
    pub const fn seq(&self) -> u32 {
        self.seq
    }

    /// `numericStageOrder` そのもの — phase → seq の**整数比較のみ**。
    ///
    /// 前置ゼロ違い (`"1.01"` と `"1.1"`) は **`Equal`** を返す (upstream の比較関数と同じ)。
    /// 数値順ソートは本比較 + **安定ソート**で行うこと — 同値は upstream 同様に文書順が残る。
    /// `Ord` は `Eq` との整合のために生表現で決着させる別物なので、数値順の経路に使わない。
    #[must_use]
    pub const fn numeric_cmp(&self, other: &StageNumberView) -> Ordering {
        if self.phase_index != other.phase_index {
            if self.phase_index < other.phase_index {
                return Ordering::Less;
            }
            return Ordering::Greater;
        }
        if self.seq != other.seq {
            if self.seq < other.seq {
                return Ordering::Less;
            }
            return Ordering::Greater;
        }
        Ordering::Equal
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

impl Ord for StageNumberView {
    /// `Eq` (生表現の等値) と整合する全順序。整数組で比較し、前置ゼロ違いは生表現で決着させる。
    ///
    /// **数値順の経路には使わない** — `numericStageOrder` の意味論は
    /// [`StageNumberView::numeric_cmp`] が担う。
    fn cmp(&self, other: &StageNumberView) -> Ordering {
        self.numeric_cmp(other)
            .then_with(|| self.raw.cmp(&other.raw))
    }
}

impl PartialOrd for StageNumberView {
    fn partial_cmp(&self, other: &StageNumberView) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn n(s: &str) -> StageNumberView {
        StageNumberView::parse(s).unwrap()
    }

    #[test]
    fn the_second_segment_is_compared_as_an_integer_not_lexically() {
        assert_eq!(n("1.10").numeric_cmp(&n("1.9")), Ordering::Greater);
        assert_eq!(n("3.7").numeric_cmp(&n("4.1")), Ordering::Less);
        assert!(n("1.10") > n("1.9"));
    }

    #[test]
    fn the_raw_representation_is_never_normalized() {
        let v = n("3.10");
        assert_eq!(v.as_str(), "3.10");
        assert_eq!((v.phase_index(), v.seq()), (3, 10));
        assert_eq!(n("1.01").seq(), 1);
        assert_ne!(n("1.01"), n("1.1"));
    }

    #[test]
    fn numeric_cmp_treats_leading_zero_variants_as_equal_while_ord_stays_total() {
        assert_eq!(n("1.01").numeric_cmp(&n("1.1")), Ordering::Equal);
        assert_ne!(n("1.01").cmp(&n("1.1")), Ordering::Equal);
    }

    #[test]
    fn rejects_shapes_that_are_not_phase_dot_seq() {
        assert_eq!(StageNumberView::parse(""), Err(StageNumberError::Empty));
        assert_eq!(
            StageNumberView::parse("one"),
            Err(StageNumberError::MalformedSeparator { dot_count: 0 })
        );
        assert_eq!(
            StageNumberView::parse("1.2.3"),
            Err(StageNumberError::MalformedSeparator { dot_count: 2 })
        );
        assert_eq!(
            StageNumberView::parse(".2"),
            Err(StageNumberError::EmptyPhaseIndex)
        );
        assert_eq!(
            StageNumberView::parse("1."),
            Err(StageNumberError::EmptySeq)
        );
        assert_eq!(
            StageNumberView::parse("-1.2"),
            Err(StageNumberError::NonDigit('-'))
        );
        assert_eq!(
            StageNumberView::parse("4294967296.0"),
            Err(StageNumberError::Overflow)
        );
    }

    #[test]
    fn every_rejection_renders_its_material() {
        let message = |raw: &str| StageNumberView::parse(raw).unwrap_err().to_string();
        assert_eq!(message(""), "stage number is empty");
        assert_eq!(message("1"), "stage number has 0 dots (expected 1)");
        assert_eq!(message(".1"), "stage number has an empty phase");
        assert_eq!(message("1."), "stage number has an empty sequence");
        assert_eq!(message("a.1"), "stage number has a non-digit: 'a'");
        assert_eq!(message("99999999999.1"), "stage number does not fit in u32");
    }

    proptest! {
        /// 正準表現ならパースは往復し、セグメントは逐語で読み出せる。
        #[test]
        fn parse_round_trips_canonical_numbers(p in 0u32..1000, s in 0u32..1000) {
            let raw = format!("{p}.{s}");
            let v = n(&raw);
            prop_assert_eq!(v.as_str(), raw.as_str());
            prop_assert_eq!((v.phase_index(), v.seq()), (p, s));
        }

        /// `numeric_cmp` は `(phaseIndex, seq)` の整数比較と一致する。
        #[test]
        fn numeric_cmp_agrees_with_integer_pair_comparison(
            p1 in 0u32..6, s1 in 0u32..40,
            p2 in 0u32..6, s2 in 0u32..40,
        ) {
            let a = n(&format!("{p1}.{s1}"));
            let b = n(&format!("{p2}.{s2}"));
            prop_assert_eq!(a.numeric_cmp(&b), (p1, s1).cmp(&(p2, s2)));
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
