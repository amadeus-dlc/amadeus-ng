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
//! 値は「走査していない」という事実の記録であって、観測結果ではない — 下記
//! [`UnscannedWorkspace`] の doc を参照。

use core_command_domain::orchestration::WorkspaceScan;
use core_command_domain::workflow_definition::BrownfieldGreenfield;
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

/// **暫定実装** — 走査せず「未走査」を意味する固定値を返す。
///
/// # これは観測結果ではない
///
/// 返す `Greenfield` / `Unknown` × 3 は、**走査が未実装であること**の記録であって、
/// ワークスペースを見た結果ではない。実走査（upstream `aidlc-utility.ts` の `scanSignals` /
/// `detectFrameworks` / `detectBuildSystem` / `countFilesByLang` — 言語カウントの深さ制限・
/// フレームワーク検出表・ネストプロジェクトのフォールバックを持つ）は後続 Bolt で実装する。
///
/// この値は誕生イベントに載って歴史に焼き込まれるので、実走査が入っても**既存の intent の
/// 記録は遡って直らない**。実走査の Bolt は、既存記録の扱い（そのまま / 再走査して訂正
/// イベントを足す）を併せて裁定する必要がある。
#[derive(Debug, Clone, Copy, Default)]
pub struct UnscannedWorkspace;

impl UnscannedWorkspace {
    /// 単位型を作る（走査しないので設定項目は無い）。
    #[must_use]
    pub const fn new() -> UnscannedWorkspace {
        UnscannedWorkspace
    }
}

impl WorkspaceScanner for UnscannedWorkspace {
    fn scan(&self) -> Result<WorkspaceScan, UnsafeLineChar> {
        // 4 値とも単一行の定数なので、この実装では検査は必ず通る。`Err` を返しうるのは
        // trait の契約であって、この実装の振る舞いではない。
        WorkspaceScan::new(BrownfieldGreenfield::Greenfield, UNKNOWN, UNKNOWN, UNKNOWN)
    }
}

/// 走査していないことを表す欄の値（upstream の `scan.languages` 等と同じ綴り）。
const UNKNOWN: &str = "Unknown";

#[cfg(test)]
mod tests {
    use super::*;

    /// ジェネリック関数から機構越しに使えること（静的束縛 — 合成ルートはこの形で組む）。
    fn scan_with<S: WorkspaceScanner>(scanner: &S) -> WorkspaceScan {
        scanner.scan().expect("暫定走査は定数なので必ず通る")
    }

    #[test]
    fn the_provisional_scanner_reports_an_unscanned_workspace() {
        let scan = scan_with(&UnscannedWorkspace::new());
        assert_eq!(scan.project_kind(), BrownfieldGreenfield::Greenfield);
        assert_eq!(scan.languages(), UNKNOWN);
        assert_eq!(scan.frameworks(), UNKNOWN);
        assert_eq!(scan.build_system(), UNKNOWN);
    }

    /// 同じ走査源は何度呼んでも同じ値を返す（誕生イベントに載る値が呼出順に依存しない）。
    #[test]
    fn the_provisional_scanner_is_stable_across_calls() {
        let scanner = UnscannedWorkspace::new();
        assert_eq!(scanner.scan(), scanner.scan());
    }
}
