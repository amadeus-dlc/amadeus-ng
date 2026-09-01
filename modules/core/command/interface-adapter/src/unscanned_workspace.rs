//! `UnscannedWorkspace` — [`WorkspaceScanner`](crate::WorkspaceScanner) の暫定実装。

use core_command_domain::orchestration::WorkspaceScan;
use core_command_domain::workflow_definition::BrownfieldGreenfield;
use core_command_domain::workspace::UnsafeLineChar;

use crate::workspace_scanner::WorkspaceScanner;

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
