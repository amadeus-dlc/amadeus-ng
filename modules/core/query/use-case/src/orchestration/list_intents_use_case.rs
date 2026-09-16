//! `ListIntentsUseCase` — 空間の依頼一覧を引く (upstream `listIntents` / `printIntentListing`)。

use crate::orchestration::{IntentListingDao, IntentListingView, ReadModelReadError};

/// 空間の依頼一覧を引く。
///
/// 活動中の記録ディレクトリ名は**境界で解決して渡す** — カーソルの所在も優先順位も合成
/// ルートが握っており、ここが設定ソースを問い合わせることはない
/// (`coding-rules/tell-dont-ask.md`)。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListIntentsUseCase<D: IntentListingDao> {
    intent_listing_dao: D,
}

impl<D: IntentListingDao> ListIntentsUseCase<D> {
    /// 引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(intent_listing_dao: D) -> ListIntentsUseCase<D> {
        ListIntentsUseCase { intent_listing_dao }
    }

    /// 一覧を引き、渡された活動中の記録ディレクトリ名と束ねる。
    ///
    /// # Errors
    ///
    /// 一覧を引けない ([`ReadModelReadError`])。
    pub fn execute(&self, active: Option<&str>) -> Result<IntentListingView, ReadModelReadError> {
        Ok(IntentListingView::new(
            active.map(str::to_string),
            self.intent_listing_dao.find()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use crate::orchestration::IntentListingRowView;

    struct Rows(Vec<IntentListingRowView>);

    impl IntentListingDao for Rows {
        fn find(&self) -> Result<Vec<IntentListingRowView>, ReadModelReadError> {
            Ok(self.0.clone())
        }
    }

    fn row(directory: Option<&str>) -> IntentListingRowView {
        IntentListingRowView::new(
            "uuid".to_string(),
            "slug".to_string(),
            "active".to_string(),
            Vec::new(),
            directory.map(str::to_string),
        )
    }

    /// カーソルが指す記録だけが活動中になる。
    #[test]
    fn only_the_record_the_cursor_names_is_active() {
        let use_case = Rows(vec![row(Some("a-1")), row(Some("b-2"))]);
        let view = ListIntentsUseCase::new(use_case)
            .execute(Some("b-2"))
            .expect("一覧");
        assert_eq!(view.active(), Some("b-2"));
        assert!(!view.is_active(&row(Some("a-1"))));
        assert!(view.is_active(&row(Some("b-2"))));
    }

    /// 記録ディレクトリを持たない行は、カーソルが不在でも活動中にならない。
    #[test]
    fn a_row_without_a_record_directory_is_never_active() {
        let view = ListIntentsUseCase::new(Rows(vec![row(None)]))
            .execute(None)
            .expect("一覧");
        assert_eq!(view.active(), None);
        assert!(!view.is_active(&row(None)));
    }
}
