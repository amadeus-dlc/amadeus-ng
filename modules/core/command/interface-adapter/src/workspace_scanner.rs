//! ワークスペース走査 — **横断機構の注入シームであって Gateway ではない**
//! (clean-architecture: 走査は Infrastructure が所有する機構。
//! `coding-rules/gateway-taxonomy.md` §1)。
//!
//! [`Clock`](crate::Clock) と同じ位置づけである。どのユースケースもこの trait を消費しない —
//! `CreateIntentUseCase` は走査**結果**の値オブジェクト [`WorkspaceScan`] を引数で受け取る
//! （`coding-rules/use-case-rules.md` §2b「execute の引数は集約 ID と値オブジェクトのみ」）。
//! 走査を実行して値を作るのは合成ルートの仕事なので、アプリ境界のポートとして use-case 層に
//! は置かず、実装と同じアダプタ層に閉じ込める。
//!
//! # 走査結果は歴史に焼き込まれる
//!
//! ここが返した値は `Intent::create` を通って誕生イベント `Created` に載り、**二度と
//! 書き換わらない**（イベントは歴史であり、後から訂正できない）。したがって暫定実装が返す
//! 値は「走査していない」という事実の記録であって、観測結果ではない — 実装
//! [`UnscannedWorkspace`](crate::UnscannedWorkspace) の doc を参照。

use core_command_domain::orchestration::WorkspaceScan;
use core_command_domain::workspace::UnsafeLineChar;

/// ワークスペース走査の抽象。テストで fake を注入するための唯一の走査源。
pub trait WorkspaceScanner {
    /// ワークスペースを走査して、プロジェクト種別・言語・フレームワーク・ビルドシステムを返す。
    ///
    /// # Errors
    ///
    /// 走査で拾った値に行終端・制御文字が混ざっていれば `UnsafeLineChar`。実走査は
    /// ディレクトリ名やマニフェストの中身という**外から来た文字列**を読むので、これは
    /// 防御的な変種ではなく実際に起こりうる失敗である（走査結果は状態ファイルの bullet 行に
    /// 書かれるので、改行が混ざると 2 行目以降がフィールドとして読めなくなる）。
    fn scan(&self) -> Result<WorkspaceScan, UnsafeLineChar>;
}
