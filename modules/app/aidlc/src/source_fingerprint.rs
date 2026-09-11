//! 計画承認の前後で比較するソース指紋の入力境界。
use core_infrastructure::hash::{sha256_hex, sha256_read};
use std::{fs, io, path::Path};
const HARD_EXCLUDED: &[&str] = &[
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
pub(crate) fn read(root: &Path) -> io::Result<String> {
    let root = fs::canonicalize(root)?;
    let mut scan = Scan::default();
    scan.walk(&root, "", true)?;
    scan.lines
        .insert(0, "aidlc-filesystem-source-v2".to_string());
    let filesystem = sha256_hex(scan.lines.join("\n").as_bytes());
    Ok(sha256_hex(
        format!("aidlc-workspace-source-v2\nfilesystem={filesystem}").as_bytes(),
    ))
}
#[derive(Default)]
struct Scan {
    lines: Vec<String>,
    entries: usize,
    directories: usize,
    files: usize,
    bytes: u64,
}
impl Scan {
    fn walk(&mut self, dir: &Path, relative: &str, root: bool) -> io::Result<()> {
        self.directories += 1;
        if self.directories > 100_000 {
            return Err(io::Error::other("source directory budget exceeded"));
        }
        let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
        self.entries += entries.len();
        if self.entries > 250_000 {
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
            let name = name.to_str().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "source path is not UTF-8")
            })?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            if name == ".git" {
                continue;
            }
            if (metadata.is_dir() || metadata.file_type().is_symlink())
                && (HARD_EXCLUDED.contains(&name)
                    || CONDITIONAL.contains(&name)
                    || root && (matches!(name, "aidlc" | ".aidlc") || is_harness(&path, name)))
            {
                continue;
            }
            let logical = if relative.is_empty() {
                name.to_string()
            } else {
                format!("{relative}/{name}")
            };
            if metadata.is_dir() {
                self.walk(&path, &logical, false)?;
            } else if metadata.is_file() {
                self.files += 1;
                self.bytes += metadata.len();
                if self.files > 250_000 || self.bytes > 4 * 1024 * 1024 * 1024 {
                    return Err(io::Error::other("source file budget exceeded"));
                }
                let mut file = fs::File::open(&path)?;
                let before = file.metadata()?;
                let digest = sha256_read(&mut file)?;
                let after = file.metadata()?;
                if before.len() != after.len()
                    || before.modified()? != after.modified()?
                    || metadata_changed(&before, &after)
                {
                    return Err(io::Error::other("source changed while reading"));
                }
                self.lines.push(format!(
                    "file:{logical}:{}={digest}",
                    if executable(&before) { "x" } else { "-" }
                ));
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "source file kind is not connected",
                ));
            }
        }
        Ok(())
    }
}
pub(super) fn is_harness(path: &Path, name: &str) -> bool {
    let Some(name) = name.strip_prefix('.') else {
        return false;
    };
    if name.is_empty()
        || !name.starts_with(|character: char| character.is_ascii_alphanumeric())
        || !name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
    {
        return false;
    }
    let manifest = path.join("tools/data/harness.json");
    let Ok(metadata) = fs::symlink_metadata(&manifest) else {
        return false;
    };
    if !metadata.is_file() || metadata.len() > 64 * 1024 {
        return false;
    }
    fs::read_to_string(manifest)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .is_some_and(|value| {
            value
                .get("name")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|name| !name.trim().is_empty())
        })
}
#[cfg(unix)]
pub(super) fn executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    metadata.permissions().mode() & 0o111 != 0
}
#[cfg(not(unix))]
pub(super) fn executable(_metadata: &fs::Metadata) -> bool {
    false
}
#[cfg(unix)]
pub(super) fn metadata_changed(before: &fs::Metadata, after: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    before.ctime() != after.ctime() || before.ctime_nsec() != after.ctime_nsec()
}
#[cfg(not(unix))]
pub(super) fn metadata_changed(_before: &fs::Metadata, _after: &fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::read;
    #[test]
    fn source_fingerprint_matches_the_fixed_upstream_observation() {
        let corpus: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/golden/selfhost-stage1/source-fingerprint.json"
        ))
        .unwrap();
        let observations = corpus.get("observations").unwrap().as_array().unwrap();
        assert!(!observations.is_empty());
        for case in observations {
            let root = tempfile::tempdir().unwrap();
            for (path, body) in case.get("files").unwrap().as_object().unwrap() {
                let path = root.path().join(path);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, body.as_str().unwrap()).unwrap();
            }
            let actual = read(root.path());
            assert!(actual.is_ok(), "{}: {actual:?}", case.get("id").unwrap());
            assert_eq!(
                Some(actual.unwrap().as_str()),
                case.get("fingerprint").unwrap().as_str()
            );
        }
    }

    /// `.git` は指紋に入らず、FIFO のような読めない種類は黙って飛ばさず `Unsupported` で止める。
    #[cfg(unix)]
    #[test]
    fn a_special_file_stops_the_fingerprint_instead_of_being_skipped() {
        use std::fs;
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join(".git/objects")).unwrap();
        fs::write(root.path().join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(root.path().join("main.rs"), "fn main() {}\n").unwrap();
        let clean = read(root.path()).unwrap();
        fs::write(root.path().join(".git/HEAD"), "ref: refs/heads/other\n").unwrap();
        assert_eq!(
            read(root.path()).unwrap(),
            clean,
            ".git の中身は指紋に入らない"
        );
        let status = std::process::Command::new("mkfifo")
            .arg(root.path().join("pipe"))
            .status()
            .unwrap();
        assert!(status.success());
        let error = read(root.path()).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::Unsupported);
        assert_eq!(error.to_string(), "source file kind is not connected");
    }

    /// ハーネスディレクトリの判定 — 名前の形と `tools/data/harness.json` の中身の両方を見る。
    #[test]
    fn a_harness_directory_needs_a_dotted_name_and_a_named_manifest() {
        use std::fs;
        let root = tempfile::tempdir().unwrap();
        let harness = root.path().join(".claude");
        fs::create_dir_all(harness.join("tools/data")).unwrap();
        assert!(!super::is_harness(&harness, "claude"), "先頭の点が要る");
        assert!(!super::is_harness(&harness, ".-claude"), "点の直後は英数字");
        assert!(!super::is_harness(&harness, ".cla ude"), "空白は不可");
        assert!(!super::is_harness(&harness, ".claude"), "manifest が無い");
        fs::create_dir(harness.join("tools/data/harness.json")).unwrap();
        assert!(
            !super::is_harness(&harness, ".claude"),
            "manifest がディレクトリ"
        );
        fs::remove_dir(harness.join("tools/data/harness.json")).unwrap();
        fs::write(harness.join("tools/data/harness.json"), r#"{"name":"  "}"#).unwrap();
        assert!(
            !super::is_harness(&harness, ".claude"),
            "空の name は名乗りではない"
        );
        fs::write(
            harness.join("tools/data/harness.json"),
            r#"{"name":"claude"}"#,
        )
        .unwrap();
        assert!(super::is_harness(&harness, ".claude"));
    }
}
