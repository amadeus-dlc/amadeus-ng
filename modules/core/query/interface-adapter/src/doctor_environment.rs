//! 自己診断が見る環境 — PATH・HOME・管理設定の所在 (合成ルートが `std::env` から写す)。

use std::ffi::OsString;
use std::path::PathBuf;

/// 環境変数の写し。テストはプロセス環境を触らずに値を差し替えられる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorEnvironment {
    path: Option<OsString>,
    home: Option<PathBuf>,
    managed_settings_path: Option<PathBuf>,
}

impl DoctorEnvironment {
    /// `PATH`・`HOME`・`AIDLC_MANAGED_SETTINGS_PATH` を束ねる。
    #[must_use]
    pub const fn new(
        path: Option<OsString>,
        home: Option<PathBuf>,
        managed_settings_path: Option<PathBuf>,
    ) -> Self {
        Self {
            path,
            home,
            managed_settings_path,
        }
    }

    /// `PATH`。
    #[must_use]
    pub const fn path(&self) -> Option<&OsString> {
        self.path.as_ref()
    }

    /// `HOME` (Windows では `USERPROFILE` を写す)。
    #[must_use]
    pub const fn home(&self) -> Option<&PathBuf> {
        self.home.as_ref()
    }

    /// 管理設定の明示的な所在 (本家 `AIDLC_MANAGED_SETTINGS_PATH`)。
    #[must_use]
    pub const fn managed_settings_path(&self) -> Option<&PathBuf> {
        self.managed_settings_path.as_ref()
    }
}
