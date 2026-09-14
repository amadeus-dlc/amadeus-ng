//! `DoctorObservationDao` の実 Gateway — 環境・ファイル・SQLite を読取専用で観測する。
//!
//! 観測元ごとの読取は私有モジュールに分け、ここは束ねるだけである。どの読取も
//! 状態・監査・ストアを作らず、修復もしない (C7 `automatic_repair: forbidden`)。

use core_query_use_case::orchestration::{
    DoctorObservationDao, DoctorObservationView, ReadModelReadError,
};

use super::doctor_environment::DoctorEnvironment;
use super::doctor_paths::DoctorPaths;
use super::native_doctor_facts::NativeDoctorFacts;
use super::state_version_classifier::StateVersionClassifier;

mod audit_ledger;
mod definition;
mod environment;
mod heartbeat;
mod record;
mod settings;
mod store;

/// 観測先・環境・この build の事実・分類器を保持する実装。
#[derive(Debug)]
pub struct DoctorObservationDaoImpl<C> {
    paths: DoctorPaths,
    environment: DoctorEnvironment,
    facts: NativeDoctorFacts,
    classifier: C,
}

impl<C: StateVersionClassifier> DoctorObservationDaoImpl<C> {
    /// 観測先と注入物を束ねる。
    #[must_use]
    pub const fn new(
        paths: DoctorPaths,
        environment: DoctorEnvironment,
        facts: NativeDoctorFacts,
        classifier: C,
    ) -> Self {
        Self {
            paths,
            environment,
            facts,
            classifier,
        }
    }
}

impl<C: StateVersionClassifier> DoctorObservationDao for DoctorObservationDaoImpl<C> {
    fn find(&self) -> Result<DoctorObservationView, ReadModelReadError> {
        Ok(DoctorObservationView::new(
            environment::bun_found(&self.environment),
            environment::entry_points(&self.facts),
            settings::observe(&self.paths, &self.environment, &self.facts),
            heartbeat::observe(&self.paths),
            definition::shell(&self.paths),
            definition::assets(&self.paths),
            record::observe(&self.paths, &self.classifier),
        ))
    }
}
