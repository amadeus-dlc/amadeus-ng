//! 契約テストが子プロセスとして起動する `aidlc` マルチコールバイナリを、
//! 一時ワークスペースへ「配置」する補助。
//!
//! 39 MB のバイナリを試験ごとに `fs::copy` すると、macOS では新しい実行ファイルの
//! 初回起動ごとに OS の検査（dyld の同期通知）で十数秒待たされ、全通しが数時間に
//! 膨らんだ（2026-09-11 実測: コピー 10 回の起動 2 分 39 秒、シンボリックリンク
//! 10 回 0.15 秒）。バイナリは `argv[0]` の葉名で顔（`aidlc-utility` 等）を選ぶので、
//! リンク名だけ変えれば十分であり、内容の複製は要らない。
use std::path::Path;

/// `target` の名前で `aidlc` バイナリを配置する。まずハードリンク（同じ実体なので OS の
/// 初回検査を再び払わず、走査からは通常ファイルに見える）、跨ぐファイルシステムでは
/// シンボリックリンク、それも駄目なら従来どおりコピーする。
pub(super) fn link_tool(target: &Path) -> std::io::Result<()> {
    let source = env!("CARGO_BIN_EXE_aidlc");
    if std::fs::hard_link(source, target).is_ok() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        if std::os::unix::fs::symlink(source, target).is_ok() {
            return Ok(());
        }
    }
    std::fs::copy(source, target).map(|_| ())
}
