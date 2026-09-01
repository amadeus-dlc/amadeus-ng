//! アウトプットポート — インタラクタが依存する**契約 (trait)** と、その契約に依存する型
//! (返却レコード・エラー・ポート入出力 VO) の置き場。
//!
//! 実装 (Gateway) は `core-command-interface-adapter` に置く (DIP — 01 §7)。ここに住んで
//! よいのは「ポート面に現れる型」だけである: 契約そのものと、契約のエラー
//! ([`RepositoryError`] 1 本)。インタラクタ・インタラクタ入力 VO・ユースケース自身のエラー
//! 封筒は親モジュールに住む。
//!
//! ポートごとの専用エラーは持たない (`coding-rules/error-handling.md`「Repository エラーは
//! ジェネリック 1 本」)。定義 3 入力の読取失敗を 6 変種で表していたポート専用エラー型は
//! 2026-08-31 のオーナー裁定で廃止し、`RepositoryError<WorkflowDefinitionId>` へ収束させた —
//! リポジトリにビジネスロジックのエラーを扱わせない。読むだけの動詞が要した steering 連鎖の
//! 読取ポートと継続トークンの開封も、同じ Bolt でクエリ側へ移った。
//!
//! 契約が返すレコード (`RehydratedIntentExecution`)、契約へ渡す三つ組 VO (`StatePosition`)、
//! および版の newtype (`StoreVersion`) は**すべて廃止済み**である — 集約が通番と楽観 version
//! を基本データ型で持つようになったので、ポートは集約そのものを授受すれば足りる
//! (オーナー裁定 2026-08-30)。
//!
//! 型ファイルの mod は private。公開 API は親モジュールの `pub use` ファサードが唯一の宣言
//! (`coding-rules/module-visibility.md`)。

mod definition_artifacts;
mod definition_artifacts_client;
mod definition_artifacts_error;
mod intent_execution_repository;
mod intent_repository;
mod repository_error;
mod workflow_definition_repository;

pub use definition_artifacts::DefinitionArtifacts;
pub use definition_artifacts_client::DefinitionArtifactsClient;
pub use definition_artifacts_error::DefinitionArtifactsError;
pub use intent_execution_repository::IntentExecutionRepository;
pub use intent_repository::IntentRepository;
pub use repository_error::RepositoryError;
pub use workflow_definition_repository::WorkflowDefinitionRepository;
