//! ジャーナル行の型判別子 (本家 v3 の `manifest` 列に書く値)。

/// `IntentEvent` を運ぶ封筒の `manifest`。
///
/// 本家 v3 の `manifest` は「利用者供給・自由形式の型判別子」であり、ライブラリは値を解釈せず
/// 運搬するだけである。我々はここに **payload の型と読み方の版**を書く — 旧 `schema_version`
/// (イベント JSON の予約フィールド) の後継であり、payload そのものに輸送のメタデータを混ぜずに
/// 同じ検査を行うための場所である。
///
/// 綴りは `<型>/<版>`。版を上げるのは payload の読み方が変わるときだけで、変種の追加のような
/// additive-safe な変更では上げない。
///
/// 書くのはコマンド側の `IntentRepositoryImpl`、照合するのは中間である RMU の
/// `JournalReaderImpl` である。直列化版の型判別子はイベント語彙の Published Language そのもの
/// なので、書く側と検める側が**同じ正本**を見る必要がある。したがってイベント enum の隣に置く —
/// RMU は中間としてドメインに依存してよいので、写しを作らずにここを参照できる
/// (`coding-rules/cqrs-boundaries.md` 判定表)。
pub const EVENT_MANIFEST: &str = "workflow-execution-event/1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_names_the_payload_type_and_its_reading_version() {
        // 綴りは行に書かれて残る値である — 変えると既存行が読めなくなるので逐語で固定する。
        assert_eq!(EVENT_MANIFEST, "workflow-execution-event/1");
    }
}
