//! `IntentReposDao` ポート — intent に記録されたリポジトリ集合を引く DAO。

/// intent 登録簿の行が持つ `repos` を引く（**読取専用**）。
///
/// codekb は**リポジトリ名で鍵付けられ** intent をまたいで共有されるので、「どのリポジトリの
/// 知識か」を決めるのにこの記録が要る。記録が無い（単一リポジトリ構成）ときは空で返し、
/// 呼び手がワークスペース名へ後退する。
///
/// # 失敗の通路を持たない
///
/// 戻り値が `Result` ではないのは、upstream が登録簿の不在・壊れた JSON を**すべて同じ
/// 空配列**に畳んでいるからである（`readIntentRegistry` の `catch` → `[]`）。ここに失敗の
/// 通路を足すと、upstream なら名前の後退で答える場面をこちらだけが拒否してしまう。
pub trait IntentReposDao {
    /// 記録ディレクトリ名に対応する行の `repos` を引く（行が無ければ空）。
    fn find(&self, record_dir_name: &str) -> Vec<String>;
}
