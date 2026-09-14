//! `CodekbRepository` の実装 — codekb ストアを 9 成果物のディレクトリとして読み書きする。
//!
//! 集約 [`Codekb`] の媒体は `aidlc/spaces/<space>/codekb/<repo>/` のディレクトリである。
//! 媒体がファイルであることは**この実装の内部詳細**で、ポート面には現れない
//! (`coding-rules/gateway-taxonomy.md` §2)。集約 `CompiledDefinition` の Repository と同型で
//! ある — どちらも配布・共有される成果物そのものが集約の住処である。
//!
//! # 置き換えはディレクトリの rename 1 回で見せる
//!
//! 9 枚を 1 枚ずつ上書きすると、読み手が「3 枚だけ新しい」中間状態を見うる。そこで新しい木を
//! トランザクションディレクトリの下に組み立ててから、**rename 1 回**でストアの位置へ差し込む。
//! 既存のストアは先に `backup` へ退避し、差し込みに失敗したら戻す。
//!
//! # 中断した公開は「観測して、集約が畳み、`store` が果たす」
//!
//! 退避まで済んで差し込みの前に落ちると、ストアが不在で退避だけが残る。この形は
//! [`CodekbRepository::find_by_id`] が**観測結果の一部**として集約に載せ、畳むかどうかは集約
//! ([`Codekb::settle_interrupted_publication`]) が決める。読取は 1 バイトも書き換えない —
//! 読取メソッドが黙ってディレクトリを rename するのは CQS 違反であり
//! (`coding-rules/command-query-separation.md`)、呼出側からは書込が見えなくなる。実際に畳む
//! のは決着の事実を受けた [`CodekbRepository::store`] で、その手順 (退避を戻し、痕跡を消す)
//! はこの実装の内部詳細である (upstream `recoverCodekbTransactions`)。
//!
//! upstream との差は 1 つだけで、**観測できない**: upstream は公開後にトランザクションの
//! 根を空のまま残し、次回の復旧で消す。こちらは空になった根をその場で片付ける。CLI の
//! stdout・終了コードは変わらず、残るのは git 管理外の作業ディレクトリだけである。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use core_command_domain::workspace::{Codekb, CodekbEvent, CodekbGeneration, CodekbRepoId};
use core_command_use_case::orchestration::{CodekbRepository, RepositoryError};
use core_infrastructure::collections::FirstClassCollection as _;
use core_infrastructure::tree_hash::hash_tree;

/// トランザクションディレクトリの名前を同一プロセス内で衝突させないための連番。
static TRANSACTION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 組み立て中の新しい木を置く名前 (upstream 逐語)。
const NEXT_DIR: &str = "next";
/// 既存のストアを退避する名前 (upstream 逐語)。
const BACKUP_DIR: &str = "backup";
/// 世代を畳むときに渡す唯一のパス指定 (ストア全体)。
const WHOLE_STORE: &str = "./";

/// codekb ストアの実 Gateway。
///
/// 置き場 (ストアとトランザクションの根) は**合成ルートが解決して渡す** — `aidlc/` 配下の
/// 配置を決めるのは Layout の仕事であり、Gateway はパスの組み立て方を知らない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodekbRepositoryImpl {
    store_dir: PathBuf,
    transaction_root: PathBuf,
}

impl CodekbRepositoryImpl {
    /// ストアの置き場と、トランザクションの根を受け取る (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn new(store_dir: &Path, transaction_root: &Path) -> CodekbRepositoryImpl {
        CodekbRepositoryImpl {
            store_dir: store_dir.to_path_buf(),
            transaction_root: transaction_root.to_path_buf(),
        }
    }

    /// 中断した公開が残っているか (**読むだけ** — 畳まない)。
    ///
    /// 媒体では「トランザクションの根が残っている」ことがその姿である。
    fn holds_an_interrupted_publication(&self) -> bool {
        self.transaction_root.exists()
    }

    /// 中断した公開を畳んで、ストアを「いま在る」姿へ確定させる
    /// (upstream `recoverCodekbTransactions`)。
    ///
    /// 退避が残っていて**ストアが不在のときだけ**戻す。ストアが在るなら、そちらのほうが
    /// 新しい (差し込みまで済んでいた) ので触らない。
    fn settle_interrupted_publication(&mut self) -> Result<(), RepositoryError<CodekbRepoId>> {
        if !self.transaction_root.exists() {
            return Ok(());
        }
        let entries = fs::read_dir(&self.transaction_root)
            .map_err(|error| io_at(&self.transaction_root, &error))?;
        let mut transactions: Vec<PathBuf> = Vec::new();
        for entry in entries {
            transactions.push(
                entry
                    .map_err(|error| io_at(&self.transaction_root, &error))?
                    .path(),
            );
        }
        transactions.sort();
        for transaction in transactions {
            if !is_real_directory(&transaction) {
                // ディレクトリでないもの・リンクは畳まない (差し替えの入口にしない)。
                continue;
            }
            let backup = transaction.join(BACKUP_DIR);
            if !self.store_dir.exists() && is_real_directory(&backup) {
                if let Some(parent) = self.store_dir.parent() {
                    fs::create_dir_all(parent).map_err(|error| io_at(parent, &error))?;
                }
                fs::rename(&backup, &self.store_dir)
                    .map_err(|error| io_at(&self.store_dir, &error))?;
            }
            fs::remove_dir_all(&transaction).map_err(|error| io_at(&transaction, &error))?;
        }
        fs::remove_dir_all(&self.transaction_root)
            .map_err(|error| io_at(&self.transaction_root, &error))?;
        Ok(())
    }

    /// いまディスクに在るストアの世代を観測する。
    fn observe_generation(
        &self,
        id: &CodekbRepoId,
    ) -> Result<CodekbGeneration, RepositoryError<CodekbRepoId>> {
        if !self.store_dir.exists() {
            return Ok(CodekbGeneration::absent());
        }
        hash_tree(&self.store_dir, &[WHOLE_STORE.to_string()], &[])
            .map(|hash| CodekbGeneration::of_tree_hash(&hash))
            .ok_or_else(|| {
                corrupt(
                    id,
                    format!(
                        "cannot compute the CodeKB store generation for {}",
                        self.store_dir.display()
                    ),
                )
            })
    }
}

impl CodekbRepository for CodekbRepositoryImpl {
    async fn find_by_id(&self, id: &CodekbRepoId) -> Result<Codekb, RepositoryError<CodekbRepoId>> {
        // 読取だけ — 畳む前のディスクの姿をそのまま観測する。中断した公開が残っていることも
        // 観測結果であり、集約はそれを抱えたまま再構成される。
        let generation = self.observe_generation(id)?;
        if self.holds_an_interrupted_publication() {
            return Ok(Codekb::observed_with_interrupted_publication(
                id.clone(),
                generation,
            ));
        }
        Ok(Codekb::observed(id.clone(), generation))
    }

    async fn store(
        &mut self,
        event: &CodekbEvent,
        codekb: &Codekb,
    ) -> Result<(), RepositoryError<CodekbRepoId>> {
        let id = codekb.id();
        // 対の取り違えは「歴史と保存像が別の内容を語る」書込契約違反として拒む
        // (`IntentRepositoryImpl` / `CompiledDefinitionRepositoryImpl` と同じ作法)。
        if !codekb.describes(event) {
            return Err(corrupt(
                id,
                "store pair mismatch: the event does not describe the aggregate".to_string(),
            ));
        }
        match event {
            CodekbEvent::Published(published) => self.lay_down(published.artifacts()),
            // 決着の事実が運ぶのは「どの codekb か」だけで、畳む手順はこの実装の内部詳細である。
            CodekbEvent::InterruptedPublicationSettled(_) => self.settle_interrupted_publication(),
        }
    }
}

impl CodekbRepositoryImpl {
    /// 公開した 9 成果物を、トランザクションの下で組み立ててからストアの位置へ差し込む。
    fn lay_down(
        &self,
        artifacts: &core_command_domain::workspace::CodekbArtifacts,
    ) -> Result<(), RepositoryError<CodekbRepoId>> {
        let transaction = self.transaction_root.join(format!(
            "{}-{}",
            std::process::id(),
            TRANSACTION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let next = transaction.join(NEXT_DIR);
        let backup = transaction.join(BACKUP_DIR);
        if let Err(error) = fs::create_dir_all(&next) {
            return Err(io_at(&next, &error));
        }

        let outcome = self.swap_in(artifacts, &next, &backup);
        // 成否によらずトランザクションの痕跡は片付ける。
        drop(fs::remove_dir_all(&transaction));
        drop(fs::remove_dir(&self.transaction_root));
        outcome
    }

    /// 新しい木を組み立てて、rename 1 回でストアの位置へ差し込む。
    fn swap_in(
        &self,
        artifacts: &core_command_domain::workspace::CodekbArtifacts,
        next: &Path,
        backup: &Path,
    ) -> Result<(), RepositoryError<CodekbRepoId>> {
        let mut index = 0;
        while let Some(artifact) = artifacts.at(index) {
            let path = next.join(artifact.name().as_str());
            fs::write(&path, artifact.bytes()).map_err(|error| io_at(&path, &error))?;
            index += 1;
        }
        if let Some(parent) = self.store_dir.parent() {
            fs::create_dir_all(parent).map_err(|error| io_at(parent, &error))?;
        }
        let had_store = self.store_dir.exists();
        if had_store {
            fs::rename(&self.store_dir, backup).map_err(|error| io_at(backup, &error))?;
        }
        if let Err(error) = fs::rename(next, &self.store_dir) {
            // 差し込みに失敗したら、退避しておいた元のストアを戻す。
            if had_store && backup.exists() && !self.store_dir.exists() {
                drop(fs::rename(backup, &self.store_dir));
            }
            return Err(io_at(&self.store_dir, &error));
        }
        drop(fs::remove_dir_all(backup));
        Ok(())
    }
}

/// シンボリックリンクでない実ディレクトリか。
fn is_real_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
}

/// OS 由来の失敗をポート契約のエラーへ写す。
fn io_at(path: &Path, error: &io::Error) -> RepositoryError<CodekbRepoId> {
    RepositoryError::Io {
        kind: error.kind(),
        path: Some(path.to_path_buf()),
    }
}

/// 「読めたが内容が壊れている」をポート契約のエラーへ写す。
///
/// 分類は契約に載せない (裁定 6) — 原因はアダプタ私有の型を `Error::source` の連鎖で運ぶ。
fn corrupt(id: &CodekbRepoId, detail: String) -> RepositoryError<CodekbRepoId> {
    RepositoryError::Corrupt {
        id: id.clone(),
        seq_nr: None,
        source: Box::new(CodekbStoreCorruption(detail)),
    }
}

/// ストアが壊れていた原因 (アダプタ私有 — 診断表示だけを運ぶ)。
#[derive(Debug)]
struct CodekbStoreCorruption(String);

impl std::fmt::Display for CodekbStoreCorruption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CodekbStoreCorruption {}
