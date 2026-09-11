//! 名指された工程への移動を保存する。
use super::{
    JumpError,
    port::{IntentExecutionRepository, IntentRepository, WorkflowDefinitionRepository},
};
use core_command_domain::{
    orchestration::{IntentExecutionEventId, IntentExecutionId, JumpObservation},
    workflow_definition::StageSlug,
};
/// 状態判断は集約、永続化は注入したRepositoryに委譲する。
#[derive(Debug)]
pub struct JumpUseCase<
    E: IntentExecutionRepository,
    I: IntentRepository,
    D: WorkflowDefinitionRepository,
> {
    executions: E,
    intents: I,
    definitions: D,
}
impl<E: IntentExecutionRepository, I: IntentRepository, D: WorkflowDefinitionRepository>
    JumpUseCase<E, I, D>
{
    /// 2つの書込側ポートと、別 scope の列を引く定義ポートを注入する。
    #[must_use]
    pub const fn new(executions: E, intents: I, definitions: D) -> Self {
        Self {
            executions,
            intents,
            definitions,
        }
    }
    /// 移動の成功値はunitのみ。結果はイベントIDでRMU/Queryから取得する。
    /// # Errors
    /// 対象不正・状態前提・保存エラーを伝播する。
    #[expect(
        clippy::too_many_arguments,
        reason = "跳躍の入力 (対象・方向・別 scope・観測) を束ねる要求型を新設せず、\
                  コマンドの項目がそのまま引数になる"
    )]
    pub async fn execute(
        &mut self,
        execution_id: &IntentExecutionId,
        event_id: &IntentExecutionEventId,
        target: &StageSlug,
        direction: core_command_domain::orchestration::JumpDirection,
        scope: Option<&str>,
        observation: JumpObservation,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), JumpError> {
        let mut aggregate = self.executions.find_by_id(execution_id).await?;
        let intent = self.intents.find_for_execution(&aggregate).await?;
        // 別 scope は本家どおり静的な列だけを参照し、state の suffix を見ない。
        let scope = match scope {
            Some(name) => {
                let definition = self
                    .definitions
                    .find_for_intent(&intent)
                    .await
                    .map_err(JumpError::DefinitionRepository)?;
                Some(
                    definition
                        .jump_scope(name)
                        .ok_or_else(|| JumpError::UnknownScope(name.to_string()))?,
                )
            }
            None => None,
        };
        let target = aggregate.resolve_jump_target(target)?;
        let event = aggregate.jump(
            &intent,
            target,
            event_id.clone(),
            direction,
            Some(observation),
            scope,
            at,
        )?;
        self.executions.store(&event, &aggregate).await?;
        Ok(())
    }
}
