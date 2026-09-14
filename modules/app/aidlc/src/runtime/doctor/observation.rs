//! 観測 View → ドメインの観測値オブジェクトの写像。
//!
//! # なぜ合成ルートに置くのか
//!
//! 両側を知ってよいのは **RMU と合成ルートだけ**である
//! (`coding-rules/cqrs-boundaries.md`)。観測 (ファイル・環境・ストアの読取) は「読むだけ」
//! なのでクエリ側 (`DoctorObservationDaoImpl`) に在り、判断は集約 (`WorkspaceDoctor`) に
//! 在る。その 2 つを繋ぐ写像は、どちらの側にも置けない — クエリ側のクレートはドメインに
//! 依存しないことが `Cargo.toml` で機械強制されているからである。
//!
//! 写すだけで、判断・導出・文言組立は 1 つも持たない。

use core_command_domain::workspace as domain;
use core_query_use_case::orchestration as view;

/// 1 回の診断の観測をドメインの値オブジェクトへ写す。
pub(super) fn to_domain(observed: &view::DoctorObservationView) -> domain::DoctorObservation {
    domain::DoctorObservation::new(
        observed.bun_found(),
        entry_points(observed.entry_points()),
        hook_wiring(observed.hook_wiring()),
        heartbeat(observed.heartbeat()),
        domain::WorkspaceShell::new(
            observed.shell().harness_dir_exists(),
            observed.shell().memory_dir_exists(),
        ),
        definition(observed.definition()),
        observed.record().map(record),
    )
}

/// 観測できなかった理由 (材料のみ)。
fn failure(value: &view::ObservationFailure) -> domain::ObservationFailure {
    domain::ObservationFailure::new(value.cause().to_string())
}

/// `Result<T, ObservationFailure>` を要素ごとに写す。
fn observed<T, U>(
    value: &Result<Vec<T>, view::ObservationFailure>,
    map: impl Fn(&T) -> U,
) -> Result<Vec<U>, domain::ObservationFailure> {
    match value {
        Ok(items) => Ok(items.iter().map(map).collect()),
        Err(cause) => Err(failure(cause)),
    }
}

fn entry_points(value: &view::NativeEntryPointsView) -> domain::NativeEntryPoints {
    domain::NativeEntryPoints::new(
        value.binary().clone().map_err(|cause| failure(&cause)),
        value.missing_entry_points().to_vec(),
    )
}

fn hook_wiring(value: &view::HookWiringView) -> domain::HookWiring {
    domain::HookWiring::new(
        value.settings_present(),
        observed(value.wired_hooks(), |hook| {
            domain::WiredHook::new(hook.name().to_string(), hook.present())
        }),
        value.hooks_disabled_by().map(str::to_string),
        value.managed_hooks_only(),
        observed(value.bindings(), binding),
        value.native_hook_names().to_vec(),
        declaration(value.declaration()),
    )
}

/// 接続定義の写し (クエリ側 View → ドメイン)。
fn declaration(value: &view::HookBindingDeclaration) -> domain::HookBindingDeclaration {
    match value {
        view::HookBindingDeclaration::Absent => domain::HookBindingDeclaration::Absent,
        view::HookBindingDeclaration::Declared {
            native,
            distributed,
        } => domain::HookBindingDeclaration::Declared {
            native: native.clone(),
            distributed: distributed.clone(),
        },
        view::HookBindingDeclaration::Unreadable(cause) => {
            domain::HookBindingDeclaration::Unreadable(domain::ObservationFailure::new(
                cause.cause().to_string(),
            ))
        }
    }
}

fn binding(value: &view::HookBindingView) -> domain::HookBinding {
    domain::HookBinding::new(
        value.event().to_string(),
        value.matcher().to_string(),
        value.command().to_string(),
        match value.target() {
            view::HookBindingTarget::Native(name) => {
                domain::HookBindingTarget::Native(name.clone())
            }
            view::HookBindingTarget::Distributed(name) => {
                domain::HookBindingTarget::Distributed(name.clone())
            }
            view::HookBindingTarget::Unknown => domain::HookBindingTarget::Unknown,
        },
    )
}

fn timestamp(value: &view::TimestampView) -> domain::ObservedTimestamp {
    domain::ObservedTimestamp::new(value.raw().to_string(), value.millis())
}

fn heartbeat(value: &view::HeartbeatView) -> domain::HeartbeatObservation {
    domain::HeartbeatObservation::new(
        value.health_dir_exists(),
        value.has_heartbeat_files(),
        value
            .entries()
            .iter()
            .map(|entry| {
                domain::HeartbeatEntry::new(entry.hook().to_string(), timestamp(entry.timestamp()))
            })
            .collect(),
        value.progressed_stage_count(),
        value.stage_started(),
        value.newest_progress().map(timestamp),
    )
}

fn definition(value: &view::DefinitionAssetsView) -> domain::DefinitionAssets {
    domain::DefinitionAssets::new(
        observed(value.graph(), stage),
        observed(value.scope_grid(), |entry| {
            domain::ScopeGridEntry::new(entry.scope().to_string(), entry.stages().to_vec())
        }),
        match value.scope_names() {
            Ok(names) => Ok(names.clone()),
            Err(cause) => Err(failure(cause)),
        },
        observed(value.stage_files(), |file| {
            domain::StageFile::new(
                file.phase().to_string(),
                file.slug().to_string(),
                file.content().clone().map_err(|cause| failure(&cause)),
            )
        }),
        match value.agents() {
            Ok(agents) => Ok(agents.clone()),
            Err(cause) => Err(failure(cause)),
        },
    )
}

fn stage(value: &view::GraphStageView) -> domain::GraphStage {
    domain::GraphStage::new(
        value.slug().to_string(),
        value.phase().to_string(),
        value.number().to_string(),
        value.enabled(),
        value.requires_stage().to_vec(),
        domain::StageArtifacts::new(
            value.artifacts().produces().to_vec(),
            value.artifacts().optional_produces().to_vec(),
            value
                .artifacts()
                .consumes()
                .iter()
                .map(|consume| {
                    domain::ConsumedArtifact::new(
                        consume.artifact().to_string(),
                        consume.required(),
                        consume.conditional_on().map(str::to_string),
                    )
                })
                .collect(),
        ),
    )
}

fn record(value: &view::RecordObservationView) -> domain::RecordObservation {
    domain::RecordObservation::new(
        domain::RecordLocation::new(
            value.location().intents_relative().to_string(),
            value.location().store_relative().to_string(),
            value.location().records().to_vec(),
            value.location().selected().map(str::to_string),
            value.location().cursor_target().map(str::to_string),
        ),
        state(value.state()),
        match value.cursor() {
            Ok(cursor) => Ok(cursor.as_ref().map(|cursor| {
                domain::ExecutionCursorObservation::new(
                    cursor.execution_id().to_string(),
                    cursor.intent_id().to_string(),
                )
            })),
            Err(cause) => Err(failure(cause)),
        },
        match value.registry_directory() {
            Ok(directory) => Ok(directory.clone()),
            Err(cause) => Err(failure(cause)),
        },
        store(value.store()),
        match value.projection() {
            Ok(projection) => Ok(domain::ProjectionObservation::new(
                projection.execution_intent_id().map(str::to_string),
                projection.checkpoint(),
                projection.latest_execution_event(),
                projection.pending_publications(),
                projection.audit_shard_count(),
            )),
            Err(cause) => Err(failure(cause)),
        },
    )
}

fn state(value: &view::StateFileObservationView) -> domain::StateFileObservation {
    match value {
        view::StateFileObservationView::Absent => domain::StateFileObservation::Absent,
        view::StateFileObservationView::Unreadable(cause) => {
            domain::StateFileObservation::Unreadable(failure(cause))
        }
        view::StateFileObservationView::Classified(version) => {
            domain::StateFileObservation::Classified(domain::StateVersionObservation::new(
                match version.kind() {
                    view::StateVersionKindView::Ok => domain::StateVersionKind::Ok,
                    view::StateVersionKindView::Unparseable => {
                        domain::StateVersionKind::Unparseable
                    }
                    view::StateVersionKindView::Past => domain::StateVersionKind::Past,
                    view::StateVersionKindView::Future => domain::StateVersionKind::Future,
                },
                version.message().map(str::to_string),
            ))
        }
    }
}

fn store(value: &view::StoreObservationView) -> domain::StoreObservation {
    match value {
        view::StoreObservationView::Absent => domain::StoreObservation::Absent,
        view::StoreObservationView::Unreadable(cause) => {
            domain::StoreObservation::Unreadable(failure(cause))
        }
        view::StoreObservationView::Opened(schema) => domain::StoreObservation::Opened(
            domain::StoreSchema::new(schema.tables().to_vec(), schema.schema_version()),
        ),
    }
}
