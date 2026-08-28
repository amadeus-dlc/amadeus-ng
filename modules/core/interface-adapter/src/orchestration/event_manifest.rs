//! ジャーナル行の型判別子 (本家 v3 の `manifest` 列に書く値)。

/// `WorkflowExecutionEvent` を運ぶ封筒の `manifest`。
///
/// 本家 v3 の `manifest` は「利用者供給・自由形式の型判別子」であり、ライブラリは値を解釈せず
/// 運搬するだけである。我々はここに **payload の型と読み方の版**を書く — 旧 `schema_version`
/// (イベント JSON の予約フィールド) の後継であり、payload そのものに輸送のメタデータを混ぜずに
/// 同じ検査を行うための場所である。
///
/// 綴りは `<型>/<版>`。版を上げるのは payload の読み方が変わるときだけで、変種の追加のような
/// additive-safe な変更では上げない。
///
/// 書くのは `WorkflowExecutionRepositoryImpl`、照合するのは `JournalReaderImpl` である。
/// 両方が同じ綴りを見る必要があるので、コンテキスト直下の中立な場所に 1 つだけ置く
/// (どちらか一方が所有すると、コマンド側とクエリ側のどちらかが相手を知ることになる)。
pub(super) const EVENT_MANIFEST: &str = "workflow-execution-event/1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_names_the_payload_type_and_its_reading_version() {
        // 綴りは行に書かれて残る値である — 変えると既存行が読めなくなるので逐語で固定する。
        assert_eq!(EVENT_MANIFEST, "workflow-execution-event/1");
    }
}
