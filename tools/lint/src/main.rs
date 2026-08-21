//! amadeus-lint — `cargo lint` で起動するリポジトリ専用リンター。
//!
//! 目的は Tell, Don't Ask 違反の**再発防止**であって、getter の存在そのものを咎めることでは
//! ない。検出するのは「他オブジェクトから状態を抜き出して所有者の判断を呼出側で代行する」
//! 濫用パターンだけ。ルール本体と検査力テストは [`check`] にある。
//!
//! 使い方 (リポジトリルートから):
//!
//! ```text
//! cargo lint            # modules/ 配下を検査し、所見があれば exit 1
//! cargo lint <root>     # 検査対象のリポジトリルートを明示する
//! ```
//!
//! 抑制は所見の直前行のコメントで行う:
//!
//! ```text
//! // amadeus-lint: allow(checkbox-vocabulary) — 抑制の理由をここに書く
//! ```

#![forbid(unsafe_code)]

mod check;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use check::{Finding, check_source};

/// 走査対象。リポジトリルートからの相対ディレクトリ。
const SCAN_ROOTS: [&str; 1] = ["modules"];
/// 走査から外すディレクトリ名。
const SKIPPED_DIRS: [&str; 1] = ["target"];

fn main() -> ExitCode {
    let root = match resolve_root() {
        Ok(root) => root,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    let mut files = Vec::new();
    for scan_root in SCAN_ROOTS {
        let dir = root.join(scan_root);
        if !dir.is_dir() {
            eprintln!("error: 走査対象 {} が見つからない", dir.display());
            return ExitCode::FAILURE;
        }
        if let Err(error) = collect_rust_files(&dir, &mut files) {
            eprintln!("error: {} の走査に失敗した: {error}", dir.display());
            return ExitCode::FAILURE;
        }
    }
    files.sort();

    let mut findings = 0usize;
    let mut failed = false;
    for file in &files {
        let relative = relative_slash_path(&root, file);
        let source = match std::fs::read_to_string(file) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("error: {relative} を読めない: {error}");
                failed = true;
                continue;
            }
        };
        match check_source(&relative, &source) {
            Ok(file_findings) => {
                for finding in &file_findings {
                    report(&relative, finding);
                }
                findings += file_findings.len();
            }
            Err(error) => {
                eprintln!("error: {relative} を構文解析できない: {error}");
                failed = true;
            }
        }
    }

    if findings > 0 {
        eprintln!(
            "amadeus-lint: {findings} 件の所見 ({} ファイル走査)",
            files.len()
        );
    }
    if failed || findings > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// rustc 風の 3 行形式で stderr に出す。
fn report(relative: &str, finding: &Finding) {
    eprintln!("error[{}]: {}", finding.rule, finding.message);
    eprintln!("  --> {relative}:{}", finding.line);
    eprintln!("  help: {}", finding.help);
    eprintln!();
}

/// 検査対象ルートの決定。
///
/// 1. 第 1 引数が与えられればそれ (テストや ad-hoc 実行向け)
/// 2. カレントディレクトリ (`cargo lint` をリポジトリルートで叩く通常運用)
/// 3. このクレートの位置から 2 階層遡ったリポジトリルート (`tools/lint` の親の親)
fn resolve_root() -> Result<PathBuf, String> {
    if let Some(arg) = std::env::args().nth(1) {
        let root = PathBuf::from(arg);
        if root.is_dir() {
            return Ok(root);
        }
        return Err(format!("指定されたルート {} が存在しない", root.display()));
    }
    if let Ok(cwd) = std::env::current_dir()
        && cwd.join(SCAN_ROOTS[0]).is_dir()
    {
        return Ok(cwd);
    }
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    match manifest_dir.parent().and_then(Path::parent) {
        Some(root) if root.join(SCAN_ROOTS[0]).is_dir() => Ok(root.to_path_buf()),
        _ => Err(
            "リポジトリルートを特定できない (ルートで `cargo lint` を実行するか、\
第 1 引数でルートを指定する)"
                .to_string(),
        ),
    }
}

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if SKIPPED_DIRS.contains(&name.as_ref()) || name.starts_with('.') {
                continue;
            }
            collect_rust_files(&path, out)?;
        } else if file_type.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn relative_slash_path(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}
