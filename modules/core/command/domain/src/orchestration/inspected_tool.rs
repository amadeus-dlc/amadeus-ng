//! `InspectedTool` — reviewer-scope が読み取り範囲を検査する工具の閉集合。
/// 読み取り範囲の判定にかける工具 (upstream の 10 種・掲載順)。
///
/// この列挙に無い工具の呼出しは素通しする。呼び手が名乗る綴りをそのまま監査の `Tool`
/// へ載せるため、[`InspectedTool::as_str`] は upstream の逐語を返す。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InspectedTool {
    /// ファイルを開く。
    Read,
    /// ノートブックを開く。
    NotebookRead,
    /// ファイルを書き換える。
    Edit,
    /// ファイルを複数箇所書き換える。
    MultiEdit,
    /// ファイルを書く。
    Write,
    /// ノートブックを書き換える。
    NotebookEdit,
    /// ディレクトリを一覧する。
    Ls,
    /// パターンでファイルを探す。
    Glob,
    /// 内容を検索する。
    Grep,
    /// シェルコマンドを実行する。
    Bash,
}
impl InspectedTool {
    /// 工具名を検査して読む (**この型の唯一の構築経路**)。閉集合の外は `None`。
    #[must_use]
    pub fn parse(raw: &str) -> Option<InspectedTool> {
        match raw {
            "Read" => Some(InspectedTool::Read),
            "NotebookRead" => Some(InspectedTool::NotebookRead),
            "Edit" => Some(InspectedTool::Edit),
            "MultiEdit" => Some(InspectedTool::MultiEdit),
            "Write" => Some(InspectedTool::Write),
            "NotebookEdit" => Some(InspectedTool::NotebookEdit),
            "LS" => Some(InspectedTool::Ls),
            "Glob" => Some(InspectedTool::Glob),
            "Grep" => Some(InspectedTool::Grep),
            "Bash" => Some(InspectedTool::Bash),
            _ => None,
        }
    }
    /// 監査項目 `Tool` へ載せる逐語の綴り。
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            InspectedTool::Read => "Read",
            InspectedTool::NotebookRead => "NotebookRead",
            InspectedTool::Edit => "Edit",
            InspectedTool::MultiEdit => "MultiEdit",
            InspectedTool::Write => "Write",
            InspectedTool::NotebookEdit => "NotebookEdit",
            InspectedTool::Ls => "LS",
            InspectedTool::Glob => "Glob",
            InspectedTool::Grep => "Grep",
            InspectedTool::Bash => "Bash",
        }
    }
    /// 経路を名指す鍵を持つ工具か (upstream `candidateStrings` の `"path"` 群)。
    #[must_use]
    pub const fn names_paths(&self) -> bool {
        matches!(
            self,
            InspectedTool::Read
                | InspectedTool::NotebookRead
                | InspectedTool::Edit
                | InspectedTool::MultiEdit
                | InspectedTool::Write
                | InspectedTool::NotebookEdit
        )
    }
}

#[cfg(test)]
mod tests {
    use super::InspectedTool;
    #[test]
    fn the_ten_inspected_tools_round_trip_their_spelling() {
        for raw in [
            "Read",
            "NotebookRead",
            "Edit",
            "MultiEdit",
            "Write",
            "NotebookEdit",
            "LS",
            "Glob",
            "Grep",
            "Bash",
        ] {
            assert_eq!(InspectedTool::parse(raw).map(|t| t.as_str()), Some(raw));
        }
    }
    #[test]
    fn anything_outside_the_closed_set_is_not_inspected() {
        for raw in ["Task", "WebFetch", "read", "bash", ""] {
            assert_eq!(InspectedTool::parse(raw), None, "{raw}");
        }
    }
    #[test]
    fn only_the_six_file_tools_name_paths() {
        assert!(InspectedTool::parse("Write").is_some_and(|t| t.names_paths()));
        assert!(InspectedTool::parse("LS").is_some_and(|t| !t.names_paths()));
        assert!(InspectedTool::parse("Bash").is_some_and(|t| !t.names_paths()));
    }
}
