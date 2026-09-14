//! 観測から 1 回の自己診断を実施し、その事実を保存する。
//!
//! 判断 (D1.a〜D5.b の成否・ラベル・修復案) は集約 `WorkspaceDoctor` が所有する。ここが
//! 持つのは規則 5 の正規形だけである — `find_by_id` で集約を取得 (無ければ誕生) →
//! 集約のコマンドを呼ぶ → 生成された 1 事実を `store` で保存する
//! (`coding-rules/cqrs-boundaries.md`「リポジトリの使い方」)。
use super::{RepositoryError, WorkspaceDoctorCommandError, WorkspaceDoctorRepository};
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{DoctorObservation, HookHealthTarget, WorkspaceDoctor};
/// 観測を評価して診断事実を保存する更新ユースケース。
#[derive(Debug)]
pub struct DiagnoseWorkspaceUseCase<R: WorkspaceDoctorRepository> {
    repository: R,
}
impl<R: WorkspaceDoctorRepository> DiagnoseWorkspaceUseCase<R> {
    /// 診断集約の保存ポートを注入する。
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
    /// 診断を 1 回実施して保存する。
    ///
    /// 何も返さない — 行はリードモデル側 (RMU の投影をクエリ側が引く) が持つ
    /// (`coding-rules/command-query-separation.md`)。読み手は診断対象から鍵を導ける。
    /// # Errors
    /// 集約の拒否、保存・再構成の失敗。
    pub async fn execute(
        &mut self,
        target: &HookHealthTarget,
        observation: &DoctorObservation,
        at: DateTime<Utc>,
    ) -> Result<(), WorkspaceDoctorCommandError> {
        let (doctor, event) = match self
            .repository
            .find_by_id(&WorkspaceDoctor::id_for(target))
            .await
        {
            Ok(mut doctor) => {
                let event = doctor
                    .diagnose(observation, at)
                    .map_err(WorkspaceDoctorCommandError::Domain)?;
                (doctor, event)
            }
            Err(RepositoryError::NotFound { .. }) => {
                WorkspaceDoctor::start(target.clone(), observation, at)
                    .map_err(WorkspaceDoctorCommandError::Domain)?
            }
            Err(error) => return Err(WorkspaceDoctorCommandError::Repository(error)),
        };
        self.repository
            .store(&event, &doctor)
            .await
            .map_err(WorkspaceDoctorCommandError::Repository)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]
    use super::*;
    use crate::orchestration::RepositoryError;
    use crate::orchestration::test_support::{
        InMemoryWorkspaceDoctorRepository, at, doctor_observation, doctor_target,
    };

    fn bun_row_is_passed(doctor: &WorkspaceDoctor) -> bool {
        doctor
            .checks()
            .as_slice()
            .iter()
            .any(|check| check.label().starts_with("bun ") && check.is_passed())
    }

    #[tokio::test]
    async fn the_first_diagnosis_starts_the_aggregate_and_stores_one_fact() {
        let mut use_case =
            DiagnoseWorkspaceUseCase::new(InMemoryWorkspaceDoctorRepository::empty());
        use_case
            .execute(&doctor_target(), &doctor_observation(true), at())
            .await
            .unwrap();
        let id = WorkspaceDoctor::id_for(&doctor_target());
        assert_eq!(use_case.repository.stored().len(), 1);
        let held = use_case.repository.held(&id).unwrap();
        assert_eq!(held.seq_nr(), 1);
        assert_eq!(held.checks(), use_case.repository.stored()[0].checks());
        assert!(bun_row_is_passed(held));
    }

    #[tokio::test]
    async fn a_later_diagnosis_advances_the_existing_aggregate() {
        let (doctor, _) =
            WorkspaceDoctor::start(doctor_target(), &doctor_observation(true), at()).unwrap();
        let mut use_case =
            DiagnoseWorkspaceUseCase::new(InMemoryWorkspaceDoctorRepository::holding(doctor));
        let later = at() + chrono::Duration::seconds(5);
        use_case
            .execute(&doctor_target(), &doctor_observation(false), later)
            .await
            .unwrap();
        let held = use_case
            .repository
            .held(&WorkspaceDoctor::id_for(&doctor_target()))
            .unwrap();
        assert_eq!(held.seq_nr(), 2);
        assert_eq!(held.diagnosed_at(), later);
        assert!(!bun_row_is_passed(held));
        assert_eq!(
            use_case.repository.stored().len(),
            1,
            "1 コマンド 1 イベント"
        );
    }

    #[tokio::test]
    async fn every_run_writes_so_the_use_case_never_only_reads() {
        let mut use_case =
            DiagnoseWorkspaceUseCase::new(InMemoryWorkspaceDoctorRepository::empty());
        for step in 0..3_i64 {
            use_case
                .execute(
                    &doctor_target(),
                    &doctor_observation(true),
                    at() + chrono::Duration::seconds(step),
                )
                .await
                .unwrap();
        }
        assert_eq!(use_case.repository.stored().len(), 3);
        let id = WorkspaceDoctor::id_for(&doctor_target());
        assert_eq!(use_case.repository.held(&id).unwrap().seq_nr(), 3);
    }

    #[tokio::test]
    async fn a_read_failure_other_than_not_found_stops_before_any_write() {
        let mut use_case =
            DiagnoseWorkspaceUseCase::new(InMemoryWorkspaceDoctorRepository::failing_on_find());
        let error = use_case
            .execute(&doctor_target(), &doctor_observation(true), at())
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            WorkspaceDoctorCommandError::Repository(RepositoryError::Io { .. })
        ));
        assert!(use_case.repository.stored().is_empty());
    }

    #[tokio::test]
    async fn a_store_failure_is_surfaced_and_leaves_no_fact() {
        let mut use_case =
            DiagnoseWorkspaceUseCase::new(InMemoryWorkspaceDoctorRepository::failing_on_store());
        let error = use_case
            .execute(&doctor_target(), &doctor_observation(true), at())
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            WorkspaceDoctorCommandError::Repository(RepositoryError::Io { .. })
        ));
        assert!(use_case.repository.stored().is_empty());
    }

    #[tokio::test]
    async fn the_stored_fact_carries_the_same_target_as_the_request() {
        let mut use_case =
            DiagnoseWorkspaceUseCase::new(InMemoryWorkspaceDoctorRepository::empty());
        use_case
            .execute(&doctor_target(), &doctor_observation(true), at())
            .await
            .unwrap();
        let event = &use_case.repository.stored()[0];
        assert_eq!(event.target(), &doctor_target());
        assert_eq!(
            event.aggregate_id(),
            &WorkspaceDoctor::id_for(&doctor_target())
        );
    }
}
