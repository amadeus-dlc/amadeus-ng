//! 開始コマンドへ渡すソース比較基準の採取境界。
use crate::source_fingerprint::{executable, is_harness, metadata_changed};
use core_command_domain::orchestration::SourceBaseline;
use core_infrastructure::hash::{sha256_hex, sha256_read};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Read as _},
    path::{Path, PathBuf},
};
const HARD: &[&str] = &[
    ".cache",
    ".git",
    ".gradle",
    ".mypy_cache",
    ".next",
    ".nuxt",
    ".pytest_cache",
    ".ruff_cache",
    ".tox",
    ".venv",
    "node_modules",
    "venv",
];
const CONDITIONAL: &[&str] = &["build", "coverage", "dist", "logs", "target", "tmp"];

pub(crate) fn read(root: &Path) -> io::Result<SourceBaseline> {
    let listing = collect(root)
        .ok()
        .or_else(|| (!root.join(".git").exists()).then(String::new));
    SourceBaseline::new(listing).map_err(io::Error::other)
}
fn collect(root: &Path) -> io::Result<String> {
    let root = fs::canonicalize(root)?;
    if root.join(".aidlc/worktree-meta.json").exists() {
        return Err(io::Error::other(
            "worktree source exclusions are not connected",
        ));
    }
    let mut scan = Scan {
        root: root.clone(),
        listing: BTreeMap::new(),
        registered: registry(&root)?,
        active: BTreeSet::new(),
        visited: BTreeSet::new(),
        entries: 0,
        directories: 0,
        files: 0,
        bytes: 0,
        symlinks: 0,
        external_files: 0,
        external_bytes: 0,
    };
    scan.walk(&root, "", "", false, false, true)?;
    let mut entries: Vec<_> = scan.listing.into_iter().collect();
    entries.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
    Ok(entries
        .into_iter()
        .map(|(path, (mode, hash))| format!("\t{}\t{mode}\t{hash}\n", escape(&path)))
        .collect())
}
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
fn budget(name: &str, fallback: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|n| n.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(fallback)
}
fn sensor_cache(path: &str) -> bool {
    let parts: Vec<_> = path.split('/').collect();
    parts.windows(4).enumerate().any(|(index, window)| {
        matches!(window, ["aidlc", "spaces", _, "intents"])
            && parts
                .iter()
                .skip(index + 4)
                .any(|part| *part == ".aidlc-sensors")
    })
}

fn excluded(path: &str, root: &Path) -> bool {
    let mut parts = path.split('/');
    let first = parts.next().unwrap_or_default();
    matches!(
        first,
        "aidlc" | ".aidlc" | ".claude" | ".codex" | ".kiro" | ".cursor" | ".kimi-code"
    ) || is_harness(&root.join(first), first)
        || path.split('/').any(|p| HARD.contains(&p))
        || sensor_cache(path)
}
fn registry(root: &Path) -> io::Result<BTreeSet<String>> {
    let path = root.join(".aidlc-source-paths.json");
    let metadata = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(e) => return Err(e),
    };
    if !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return Err(io::Error::other("invalid source registry"));
    }
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    let paths = value
        .get("paths")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| io::Error::other("invalid source registry"))?;
    if value.get("version").and_then(serde_json::Value::as_u64) != Some(1) || paths.len() > 10_000 {
        return Err(io::Error::other("invalid source registry"));
    }
    let mut registered = BTreeSet::new();
    for raw in paths {
        let raw = raw
            .as_str()
            .ok_or_else(|| io::Error::other("invalid registered path"))?;
        let path = raw.replace('\\', "/");
        let path = path
            .strip_prefix("./")
            .unwrap_or(&path)
            .trim_end_matches('/');
        if path.is_empty()
            || path.len() > 4096
            || path.contains('\0')
            || path
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == "..")
            || path.as_bytes().get(1) == Some(&b':')
            || excluded(path, root)
        {
            return Err(io::Error::other("invalid registered path"));
        }
        registered.insert(path.to_string());
        if let Ok(target) = fs::canonicalize(root.join(path))
            && let Ok(relative) = target.strip_prefix(root)
        {
            let relative = relative
                .to_str()
                .ok_or_else(|| io::Error::other("invalid source path"))?;
            if excluded(relative, root) {
                return Err(io::Error::other("registered source is excluded"));
            }
            if !relative.is_empty() {
                registered.insert(relative.to_string());
            }
        }
    }
    Ok(registered)
}
struct Scan {
    root: PathBuf,
    listing: BTreeMap<String, (String, String)>,
    registered: BTreeSet<String>,
    active: BTreeSet<PathBuf>,
    visited: BTreeSet<(PathBuf, String)>,
    entries: usize,
    directories: usize,
    files: usize,
    bytes: u64,
    symlinks: usize,
    external_files: usize,
    external_bytes: u64,
}
impl Scan {
    fn includes(&self, path: &str) -> bool {
        self.registered
            .iter()
            .any(|r| path == r || path.starts_with(&format!("{r}/")))
    }
    fn relevant(&self, path: &str) -> bool {
        self.includes(path)
            || self
                .registered
                .iter()
                .any(|r| r.starts_with(&format!("{path}/")))
    }
    fn walk(
        &mut self,
        dir: &Path,
        logical: &str,
        registry_path: &str,
        source_only: bool,
        registered_only: bool,
        snapshot: bool,
    ) -> io::Result<()> {
        let real = fs::canonicalize(dir)?;
        let mode = if registered_only {
            format!("registered:{registry_path}")
        } else {
            source_only.to_string()
        };
        if self.active.contains(&real) || !self.visited.insert((real.clone(), mode)) {
            return Ok(());
        }
        self.active.insert(real.clone());
        self.directories += 1;
        if self.directories > budget("AIDLC_TEST_SOURCE_MAX_DIRECTORIES", 100_000) {
            return Err(io::Error::other("source directory budget exceeded"));
        }
        let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
        self.entries += entries.len();
        if self.entries > budget("AIDLC_TEST_SOURCE_MAX_ENTRIES", 250_000) {
            return Err(io::Error::other("source entry budget exceeded"));
        }
        entries.sort_by(|a, b| {
            a.file_name()
                .to_string_lossy()
                .encode_utf16()
                .cmp(b.file_name().to_string_lossy().encode_utf16())
        });
        for entry in entries {
            let name = entry.file_name();
            let name = name
                .to_str()
                .ok_or_else(|| io::Error::other("source path is not UTF-8"))?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            let child = joined(logical, name);
            let registration = joined(registry_path, name);
            let container = metadata.is_dir() || metadata.is_symlink();
            if metadata.is_symlink() {
                self.symlinks += 1;
                if self.symlinks > budget("AIDLC_TEST_SOURCE_MAX_SYMLINKS", 100_000) {
                    return Err(io::Error::other("source symlink budget exceeded"));
                }
            }
            if registered_only && !self.relevant(&registration)
                || name == ".git"
                || container && HARD.contains(&name)
                || container
                    && logical.is_empty()
                    && (matches!(name, "aidlc" | ".aidlc") || is_harness(&path, name))
                || container && sensor_cache(&child)
            {
                continue;
            }
            let conditional = container && CONDITIONAL.contains(&name);
            if conditional && !self.relevant(&registration) {
                continue;
            }
            let registered = registered_only || conditional;
            if metadata.is_symlink() {
                let link = fs::read_link(&path)?;
                let text = link
                    .to_str()
                    .ok_or_else(|| io::Error::other("link is not UTF-8"))?;
                self.listing.insert(
                    child.clone(),
                    ("120000".into(), sha256_hex(text.as_bytes())),
                );
                let target = match fs::canonicalize(&path) {
                    Ok(p) => p,
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {
                        self.listing.insert(
                            format!("{child}@target"),
                            ("000000".into(), sha256_hex(b"missing")),
                        );
                        continue;
                    }
                    Err(e) => return Err(e),
                };
                let internal = target.strip_prefix(&self.root).ok();
                let target_logical = internal
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| format!("{child}@target"));
                let meta = fs::metadata(&target)?;
                if meta.is_dir() {
                    self.walk(
                        &target,
                        &target_logical,
                        &registration,
                        source_only || internal.is_none(),
                        registered,
                        snapshot && internal.is_some() && !target.join(".git").exists(),
                    )?;
                } else if meta.is_file() {
                    if !registered || self.includes(&registration) {
                        self.file(&target, &target_logical, &meta, source_only, registered)?;
                    }
                } else {
                    self.special(&format!("{child}@target"), &meta);
                }
            } else if metadata.is_dir() {
                let nested = !conditional && path.join(".git").exists();
                if nested && snapshot {
                    let output = std::process::Command::new("git")
                        .arg("-C")
                        .arg(&path)
                        .args(["rev-parse", "--verify", "HEAD^{commit}"])
                        .output()?;
                    let oid = String::from_utf8(output.stdout)
                        .map_err(io::Error::other)?
                        .trim()
                        .to_string();
                    if !output.status.success()
                        || !matches!(oid.len(), 40 | 64)
                        || !oid.bytes().all(|c| c.is_ascii_hexdigit())
                    {
                        return Err(io::Error::other("embedded repository has no valid commit"));
                    }
                    self.listing.insert(child.clone(), ("160000".into(), oid));
                }
                self.walk(
                    &path,
                    &child,
                    &registration,
                    source_only && !conditional,
                    registered,
                    snapshot && !nested,
                )?;
            } else if metadata.is_file() {
                if !registered || self.includes(&registration) {
                    self.file(&path, &child, &metadata, source_only, registered)?;
                }
            } else {
                self.special(&child, &metadata);
            }
        }
        self.active.remove(&real);
        Ok(())
    }
    fn file(
        &mut self,
        path: &Path,
        logical: &str,
        metadata: &fs::Metadata,
        source_only: bool,
        registered: bool,
    ) -> io::Result<()> {
        if source_only && !registered && !source_like(path)? {
            return Ok(());
        }
        self.files += 1;
        self.bytes += metadata.len();
        if source_only {
            self.external_files += 1;
            self.external_bytes += metadata.len();
        }
        if self.files > 250_000
            || self.bytes > 4 * 1024 * 1024 * 1024
            || self.external_files > 10_000
            || self.external_bytes > 64 * 1024 * 1024
        {
            return Err(io::Error::other("source file budget exceeded"));
        }
        let mut file = fs::File::open(path)?;
        let before = file.metadata()?;
        let hash = sha256_read(&mut file)?;
        let after = file.metadata()?;
        if before.len() != after.len()
            || before.modified()? != after.modified()?
            || metadata_changed(&before, &after)
        {
            return Err(io::Error::other("source changed while reading"));
        }
        self.listing.insert(
            logical.to_string(),
            (
                if executable(&before) {
                    "100755"
                } else {
                    "100644"
                }
                .into(),
                hash,
            ),
        );
        Ok(())
    }
    fn special(&mut self, path: &str, metadata: &fs::Metadata) {
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::MetadataExt as _;
            metadata.mode()
        };
        #[cfg(not(unix))]
        let mode = {
            let _ = metadata;
            0
        };
        self.listing.insert(
            path.to_string(),
            (
                "000000".into(),
                sha256_hex(format!("special:{mode}").as_bytes()),
            ),
        );
    }
}
fn joined(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}
fn source_like(path: &Path) -> io::Result<bool> {
    const EXT: &[&str] = &[
        "astro", "c", "cc", "cmake", "cpp", "cs", "css", "dart", "erl", "ex", "exs", "go", "gql",
        "graphql", "h", "hcl", "hpp", "hrl", "html", "java", "js", "jsx", "json", "kt", "kts",
        "lua", "m", "mm", "php", "proto", "ps1", "psd1", "psm1", "py", "r", "rb", "rs", "scala",
        "sh", "sol", "sql", "svelte", "swift", "tf", "tfvars", "toml", "ts", "tsx", "vue", "xml",
        "yaml", "yml", "zig",
    ];
    if path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| EXT.contains(&s.to_ascii_lowercase().as_str()))
    {
        return Ok(true);
    }
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if [
        "build",
        "cmakelists.txt",
        "dockerfile",
        "gemfile",
        "justfile",
        "makefile",
        "procfile",
        "tiltfile",
        "workspace",
    ]
    .contains(&name.as_str())
        || name.starts_with("dockerfile.")
    {
        return Ok(true);
    }
    let mut sample = [0; 8192];
    let count = fs::File::open(path)?.read(&mut sample)?;
    Ok(!sample.get(..count).unwrap_or_default().contains(&0))
}
/// 報告境界で配布定義とソースを観測する。次工程の選択は実行集約が行う。
pub(crate) fn for_report(
    layout: &crate::layout::Layout,
) -> Result<
    (
        Option<SourceBaseline>,
        core_command_domain::orchestration::StageSlugSet,
    ),
    String,
> {
    use core_command_domain::{orchestration::StageSlugSet, workflow_definition::StageSlug};
    let graph: Vec<serde_json::Value> = serde_json::from_slice(
        &fs::read(layout.definition_data_dir().join("stage-graph.json"))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let stages = graph
        .iter()
        .filter(|stage| {
            stage
                .get("workspace_requires")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
        })
        .map(|stage| {
            let slug = stage
                .get("slug")
                .and_then(serde_json::Value::as_str)
                .ok_or("workspace stage has no slug")?;
            StageSlug::parse(slug).map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, String>>()?;
    if stages.is_empty() {
        return Ok((None, StageSlugSet::empty()));
    }
    Ok((
        Some(read(layout.project_dir()).map_err(|error| error.to_string())?),
        StageSlugSet::new(stages),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captured_listings_match_fixed_upstream_bytes() {
        let corpus: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/golden/selfhost-stage1/source-baseline.json"
        ))
        .unwrap();
        for case in corpus.get("observations").unwrap().as_array().unwrap() {
            let parent = tempfile::tempdir().unwrap();
            let root = parent.path().join("workspace");
            fs::create_dir(&root).unwrap();
            for (base, files) in [
                (
                    &root,
                    &case.get("files").unwrap_or(&serde_json::Value::Null),
                ),
                (
                    &parent.path().join("outside"),
                    &case.get("outside").unwrap_or(&serde_json::Value::Null),
                ),
            ] {
                if let Some(files) = files.as_object() {
                    for (path, body) in files {
                        let path = base.join(path);
                        fs::create_dir_all(path.parent().unwrap()).unwrap();
                        fs::write(path, body.as_str().unwrap()).unwrap();
                    }
                }
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::{PermissionsExt, symlink};
                if let Some(paths) = case
                    .get("executable")
                    .unwrap_or(&serde_json::Value::Null)
                    .as_array()
                {
                    for path in paths {
                        fs::set_permissions(
                            root.join(path.as_str().unwrap()),
                            fs::Permissions::from_mode(0o755),
                        )
                        .unwrap();
                    }
                }
                if let Some(links) = case
                    .get("symlinks")
                    .unwrap_or(&serde_json::Value::Null)
                    .as_object()
                {
                    for (path, target) in links {
                        let path = root.join(path);
                        fs::create_dir_all(path.parent().unwrap()).unwrap();
                        symlink(target.as_str().unwrap(), path).unwrap();
                    }
                }
            }
            if let Some(repositories) = case
                .get("git_repositories")
                .unwrap_or(&serde_json::Value::Null)
                .as_array()
            {
                for repository in repositories {
                    for args in [
                        vec!["init", "--initial-branch=main"],
                        vec!["add", "."],
                        vec!["-c", "commit.gpgsign=false", "commit", "-m", "baseline"],
                    ] {
                        let output = std::process::Command::new("git")
                            .arg("-C")
                            .arg(root.join(repository.as_str().unwrap()))
                            .args(args)
                            .env_clear()
                            .env("PATH", "/usr/bin:/bin")
                            .env("HOME", parent.path())
                            .env("GIT_CONFIG_NOSYSTEM", "1")
                            .env("GIT_CONFIG_GLOBAL", "/dev/null")
                            .env("GIT_AUTHOR_NAME", "Baseline")
                            .env("GIT_AUTHOR_EMAIL", "baseline@example.invalid")
                            .env("GIT_COMMITTER_NAME", "Baseline")
                            .env("GIT_COMMITTER_EMAIL", "baseline@example.invalid")
                            .env("GIT_AUTHOR_DATE", "2026-09-09T00:00:00Z")
                            .env("GIT_COMMITTER_DATE", "2026-09-09T00:00:00Z")
                            .output()
                            .unwrap();
                        assert!(output.status.success(), "{output:?}");
                    }
                }
            }
            let expected = case
                .get("listing")
                .unwrap_or(&serde_json::Value::Null)
                .as_str()
                .unwrap_or("");
            let actual = read(&root).unwrap();
            assert_eq!(
                actual.listing(),
                Some(expected),
                "{}",
                case.get("id").unwrap_or(&serde_json::Value::Null)
            );
        }
    }

    fn workspace() -> (tempfile::TempDir, PathBuf) {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("workspace");
        fs::create_dir(&root).unwrap();
        (parent, root)
    }

    /// worktree のメタが在る根は「切り出し除外が未接続」として採取を断る。`.git` が在れば
    /// `read` は基準を `None`（採取不能）にし、無ければ空の一覧として扱う。
    #[test]
    fn a_worktree_root_is_refused_and_read_marks_the_baseline_unavailable() {
        let (_parent, root) = workspace();
        fs::create_dir_all(root.join(".aidlc")).unwrap();
        fs::write(root.join(".aidlc/worktree-meta.json"), "{}\n").unwrap();
        let error = collect(&root).unwrap_err();
        assert_eq!(
            error.to_string(),
            "worktree source exclusions are not connected"
        );
        assert_eq!(read(&root).unwrap().listing(), Some(""));
        fs::create_dir_all(root.join(".git")).unwrap();
        assert_eq!(read(&root).unwrap().listing(), None);
    }

    /// 壊れた登録簿は黙って無視しない — 形・版・除外先のどれが壊れていても採取を断る。
    #[cfg(unix)]
    #[test]
    fn a_broken_source_registry_is_refused_one_reason_at_a_time() {
        let (_parent, root) = workspace();
        let registry_path = root.join(".aidlc-source-paths.json");
        fs::create_dir(&registry_path).unwrap();
        assert_eq!(
            collect(&root).unwrap_err().to_string(),
            "invalid source registry",
            "ディレクトリは登録簿ではない"
        );
        fs::remove_dir(&registry_path).unwrap();
        fs::write(&registry_path, r#"{"version":2,"paths":[]}"#).unwrap();
        assert_eq!(
            collect(&root).unwrap_err().to_string(),
            "invalid source registry",
            "版 1 以外は読まない"
        );
        fs::write(&registry_path, r#"{"version":1,"paths":["../outside"]}"#).unwrap();
        assert_eq!(
            collect(&root).unwrap_err().to_string(),
            "invalid registered path"
        );
        // 登録先の実体が除外ディレクトリを指す（シンボリックリンク経由）。
        fs::create_dir_all(root.join("node_modules/dep")).unwrap();
        std::os::unix::fs::symlink(root.join("node_modules/dep"), root.join("vendored")).unwrap();
        fs::write(&registry_path, r#"{"version":1,"paths":["./vendored/"]}"#).unwrap();
        assert_eq!(
            collect(&root).unwrap_err().to_string(),
            "registered source is excluded"
        );
    }

    /// 通常ファイルでないもの（FIFO）は読まずに `000000` とモードの要約で載せる。
    /// シンボリックリンクは先の種類（ファイル・ディレクトリ・特殊）ごとに扱いが分かれる。
    #[cfg(unix)]
    #[test]
    fn special_files_and_symlink_targets_are_listed_without_being_read() {
        let (_parent, root) = workspace();
        fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();
        fs::create_dir(root.join("lib")).unwrap();
        fs::write(root.join("lib/util.rs"), "pub fn util() {}\n").unwrap();
        let status = std::process::Command::new("mkfifo")
            .arg(root.join("pipe"))
            .status()
            .unwrap();
        assert!(status.success());
        std::os::unix::fs::symlink(root.join("main.rs"), root.join("main-link.rs")).unwrap();
        std::os::unix::fs::symlink(root.join("lib"), root.join("lib-link")).unwrap();
        std::os::unix::fs::symlink(root.join("pipe"), root.join("pipe-link")).unwrap();
        let listing = collect(&root).unwrap();
        let rows: Vec<&str> = listing.lines().collect();
        let mode_of = |name: &str| {
            rows.iter()
                .find(|row| row.split('\t').nth(1) == Some(name))
                .map(|row| row.split('\t').nth(2).unwrap().to_string())
        };
        assert_eq!(mode_of("pipe").as_deref(), Some("000000"));
        assert_eq!(mode_of("pipe-link").as_deref(), Some("120000"));
        assert_eq!(mode_of("pipe-link@target").as_deref(), Some("000000"));
        assert_eq!(mode_of("main-link.rs").as_deref(), Some("120000"));
        assert_eq!(mode_of("lib-link").as_deref(), Some("120000"));
        assert_eq!(mode_of("main.rs").as_deref(), Some("100644"));
        assert_eq!(mode_of("lib/util.rs").as_deref(), Some("100644"));
        assert!(
            listing.matches("\tlib/util.rs\t").count() == 1,
            "リンク先のディレクトリは実体として 1 度だけ載る: {listing}"
        );
    }

    /// 既知のビルド定義ファイル名は拡張子が無くてもソースとみなす。
    #[test]
    fn well_known_build_files_count_as_source_without_an_extension() {
        let (_parent, root) = workspace();
        for name in ["Makefile", "Dockerfile.dev", "Gemfile"] {
            fs::write(root.join(name), "all:\n").unwrap();
            assert!(source_like(&root.join(name)).unwrap(), "{name}");
        }
        fs::write(root.join("notes"), "plain text\n").unwrap();
        assert!(source_like(&root.join("notes")).unwrap());
        fs::write(root.join("blob"), [0u8, 1, 2]).unwrap();
        assert!(!source_like(&root.join("blob")).unwrap());
    }

    /// 互いを指すシンボリックリンクの輪は「無い先」ではないので、黙って飛ばさず採取を止める。
    /// 先の無いリンクは `@target` を `missing` として載せる。
    #[cfg(unix)]
    #[test]
    fn a_symlink_loop_stops_the_scan_while_a_dangling_link_is_listed_as_missing() {
        let (_parent, root) = workspace();
        std::os::unix::fs::symlink("nowhere", root.join("dangling")).unwrap();
        let listing = collect(&root).unwrap();
        assert!(listing.contains("\tdangling@target\t000000\t"), "{listing}");
        std::os::unix::fs::symlink("b", root.join("a")).unwrap();
        std::os::unix::fs::symlink("a", root.join("b")).unwrap();
        let error = collect(&root).unwrap_err();
        assert_ne!(error.kind(), io::ErrorKind::NotFound);
    }
}
