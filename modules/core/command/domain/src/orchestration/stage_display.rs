//! `StageDisplay` — 解決済み計画の**表示属性**（ステージ番号・表題・担当エージェント）。

use crate::workflow_definition::StageNumber;
use crate::workspace::{StateFieldValue, UnsafeLineChar};

/// 1 ステージ分の表示属性 — リードモデルを描くのに要る 3 値。
///
/// # なぜ定義ではなくイベントが運ぶのか
///
/// 投影（ReadModelUpdater）は**ジャーナルだけ**を材料にリードモデルを描けなければならない。
/// クラッシュ後の再構成で当時と同一のバイトを得るには、当時の値がジャーナルに残っている必要が
/// あるためである（NFR3）。ところがこの 3 値は `StageNode`（ワークフロー定義）側の材料であり、
/// 定義は版が上がれば変わる — 投影のたびに定義を引くと、**過去のイベントを今の定義で描く**
/// ことになり、再構成が当時と一致しなくなる。
///
/// したがって `Started` が計画を解決した時点の値を事実として運ぶ（オーナー裁定 2026-08-29）。
/// ADR-008 の「定義を間接参照し詳細を複製しない」は**定義全体の複製**を禁じたものであり、
/// 解決済み計画の表示属性はその限定的な例外である — 運ぶのは描画に要る 3 値だけで、
/// `consumes` / `produces` / `sensors` といった定義の本体は依然としてイベントに載らない。
///
/// # 3 値がどこに現れるか（upstream 実バイト）
///
/// - `number` — `**Details**: FORWARD jump from … to domain-design (2.6). …`（`STAGE_JUMPED`）と、
///   状態ファイルの `- **Stages to Execute**: 0.1, 0.2, …` / `- **Stages to Skip**: 1.1 (intent-capture), …`
/// - `name` — 状態ファイルの `- **Next Action**: Execute Refined Mockups`
/// - `lead_agent` — `**Agent**: aidlc-design-agent`（`STAGE_STARTED`）と、状態ファイルの
///   `- **Active Agent**: aidlc-design-agent`
///
/// 表題と担当は**単一行**であることが型で保証される（状態ファイルの bullet 行に書くので、
/// 改行が混ざると 2 行目以降がフィールドとして読めなくなる）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageDisplay {
    number: StageNumber,
    name: StateFieldValue,
    lead_agent: StateFieldValue,
}

impl StageDisplay {
    /// 表示属性 3 点を束ねる（単一行検査つきの唯一の構成関数）。
    ///
    /// # Errors
    ///
    /// 表題または担当に行終端・制御文字が混ざっていれば `UnsafeLineChar`。
    pub fn new(
        number: StageNumber,
        name: &str,
        lead_agent: &str,
    ) -> Result<StageDisplay, UnsafeLineChar> {
        Ok(StageDisplay {
            number,
            name: StateFieldValue::parse(name)?,
            lead_agent: StateFieldValue::parse(lead_agent)?,
        })
    }

    /// ステージ番号（`0.1` / `2.6` — `<phase>.<seq>`）。
    #[must_use]
    pub const fn number(&self) -> &StageNumber {
        &self.number
    }

    /// 表示名（著者の `name:` が無ければ slug の title case）。
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// このステージを率いるエージェント（`orchestrator` / `aidlc-design-agent`）。
    #[must_use]
    pub fn lead_agent(&self) -> &str {
        self.lead_agent.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(raw: &str) -> StageNumber {
        StageNumber::parse(raw).expect("テストのステージ番号は文法内")
    }

    fn display(name: &str, agent: &str) -> Result<StageDisplay, UnsafeLineChar> {
        StageDisplay::new(number("2.6"), name, agent)
    }

    #[test]
    fn the_display_carries_the_three_values_the_read_model_needs() {
        let shown = display("Refined Mockups", "aidlc-design-agent").expect("単一行");
        assert_eq!(shown.number().as_str(), "2.6");
        assert_eq!(shown.name(), "Refined Mockups");
        assert_eq!(shown.lead_agent(), "aidlc-design-agent");
    }

    #[test]
    fn a_multiline_name_cannot_be_constructed() {
        // 状態ファイルの bullet 行に書く値なので、改行が混ざると 2 行目以降が読めなくなる。
        assert_eq!(
            display("Refined\nMockups", "aidlc-design-agent"),
            Err(UnsafeLineChar::new('\n'))
        );
    }

    #[test]
    fn a_multiline_agent_cannot_be_constructed() {
        assert_eq!(
            display("Refined Mockups", "agent\u{2028}forged"),
            Err(UnsafeLineChar::new('\u{2028}'))
        );
    }

    #[test]
    fn the_orchestrator_is_a_legal_agent_name() {
        // initialization の 3 ステージは `orchestrator` が率いる（upstream 実バイト）。
        let shown = display("Workspace Scaffold", "orchestrator").expect("単一行");
        assert_eq!(shown.lead_agent(), "orchestrator");
    }

    #[test]
    fn displays_compare_by_value() {
        let a = display("Refined Mockups", "aidlc-design-agent").expect("単一行");
        let b = display("Refined Mockups", "aidlc-design-agent").expect("単一行");
        let c = display("Domain Design", "aidlc-design-agent").expect("単一行");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
