//! 全域フラグ（`--project-dir`）を剥がした残りの引数。

/// 全域フラグを剥がした残り（upstream の `--project-dir` 抽出と同じ前処理）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    project_dir: Option<String>,
    rest: Vec<String>,
}

impl Invocation {
    /// `--project-dir <path>` を剥がす。
    ///
    /// upstream は `--` 以降を literal として扱うので、そこから先の `--project-dir` は
    /// 剥がさない（`aidlc-orchestrate.ts:6111`）。
    #[must_use]
    pub fn strip_global_flags(args: &[String]) -> Invocation {
        let mut project_dir = None;
        let mut rest = Vec::new();
        let mut literal = false;
        let mut index = 0;
        while let Some(arg) = args.get(index) {
            if arg == "--" {
                literal = true;
                rest.push(arg.clone());
            } else if !literal && arg == "--project-dir" && index + 1 < args.len() {
                project_dir = args.get(index + 1).cloned();
                index += 1;
            } else {
                rest.push(arg.clone());
            }
            index += 1;
        }
        Invocation { project_dir, rest }
    }

    /// `--project-dir` の値。
    #[must_use]
    pub fn project_dir(&self) -> Option<&str> {
        self.project_dir.as_deref()
    }

    /// 残りの引数。
    #[must_use]
    pub fn rest(&self) -> &[String] {
        &self.rest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|a| (*a).to_string()).collect()
    }

    #[test]
    fn the_project_dir_flag_is_stripped_before_the_subcommand_is_read() {
        let invocation = Invocation::strip_global_flags(&argv(&[
            "--project-dir",
            "/tmp/ws",
            "next",
            "--resume",
        ]));
        assert_eq!(invocation.project_dir(), Some("/tmp/ws"));
        assert_eq!(invocation.rest(), argv(&["next", "--resume"]).as_slice());
    }

    /// `--` 以降は literal なので、そこの `--project-dir` は剥がさない。
    #[test]
    fn a_project_dir_after_the_literal_marker_stays_in_the_arguments() {
        let invocation =
            Invocation::strip_global_flags(&argv(&["next", "--", "--project-dir", "/tmp/ws"]));
        assert_eq!(invocation.project_dir(), None);
        assert_eq!(
            invocation.rest(),
            argv(&["next", "--", "--project-dir", "/tmp/ws"]).as_slice()
        );
    }
}
