//! composition root — マルチコールバイナリ（ADR 0005 / A1）。
//!
//! **このファイルは配線だけである。** 判断・写像・パース・描画はすべて
//! [`aidlc`](aidlc) ライブラリ側にあり、単体テストが届く。ここが薄いままであることが、
//! `scripts/coverage.sh` の除外がこの 1 ファイルで済む条件である。

use std::io::Write as _;
use std::process::ExitCode;

/// 非同期ポート（AFIT）を回す唯一のランタイム。`Send` を要求しない設計なので
/// current_thread で足りる（C3 / Q3 = A）。
#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let mut argv = std::env::args();
    let argv0 = argv.next().unwrap_or_else(|| "aidlc".to_string());
    let args: Vec<String> = argv.collect();
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    let completion = aidlc::runtime::run(&argv0, &args, &cwd).await;

    // directive を出せなかったのに 0 で終わると、呼び手は「1 個出た」と信じて次へ進む
    // （1 起動 1 directive が契約なので、出せなかったことは黙って呑めない）。
    if let Some(line) = completion.line() {
        let mut stdout = std::io::stdout().lock();
        if let Err(error) = writeln!(stdout, "{line}").and_then(|()| stdout.flush()) {
            let mut stderr = std::io::stderr().lock();
            let _ = writeln!(
                stderr,
                "aidlc: cannot write the directive to stdout: {error}"
            );
            return ExitCode::FAILURE;
        }
    }
    if let Some(diagnostic) = completion.diagnostic() {
        let mut stderr = std::io::stderr().lock();
        let _ = writeln!(stderr, "{diagnostic}");
    }
    ExitCode::from(completion.code())
}
