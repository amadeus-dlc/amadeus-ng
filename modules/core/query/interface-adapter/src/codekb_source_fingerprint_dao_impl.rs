//! `CodekbSourceFingerprintDao` の実 Gateway — git の一時インデックス経由で作業ツリーの
//! 内容指紋を取る。
//!
//! 相手方 (git) の契約を知る gateway なのでインターフェイスアダプタ層に置く
//! (`coding-rules/infrastructure-layer.md`「RPC クライアント・DB アクセスは置かない」の裏返し
//! — 機構ではなく相手の契約を知る口である)。
//!
//! # なぜコミットではなく作業ツリーなのか
//!
//! `git add -A` で一時インデックスを作り `git write-tree` を取る。**ディスクに在るものを**
//! 内容アドレスで表すので、リベース・squash・amend でコミットが消えても比較は壊れず、編集を
//! 元に戻せば指紋も元に戻る。無視されたファイルは `git add` の意味論どおり入らない。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use core_query_use_case::orchestration::CodekbSourceFingerprintDao;

/// 一時インデックスの名前を同一プロセス内で衝突させないための連番。
static INDEX_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 作業ツリーの内容指紋を取る実装。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodekbSourceFingerprintDaoImpl {
    repo_dir: PathBuf,
    excluded_paths: Vec<String>,
}

impl CodekbSourceFingerprintDaoImpl {
    /// 対象リポジトリと、その中で指紋から除くパスを受け取る (**この型の唯一の構築経路**)。
    ///
    /// 単一リポジトリ構成では枠組み自身の `aidlc/` を除く — 走査記録・codekb・監査・状態を
    /// 書くこと自体が全体指紋を古くしてしまわないようにするためである。
    #[must_use]
    pub fn new(repo_dir: &Path, excluded_paths: Vec<String>) -> CodekbSourceFingerprintDaoImpl {
        CodekbSourceFingerprintDaoImpl {
            repo_dir: repo_dir.to_path_buf(),
            excluded_paths,
        }
    }

    /// `git` を対象リポジトリで走らせる。
    fn git(&self, args: &[&str], index_file: Option<&Path>) -> Option<std::process::Output> {
        let mut command = Command::new("git");
        command.arg("-C").arg(&self.repo_dir).args(args);
        if let Some(index_file) = index_file {
            command.env("GIT_INDEX_FILE", index_file);
        }
        command.output().ok()
    }

    /// 作業ツリーかどうか。
    fn is_work_tree(&self) -> bool {
        self.git(&["rev-parse", "--is-inside-work-tree"], None)
            .is_some_and(|output| {
                output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true"
            })
    }
}

impl CodekbSourceFingerprintDao for CodekbSourceFingerprintDaoImpl {
    fn find(&self, paths: &[String]) -> Option<String> {
        if paths.is_empty() {
            return None;
        }
        let exclusions: Vec<String> = self
            .excluded_paths
            .iter()
            .map(|path| normalize(path))
            .collect();
        // 除外の下に丸ごと入るパスは、そもそも指紋の材料にならない。
        let surviving: Vec<&String> = paths
            .iter()
            .filter(|path| {
                let positive = normalize(path);
                !exclusions.iter().any(|excluded| {
                    *excluded == positive || positive.starts_with(&format!("{excluded}/"))
                })
            })
            .collect();
        if surviving.is_empty() {
            return None;
        }
        // 生き残ったパスの**内側**に落ちる除外だけを git へ渡す (外側の除外は無意味で、
        // pathspec として渡すと git が不一致で落ちる)。
        let exclude_args: Vec<String> = exclusions
            .iter()
            .filter(|excluded| {
                surviving.iter().any(|path| {
                    let positive = normalize(path);
                    positive.is_empty() || excluded.starts_with(&format!("{positive}/"))
                })
            })
            .map(|excluded| format!(":(exclude,literal){excluded}"))
            .collect();

        if !self.is_work_tree() {
            return None;
        }

        let index_file = std::env::temp_dir().join(format!(
            ".aidlc-scope-index-{}-{}",
            std::process::id(),
            INDEX_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let fingerprint = self.write_tree(&surviving, &exclude_args, &index_file);
        // 後始末は best-effort — 取り残された一時インデックスは無害である。
        drop(std::fs::remove_file(&index_file));
        fingerprint
    }
}

impl CodekbSourceFingerprintDaoImpl {
    /// 一時インデックスへ stage して木を書く。どこかで転んだら `None`。
    fn write_tree(
        &self,
        surviving: &[&String],
        exclude_args: &[String],
        index_file: &Path,
    ) -> Option<String> {
        let mut add: Vec<&str> = vec!["add", "-A", "--"];
        add.extend(surviving.iter().map(|path| path.as_str()));
        add.extend(exclude_args.iter().map(String::as_str));
        if !self
            .git(&add, Some(index_file))
            .is_some_and(|output| output.status.success())
        {
            return None;
        }
        // 1 つも stage されなければ空ツリーの指紋を返さない — 「範囲が空」を「範囲が一致」と
        // 読み違えさせないためである。
        if !self
            .git(&["ls-files", "-z"], Some(index_file))
            .is_some_and(|output| output.status.success() && !output.stdout.is_empty())
        {
            return None;
        }
        let output = self.git(&["write-tree"], Some(index_file))?;
        if !output.status.success() {
            return None;
        }
        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let shaped = matches!(hash.len(), 40..=64)
            && hash
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
        shaped.then_some(hash)
    }
}

/// pathspec を突き合わせ可能な形へ正規化する (`\` を `/` へ、先頭の `./` と末尾の `/` を落とし、
/// `.` は「リポジトリ根」を表す空文字にする)。
fn normalize(path: &str) -> String {
    let mut normalized = path.replace('\\', "/");
    while let Some(rest) = normalized.strip_prefix("./") {
        normalized = rest.to_string();
    }
    let normalized = normalized.trim_end_matches('/');
    if normalized == "." {
        String::new()
    } else {
        normalized.to_string()
    }
}
