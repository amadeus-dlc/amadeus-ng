//! 自己診断の観測入力を引くポート — 環境・ファイル・ストアの事実を 1 度に読む。
//!
//! 引く鍵はワークスペースそのもの (実装が構築時に保持する) なので `find` は引数を取らない。
//! 読取専用であり、観測のために状態・監査・ストアを作らない (C7 `query: read_only`)。

use super::{DoctorObservationView, ReadModelReadError};

/// ワークスペースの観測入力を読む。
pub trait DoctorObservationDao {
    /// 観測を 1 度に読む。個々の観測元の不在・不読は View が運び、ここで失敗にしない。
    ///
    /// # Errors
    ///
    /// 観測の枠組み自体を用意できない (ワークスペース根が UTF-8 で表せない等) 場合。
    fn find(&self) -> Result<DoctorObservationView, ReadModelReadError>;
}
