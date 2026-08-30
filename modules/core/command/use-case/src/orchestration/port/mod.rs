//! アウトプットポート — インタラクタが依存する**契約 (trait)** と、その契約に依存する型
//! (返却レコード・エラー・ポート入出力 VO) の置き場。
//!
//! 実装 (Gateway) は `core-command-interface-adapter` に置く (DIP — 01 §7)。ここに住んで
//! よいのは「ポート面に現れる型」だけである: 契約そのものと、契約のエラー
//! ([`RepositoryError`] / [`GraphReadError`] / [`RuleBundleReadError`] /
//! [`InvalidContinueToken`])。インタラクタ・インタラクタ入力 VO・ユースケース自身のエラー
//! 封筒は親モジュールに住む。
//!
//! 契約が返すレコード (`RehydratedIntentExecution`)、契約へ渡す三つ組 VO (`StatePosition`)、
//! および版の newtype (`StoreVersion`) は**すべて廃止済み**である — 集約が通番と楽観 version
//! を基本データ型で持つようになったので、ポートは集約そのものを授受すれば足りる
//! (オーナー裁定 2026-08-30)。
//!
//! 型ファイルの mod は private。公開 API は親モジュールの `pub use` ファサードが唯一の宣言
//! (`coding-rules/module-visibility.md`)。

mod intent_execution_repository;
mod intent_repository;
mod repository_error;
mod rule_bundle_source;
mod workflow_definition_repository;

pub use intent_execution_repository::IntentExecutionRepository;
pub use intent_repository::IntentRepository;
pub use repository_error::RepositoryError;
pub use rule_bundle_source::{RuleBundleReadError, RuleBundleSource};
pub use workflow_definition_repository::{GraphReadError, WorkflowDefinitionRepository};
