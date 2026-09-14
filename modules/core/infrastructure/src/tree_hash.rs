//! ディレクトリ木の決定的な内容ハッシュ — 指定したパスだけを名前順に辿って畳む。
//!
//! ドメインを持たない**言語拡張**である (`coding-rules/infrastructure-layer.md`) — 何のための
//! 世代か・どのパスを除くかは呼び手が決め、ここは「与えられた木を同じ順序で同じように畳む」
//! ことだけを約束する。
//!
//! upstream `treeGeneration` (`aidlc-lib.ts:1980`) の写しであり、畳む順序と区切りバイトが
//! 観測可能な契約である。種類ごとに前置きを変える (`S` 開始 / `D` ディレクトリ / `F` ファイル /
//! `L` シンボリックリンク) ので、名前だけが同じで種類が違う木は別の値になる。
//!
//! # 畳めなければ `None`
//!
//! 在らないパス・根の外へ出る指定・glob・特殊ファイルはすべて `None` に畳む。**空木の値を
//! 返さない**のが要点で、「範囲が空」を「範囲が一致」と読み違えさせないためである。

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

/// 常に畳まないディレクトリ (upstream 逐語 — git のメタデータは内容ではない)。
const ALWAYS_EXCLUDED: &str = ".git";

/// 指定したパスに限って木を畳み、内容ハッシュを 16 進小文字で返す。
///
/// `paths` / `excluded_paths` はリポジトリ相対の綴りである。`excluded_paths` に載せた綴りは、
/// それ自身とその配下を畳まない。畳めない指定 (空・在らない・根の外・絶対パス・glob 文字・
/// 通常ファイルでもディレクトリでもリンクでもないもの) は `None` になる。
#[must_use]
pub fn hash_tree(root: &Path, paths: &[String], excluded_paths: &[String]) -> Option<String> {
    let mut normalized: Vec<String> = Vec::new();
    for path in paths {
        let portable = normalize(path)?;
        if !normalized.contains(&portable) {
            normalized.push(portable);
        }
    }
    if normalized.is_empty() {
        return None;
    }
    // 除外は「畳めない綴り」を落とすだけで失敗にしない — 除外の指定ミスで世代が計算
    // 不能になると、CAS が理由の分からない拒否に化けるためである。
    let mut excludes: Vec<String> = vec![ALWAYS_EXCLUDED.to_string()];
    excludes.extend(excluded_paths.iter().filter_map(|path| normalize(path)));

    let mut fold = Fold {
        root,
        hasher: Sha256::new(),
        seen: BTreeSet::new(),
        excludes,
    };
    for portable in &normalized {
        fold.hasher.update(format!("S\0{portable}\0").as_bytes());
        let absolute = fold.absolute(portable);
        if !fold.visit(&absolute, portable) {
            return None;
        }
    }
    Some(
        fold.hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}

/// 突き合わせ可能な綴りへ正規化する (upstream `normalizeGenerationPath`)。
///
/// 根そのものは `.` で表す。絶対パス・ドライブレター・glob 文字・`..` を含む綴りは、畳む前に
/// ここで落とす — 根の外を指す経路を作らせないためである。
fn normalize(path: &str) -> Option<String> {
    let portable = path.trim().replace('\\', "/");
    if portable.is_empty()
        || portable.starts_with('/')
        || is_drive_rooted(&portable)
        || portable.contains(['*', '?', '[', ']'])
    {
        return None;
    }
    let segments: Vec<&str> = portable
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect();
    if segments.contains(&"..") {
        return None;
    }
    if segments.is_empty() {
        return Some(".".to_string());
    }
    Some(segments.join("/"))
}

/// `C:/…` のようなドライブ起点の絶対パスか。
fn is_drive_rooted(portable: &str) -> bool {
    let mut chars = portable.chars();
    let Some(letter) = chars.next() else {
        return false;
    };
    letter.is_ascii_alphabetic() && chars.next() == Some(':') && chars.next() == Some('/')
}

/// 1 回の畳み込みが持ち回る状態。
struct Fold<'a> {
    root: &'a Path,
    hasher: Sha256,
    seen: BTreeSet<String>,
    excludes: Vec<String>,
}

impl Fold<'_> {
    /// 正規化済みの綴りを根の下の実パスへ戻す。
    fn absolute(&self, portable: &str) -> PathBuf {
        if portable == "." {
            return self.root.to_path_buf();
        }
        portable
            .split('/')
            .fold(self.root.to_path_buf(), |path, segment| path.join(segment))
    }

    /// その綴りが除外に掛かるか (それ自身、またはその配下)。
    fn is_excluded(&self, portable: &str) -> bool {
        self.excludes
            .iter()
            .any(|exclude| portable == exclude || portable.starts_with(&format!("{exclude}/")))
    }

    /// 1 つの節点を畳む。畳めなければ `false`。
    fn visit(&mut self, absolute: &Path, portable: &str) -> bool {
        if portable != "." && self.is_excluded(portable) {
            return true;
        }
        if !self.seen.insert(portable.to_string()) {
            return true;
        }
        let Ok(metadata) = fs::symlink_metadata(absolute) else {
            return false;
        };
        if metadata.is_symlink() {
            let Ok(target) = fs::read_link(absolute) else {
                return false;
            };
            self.hasher
                .update(format!("L\0{portable}\0{}\0", target.to_string_lossy()).as_bytes());
            return true;
        }
        if metadata.is_dir() {
            self.hasher.update(format!("D\0{portable}\0").as_bytes());
            return self.visit_children(absolute, portable);
        }
        if metadata.is_file() {
            self.hasher
                .update(format!("F\0{portable}\0{}\0", metadata.len()).as_bytes());
            let Ok(bytes) = fs::read(absolute) else {
                return false;
            };
            self.hasher.update(&bytes);
            self.hasher.update(b"\0");
            return true;
        }
        // 名前付きパイプ・デバイス等は畳めない — 黙って飛ばすと「読めなかった木」が
        // 「その分だけ空の木」として同じ値を持ってしまう。
        false
    }

    /// ディレクトリの子を名前順 (UTF-16 コード単位順) に畳む。
    fn visit_children(&mut self, absolute: &Path, portable: &str) -> bool {
        let Ok(entries) = fs::read_dir(absolute) else {
            return false;
        };
        let Ok(mut entries) = entries.collect::<Result<Vec<_>, _>>() else {
            return false;
        };
        entries.sort_by(|left, right| {
            left.file_name()
                .to_string_lossy()
                .encode_utf16()
                .cmp(right.file_name().to_string_lossy().encode_utf16())
        });
        for entry in entries {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let child = if portable == "." {
                name.to_string()
            } else {
                format!("{portable}/{name}")
            };
            if !self.visit(&entry.path(), &child) {
                return false;
            }
        }
        true
    }
}
