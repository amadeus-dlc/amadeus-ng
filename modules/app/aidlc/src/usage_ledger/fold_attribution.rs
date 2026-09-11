//! 1 行をどこへ算入するか — ステージ・session・作業の 3 つ組。
//!
//! 保留した群は保留時の帰属を捕まえておき、後で畳むときに当時の所属へ算入する
//! （本家 `foldFileIntoLedger` の `captured?.stageSlug ?? stageSlug` ほか）。

/// 畳み込み 1 回分の帰属。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoldAttribution {
    stage_slug: Option<String>,
    session_key: String,
    workflow_key: String,
}

impl FoldAttribution {
    /// 3 つの鍵から組む。ステージは状態ファイルが無ければ `None`。
    pub(crate) const fn new(
        stage_slug: Option<String>,
        session_key: String,
        workflow_key: String,
    ) -> Self {
        Self {
            stage_slug,
            session_key,
            workflow_key,
        }
    }

    /// `byStage` の鍵。
    pub(crate) fn stage_slug(&self) -> Option<&str> {
        self.stage_slug.as_deref()
    }

    /// `sessions` の鍵（`transcript:<path>`）。
    pub(crate) fn session_key(&self) -> &str {
        &self.session_key
    }

    /// `workflows` の鍵（`intent:<uuid>` または `record:<space>/<dir>`）。
    pub(crate) fn workflow_key(&self) -> &str {
        &self.workflow_key
    }

    /// 保留時に捕まえた帰属（`self`）を、いま畳むときの帰属に重ねる。
    ///
    /// session・作業は保留時の値を使う。ステージだけは保留時に無ければいまの値へ落ちる
    /// （本家の `??` がそう振る舞う）。
    pub(crate) fn captured_over(&self, current: &Self) -> Self {
        Self::new(
            self.stage_slug
                .clone()
                .or_else(|| current.stage_slug.clone()),
            self.session_key.clone(),
            self.workflow_key.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_captured_attribution_keeps_its_keys_but_borrows_a_missing_stage() {
        let captured = FoldAttribution::new(None, "s-old".into(), "w-old".into());
        let current = FoldAttribution::new(Some("stage".into()), "s-new".into(), "w-new".into());
        assert_eq!(
            captured.captured_over(&current),
            FoldAttribution::new(Some("stage".into()), "s-old".into(), "w-old".into())
        );
        let stamped = FoldAttribution::new(Some("old".into()), "s-old".into(), "w-old".into());
        assert_eq!(stamped.captured_over(&current).stage_slug(), Some("old"));
    }
}
