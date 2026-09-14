//! `SnapshotCodekbUseCase` — 走査の直前に 2 つの世代の写しを取る。

use core_command_domain::workspace::{
    CodekbRepoId, CodekbScopePaths, CodekbSnapshot, CodekbSourceFingerprint,
};

use super::port::CodekbRepository;
use super::snapshot_codekb_error::SnapshotCodekbError;

/// 走査が拠って立つ 2 つの世代の写しを取る (`aidlc-utility codekb-snapshot`)。
///
/// # なぜコマンド側なのか
///
/// 写しはこの後の公開が突き合わせる **compare-and-swap の合言葉**である。遅れて届く
/// リードモデルから採ると、採った瞬間に古い値を掴みうるので、合言葉として成立しない
/// (`coding-rules/cqrs-boundaries.md` 規則 4 —「コマンド側の最新状態は常に集約から」)。
/// したがって世代は集約から取る。加えて、この動詞は**中断した公開に決着をつける**ので、
/// ストアを直しうる — 読むだけの動詞ではない。
///
/// # 3 手 — 再構成 → 集約が判断 → 保存
///
/// 畳む前の木から世代を採ると、次の compare-and-swap が存在しない状態を指してしまう。
/// そこでロック区間の先頭で集約を再構成し、中断した公開を抱えていれば
/// [`Codekb::settle_interrupted_publication`](core_command_domain::workspace::Codekb::settle_interrupted_publication)
/// で決着をつけ、その事実を `store` で確定させてから改めて観測する
/// (`coding-rules/cqrs-boundaries.md` 追補「リポジトリの使い方」)。抱えていなければ**何も
/// 書かずに**そのまま写しを取る。upstream `handleCodekbSnapshot` が `withCodekbLock` の下で
/// `recoverCodekbTransactions` を先に走らせるのと同じ順序である。
///
/// # ここに無いもの
///
/// - **源の指紋の実測**。git / 木のハッシュを採るのは合成ルートの仕事で、ここは受け取った値を
///   そのまま写しへ載せる (集約は外界を読まない)。
/// - **文言と描画**。`STORE_GENERATION …` の 3 行も JSON も出す側が組む。
#[derive(Debug)]
pub struct SnapshotCodekbUseCase<R: CodekbRepository> {
    codekb_repository: R,
}

impl<R: CodekbRepository> SnapshotCodekbUseCase<R> {
    /// ポートの実装を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(codekb_repository: R) -> SnapshotCodekbUseCase<R> {
        SnapshotCodekbUseCase { codekb_repository }
    }

    /// 写しを 1 つ取る。
    ///
    /// # Errors
    ///
    /// ストアを再構成できない、または中断した公開の決着を保存できなければ
    /// [`SnapshotCodekbError`] を返す。
    pub async fn execute(
        &mut self,
        id: &CodekbRepoId,
        paths: CodekbScopePaths,
        source_fingerprint: CodekbSourceFingerprint,
    ) -> Result<CodekbSnapshot, SnapshotCodekbError> {
        let mut codekb = self.codekb_repository.find_by_id(id).await?;
        if let Some(settlement) = codekb.settle_interrupted_publication() {
            self.codekb_repository.store(&settlement, &codekb).await?;
            // 畳んだあとの世代はディスクのバイトから観測する値なので、引き直す。
            codekb = self.codekb_repository.find_by_id(id).await?;
        }
        Ok(codekb.take_snapshot(paths, source_fingerprint))
    }

    /// 注入されたポートの実装（テストが**効果**を観測するための継ぎ目）。
    #[cfg(test)]
    pub(crate) const fn codekb_repository(&self) -> &R {
        &self.codekb_repository
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{InMemoryCodekbRepository, codekb_repo, scope_paths};
    use super::super::{RepositoryError, SnapshotCodekbError, SnapshotCodekbUseCase};
    use core_command_domain::workspace::{CodekbEvent, CodekbGeneration, CodekbSourceFingerprint};

    fn source() -> CodekbSourceFingerprint {
        CodekbSourceFingerprint::of_git("3c55f6af")
    }

    /// 写しは**実測したストアの世代**を運ぶ (リードモデルではなく集約から取る)。
    #[tokio::test]
    async fn the_snapshot_carries_the_generation_observed_on_the_store() {
        let mut use_case = SnapshotCodekbUseCase::new(InMemoryCodekbRepository::holding(
            CodekbGeneration::of_tree_hash("current"),
        ));
        let snapshot = use_case
            .execute(&codekb_repo(), scope_paths(&["src/"]), source())
            .await
            .expect("写しは取れる");
        assert_eq!(snapshot.store_generation().as_str(), "sha256:current");
        assert_eq!(snapshot.source_fingerprint().as_str(), "git:3c55f6af");
    }

    /// 中断した公開は**集約が畳み、`store` が確定させる** — 畳んだ**あと**の世代が写しに載る。
    /// 畳む前に読んでいたら `none` になり、次の compare-and-swap が存在しない状態を指す。
    #[tokio::test]
    async fn the_snapshot_settles_the_interrupted_publication_before_it_observes() {
        let mut use_case = SnapshotCodekbUseCase::new(InMemoryCodekbRepository::interrupted(
            CodekbGeneration::of_tree_hash("folded"),
        ));
        let snapshot = use_case
            .execute(&codekb_repo(), scope_paths(&["src/"]), source())
            .await
            .expect("写しは取れる");
        assert_eq!(
            snapshot.store_generation().as_str(),
            "sha256:folded",
            "畳む前の `none` ではなく、畳んだあとの世代を観測する"
        );
        assert!(
            matches!(
                use_case.codekb_repository().committed(),
                [CodekbEvent::InterruptedPublicationSettled(_)]
            ),
            "決着の事実がちょうど 1 件保存される"
        );
    }

    /// 畳むものが無ければ**何も書かない** — 空振りの書込を作らない。
    #[tokio::test]
    async fn a_store_without_an_interrupted_publication_is_not_written_to() {
        let mut use_case = SnapshotCodekbUseCase::new(InMemoryCodekbRepository::holding(
            CodekbGeneration::of_tree_hash("current"),
        ));
        drop(
            use_case
                .execute(&codekb_repo(), scope_paths(&["src/"]), source())
                .await
                .expect("写しは取れる"),
        );
        assert!(
            use_case.codekb_repository().committed().is_empty(),
            "畳むものが無いのに書込が起きてはならない"
        );
    }

    /// 決着を保存できなかったら、写しを取らずにポートの失敗を運ぶ。
    #[tokio::test]
    async fn a_failing_settlement_is_propagated_instead_of_a_snapshot() {
        let mut use_case = SnapshotCodekbUseCase::new(InMemoryCodekbRepository::unsettleable());
        let error = use_case
            .execute(&codekb_repo(), scope_paths(&["src/"]), source())
            .await
            .expect_err("畳めない");
        assert!(
            matches!(
                error,
                SnapshotCodekbError::Repository(RepositoryError::Io { .. })
            ),
            "{error:?}"
        );
    }

    /// ストアがまだ無くても写しは取れる — 不在も 1 つの世代である。
    #[tokio::test]
    async fn an_absent_store_still_yields_a_snapshot() {
        let mut use_case = SnapshotCodekbUseCase::new(InMemoryCodekbRepository::holding(
            CodekbGeneration::absent(),
        ));
        let snapshot = use_case
            .execute(&codekb_repo(), scope_paths(&["src/"]), source())
            .await
            .expect("不在でも取れる");
        assert_eq!(snapshot.store_generation().as_str(), "none");
    }

    /// 再構成そのものが失敗したら、ポートの失敗をそのまま運ぶ。
    #[tokio::test]
    async fn a_failing_port_is_propagated() {
        let mut use_case = SnapshotCodekbUseCase::new(InMemoryCodekbRepository::unreadable());
        let error = use_case
            .execute(&codekb_repo(), scope_paths(&["src/"]), source())
            .await
            .expect_err("引けない");
        assert!(
            matches!(
                error,
                SnapshotCodekbError::Repository(RepositoryError::Io { .. })
            ),
            "{error:?}"
        );
    }
}
