//! **リードモデル更新の共通契約** — RMU のすべての更新入口が実装する trait。
//!
//! RMU には投影の単位ごとに更新器がある（取得ループ本体・構造化面・テスト契約・計画指紋・
//! Code Generation 開始可否・runtime-graph・停止制御・心拍・自己診断・承認ランタイム）。
//! 以前はそれぞれが形の違う更新メソッドを持ち、引数も戻り値も同期・非同期もばらばらだった。
//! 呼出側（合成ルート）から見て「リードモデルを最新にする」という同じ仕事が型ごとに違う
//! 綴りで現れるのは、契約が無いことの表れである。本 trait がその契約を 1 つに揃える。

use std::future::Future;

/// リードモデルを更新する。RMU の更新入口はすべてこれを実装する。
///
/// # 契約
///
/// - **コマンドである**（`coding-rules/command-query-separation.md`）。`&mut self` を取り、
///   成功時に値を返さない。「どこまで進んだか」を知りたい側は、更新器が別に持つクエリ
///   （例: [`super::OrchestrationReadModelUpdater::checkpoint`]）か、ジャーナルの読み手を通して
///   読む — 書込の戻り値を読取チャネルにしない。
/// - **対象は構築時に束ねる**。どの実行・どの書込先・どの参照入力を描くかは、更新器を
///   組むとき（`new` / `open`）に渡す。`update_read_models` は引数を取らないので、呼出側は
///   型によらず同じ 1 行で更新を起動できる。同じ更新器を何度呼んでも同じ対象を描き直す。
/// - **境界は非同期**。内部が同期 I/O だけの更新器も、この境界では `Future` を返す。
///   合成ルートの呼出を 1 つの形に揃えるためであり、内部を非同期化する義務は無い。
///
/// 失敗の型は更新器ごとに選ぶ（取得ループ系は [`super::ReadModelUpdateError`]、manifest を
/// 1 つだけ投影する小さな単位は [`super::JournalReadError`]）。
///
/// `cargo lint` の `read-model-updater-contract` が、名前が `ReadModelUpdater` で終わる
/// 公開型に本 trait の実装を要求し、本 trait を実装する型にその名前を要求する。
pub trait ReadModelUpdater {
    /// 更新の失敗。
    type Error: std::error::Error;

    /// 構築時に束ねた対象のリードモデルを最新にする。
    ///
    /// # Errors
    ///
    /// 履歴の読取・投影・リードモデルの書込のいずれかに失敗した場合。詳細は実装ごとの
    /// [`Self::Error`] を見る。
    fn update_read_models(&mut self) -> impl Future<Output = Result<(), Self::Error>>;
}
