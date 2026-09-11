//! Pipeline履歴のsnapshot表現。
use super::{
    dto_decode_error::DtoDecodeError, pipeline_link_completed_dto::PipelineLinkCompletedDto,
};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::PipelineRecord;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum PipelineRecordDto {
    Closed(String),
    Boundary {
        stage: Option<String>,
        single: bool,
        at: DateTime<Utc>,
    },
    Completed(PipelineLinkCompletedDto),
}
impl PipelineRecordDto {
    pub(super) fn of(record: &PipelineRecord) -> Self {
        match record {
            PipelineRecord::Closed(stage) => Self::Closed(stage.clone()),
            PipelineRecord::Boundary { stage, single, at } => Self::Boundary {
                stage: stage.clone(),
                single: *single,
                at: *at,
            },
            PipelineRecord::Completed(e) => Self::Completed(PipelineLinkCompletedDto::of(e)),
        }
    }
    pub(super) fn to_domain(&self) -> Result<PipelineRecord, DtoDecodeError> {
        Ok(match self {
            Self::Closed(stage) => PipelineRecord::Closed(stage.clone()),
            Self::Boundary { stage, single, at } => PipelineRecord::Boundary {
                stage: stage.clone(),
                single: *single,
                at: *at,
            },
            Self::Completed(e) => PipelineRecord::Completed(e.to_domain()?),
        })
    }
}
