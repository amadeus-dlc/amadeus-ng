//! ジャーナル行の型判別子 (本家 v3 の `manifest` 列に書く値)。

/// `IntentExecutionEvent` を運ぶ封筒の `manifest`。
///
/// 本家 v3 の `manifest` は「利用者供給・自由形式の型判別子」であり、ライブラリは値を解釈せず
/// 運搬するだけである。我々はここに **payload の型と読み方の版**を書く — 旧 `schema_version`
/// (イベント JSON の予約フィールド) の後継であり、payload そのものに輸送のメタデータを混ぜずに
/// 同じ検査を行うための場所である。
///
/// 綴りは `<型>/<版>`。版を上げるのは payload の読み方が変わるときだけで、変種の追加のような
/// additive-safe な変更では上げない。
///
/// 書くのはコマンド側の `IntentExecutionRepositoryImpl`、照合するのは中間である RMU の
/// `JournalReaderImpl` である。直列化版の型判別子はイベント語彙の Published Language そのもの
/// なので、書く側と検める側が**同じ正本**を見る必要がある。したがってイベント enum の隣に置く —
/// RMU は中間としてドメインに依存してよいので、写しを作らずにここを参照できる
/// (`coding-rules/cqrs-boundaries.md` 判定表)。
///
/// 綴りは集約名に合わせて `workflow-execution-event/1` から改めた。**未配布期の改名は
/// `coding-rules/no-backward-compatibility.md` による** — ジャーナルはクローンごとの使い捨て
/// ランタイム (git 管理外) であり、配布済みの行が存在しない以上、旧綴りを温存する対価が発生
/// しないからである。**配布後は同じ改名が破壊的変更になる**ので、そのときは版を上げるか移行を
/// 用意すること。
pub const EVENT_MANIFEST: &str = "intent-execution-event/1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_names_the_payload_type_and_its_reading_version() {
        // 綴りは行に書かれて残る値である — 逐語で固定して、意図しない揺れを落とす。
        assert_eq!(EVENT_MANIFEST, "intent-execution-event/1");
    }
}
