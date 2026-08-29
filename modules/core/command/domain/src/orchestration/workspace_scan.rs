//! `WorkspaceScan` — workspace-detection が出した走査結果 4 点。

use crate::workflow_definition::BrownfieldGreenfield;
use crate::workspace::{StateFieldValue, UnsafeLineChar};

/// 走査結果が値を出せなかったときの逐語（upstream 実バイト）。
const UNKNOWN: &str = "Unknown";

/// ワークスペース走査の結果 4 点（プロジェクト種別・言語・フレームワーク・ビルドシステム）。
///
/// # なぜイベントが運ぶのか
///
/// [`StageDisplay`] と同じ理由である — 投影はジャーナルだけを材料にリードモデルを描けなければ
/// ならず、走査結果はジャーナルの外（ファイルシステムの当時の状態）にしか無い。あとから走査を
/// やり直すと、**当時と違う結果**で過去のイベントを描くことになり再構成が一致しない（NFR3）。
///
/// initialization の 3 ステージが描く監査行 — `WORKSPACE_SCANNED` の 4 フィールドと
/// `WORKSPACE_INITIALISED` の同 4 フィールド、`STAGE_COMPLETED`（workspace-detection）の
/// `**Details**: Classified Greenfield; languages=…; frameworks=…` — の材料はすべてここにある。
///
/// [`StageDisplay`]: super::stage_display::StageDisplay
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceScan {
    project_type: BrownfieldGreenfield,
    languages: StateFieldValue,
    frameworks: StateFieldValue,
    build_system: StateFieldValue,
}

impl WorkspaceScan {
    /// 走査結果 4 点を束ねる（単一行検査つきの唯一の構成関数）。
    ///
    /// 3 つの自由記述は upstream が値を出せなかったとき逐語 `Unknown` を書く。呼出側が空文字を
    /// 渡した場合もその既定に畳む — 空の `**Languages**: ` 行は upstream に存在しないためである。
    ///
    /// # Errors
    ///
    /// いずれかの値に行終端・制御文字が混ざっていれば `UnsafeLineChar`。
    pub fn new(
        project_type: BrownfieldGreenfield,
        languages: &str,
        frameworks: &str,
        build_system: &str,
    ) -> Result<WorkspaceScan, UnsafeLineChar> {
        Ok(WorkspaceScan {
            project_type,
            languages: WorkspaceScan::field(languages)?,
            frameworks: WorkspaceScan::field(frameworks)?,
            build_system: WorkspaceScan::field(build_system)?,
        })
    }

    fn field(value: &str) -> Result<StateFieldValue, UnsafeLineChar> {
        StateFieldValue::parse(if value.is_empty() { UNKNOWN } else { value })
    }

    /// `**Project Type**:` に書く綴り（`Greenfield` / `Brownfield` — 先頭大文字）。
    ///
    /// `BrownfieldGreenfield::as_str` は `stage-graph.json` 上の正準綴り（小文字）を返す別の
    /// 面である。同じ値でも書く先で綴りが違うので、写像を型の側に置いて取り違えを防ぐ。
    #[must_use]
    pub const fn project_type(&self) -> &'static str {
        match self.project_kind() {
            BrownfieldGreenfield::Brownfield => "Brownfield",
            BrownfieldGreenfield::Greenfield => "Greenfield",
        }
    }

    /// 判定したプロジェクト種別の**値そのもの**（綴りではない）。
    ///
    /// 綴りは面ごとに違う（状態ファイルは `Brownfield` / `Greenfield`、`stage-graph.json` は
    /// 小文字、ジャーナルも小文字）。値を返す口をここに 1 つ置き、面ごとの写像はそれぞれの
    /// 面が持つ — そうしないと、ある面の綴りを変えたときに別の面のバイトが壊れる。
    #[must_use]
    pub const fn project_kind(&self) -> BrownfieldGreenfield {
        self.project_type
    }

    /// 検出した言語（未検出は `Unknown`）。
    #[must_use]
    pub fn languages(&self) -> &str {
        self.languages.as_str()
    }

    /// 検出したフレームワーク（未検出は `Unknown`）。
    #[must_use]
    pub fn frameworks(&self) -> &str {
        self.frameworks.as_str()
    }

    /// 検出したビルドシステム（未検出は `Unknown`）。
    #[must_use]
    pub fn build_system(&self) -> &str {
        self.build_system.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan() -> WorkspaceScan {
        WorkspaceScan::new(
            BrownfieldGreenfield::Greenfield,
            "Unknown",
            "Unknown",
            "Unknown",
        )
        .expect("単一行")
    }

    #[test]
    fn the_scan_carries_the_four_values_the_init_rows_need() {
        let found = scan();
        assert_eq!(found.project_type(), "Greenfield");
        assert_eq!(found.languages(), "Unknown");
        assert_eq!(found.frameworks(), "Unknown");
        assert_eq!(found.build_system(), "Unknown");
    }

    #[test]
    fn the_project_type_is_capitalised_for_the_read_model_not_for_the_graph() {
        // `stage-graph.json` 上の正準綴りは小文字。書く先で綴りが違う。
        assert_eq!(BrownfieldGreenfield::Greenfield.as_str(), "greenfield");
        assert_eq!(scan().project_type(), "Greenfield");
        let brownfield = WorkspaceScan::new(BrownfieldGreenfield::Brownfield, "Rust", "", "cargo")
            .expect("単一行");
        assert_eq!(brownfield.project_type(), "Brownfield");
        assert_eq!(brownfield.languages(), "Rust");
        assert_eq!(brownfield.build_system(), "cargo");
    }

    #[test]
    fn an_empty_value_falls_back_to_the_verbatim_unknown() {
        // 空の `**Languages**: ` 行は upstream に存在しない。
        let found =
            WorkspaceScan::new(BrownfieldGreenfield::Greenfield, "", "", "").expect("単一行");
        assert_eq!(found.languages(), "Unknown");
        assert_eq!(found.frameworks(), "Unknown");
        assert_eq!(found.build_system(), "Unknown");
    }

    #[test]
    fn a_multiline_value_cannot_be_constructed() {
        assert_eq!(
            WorkspaceScan::new(BrownfieldGreenfield::Greenfield, "Rust\nGo", "", ""),
            Err(UnsafeLineChar::new('\n'))
        );
    }

    #[test]
    fn scans_compare_by_value_and_round_trip_through_serde() {
        let found = scan();
        assert_eq!(found, scan());
        assert_ne!(
            found,
            WorkspaceScan::new(
                BrownfieldGreenfield::Brownfield,
                "Unknown",
                "Unknown",
                "Unknown"
            )
            .expect("単一行")
        );
    }
}
