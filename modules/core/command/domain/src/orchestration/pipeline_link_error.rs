//! Pipelineの引継ぎ受領を拒否した理由。公開文言は出力側で組む。
/// 受領の前提を満たさなかった材料。
#[derive(Debug)]
pub enum PipelineLinkError {
    /// 定義上pipelineではない。
    NotPipeline {
        /// 対象ステージ。
        stage: String,
    },
    /// 宣言されたlinkではない。
    UnknownLink {
        /// 対象ステージ。
        stage: String,

        /// 指定されたlink。
        link: String,

        /// 定義のlead/support連鎖。
        declared: String,
    },
    /// 登録のないrepo識別を指定した。
    UnregisteredRepo {
        /// 対象ステージ。
        stage: String,
    },
    /// 現在の試行で既に受領済み。
    Duplicate {
        /// 対象ステージ。
        stage: String,

        /// 指定されたlink。
        link: String,

        /// 指定されたrepo。
        repo: Option<String>,
    },
    /// 前のlinkが未完了。
    OutOfOrder {
        /// 対象ステージ。
        stage: String,

        /// 指定されたlink。
        link: String,

        /// 必要な直前のlink。
        previous: String,

        /// 連鎖の位置（1始まり）。
        position: usize,

        /// 連鎖の総数。
        total: usize,

        /// 指定されたrepo。
        repo: Option<String>,
    },
    /// developerのartifactが未指定。
    ArtifactRequired,
    /// artifactの場所が異なる。
    ArtifactPath {
        /// 期待する相対パス。
        expected: String,
    },
    /// artifactが存在しない。
    ArtifactMissing {
        /// 渡されたartifact指定。
        supplied: String,
    },
    /// 通常ファイルを安全に読めない。
    ArtifactUnreadable {
        /// I/O境界で確認した原因。
        cause: String,
    },
    /// 現在の試行より古い。
    ArtifactStale {
        /// 対象の相対パス。
        path: String,
    },
    /// 前回受領後に書き直されていない。
    ArtifactNotRewritten {
        /// 対象の相対パス。
        path: String,
    },
    /// handoffの保存値が不正。
    InvalidHandoff,
    /// 実行集約による拒否。
    Command(super::CommandError),
}
impl std::fmt::Display for PipelineLinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pipeline link: {self:?}")
    }
}
impl std::error::Error for PipelineLinkError {}
