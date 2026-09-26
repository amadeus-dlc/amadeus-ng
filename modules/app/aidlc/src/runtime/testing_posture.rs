//! テスト契約の公開入口。規則の解釈はRMUから呼ぶドメインが担う。
use super::{Completion, Layout, active_execution, store_path};
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};
use core_query_interface_adapter::ReadModelDaos;
use core_query_use_case::orchestration::{FindTestingContractUseCase, TestingContractView};
use core_read_model_updater::orchestration::{
    CodeGenerationApprovalReadModelUpdater, JournalReaderImpl, PlanFingerprintReadModelUpdater,
    ReadModelUpdater, SteeringSource, TestingReadModelUpdater,
};

pub(super) async fn run(layout: &Layout, args: &[String]) -> Completion {
    let command = args
        .iter()
        .find(|arg| {
            matches!(
                arg.as_str(),
                "resolve" | "render" | "fingerprint" | "verify" | "begin" | "brief"
            )
        })
        .map(String::as_str);
    let result = match command {
        Some("render" | "resolve") => load(layout).await.and_then(|view| {
            if let Some(error) = view.error() {
                return Err(error.to_string());
            }
            let text = if command == Some("render") {
                view.rendered()
            } else {
                view.contract()
            };
            text.map(|text| text.strip_suffix('\n').unwrap_or(text).to_string())
                .ok_or_else(|| "Projected Testing Contract is incomplete".to_string())
        }),
        Some("fingerprint") => fingerprint(layout, args).await,
        Some("begin") => begin(layout, args).await,
        Some("brief") => brief(layout, args).await,
        Some(command) => Err(format!("Testing Posture {command} is not connected")),
        None => Err(
            "Unknown subcommand: (none). Valid: resolve, render, fingerprint, verify, begin, brief"
                .to_string(),
        ),
    };
    match result {
        Ok(text) => Completion::emitted(text),
        Err(message) => failure(&message),
    }
}
pub(super) async fn load(layout: &Layout) -> Result<TestingContractView, String> {
    let store = store_path(layout)?;
    let mut reader = JournalReaderImpl::open(&store).map_err(|error| error.to_string())?;
    let source = SteeringSource::new(layout.memory_dir());
    TestingReadModelUpdater::open(&mut reader, store.as_path(), &source)
        .map_err(|error| error.to_string())?
        .update_read_models()
        .await
        .map_err(|error| error.to_string())?;
    let cursor = active_execution(layout).map_err(|error| error.to_string())?;
    let id = cursor
        .as_ref()
        .map_or("bare-space", |cursor| cursor.intent_id().as_str());
    FindTestingContractUseCase::new(
        ReadModelDaos::open(store.as_path())
            .map_err(|error| error.to_string())?
            .testing_contract(),
    )
    .execute(id)
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "Projected Testing Contract is missing".to_string())
}
fn failure(message: &str) -> Completion {
    let mut fields = ObjectMembers::new();
    fields.insert("error", JsonValue::String(message.to_string()));
    Completion::refused(serialize(
        &JsonValue::Object(fields),
        SerializationProfile::ContractCompact,
    ))
}

pub(super) fn target(
    args: &[String],
    command: &str,
) -> Result<core_command_domain::orchestration::PlanTarget, String> {
    use core_command_domain::orchestration::PlanTarget;
    let unit = args.iter().position(|arg| arg == "--unit");
    let stage = args.iter().any(|arg| arg == "--stage-level");
    if unit.is_some() && stage {
        return Err(format!(
            "{command} accepts exactly one of --unit <unit> or --stage-level"
        ));
    }
    if let Some(index) = unit {
        let value = args
            .get(index + 1)
            .filter(|value| {
                !value.starts_with("--") && !core_infrastructure::ecmascript::trim(value).is_empty()
            })
            .ok_or_else(|| format!("{command} requires a non-blank --unit <unit>"))?;
        return PlanTarget::for_unit(value).map_err(|error| error.to_string());
    }
    if stage {
        Ok(PlanTarget::stage_level())
    } else {
        Err(format!(
            "{command} requires exactly one of --unit <unit> or --stage-level"
        ))
    }
}
async fn fingerprint(layout: &Layout, args: &[String]) -> Result<String, String> {
    use core_query_use_case::orchestration::FindPlanFingerprintUseCase;
    use core_read_model_updater::orchestration::PlanSource;
    let target = target(args, "fingerprint")?;
    let record = layout.record_dir().ok_or_else(|| {
        "Code Generation approval authority requires an active workflow state".to_string()
    })?;
    let cursor = active_execution(layout).map_err(|error| error.to_string())?.ok_or_else(|| "Code Generation approval authority is unavailable because the active directive is missing, stale, or legacy; run a fresh `next`".to_string())?;
    let input = PlanSource::new(
        layout.project_dir().to_path_buf(),
        record.to_path_buf(),
        layout.memory_dir(),
        target.clone(),
    )
    .read(None)
    .map_err(|error| error.to_string())?;
    let store = store_path(layout)?;
    let mut reader = JournalReaderImpl::open(&store).map_err(|error| error.to_string())?;
    PlanFingerprintReadModelUpdater::open(
        &mut reader,
        store.as_path(),
        cursor.execution_id(),
        &input,
    )
    .map_err(|error| error.to_string())?
    .update_read_models()
    .await
    .map_err(|error| error.to_string())?;
    let daos = ReadModelDaos::open(store.as_path()).map_err(|error| error.to_string())?;
    let view = FindPlanFingerprintUseCase::new(daos.plan_fingerprint())
        .execute(cursor.execution_id().as_str(), &target.id())
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Projected plan fingerprint is missing".to_string())?;
    if let Some(error) = view.error() {
        return Err(error.to_string());
    }
    let fingerprint = view
        .fingerprint()
        .ok_or_else(|| "Projected plan fingerprint is incomplete".to_string())?;
    // 2.8.2 は計画承認節へそのまま写す 2 行のタグを出す — 内容の指紋と、この計画が
    // 書かれたときのワークスペースのソース（`aidlc-testing-posture.ts` `fingerprint`）。
    let planned = crate::source_fingerprint::read(layout.project_dir())
        .unwrap_or_else(|_| "unbindable".to_string());
    Ok(format!(
        "[Approval Fingerprint]: {fingerprint}\n[Planned Source]: {planned}"
    ))
}

/// 承認済みの作業ブリーフ（upstream `workerBrief`）。**読取専用**である。
///
/// # 渡すのは指紋が束ねた形だけである
///
/// 作業者へ渡す計画とテスト指示は [`PlanApprovalDocuments::approved_plan`] /
/// [`PlanApprovalDocuments::approved_instructions`] — 承認の指紋
/// （[`core_command_domain::orchestration::CodeGenerationAuthority::approval_fingerprint`]）が
/// 束ねる形そのものである（upstream も `projectPlanApprovalContent(plan)` を渡す）。指紋は
/// 末尾の `## Review` 付録と進捗の印を見ないので、原文を渡すと、承認後に付録へ足した手順や
/// 印を書き換えた手順が、指紋を変えないまま作業として届いてしまう。
///
/// [`PlanApprovalDocuments::approved_plan`]: core_command_domain::orchestration::PlanApprovalDocuments::approved_plan
/// [`PlanApprovalDocuments::approved_instructions`]: core_command_domain::orchestration::PlanApprovalDocuments::approved_instructions
///
/// # 読み直さない
///
/// upstream は評価と受け渡しの間にファイルが変わる窓を閉じるため、渡す直前に読み直して
/// 指紋を計算し直す。こちらは評価に使った [`PlanSource`] の読取り結果をそのまま渡すので、
/// その窓がそもそも開かない。
async fn brief(layout: &Layout, args: &[String]) -> Result<String, String> {
    let target = target(args, "brief")?;
    let approval = approval(layout, &target).await?;
    if !approval.ok {
        return Err(crate::wording::worker_brief_refused(
            approval.unit.as_deref(),
            &approval.reason,
        ));
    }
    let contract_hash = approval
        .contract_hash
        .as_deref()
        .ok_or_else(|| "Projected Code Generation approval is incomplete".to_string())?;
    let assembled = crate::wording::worker_brief(
        approval.unit.as_deref(),
        contract_hash,
        &approval.plan,
        &approval.instructions,
    );
    // 末尾改行は `main.rs` の `writeln!` が付す（upstream は改行を足さずに書く）。
    Ok(assembled
        .strip_suffix('\n')
        .unwrap_or(&assembled)
        .to_string())
}

/// ある承認対象の、いまの計画承認の評価（`brief` と plan-approval-guard が共有する）。
pub(super) struct PlanApprovalState {
    /// 承認が現在のものか。
    ok: bool,
    /// 現在でない理由（`ok` なら `approved`）。
    reason: String,
    /// 承認された Testing Contract の指紋。
    contract_hash: Option<String>,
    /// 対象の Unit（段階全体なら `None`）。
    unit: Option<String>,
    /// 作業者へ渡す計画（承認の指紋が束ねた形 — 付録を落とし、進捗の印を戻したもの）。
    plan: String,
    /// 作業者へ渡すテスト指示（承認の指紋が束ねた形）。
    instructions: String,
}

impl PlanApprovalState {
    /// 承認が現在のものか。
    pub(super) const fn is_current(&self) -> bool {
        self.ok
    }

    /// 現在の承認が、依頼文の名乗る Testing Contract の指紋を承認しているか。
    pub(super) fn approves_contract(&self, contract: &str) -> bool {
        self.ok && self.contract_hash.as_deref() == Some(contract)
    }

    /// 現在でない理由（拒否文の detail）。現在の承認と、理由の空な評価は `None`。
    pub(super) fn refusal_reason(&self) -> Option<&str> {
        (!self.ok && !self.reason.is_empty()).then_some(self.reason.as_str())
    }
}

/// 承認対象の計画承認を評価する（**読取専用** — 共有承認ストアを作らない）。
///
/// # Errors
///
/// 記録・実行カーソル・ストアが読めない、または投影が失敗した場合。
pub(super) async fn approval(
    layout: &Layout,
    target: &core_command_domain::orchestration::PlanTarget,
) -> Result<PlanApprovalState, String> {
    use core_command_domain::orchestration::PlanReceipts;
    use core_command_domain::workspace::StorePath;
    use core_query_use_case::orchestration::FindCodeGenerationApprovalUseCase;
    use core_read_model_updater::orchestration::{PlanApprovalJournalReaderImpl, PlanSource};
    let record = layout.record_dir().ok_or_else(|| {
        "Code Generation approval authority requires an active workflow state".to_string()
    })?;
    let cursor = active_execution(layout).map_err(|error| error.to_string())?.ok_or_else(|| "Code Generation approval authority is unavailable because the active directive is missing, stale, or legacy; run a fresh `next`".to_string())?;
    let input = PlanSource::new(
        layout.project_dir().to_path_buf(),
        record.to_path_buf(),
        layout.memory_dir(),
        target.clone(),
    )
    .read(crate::source_fingerprint::read(layout.project_dir()).ok())
    .map_err(|error| error.to_string())?;
    // 共有承認ストアがまだ無いワークスペースには受領が 1 件も無い。読取専用の動詞は
    // ストアを作らない（観測が状態を作ってはならない）ので、空の受領のまま判断へ進む。
    // ドメインは「一致する受領が無い」として拒否するので、空が成功へ倒れることはない。
    let runtime_store = StorePath::for_runtime(&layout.aidlc_root());
    let receipts = if runtime_store.as_path().is_file() {
        let reader = PlanApprovalJournalReaderImpl::open(&runtime_store)
            .map_err(|error| error.to_string())?;
        core_read_model_updater::orchestration::plan_approval_receipts(&reader)
            .map_err(|error| error.to_string())?
    } else {
        PlanReceipts::default()
    };
    let store = store_path(layout)?;
    let mut reader = JournalReaderImpl::open(&store).map_err(|error| error.to_string())?;
    CodeGenerationApprovalReadModelUpdater::open(
        &mut reader,
        store.as_path(),
        cursor.execution_id(),
        &input,
        &receipts,
    )
    .map_err(|error| error.to_string())?
    .update_read_models()
    .await
    .map_err(|error| error.to_string())?;
    let daos = ReadModelDaos::open(store.as_path()).map_err(|error| error.to_string())?;
    let view = FindCodeGenerationApprovalUseCase::new(daos.code_generation_approval())
        .execute(cursor.execution_id().as_str(), &target.id())
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Projected Code Generation approval is missing".to_string())?;
    Ok(PlanApprovalState {
        ok: view.ok(),
        reason: view.reason().to_string(),
        contract_hash: view.contract_hash().map(str::to_string),
        unit: view.unit().map(str::to_string),
        plan: input.documents().approved_plan(),
        instructions: input.documents().approved_instructions(),
    })
}

async fn begin(layout: &Layout, args: &[String]) -> Result<String, String> {
    let target = target(args, "begin")?;
    begin_generation(layout, target).await
}

/// 承認済みの計画から生成を始めたことを受領に記録する（`begin` と plan-approval-guard が
/// 共有する — 2.8.2 `beginCodeGeneration`）。何度呼んでも同じ受領のままである。
///
/// # Errors
///
/// 承認が現在でない、またはストアが読めない場合。
pub(super) async fn begin_generation(
    layout: &Layout,
    target: core_command_domain::orchestration::PlanTarget,
) -> Result<String, String> {
    use core_command_domain::orchestration::PlanApprovalOperationId;
    use core_command_domain::workspace::StorePath;
    use core_query_use_case::orchestration::PlanGenerationUseCase;
    use core_read_model_updater::orchestration::PlanSource;
    let record = layout
        .record_dir()
        .ok_or("Code Generation approval authority requires an active workflow state")?;
    let cursor=active_execution(layout).map_err(|error|error.to_string())?.ok_or("Code Generation approval authority is unavailable because the active directive is missing, stale, or legacy; run a fresh `next`")?;
    let mut access = super::plan_approval::PlanApprovalAccess::open(layout).await?;
    let source = crate::source_fingerprint::read(layout.project_dir()).ok();
    let input = PlanSource::new(
        layout.project_dir().to_path_buf(),
        record.to_path_buf(),
        layout.memory_dir(),
        target,
    )
    .read(source)
    .map_err(|error| error.to_string())?;
    let id = PlanApprovalOperationId::generate();
    access
        .begin(layout, id.clone(), cursor.execution_id(), &input)
        .await?;
    let store = StorePath::for_runtime(&layout.aidlc_root());
    let daos = ReadModelDaos::open(store.as_path()).map_err(|error| error.to_string())?;
    let result = PlanGenerationUseCase::new(daos.plan_generation())
        .execute(id.as_str())
        .map_err(|error| error.to_string())?
        .ok_or("Generation result projection is missing")?;
    if let Some(error) = result.error() {
        return Err(error.to_string());
    }
    if result.status() != "generation" {
        return Err("Generation publication is not certified".to_string());
    }
    let mut fields = ObjectMembers::new();
    fields.insert("status", JsonValue::String(result.status().to_string()));
    let mut target = ObjectMembers::new();
    target.insert(
        "unit",
        result
            .unit()
            .map_or(JsonValue::Null, |unit| JsonValue::String(unit.to_string())),
    );
    fields.insert("target", JsonValue::Object(target));
    Ok(serialize(
        &JsonValue::Object(fields),
        SerializationProfile::ContractCompact,
    ))
}
