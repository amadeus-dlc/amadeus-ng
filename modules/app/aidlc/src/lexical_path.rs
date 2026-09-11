//! 入力境界で使う、ファイルを開かない字句的なパス解決。
use std::path::{Component, Path, PathBuf};
pub(crate) fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `.` は落とし、`..` は直前の成分を戻す — ファイルを開かずに綴りだけで解決する。
    #[test]
    fn dot_segments_are_resolved_lexically_without_touching_the_filesystem() {
        assert_eq!(
            normalize(Path::new("/w/./a/../b/./c")),
            PathBuf::from("/w/b/c")
        );
        assert_eq!(normalize(Path::new("../x")), PathBuf::from("x"));
        assert_eq!(normalize(Path::new("./")), PathBuf::new());
    }
}
