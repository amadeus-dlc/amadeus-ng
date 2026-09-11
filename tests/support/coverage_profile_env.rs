//! カバレッジ計測（cargo llvm-cov）下で子プロセスを起動する契約テストの補助。
//!
//! 計測済みバイナリは終了時に `LLVM_PROFILE_FILE` の先へプロファイルを書く。
//! `env_clear` でその変数を落とすと既定名 `default_*.profraw` が cwd（多くは
//! 一時ワークスペース）へ生成され、「読むだけ」「初期化を起こさない」を
//! ファイル一覧で検査する契約が計測の副産物で崩れる。設定されているときだけ
//! 引き継ぎ、計測外では何も渡さない。
pub(super) fn coverage_profile_env() -> Option<(&'static str, std::ffi::OsString)> {
    std::env::var_os("LLVM_PROFILE_FILE").map(|value| ("LLVM_PROFILE_FILE", value))
}
