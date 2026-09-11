//! 最新の指示発行の事実。
use super::{CommandError, DirectivePublication, IntentId, PublishedDirective};
/// 実行の指示発行状態。公開ファイルの表現はRMUが決める。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveDirective {
    owner_session: String,
    owner_epoch: u64,
    context_epoch: u64,
    issuance_revision: u64,
    revision: u64,
    intent_id: IntentId,
    publication: DirectivePublication,
    initial_state_sha256: String,
}
impl ActiveDirective {
    /// この発行と共有承認の失効を対応付ける操作。
    #[must_use]
    pub const fn approval_operation_id(&self) -> Option<&super::PlanApprovalOperationId> {
        self.publication.approval_operation_id()
    }

    /// 完全な状態から組む。
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "保存された指示の完全な状態を唯一の構築口へ渡す"
    )]
    pub const fn new(
        revision: u64,
        intent_id: IntentId,
        publication: DirectivePublication,
        initial_state_sha256: String,
        owner_session: String,
        owner_epoch: u64,
        context_epoch: u64,
        issuance_revision: u64,
    ) -> Self {
        Self {
            issuance_revision,
            owner_session,
            owner_epoch,
            context_epoch,
            revision,
            intent_id,
            publication,
            initial_state_sha256,
        }
    }
    pub(crate) fn issue(
        previous: Option<&Self>,
        intent_id: &IntentId,
        publication: &DirectivePublication,
    ) -> Result<Self, CommandError> {
        let previous = previous.filter(|previous| {
            previous.intent_id == *intent_id
                && previous.project_sha256() == publication.project_sha256()
        });
        let revision = previous
            .map_or(0, Self::revision)
            .checked_add(1)
            .ok_or(CommandError::SequenceExhausted)?;
        let initial = previous.map_or_else(
            || publication.state_sha256().to_string(),
            |previous| previous.initial_state_sha256.clone(),
        );
        let publication = if publication.directive().stage().as_str() == "code-generation" {
            previous
                .filter(|previous| previous.directive().stage().as_str() == "code-generation")
                .and_then(Self::source_floor)
                .map_or_else(
                    || publication.clone(),
                    |floor| {
                        publication
                            .clone()
                            .with_source_floor(Some(floor.to_string()))
                    },
                )
        } else {
            publication.clone()
        };
        let owner = previous.map_or_else(
            || {
                format!(
                    "sessionless:{}",
                    publication
                        .project_sha256()
                        .chars()
                        .take(16)
                        .collect::<String>()
                )
            },
            |previous| previous.owner_session.clone(),
        );
        let owner_epoch = previous.map_or(0, Self::owner_epoch);
        let context_epoch = previous.map_or(0, Self::context_epoch);
        Ok(Self::new(
            revision,
            intent_id.clone(),
            publication,
            initial,
            owner,
            owner_epoch,
            context_epoch,
            revision,
        ))
    }
    /// 発火元と保存済みの文脈がすべて一致するか。
    #[must_use]
    pub fn matches_context(&self, request: &super::DirectiveContextInvalidation) -> bool {
        !request.session().is_empty()
            && self.owner_session() == request.session()
            && self.intent_id() == request.intent_id()
            && self.project_sha256() == request.project_sha256()
            && self.state_sha256() == request.state_sha256()
    }
    pub(crate) fn invalidate_context(
        &self,
        operation: &super::PlanApprovalOperationId,
        request: &super::DirectiveContextInvalidation,
    ) -> Result<Self, CommandError> {
        if !self.matches_context(request) {
            return Err(CommandError::PlanResponseUnavailable);
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(CommandError::SequenceExhausted)?;
        let epoch = self
            .context_epoch
            .checked_add(1)
            .ok_or(CommandError::SequenceExhausted)?;
        let publication = DirectivePublication::new(
            self.project_sha256().to_string(),
            self.state_sha256().to_string(),
            PublishedDirective::Error {
                stage: self.directive().stage().clone(),
            },
        )
        .with_source_floor(self.source_floor().map(str::to_string))
        .with_approval_operation(Some(operation.clone()));
        Ok(Self::new(
            revision,
            self.intent_id.clone(),
            publication,
            self.initial_state_sha256.clone(),
            self.owner_session.clone(),
            self.owner_epoch,
            epoch,
            self.issuance_revision,
        ))
    }
    /// 発行を所有するセッション。通常Claudeの未取得所有者は sessionless を保つ。
    #[must_use]
    pub fn owner_session(&self) -> &str {
        &self.owner_session
    }
    /// 所有権の世代。
    #[must_use]
    pub const fn owner_epoch(&self) -> u64 {
        self.owner_epoch
    }
    /// 文脈が失効するたびに増える世代。
    #[must_use]
    pub const fn context_epoch(&self) -> u64 {
        self.context_epoch
    }
    /// 最後に作業指示を発行した回数。文脈失効では更新しない。
    #[must_use]
    pub const fn issuance_revision(&self) -> u64 {
        self.issuance_revision
    }
    /// 発行回数。
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    /// 発行対象の依頼。
    #[must_use]
    pub const fn intent_id(&self) -> &IntentId {
        &self.intent_id
    }
    /// プロジェクトの照合子。
    #[must_use]
    pub fn project_sha256(&self) -> &str {
        self.publication.project_sha256()
    }
    /// 発行時の状態本文。
    #[must_use]
    pub fn state_sha256(&self) -> &str {
        self.publication.state_sha256()
    }
    /// 最初の発行時の状態本文。
    #[must_use]
    pub fn initial_state_sha256(&self) -> &str {
        &self.initial_state_sha256
    }
    /// 計画承認のソース基準。
    #[must_use]
    pub fn source_floor(&self) -> Option<&str> {
        self.publication.source_floor()
    }
    /// 実際に発行した指示。
    #[must_use]
    pub const fn directive(&self) -> &PublishedDirective {
        self.publication.directive()
    }
}
