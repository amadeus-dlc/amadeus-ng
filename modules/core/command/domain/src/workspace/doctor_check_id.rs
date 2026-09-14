//! 自己診断の検査 ID — 契約 C7 の採用表 (D1.a〜D5.b)。

use super::WorkspaceDoctorError;

/// C7 の表の行 ID。1 つの ID が複数行を出すことがある (D2.a のフック別行、D3.c / D3.d の 2 行)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorCheckId {
    /// Bun の存在。
    D1a,
    /// この build の入口。
    D1b,
    /// 設定が名指すフックの存在。
    D2a,
    /// `disableAllHooks` の解決。
    D2b,
    /// `allowManagedHooksOnly` の制限。
    D2c,
    /// 設定ファイルの存在。
    D2d,
    /// この build へのフック接続。
    D2e,
    /// hook heartbeat。
    D2f,
    /// 配布シェルの配置。
    D3a,
    /// スコープ検証。
    D3b,
    /// 循環と必要ステージの実ファイル。
    D3c,
    /// ステージ schema と参照。
    D3d,
    /// 状態版の分類。
    D4a,
    /// 状態ファイルの可読性。
    D4b,
    /// 選択中の実行の識別。
    D4c,
    /// イベントストアの可読性。
    D5a,
    /// 投影の整合。
    D5b,
}

impl DoctorCheckId {
    /// C7 の表の綴り (`D1.a` …)。保存・投影・表示の境界へ渡す。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::D1a => "D1.a",
            Self::D1b => "D1.b",
            Self::D2a => "D2.a",
            Self::D2b => "D2.b",
            Self::D2c => "D2.c",
            Self::D2d => "D2.d",
            Self::D2e => "D2.e",
            Self::D2f => "D2.f",
            Self::D3a => "D3.a",
            Self::D3b => "D3.b",
            Self::D3c => "D3.c",
            Self::D3d => "D3.d",
            Self::D4a => "D4.a",
            Self::D4b => "D4.b",
            Self::D4c => "D4.c",
            Self::D5a => "D5.a",
            Self::D5b => "D5.b",
        }
    }

    /// 保存された綴りを検査して戻す。
    ///
    /// # Errors
    ///
    /// C7 の表に無い綴り。
    pub fn parse(raw: &str) -> Result<Self, WorkspaceDoctorError> {
        const ALL: [DoctorCheckId; 17] = [
            DoctorCheckId::D1a,
            DoctorCheckId::D1b,
            DoctorCheckId::D2a,
            DoctorCheckId::D2b,
            DoctorCheckId::D2c,
            DoctorCheckId::D2d,
            DoctorCheckId::D2e,
            DoctorCheckId::D2f,
            DoctorCheckId::D3a,
            DoctorCheckId::D3b,
            DoctorCheckId::D3c,
            DoctorCheckId::D3d,
            DoctorCheckId::D4a,
            DoctorCheckId::D4b,
            DoctorCheckId::D4c,
            DoctorCheckId::D5a,
            DoctorCheckId::D5b,
        ];
        ALL.into_iter()
            .find(|id| id.as_str() == raw)
            .ok_or(WorkspaceDoctorError::InvalidCheckId)
    }
}
