//! 計画承認の参照文書を読む境界。
use super::{ReadModelUpdateError, SteeringSource};
use core_command_domain::orchestration::{PlanApprovalDocuments, PlanApprovalInput, PlanTarget};
use core_infrastructure::hash::sha256_hex;
use std::{fs, io, path::PathBuf};
/// 正規の記録配下から読み取る。対象名はドメインで検査済み。
#[derive(Debug, Clone)]
pub struct PlanSource {
    project: PathBuf,
    record: PathBuf,
    memory: PathBuf,
    target: PlanTarget,
}
impl PlanSource {
    /// 取得先を束ねる。
    #[must_use]
    pub const fn new(
        project: PathBuf,
        record: PathBuf,
        memory: PathBuf,
        target: PlanTarget,
    ) -> Self {
        Self {
            project,
            record,
            memory,
            target,
        }
    }
    /// 現在の原文と状態本文を読む。ソース走査結果は入力境界が渡す。
    /// # Errors
    /// 存在する入力を読めない場合。
    pub fn read(
        &self,
        source_sha256: Option<String>,
    ) -> Result<PlanApprovalInput, ReadModelUpdateError> {
        let mut directory = self.record.join("construction");
        if let Some(unit) = self.target.unit() {
            directory.push(unit);
        }
        directory.push("code-generation");
        let read = |name: &str| -> Result<String, ReadModelUpdateError> {
            let path = directory.join(name);
            match fs::read_to_string(&path) {
                Ok(text) => Ok(text),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
                Err(error) => Err(ReadModelUpdateError::SteeringRead {
                    path: path.display().to_string(),
                    kind: error.kind(),
                }),
            }
        };
        let state = self.record.join("aidlc-state.md");
        let state_sha256 = match fs::read(&state) {
            Ok(body) => Some(sha256_hex(&body)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(ReadModelUpdateError::SteeringRead {
                    path: state.display().to_string(),
                    kind: error.kind(),
                });
            }
        };
        let questions = directory.join("code-generation-questions.md");
        let relative = questions
            .strip_prefix(&self.project)
            .ok()
            .and_then(|path| path.to_str())
            .ok_or_else(|| ReadModelUpdateError::SteeringRead {
                path: questions.display().to_string(),
                kind: io::ErrorKind::InvalidData,
            })?
            .replace('\\', "/");
        let documents = PlanApprovalDocuments::new(
            read("code-generation-plan.md")?,
            read("unit-test-instructions.md")?,
            read("code-generation-questions.md")?,
            relative,
        );
        let sections = SteeringSource::new(self.memory.clone()).read_testing_sections()?;
        Ok(PlanApprovalInput::new(
            documents,
            sections,
            self.target.clone(),
            state_sha256,
            source_sha256,
        ))
    }
}
