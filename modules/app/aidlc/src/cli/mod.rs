//! argv → 型付きの要求（Controller の入口）。
//!
//! ここは**検証も業務判断も持たない**（10 §4 Controllers）。生の引数を型付きの値へ写し、
//! どの動詞へ行くかを決めるだけである。値の妥当性は消費側の値オブジェクトが決める。
//!
//! # 面の解決（マルチコール）
//!
//! 配布物は 1 つのバイナリで、**`argv[0]` がどのツールとして振る舞うかを決める**
//! （busybox 式。ADR 0002 決定 3 — 素の `aidlc-<tool>` 綴りが Markdown 資産・フック設定・
//! 文言に焼き込まれているため）。
//!
//! | 起動名 | 面 | 動詞 |
//! | --- | --- | --- |
//! | `aidlc-orchestrate` | エンジン | `next` / `continue` / `report` / `park` |
//! | `aidlc-utility` | ユーティリティ | `intent-create`（b29 の範囲） |
//! | `aidlc-log` | 対話イベントの記録 | `review`（b48 の範囲） |
//! | `aidlc-state` | 状態ファイルの書込 | `practices-promote`（b49 の範囲） |
//! | `aidlc-bolt` | Construction の Bolt | `set-autonomy`（b50 の範囲） |
//! | `aidlc-learnings` | §13 の学びの儀式 | `surface` / `persist` |
//! | `aidlc-review-brief` | レビュー判断の文脈（読取専用） | `review` / `context` / `summary` |
//! | `aidlc` | トップ | 上の 4 動詞をそのまま通す（top-passthrough） |
//!
//! **ディスパッチャの noun 形（`aidlc <noun> <verb>` の 30 経路）は実装していない。**
//! 逐語の写しが手元に無く、推測で綴りを作ると本家 `aidlc-orchestrate.ts` の ROUTES 表と
//! 食い違うためである。
//!
//! # ファイル構成
//!
//! 型ファイルの mod は private。公開 API は下の `pub use` が唯一の宣言であり、
//! `aidlc::runtime` はこれまでどおり `crate::cli::{Face, ReportArgs, IntentCreateArgs,
//! Request, Invocation, parse}` で参照する
//! (`coding-rules/module-visibility.md` / `one-public-type`)。

mod codekb_args;
mod face;
mod intent_args;
mod intent_create_args;
mod invocation;
mod promote_args;
mod report_args;
mod request;
mod review_args;
mod set_autonomy_args;

pub use codekb_args::CodekbArgs;
pub use face::Face;
pub use intent_args::IntentArgs;
pub use intent_create_args::IntentCreateArgs;
pub use invocation::Invocation;
pub use promote_args::PromoteArgs;
pub use report_args::ReportArgs;
pub use request::{Request, parse};
pub use review_args::ReviewArgs;
pub use set_autonomy_args::SetAutonomyArgs;

mod interaction_args;
pub use interaction_args::InteractionArgs;

mod link_args;
pub use link_args::LinkArgs;

mod learnings_args;
pub use learnings_args::{LearningsArgs, parse_learnings};

mod engine_route;
pub use engine_route::EngineRoute;

mod reuse_artifact_args;
pub use reuse_artifact_args::ReuseArtifactArgs;
