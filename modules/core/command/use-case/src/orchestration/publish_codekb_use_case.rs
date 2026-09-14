//! `PublishCodekbUseCase` — compare-and-swap を確かめて候補を公開する。

use core_command_domain::workspace::{
    CodekbCandidate, CodekbGeneration, CodekbRepoId, CodekbSourceFingerprint,
};

use super::port::CodekbRepository;
use super::publish_codekb_error::PublishCodekbError;

/// 候補を公開する (`aidlc-utility codekb-publish`)。
///
/// 定型は他の書込ユースケースと同じ 3 手 — **`find_by_id` で集約を再構成 → 集約コマンドで
/// 判断 → `store` で保存** (`coding-rules/cqrs-boundaries.md` 追補「リポジトリの使い方」) —
/// で、中断した公開が残っていればその決着が同じ 3 手で 1 巡先に入る。
///
/// # 順序 — 決着 → 再構成 → 判断 → 保存
///
/// 畳む前の木から世代を採ると compare-and-swap が存在しない状態を指すので、ロック区間の
/// 先頭で集約を再構成し、中断した公開を抱えていれば
/// [`Codekb::settle_interrupted_publication`](core_command_domain::workspace::Codekb::settle_interrupted_publication)
/// で決着をつけ、その事実を `store` で確定させてから世代を観測し直す。抱えていなければ
/// **何も書かずに**公開の判断へ進む。upstream `handleCodekbPublish` が `withCodekbLock` の
/// 下で `recoverCodekbTransactions` を先に走らせるのと同じ順序である。
///
/// # ここに無いもの
///
/// - **業務判断**。3 つの compare-and-swap を突き合わせるのは
///   [`Codekb::publish`](core_command_domain::workspace::Codekb::publish) である。
/// - **候補の組み立てと被覆の検査**。staged の 9 成果物を読んで候補にするのも、snapshot の
///   範囲が候補の主張を覆うか見るのも、ロックを取る前に合成ルートが済ませる (upstream も
///   ロックの外で検査する)。
/// - **実測**。いまの源の指紋と候補の指紋は、外界を読める側が採って引数で渡す。
///
/// # 成功は `Ok(())` である (CQS)
///
/// 公開後の世代は**ディスクのバイトから観測する値**であり、集約もユースケースも名乗れない。
/// `PUBLISHED <dir> <世代>` の行に要る世代は、合成ルートが公開後に改めて観測する。
#[derive(Debug)]
pub struct PublishCodekbUseCase<R: CodekbRepository> {
    codekb_repository: R,
}

impl<R: CodekbRepository> PublishCodekbUseCase<R> {
    /// ポートの実装を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(codekb_repository: R) -> PublishCodekbUseCase<R> {
        PublishCodekbUseCase { codekb_repository }
    }

    /// 候補を 1 回公開する。
    ///
    /// `current_source` / `current_candidate_fingerprint` が `None` なのは「いま計算できない」
    /// という観測であり、集約はそれを「違う」として扱う。
    ///
    /// # Errors
    ///
    /// 集約が拒んだ (`Refused`)、ストアの再構成・永続化 (中断した公開の決着を含む) に失敗した
    /// (`Repository`)。
    pub async fn execute(
        &mut self,
        id: &CodekbRepoId,
        candidate: CodekbCandidate,
        expected_store: &CodekbGeneration,
        expected_source: &CodekbSourceFingerprint,
        current_source: Option<&CodekbSourceFingerprint>,
        current_candidate_fingerprint: Option<&str>,
    ) -> Result<(), PublishCodekbError> {
        let mut codekb = self.codekb_repository.find_by_id(id).await?;
        if let Some(settlement) = codekb.settle_interrupted_publication() {
            self.codekb_repository.store(&settlement, &codekb).await?;
            // 畳んだあとの世代はディスクのバイトから観測する値なので、引き直す。
            codekb = self.codekb_repository.find_by_id(id).await?;
        }
        let event = codekb
            .publish(
                candidate,
                expected_store,
                expected_source,
                current_source,
                current_candidate_fingerprint,
            )
            .map_err(PublishCodekbError::Refused)?;
        self.codekb_repository.store(&event, &codekb).await?;
        Ok(())
    }

    /// 注入されたポートの実装（テストが**効果**を観測するための継ぎ目）。
    #[cfg(test)]
    pub(crate) const fn codekb_repository(&self) -> &R {
        &self.codekb_repository
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]

    use super::super::test_support::{InMemoryCodekbRepository, candidate, codekb_repo};
    use super::super::{PublishCodekbError, PublishCodekbUseCase, RepositoryError};
    use core_command_domain::workspace::{
        CodekbEvent, CodekbGeneration, CodekbPublishRefusal, CodekbSourceFingerprint,
    };

    fn source() -> CodekbSourceFingerprint {
        CodekbSourceFingerprint::of_git("3c55f6af")
    }

    /// 3 手 — 再構成 → 集約の判断 → 保存。成功は `Ok(())` である (CQS)。
    #[tokio::test]
    async fn a_matching_compare_and_swap_stores_one_event() {
        let mut use_case = PublishCodekbUseCase::new(InMemoryCodekbRepository::holding(
            CodekbGeneration::absent(),
        ));
        use_case
            .execute(
                &codekb_repo(),
                candidate(Some("3c55f6af")),
                &CodekbGeneration::absent(),
                &source(),
                Some(&source()),
                Some("3c55f6af"),
            )
            .await
            .expect("噛み合えば公開できる");
        assert_eq!(use_case.codekb_repository().committed().len(), 1);
    }

    /// 公開も**中断した公開の決着から始まる** — 畳んだあとの世代で compare-and-swap が
    /// 噛み合う。畳む前に読んでいたら `none` を掴んで拒否になる。
    #[tokio::test]
    async fn the_publication_settles_the_interrupted_publication_before_it_reads_the_store() {
        let mut use_case = PublishCodekbUseCase::new(InMemoryCodekbRepository::interrupted(
            CodekbGeneration::of_tree_hash("folded"),
        ));
        use_case
            .execute(
                &codekb_repo(),
                candidate(Some("3c55f6af")),
                &CodekbGeneration::of_tree_hash("folded"),
                &source(),
                Some(&source()),
                Some("3c55f6af"),
            )
            .await
            .expect("畳んだあとの世代と噛み合う");
        assert!(
            matches!(
                use_case.codekb_repository().committed(),
                [
                    CodekbEvent::InterruptedPublicationSettled(_),
                    CodekbEvent::Published(_)
                ]
            ),
            "決着 → 公開の順に 2 件保存される"
        );
    }

    /// 決着を保存できなかったら、公開の判断にも保存にもいかない。
    #[tokio::test]
    async fn a_failing_settlement_stores_nothing() {
        let mut use_case = PublishCodekbUseCase::new(InMemoryCodekbRepository::unsettleable());
        let error = use_case
            .execute(
                &codekb_repo(),
                candidate(Some("3c55f6af")),
                &CodekbGeneration::absent(),
                &source(),
                Some(&source()),
                Some("3c55f6af"),
            )
            .await
            .expect_err("決着を保存できない");
        assert!(
            matches!(
                error,
                PublishCodekbError::Repository(RepositoryError::Io { .. })
            ),
            "{error:?}"
        );
        assert!(use_case.codekb_repository().committed().is_empty());
    }

    /// 集約が拒んだら**何も保存しない** — 拒否はそのまま伝播する。
    #[tokio::test]
    async fn a_refused_publication_stores_nothing() {
        let mut use_case = PublishCodekbUseCase::new(InMemoryCodekbRepository::holding(
            CodekbGeneration::of_tree_hash("current"),
        ));
        let error = use_case
            .execute(
                &codekb_repo(),
                candidate(Some("3c55f6af")),
                &CodekbGeneration::of_token("sha256:stale").expect("合言葉"),
                &source(),
                Some(&source()),
                Some("3c55f6af"),
            )
            .await
            .expect_err("世代が違えば拒否");
        assert!(
            matches!(
                error,
                PublishCodekbError::Refused(CodekbPublishRefusal::StoreChanged { .. })
            ),
            "{error:?}"
        );
        assert!(use_case.codekb_repository().committed().is_empty());
    }

    /// 保存が失敗したら、ポートの失敗をそのまま運ぶ。
    #[tokio::test]
    async fn a_failing_store_is_propagated() {
        let mut use_case = PublishCodekbUseCase::new(InMemoryCodekbRepository::unwritable(
            CodekbGeneration::absent(),
        ));
        let error = use_case
            .execute(
                &codekb_repo(),
                candidate(Some("3c55f6af")),
                &CodekbGeneration::absent(),
                &source(),
                Some(&source()),
                Some("3c55f6af"),
            )
            .await
            .expect_err("書けない");
        assert!(
            matches!(
                error,
                PublishCodekbError::Repository(RepositoryError::Io { .. })
            ),
            "{error:?}"
        );
    }
}
