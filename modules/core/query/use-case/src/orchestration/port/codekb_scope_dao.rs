//! `CodekbScopeDao` ポート — 走査範囲を記録した 1 面を引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::ReScopeParseView;

/// `reverse-engineering-timestamp.md` 1 面の走査範囲ブロックを引く（**読取専用**）。
///
/// 引く先は 2 通りあるが**契約は 1 つ**である — durable な codekb ストアの記録と、突合の
/// ために渡された取込側の走査記録。どちらを見るかは実装が握る（合成ルートが結線する）ので、
/// ポート面は鍵を取らない（[`super::StateFileDao`] と同じ流儀）。
///
/// # 媒体はポート契約に漏らさない
///
/// 実装は Markdown を読んで中の fenced yaml を解くが、その事実はここに現れない
/// (`port/mod.rs` の「媒体はポート契約に漏らさない」)。
pub trait CodekbScopeDao {
    /// 走査範囲の読取結果を引く。
    ///
    /// **不在は失敗ではない** — ストアがまだ無いのは正常な観測なので `Ok(None)` で返す。
    /// それを「初回の走査」と見るか「突合の相手が無い」と見るかは読んだ側が決める。
    /// ブロックが読めないこと自体も失敗ではなく [`ReScopeParseView`] が運ぶ観測である。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(&self) -> Result<Option<ReScopeParseView>, ReadModelReadError>;
}
