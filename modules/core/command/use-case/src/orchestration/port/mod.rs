//! アウトプットポート — インタラクタが依存する**契約 (trait)** と、その契約に依存する型
//! (返却レコード・エラー・ポート入出力 VO) の置き場。
//!
//! 実装 (Gateway) は `core-command-interface-adapter` に置く (DIP — 01 §7)。ここに住んで
//! よいのは「ポート面に現れる型」だけである: 契約そのもの、契約が返すレコード
//! ([`RehydratedIntentExecution`])、契約のエラー ([`RepositoryError`] / [`GraphReadError`] /
//! [`RuleBundleReadError`] / [`InvalidContinueToken`])、契約へ渡す VO ([`StatePosition`] と
//! その [`StoreVersion`])。インタラクタ・インタラクタ入力 VO・ユースケース自身のエラー封筒は
//! 親モジュールに住む。
//!
//! 型ファイルの mod は private。公開 API は親モジュールの `pub use` ファサードが唯一の宣言
//! (`coding-rules/module-visibility.md`)。

mod command_spelling;
mod continue_token_codec;
mod intent_execution_repository;
mod intent_repository;
mod rehydrated_intent_execution;
mod repository_error;
mod rule_bundle_source;
mod state_position;
mod store_version;
mod workflow_definition_repository;

pub use command_spelling::CommandSpelling;
pub use continue_token_codec::{ContinueTokenCodec, InvalidContinueToken};
pub use intent_execution_repository::IntentExecutionRepository;
pub use intent_repository::IntentRepository;
pub use rehydrated_intent_execution::RehydratedIntentExecution;
pub use repository_error::RepositoryError;
pub use rule_bundle_source::{RuleBundleReadError, RuleBundleSource};
pub use state_position::StatePosition;
pub use store_version::StoreVersion;
pub use workflow_definition_repository::{GraphReadError, WorkflowDefinitionRepository};
