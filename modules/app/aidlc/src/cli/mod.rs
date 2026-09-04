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
//! | `aidlc` | トップ | 上の 4 動詞をそのまま通す（top-passthrough） |
//!
//! **ディスパッチャの noun 形（`aidlc <noun> <verb>` の 30 経路）は実装していない。**
//! 逐語の写しが手元に無く、推測で綴りを作ると逸脱台帳 #1 の写像表と食い違うためである。
//!
//! # ファイル構成
//!
//! 型ファイルの mod は private。公開 API は下の `pub use` が唯一の宣言であり、
//! `aidlc::runtime` はこれまでどおり `crate::cli::{Face, ReportArgs, IntentCreateArgs,
//! Request, Invocation, parse}` で参照する
//! (`coding-rules/module-visibility.md` / `one-public-type`)。

mod face;
mod intent_create_args;
mod invocation;
mod report_args;
mod request;
mod review_args;

pub use face::Face;
pub use intent_create_args::IntentCreateArgs;
pub use invocation::Invocation;
pub use report_args::ReportArgs;
pub use request::{Request, parse};
pub use review_args::ReviewArgs;
