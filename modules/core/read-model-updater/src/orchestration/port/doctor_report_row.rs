//! `read_doctor_report` の 1 行 — 診断対象ごとの集計。

/// `read_doctor_report` の 1 行。
///
/// 値はすべて集約 `WorkspaceDoctor` のクエリの答えの写しであり、更新器の投影が組む。
/// クエリ側が数えずに済むよう、集計 (`passed` / `failed`) と終了コードまで焼き込む
/// (`coding-rules/cqrs-boundaries.md` 規則 6)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReportRow {
    id: String,
    target: String,
    passed: u64,
    failed: u64,
    exit_code: u8,
    seq_nr: usize,
}

impl DoctorReportRow {
    /// 行の値を束ねる。
    #[must_use]
    pub const fn new(
        id: String,
        target: String,
        passed: u64,
        failed: u64,
        exit_code: u8,
        seq_nr: usize,
    ) -> Self {
        Self {
            id,
            target,
            passed,
            failed,
            exit_code,
            seq_nr,
        }
    }

    /// 主キー (集約 id)。DAO が列へ書く境界のための読取。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 診断対象 (`spaces/<space>/intents[/<record>]`)。DAO が列へ書く境界のための読取。
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// 合格した行の数。DAO が列へ書く境界のための読取。
    #[must_use]
    pub const fn passed(&self) -> u64 {
        self.passed
    }

    /// 失敗した行の数。DAO が列へ書く境界のための読取。
    #[must_use]
    pub const fn failed(&self) -> u64 {
        self.failed
    }

    /// 終了コード。DAO が列へ書く境界のための読取。
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        self.exit_code
    }

    /// 集約の履歴通番。DAO が列へ書く境界のための読取。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }
}
