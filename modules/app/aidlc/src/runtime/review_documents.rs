//! 宣言されたレビュー成果物を、同一の安定読取りとして観測する入力境界。
use crate::layout::Layout;
use core_command_domain::orchestration::{ReviewArtifact, ReviewDocuments, ReviewDraft};
use serde_json::Value;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// 宣言されたレビュー成果物と、判定なら reviewer の下書き（`drafts`）を束ねる。
pub(super) fn read(layout: &Layout, stage: &str, drafts: Vec<ReviewDraft>) -> ReviewDocuments {
    let captured = observe(layout, stage).and_then(|(artifacts, require_artifacts, workspace)| {
        let source = workspace.then(|| {
            crate::source_fingerprint::read(layout.project_dir())
                .unwrap_or_else(|_| "unbindable".into())
        });
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce).map_err(|e| e.to_string())?;
        let nonce = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
        Ok(ReviewDocuments::new(
            artifacts,
            require_artifacts,
            source,
            nonce,
            true,
            drafts.clone(),
        ))
    });
    captured.unwrap_or_else(|_| {
        ReviewDocuments::new(Vec::new(), true, None, String::new(), false, drafts)
    })
}

/// 宣言されたレビュー成果物だけを観測する（読取専用の面が使う入口）。
///
/// [`read`] と同じ観測を共有しつつ、ソース指紋と nonce を採らない。どちらも束縛
/// （`bind` / `certify`）のための材料であって、描くだけの面には要らないからである。
/// # 観測できないことは「所見なし」ではない
///
/// 観測の失敗は呼び手へ返す。畳んで空にすると、成果物がハードリンク化・差し替え中・
/// symlink 越し・読取不能のときに「開いた所見は無い」という本文が承認ゲートへ出てしまう
/// （同じ観測を [`read`] 経路はドメインの `ArtifactsUnavailable` で拒否するので、
/// 同一状態で 2 入口の答えが食い違う）。upstream `aidlc-review-brief.ts` も読取りの失敗を
/// `throw` して exit 1 で終える。
///
/// 空へ倒すのは**記録がまだ選ばれていない**ときだけである（upstream `aidlc-lib.ts` の
/// `recordDir === null` と同じ 1 分岐）。そのときは描けるレビュー成果物が 1 つも無い。
/// # Errors
/// 記録は在るが、定義グラフ・成果物のいずれかを安定した原文として観測できない場合。
pub(super) fn artifacts(layout: &Layout, stage: &str) -> Result<Vec<ReviewArtifact>, String> {
    if layout.record_dir().is_none() {
        return Ok(Vec::new());
    }
    observe(layout, stage).map(|(artifacts, _, _)| artifacts)
}

/// 宣言された成果物の観測と、`require_artifacts` / `workspace_requires` の宣言。
fn observe(layout: &Layout, stage: &str) -> Result<(Vec<ReviewArtifact>, bool, bool), String> {
    let graph: Vec<Value> = serde_json::from_slice(
        &fs::read(layout.definition_data_dir().join("stage-graph.json"))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let node = graph
        .iter()
        .find(|n| n.get("slug").and_then(Value::as_str) == Some(stage))
        .ok_or("missing stage")?;
    let phase = node
        .get("phase")
        .and_then(Value::as_str)
        .ok_or("missing phase")?;
    let target = node
        .get("review_artifact")
        .and_then(Value::as_str)
        .ok_or("missing review_artifact")?;
    let record = layout.record_dir().ok_or("missing record")?;
    let per_unit = node.get("for_each").and_then(Value::as_str) == Some("unit-of-work");
    let require_artifacts = !per_unit;
    let mut artifacts = Vec::new();
    let mut reads = Vec::new();
    for (field, required) in [("produces", true), ("optional_produces", false)] {
        for name in node
            .get(field)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            let filename = if name == "traceability" {
                "traceability.json".into()
            } else {
                format!("{name}.md")
            };
            let logical = format!("{phase}/{stage}/{filename}");
            let path = record.join(&logical);
            inspect_path(record, &path)?;
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => Some(metadata),
                Err(e) if e.kind() == io::ErrorKind::NotFound => None,
                Err(e) => return Err(e.to_string()),
            };
            let Some(metadata) = metadata else {
                artifacts.push(ReviewArtifact::new(
                    logical,
                    None,
                    true,
                    required,
                    name == target,
                ));
                reads.push((path, None, None));
                continue;
            };
            if !metadata.is_file() {
                artifacts.push(ReviewArtifact::new(
                    logical,
                    None,
                    false,
                    required,
                    name == target,
                ));
                reads.push((path, Some(metadata), None));
                continue;
            }
            let mut options = fs::OpenOptions::new();
            options.read(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt as _;
                options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
            }
            let mut file = options.open(&path).map_err(|e| e.to_string())?;
            let opened = file.metadata().map_err(|e| e.to_string())?;
            if !same(&metadata, &opened) || !single_link(&opened) {
                return Err("unstable review file".into());
            }
            let mut body = Vec::new();
            use std::io::Read as _;
            file.read_to_end(&mut body).map_err(|e| e.to_string())?;
            if !same(&opened, &file.metadata().map_err(|e| e.to_string())?) {
                return Err("changed review file".into());
            }
            artifacts.push(ReviewArtifact::new(
                logical,
                Some(body),
                true,
                required,
                name == target,
            ));
            reads.push((path, Some(opened), Some(file)));
        }
    }
    for (path, previous, file) in reads {
        inspect_path(record, &path)?;
        let current = match fs::symlink_metadata(&path) {
            Ok(meta) => Some(meta),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        match (previous, current) {
            (None, None) => (),
            (Some(a), Some(b)) if same(&a, &b) => {
                if let Some(file) = file
                    && !same(&a, &file.metadata().map_err(|e| e.to_string())?)
                {
                    return Err("changed review file".into());
                }
            }
            _ => return Err("changed review set".into()),
        }
    }
    Ok((
        artifacts,
        require_artifacts,
        node.get("workspace_requires") == Some(&Value::Bool(true)),
    ))
}
fn inspect_path(root: &Path, path: &Path) -> Result<(), String> {
    let mut current = PathBuf::from(root);
    let parts = path.strip_prefix(root).map_err(|e| e.to_string())?;
    for part in parts.components() {
        if !matches!(part, std::path::Component::Normal(_)) {
            return Err("review path outside record".into());
        }
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(m) if m.file_type().is_symlink() => return Err("symlink review path".into()),
            Ok(_) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => break,
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}
fn same(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    a.len() == b.len()
        && a.modified().ok() == b.modified().ok()
        && !crate::source_fingerprint::metadata_changed(a, b)
        && identity(a, b)
}
#[cfg(unix)]
fn identity(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    a.dev() == b.dev() && a.ino() == b.ino() && a.mode() == b.mode() && a.nlink() == b.nlink()
}
#[cfg(not(unix))]
fn identity(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    a.is_file() == b.is_file() && a.is_dir() == b.is_dir()
}
#[cfg(unix)]
fn single_link(m: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    m.nlink() == 1
}
#[cfg(not(unix))]
fn single_link(_: &fs::Metadata) -> bool {
    true
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use core_command_domain::orchestration::ReviewEvidenceError;

    /// 1 段のレビュー定義と記録を持つ配置。
    fn workspace() -> (tempfile::TempDir, Layout) {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let data = root.path().join(".claude/tools/data");
        fs::create_dir_all(&data).expect("data");
        fs::write(
            data.join("stage-graph.json"),
            r#"[{"slug":"domain-design","phase":"inception","review_artifact":"domain-design",
                 "produces":["domain-design"],"optional_produces":["notes"]}]"#,
        )
        .expect("graph");
        let intents = root.path().join("aidlc/spaces/default/intents");
        let record = intents.join("260101-review-aaaaaaaa");
        fs::create_dir_all(record.join("inception/domain-design")).expect("記録");
        fs::write(record.join("aidlc-state.md"), "# state\n").expect("状態");
        fs::write(intents.join("active-intent"), "260101-review-aaaaaaaa\n").expect("カーソル");
        let layout = Layout::resolve(root.path());
        assert!(layout.record_dir().is_some());
        (root, layout)
    }

    /// 通常ファイルの成果物は束ねられ、任意成果物の欠落は束縛を妨げない。
    #[test]
    fn regular_artifacts_bind_and_missing_optional_ones_do_not_block() {
        let (_root, layout) = workspace();
        let record = layout.record_dir().expect("記録").to_path_buf();
        fs::write(
            record.join("inception/domain-design/domain-design.md"),
            "# Domain design\n",
        )
        .expect("成果物");
        let documents = read(&layout, "domain-design", Vec::new());
        let binding = documents.bind(None).expect("束縛");
        assert_eq!(
            binding.appendix_artifact(),
            "inception/domain-design/domain-design.md"
        );
        assert!(
            read(&layout, "no-such-stage", Vec::new())
                .bind(None)
                .is_err(),
            "未知の stage は束ねない"
        );
    }

    /// ディレクトリ・ハードリンク・シンボリックリンクを経る成果物は「安定した原文」ではない。
    #[cfg(unix)]
    #[test]
    fn irregular_and_multiply_linked_artifacts_are_not_stable_evidence() {
        let (_root, layout) = workspace();
        let record = layout.record_dir().expect("記録").to_path_buf();
        let artifact = record.join("inception/domain-design/domain-design.md");
        fs::create_dir(&artifact).expect("同名のディレクトリ");
        assert!(matches!(
            read(&layout, "domain-design", Vec::new()).bind(None),
            Err(ReviewEvidenceError::ArtifactsUnavailable)
        ));
        fs::remove_dir(&artifact).expect("消す");
        fs::write(&artifact, "# Domain design\n").expect("成果物");
        fs::hard_link(&artifact, record.join("twin.md")).expect("ハードリンク");
        assert!(matches!(
            read(&layout, "domain-design", Vec::new()).bind(None),
            Err(ReviewEvidenceError::ArtifactsUnavailable)
        ));
        fs::remove_file(record.join("twin.md")).expect("消す");
        assert!(
            read(&layout, "domain-design", Vec::new())
                .bind(None)
                .is_ok()
        );
        // stage ディレクトリをシンボリックリンクに差し替える。
        let stage = record.join("inception/domain-design");
        let elsewhere = record.join("elsewhere");
        fs::rename(&stage, &elsewhere).expect("退避");
        std::os::unix::fs::symlink(&elsewhere, &stage).expect("リンク");
        assert!(matches!(
            read(&layout, "domain-design", Vec::new()).bind(None),
            Err(ReviewEvidenceError::ArtifactsUnavailable)
        ));
    }

    /// 定義グラフが読めない・壊れている・成果物へ至れない・読めないなら、束縛できない。
    #[cfg(unix)]
    #[test]
    fn unreadable_definitions_and_artifacts_never_bind() {
        use std::os::unix::fs::PermissionsExt as _;
        let (root, layout) = workspace();
        let record = layout.record_dir().expect("記録").to_path_buf();
        let artifact = record.join("inception/domain-design/domain-design.md");
        fs::write(&artifact, "# Domain design\n").expect("成果物");
        // 読めない成果物（在るが権限が無い）。
        fs::set_permissions(&artifact, fs::Permissions::from_mode(0o000)).expect("権限");
        assert!(
            read(&layout, "domain-design", Vec::new())
                .bind(None)
                .is_err()
        );
        fs::set_permissions(&artifact, fs::Permissions::from_mode(0o644)).expect("権限");
        // 途中の成分がディレクトリでない。
        let directory = record.join("inception/domain-design");
        fs::remove_dir_all(&directory).expect("削除");
        fs::write(&directory, "not a directory\n").expect("ファイル化");
        assert!(
            read(&layout, "domain-design", Vec::new())
                .bind(None)
                .is_err()
        );
        // 定義グラフが壊れている・無い。
        let graph = root.path().join(".claude/tools/data/stage-graph.json");
        fs::write(&graph, "{").expect("壊す");
        assert!(
            read(&layout, "domain-design", Vec::new())
                .bind(None)
                .is_err()
        );
        fs::remove_file(&graph).expect("消す");
        assert!(
            read(&layout, "domain-design", Vec::new())
                .bind(None)
                .is_err()
        );
    }

    /// `workspace_requires` の段でソース指紋が採れなければ `unbindable` として束ねる。
    #[cfg(unix)]
    #[test]
    fn an_unreadable_source_tree_binds_as_unbindable() {
        use std::os::unix::fs::PermissionsExt as _;
        let (root, layout) = workspace();
        fs::write(
            root.path().join(".claude/tools/data/stage-graph.json"),
            r#"[{"slug":"domain-design","phase":"inception","review_artifact":"domain-design",
                 "produces":["domain-design"],"workspace_requires":true}]"#,
        )
        .expect("graph");
        let record = layout.record_dir().expect("記録").to_path_buf();
        fs::write(
            record.join("inception/domain-design/domain-design.md"),
            "# Domain design\n",
        )
        .expect("成果物");
        let sealed = root.path().join("src/sealed");
        fs::create_dir_all(&sealed).expect("src");
        fs::set_permissions(&sealed, fs::Permissions::from_mode(0o000)).expect("権限");
        let documents = read(&layout, "domain-design", Vec::new());
        fs::set_permissions(&sealed, fs::Permissions::from_mode(0o755)).expect("権限");
        let binding = documents.bind(None).expect("束縛");
        assert_eq!(binding.source(), Some("unbindable"));
    }
}
