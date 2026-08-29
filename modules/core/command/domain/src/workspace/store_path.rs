//! `StorePath` — SQLite ストアファイルの場所 (BR2.1)。
//!
//! 場所は space 単位で 1 つに決まる。導出の規則をこの型に閉じ込めるのは、パスの組み立てが
//! 呼出側に散ると「どこにストアがあるか」がコード全体の帰納問題になるためである。
//! 生の `PathBuf` を受け取る口は置かない — 場所は導出するものであって渡すものではない。

use std::path::{Path, PathBuf};

use super::space_name::SpaceName;

/// `spaces/` 直下の space ディレクトリ群を束ねるセグメント。
const SPACES_SEGMENT: &str = "spaces";
/// space 配下の記録ディレクトリを束ねるセグメント (upstream の既存ディレクトリ)。
const INTENTS_SEGMENT: &str = "intents";
/// ストアファイル名。先頭のドットで upstream の `.gitignore`
/// (`aidlc/spaces/*/intents/.aidlc-*`) に掛かり、git 管理外になる。
const STORE_FILE: &str = ".aidlc-store.sqlite";

/// SQLite ストアファイルの場所 (`<aidlc root>/spaces/<space>/intents/.aidlc-store.sqlite`)。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StorePath {
    value: PathBuf,
}

impl StorePath {
    /// space のストアファイルの場所を導く (BR2.1 — Q1 = A)。
    ///
    /// `space` は検証済みのパスセグメント (`SpaceName`) なので、`join` に渡してよい唯一の形
    /// である。`aidlc_root` は合成ルートが決める `aidlc/` ディレクトリ。
    #[must_use]
    pub fn for_space(aidlc_root: &Path, space: &SpaceName) -> StorePath {
        StorePath {
            value: aidlc_root
                .join(SPACES_SEGMENT)
                .join(space.as_str())
                .join(INTENTS_SEGMENT)
                .join(STORE_FILE),
        }
    }

    /// ファイルシステムへ渡すパス。
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn space(value: &str) -> SpaceName {
        SpaceName::parse(value).expect("テストの space 名は文法内")
    }

    #[test]
    fn the_path_is_the_store_file_under_the_intents_directory_of_the_space() {
        let path = StorePath::for_space(Path::new("/w/aidlc"), &space("default"));
        assert_eq!(
            path.as_path(),
            Path::new("/w/aidlc/spaces/default/intents/.aidlc-store.sqlite")
        );
    }

    #[test]
    fn a_different_space_gets_a_different_store() {
        let one = StorePath::for_space(Path::new("/w/aidlc"), &space("default"));
        let other = StorePath::for_space(Path::new("/w/aidlc"), &space("team-a"));
        assert_ne!(one, other);
    }

    #[test]
    fn a_relative_root_stays_relative() {
        let path = StorePath::for_space(Path::new("aidlc"), &space("default"));
        assert_eq!(
            path.as_path(),
            Path::new("aidlc/spaces/default/intents/.aidlc-store.sqlite")
        );
    }

    #[test]
    fn the_file_name_is_hidden_so_the_existing_gitignore_rule_catches_it() {
        let path = StorePath::for_space(Path::new("/w/aidlc"), &space("default"));
        let name = path
            .as_path()
            .file_name()
            .and_then(|name| name.to_str())
            .expect("ファイル名は UTF-8");
        assert!(name.starts_with(".aidlc-"), "実際: {name}");
    }
}
