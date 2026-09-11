//! 作業開始時のワークスペース観測。本家2.7.1のscanSignalsの入力境界。
use core_command_domain::orchestration::WorkspaceScan;
use core_command_domain::workflow_definition::BrownfieldGreenfield;
use core_command_domain::workspace::UnsafeLineChar;
use std::path::{Path, PathBuf};

/// ファイルを観測して作業開始の材料を作る。コマンド側の永続化処理は持たない。
#[derive(Debug)]
pub(crate) struct WorkspaceScanner {
    root: PathBuf,
}
impl WorkspaceScanner {
    pub(crate) const fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub(crate) fn scan(&self) -> Result<WorkspaceScan, UnsafeLineChar> {
        let mut languages = Vec::new();
        count_languages(&self.root, 0, &mut languages);
        for source in SOURCES {
            count_languages(&self.root.join(source), 6, &mut languages);
        }
        let cargo = self.root.join("Cargo.toml").exists();
        let source = SOURCES.iter().any(|source| self.root.join(source).exists());
        languages.sort_by_key(|entry| std::cmp::Reverse(entry.1));
        let listed = languages
            .first()
            .map(|(_, primary)| {
                let threshold = (*primary / 5).max(1);
                languages
                    .iter()
                    .filter(|(_, count)| *count >= threshold)
                    .map(|(name, _)| *name)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| "Unknown".into());
        WorkspaceScan::new(
            if cargo || source || !languages.is_empty() {
                BrownfieldGreenfield::Brownfield
            } else {
                BrownfieldGreenfield::Greenfield
            },
            &listed,
            "Unknown",
            if cargo {
                "cargo (Cargo.toml)"
            } else {
                "Unknown"
            },
        )
    }
}
const SOURCES: &[&str] = &["src", "app", "lib", "pages", "components", "tests"];
const EXCLUDE: &[&str] = &[
    ".claude",
    ".kiro",
    ".codex",
    ".opencode",
    ".aidlc",
    ".cursor",
    "aidlc-docs",
    "node_modules",
    ".git",
    "dist",
    "build",
    ".next",
    "target",
    "vendor",
];
fn count_languages(path: &Path, depth: usize, counts: &mut Vec<(&'static str, usize)>) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        if entry
            .file_name()
            .to_str()
            .is_some_and(|name| EXCLUDE.contains(&name))
        {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() && depth > 0 {
            count_languages(&entry.path(), depth - 1, counts);
        } else if kind.is_file() {
            let path = entry.path();
            let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
                continue;
            };
            let Some(language) = language(&ext.to_ascii_lowercase()) else {
                continue;
            };
            if let Some((_, count)) = counts.iter_mut().find(|(name, _)| *name == language) {
                *count += 1;
            } else {
                counts.push((language, 1));
            }
        }
    }
}
fn language(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "ts" | "tsx" => "TypeScript",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "py" => "Python",
        "java" => "Java",
        "kt" => "Kotlin",
        "go" => "Go",
        "rs" => "Rust",
        "rb" => "Ruby",
        "cs" => "C#",
        "cpp" | "hpp" => "C++",
        "c" | "h" => "C",
        "swift" => "Swift",
        "php" => "PHP",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    /// 言語は同じ拡張子を数え上げ、主言語の 1/5 未満は落とす。シンボリックリンクは辿らず、
    /// ソースディレクトリだけ深さ 6 まで潜る。
    #[cfg(unix)]
    #[test]
    fn languages_are_counted_through_source_directories_but_not_symlinks() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let src = root.path().join("src/nested/deeper");
        std::fs::create_dir_all(&src).expect("src");
        for name in ["a.rs", "b.rs", "c.rs", "d.rs", "e.rs"] {
            std::fs::write(src.join(name), "fn f() {}\n").expect("ソース");
        }
        std::fs::write(root.path().join("src/index.ts"), "export {}\n").expect("ts");
        std::fs::write(root.path().join("src/README"), "no extension\n").expect("その他");
        std::os::unix::fs::symlink(root.path().join("src"), root.path().join("src/loop"))
            .expect("リンク");
        let scan = WorkspaceScanner::new(root.path().to_path_buf())
            .scan()
            .expect("単一行");
        assert_eq!(scan.languages(), "Rust, TypeScript");
        assert_eq!(scan.build_system(), "Unknown");
        assert_eq!(scan.project_kind(), BrownfieldGreenfield::Brownfield);
    }

    /// 何も無い根は Greenfield で言語 Unknown、`Cargo.toml` だけでも Brownfield である。
    #[test]
    fn an_empty_root_is_greenfield_and_a_cargo_manifest_makes_it_brownfield() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let scan = WorkspaceScanner::new(root.path().to_path_buf())
            .scan()
            .expect("単一行");
        assert_eq!(scan.project_kind(), BrownfieldGreenfield::Greenfield);
        assert_eq!(scan.languages(), "Unknown");
        std::fs::write(root.path().join("Cargo.toml"), "[package]\n").expect("manifest");
        let scan = WorkspaceScanner::new(root.path().to_path_buf())
            .scan()
            .expect("単一行");
        assert_eq!(scan.project_kind(), BrownfieldGreenfield::Brownfield);
        assert_eq!(scan.build_system(), "cargo (Cargo.toml)");
    }
}
