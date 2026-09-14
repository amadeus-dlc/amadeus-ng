//! D1.b の観測 — 実行中バイナリの実体と、この build が配線する入口の欠落。

use super::ObservationFailure;

/// 実行中バイナリの所在と、必要な入口 (面 × 動詞) のうち配線されていないもの。
///
/// 入口の照合は合成ルート (argv → 要求の写像を知る唯一の場所) が行い、その結果だけを
/// ここへ写す。診断は入口を**実行しない** — 更新動詞を打たずに配線表だけを見る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEntryPoints {
    binary: Result<String, ObservationFailure>,
    missing_entry_points: Vec<String>,
}

impl NativeEntryPoints {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(
        binary: Result<String, ObservationFailure>,
        missing_entry_points: Vec<String>,
    ) -> Self {
        Self {
            binary,
            missing_entry_points,
        }
    }

    /// 実行中バイナリの所在 (取れなければ原因)。
    pub const fn binary(&self) -> &Result<String, ObservationFailure> {
        &self.binary
    }

    /// 配線されていない入口の名前 (空なら全入口が配線済み)。
    #[must_use]
    pub fn missing_entry_points(&self) -> &[String] {
        &self.missing_entry_points
    }
}
