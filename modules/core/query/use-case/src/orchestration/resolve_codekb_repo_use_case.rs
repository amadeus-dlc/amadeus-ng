//! `ResolveCodekbRepoUseCase` — codekb を鍵付けるリポジトリ名を決める (upstream
//! `codekbRepoName` と `handleCodekbPath` の `--repo` 優先)。

use crate::orchestration::IntentReposDao;

/// codekb の鍵になるリポジトリ名を引く。
///
/// codekb は**リポジトリ名で鍵付けられ**、intent をまたいで space 全体で共有される。したがって
/// 「どのリポジトリの知識か」は intent の記録から決まり、記録が無ければワークスペース名へ
/// 後退する。3 つの読取源を優先順で選び分けて 1 つの答えを組むのは、規則 6 (2026-09-03) が
/// クエリ側ユースケースに認めた組み立てである (`coding-rules/cqrs-boundaries.md`)。
///
/// 呼び手が名指した名前を**検証しない**のは upstream に合わせたものである — `--repo` は
/// そのままパス片として使われる。ここで弾くと、upstream が答える場面をこちらだけが落とす。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveCodekbRepoUseCase<R: IntentReposDao> {
    intent_repos_dao: R,
}

impl<R: IntentReposDao> ResolveCodekbRepoUseCase<R> {
    /// 引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(intent_repos_dao: R) -> ResolveCodekbRepoUseCase<R> {
        ResolveCodekbRepoUseCase { intent_repos_dao }
    }

    /// リポジトリ名を決める。
    ///
    /// 優先順は upstream どおり:
    /// 1. 呼び手が名指した名前 (`--repo`、空文字は名指しに数えない)
    /// 2. intent が**ちょうど 1 つ**だけ記録しているリポジトリ
    /// 3. ワークスペースのディレクトリ名 (記録が 0 個、または 2 個以上で決められないとき)
    #[must_use]
    pub fn execute(
        &self,
        requested: Option<&str>,
        record_dir_name: Option<&str>,
        workspace_name: &str,
    ) -> String {
        if let Some(requested) = requested.filter(|name| !name.is_empty()) {
            return requested.to_string();
        }
        let recorded = record_dir_name
            .map(|name| self.intent_repos_dao.find(name))
            .unwrap_or_default();
        match recorded.as_slice() {
            [only] => only.clone(),
            _ => workspace_name.to_string(),
        }
    }
}
