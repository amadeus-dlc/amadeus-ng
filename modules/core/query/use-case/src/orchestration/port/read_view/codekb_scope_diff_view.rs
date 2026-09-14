//! `CodekbScopeDiffView` — `codekb-scope-diff` が返す判定 1 件。

/// 走査範囲の突合の判定。
///
/// # 1 つの共用体に status と compare の両方が入っている理由
///
/// 2 つのモードは**先頭 3 つの観測を共有する** — ストアが無い / ストアの範囲ブロックが無い /
/// その綴りが壊れている、の 3 つは upstream がモードを見る**前に**短絡して返すものである
/// (`handleCodekbScopeDiff` は store を解いてから `flags.compare` を見る)。ここを 2 つの型に
/// 割ると、同じ 3 判定の綴りを 2 か所に書くことになり、逐語契約の所在が二重化する。
/// 判定は 1 つの共用体にまとめ、**どちらのモードを引くかは注入で決める** (合成ルートが
/// 突合相手の口を結線したかどうか) 形にしてある。
///
/// # 判定は拒否ではない
///
/// どの変種も **exit 0** で返る観測である。`codekb-scope-diff` はライフサイクル動詞ではない
/// ので、判定を拒否として返さない (upstream の逐語コメント: "Always exits 0 with the verdict
/// in the output ... refusals are for lifecycle verbs")。
///
/// # 綴りは出す側が持つ
///
/// `verdict` / `reason` / `detail` の逐語 (`NO_STORE`・`absent`・`store has no fingerprint` 等)
/// はここには無い。変種の**区別**だけを運び、綴るのはプレゼンタである
/// (`coding-rules/error-handling.md` の「材料と文言を分ける」と同じ趣旨)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodekbScopeDiffView {
    /// ストアがまだ無い（初回の走査）。
    NoStore,
    /// ストアに走査範囲ブロックが無い（scope 追跡より前のストア）。
    StoreScopeAbsent {
        /// 読取器が返した材料。
        detail: String,
    },
    /// ストアの走査範囲ブロックの綴りが通らない。
    StoreScopeMalformed {
        /// 読取器が返した材料。
        detail: String,
    },
    /// ストアが指紋を記録していないので鮮度が判定できない。
    UnverifiedWithoutFingerprint {
        /// ストアを建てた intent（記録が無ければ空）。
        store_intent: String,
        /// 網羅の別（`full` / `partial`）。
        kind: String,
        /// ストアが深く読んだと主張するパス。
        analyzed_paths: Vec<String>,
    },
    /// 現在のツリーの指紋が計算できないので鮮度が判定できない。
    UnverifiedNotComputable {
        /// ストアを建てた intent（記録が無ければ空）。
        store_intent: String,
        /// 網羅の別（`full` / `partial`）。
        kind: String,
        /// ストアが深く読んだと主張するパス。
        analyzed_paths: Vec<String>,
    },
    /// ストアを建てて以降、走査したパスは変わっていない。
    Current {
        /// ストアを建てた intent（記録が無ければ空）。
        store_intent: String,
        /// 網羅の別（`full` / `partial`）。
        kind: String,
        /// ストアが深く読んだと主張するパス。
        analyzed_paths: Vec<String>,
        /// ストアが記録した指紋。
        store_fingerprint: String,
        /// いま計算した指紋。
        current_fingerprint: String,
    },
    /// ストアを建てて以降、走査したパスが変わっている。
    Stale {
        /// ストアを建てた intent（記録が無ければ空）。
        store_intent: String,
        /// 網羅の別（`full` / `partial`）。
        kind: String,
        /// ストアが深く読んだと主張するパス。
        analyzed_paths: Vec<String>,
        /// ストアが記録した指紋。
        store_fingerprint: String,
        /// いま計算した指紋。
        current_fingerprint: String,
    },
    /// 突合相手に走査範囲ブロックが無い。
    IncomingScopeAbsent {
        /// 読取器が返した材料。
        detail: String,
    },
    /// 突合相手の走査範囲ブロックの綴りが通らない。
    IncomingScopeMalformed {
        /// 読取器が返した材料。
        detail: String,
    },
    /// 取込側の走査は、ストアが主張する範囲をすべて覆っている。
    Covers {
        /// ストアを建てた intent（記録が無ければ空）。
        store_intent: String,
        /// 取込側の intent（記録が無ければ空）。
        incoming_intent: String,
    },
    /// 取込側の走査はストアより狭い — 上書きすると検証済みの主張を失う。
    Narrower {
        /// ストアを建てた intent（記録が無ければ空）。
        store_intent: String,
        /// 取込側の intent（記録が無ければ空）。
        incoming_intent: String,
        /// 覆われなくなるパス。
        discarded_paths: Vec<String>,
        /// 覆われなくなる構成要素。
        discarded_components: Vec<String>,
    },
}
