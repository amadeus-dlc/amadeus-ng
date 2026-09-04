//! `NextAnswerDao` の実 Gateway — `next` の答え 1 行を `read_next_answer` から引く。

use std::rc::Rc;

use core_query_use_case::orchestration::{NextAnswerDao, NextAnswerView, ReadModelReadError};

use super::read_model_store::ReadModelStore;

/// 自然キー (`execution_id`, `request_kind` — UNIQUE 索引 `read_next_answer_key`) の 1 行引当。
///
/// 引くのは**この 1 表だけ**である。実行の面も run-stage の材料も結合せず、行が持つ FK 列
/// (`execution_id` / `run_stage_id`) をそのまま返す — たどるのはユースケースの仕事である
/// (オーナー裁定 2026-09-03 — `coding-rules/cqrs-boundaries.md` 規則 6)。
///
/// とりわけ `stage_slug` から `read_run_stage` を引き直してはならない。RMU は決定が
/// run-stage のときだけ `run_stage_id` を書くので、park や不整合 2 形 (`stage_slug` は
/// 非 NULL) で引き直すと**行に無い関連**を作ることになる。
const SELECT_NEXT_ANSWER: &str = "SELECT decision_kind, stage_index, stage_slug, gated, checkbox, \
execution_id, run_stage_id \
FROM read_next_answer WHERE execution_id = ?1 AND request_kind = ?2";

/// `read_next_answer` の 1 行を返す実装。
#[derive(Debug)]
pub struct NextAnswerDaoImpl {
    store: Rc<ReadModelStore>,
}

impl NextAnswerDaoImpl {
    /// 1 要求ぶんの共有ストアを受け取る (**この型の唯一の構築経路**)。
    ///
    /// 開くのは [`super::ReadModelDaos`] 1 か所で、12 実装はその 1 接続を分け合う。
    /// 実装ごとに開くと、多段の引当が別々のスナップショットを見る余地が残る。
    #[must_use]
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> NextAnswerDaoImpl {
        NextAnswerDaoImpl { store }
    }
}

impl NextAnswerDao for NextAnswerDaoImpl {
    fn find(
        &self,
        execution_id: &str,
        request_kind: &str,
    ) -> Result<Option<NextAnswerView>, ReadModelReadError> {
        self.store
            .find_one(SELECT_NEXT_ANSWER, &[&execution_id, &request_kind], |row| {
                Ok(NextAnswerView::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            })
    }
}
