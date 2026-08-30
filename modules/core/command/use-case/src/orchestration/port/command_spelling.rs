//! `CommandSpelling` — エンジンコマンドの綴りポート。
//!
//! **どの操作を名指すか** ([`EngineCommand`]) はドメインの閉じた語彙で、**どう綴るか**
//! (upstream 3 形のうち self-host 正準のマルチコール形 — ADR 0002 決定 3) はアダプタ層の
//! 実装が持つ。ディスパッチャ語彙の完全 ROUTES 写し (30 経路 + SLASH_FLAG_ALIASES) は
//! U7 / A1 で表として実体化し、差し替えはアダプタ実装 1 点で行う (逸脱台帳 #1)。

use core_command_domain::orchestration::EngineCommand;

/// エンジンコマンドの綴りを組む (読取専用 — 文言知識の注入点)。
pub trait CommandSpelling {
    /// コマンド概念を CLI 綴りに写す。
    fn spell(&self, command: &EngineCommand) -> String;
}
