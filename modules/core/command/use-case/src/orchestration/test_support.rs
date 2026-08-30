//! ユースケース層のテスト用の土台 — ポートのテストダブルと合成計画のフィクスチャ。
//!
//! # なぜアダプタ層の実装を借りないのか
//!
//! 借りられないからである。`core-command-use-case` の `Cargo.toml` に
//! `core-command-interface-adapter` を書いた瞬間に DIP のクレート分離強制が壊れ、依存も
//! 循環する (`coding-rules/use-case-rules.md` §1)。dev-dependency でも同じなので、
//! ユースケースのテストが使うポート実装は**本クレート内の `#[cfg(test)]` に置く**。
//!
//! ここに置くのは 1 つだけである — ポートのテストも `CommitVerdictUseCase` のテストも同じ
//! [`InMemoryIntentExecutionRepository`] を通す (`coding-rules/no-backward-compatibility.md`
//! — 同じ役割の口を 2 つ並立させない)。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    Created, Intent, IntentExecution, IntentExecutionEvent, IntentExecutionId, IntentId,
    StageDisplay, StageEntry, StartRequest, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};

use super::port::IntentExecutionRepository;
use super::port::IntentRepository;
use super::port::RepositoryError;

/// フィクスチャの intent 識別子 (UUIDv7)。
pub(crate) const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

/// フィクスチャの実行識別子 (UUIDv7)。
pub(crate) const EXECUTION: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";

/// イベントの発生時刻 — 集約は値を素通しするので固定値でよい (NFR3.1)。
pub(crate) fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-08-23T00:00:00Z")
        .expect("固定の ISO 8601 UTC")
        .with_timezone(&Utc)
}

/// フィクスチャの集約識別子。
pub(crate) fn intent() -> IntentId {
    IntentId::parse(INTENT).expect("フィクスチャの IntentId は UUIDv7")
}

/// フィクスチャの実行識別子。
pub(crate) fn execution_id() -> IntentExecutionId {
    IntentExecutionId::parse(EXECUTION).expect("フィクスチャの IntentExecutionId は UUIDv7")
}

/// ストアに居ない実行の識別子 (`NotFound` を見るテスト用)。
pub(crate) fn absent_execution() -> IntentExecutionId {
    IntentExecutionId::parse("018f3b2c-4d5e-7f60-8abc-def012345678")
        .expect("フィクスチャの IntentExecutionId は UUIDv7")
}

/// 合成計画の slug (文書順の位置がそのまま名前になる)。
pub(crate) fn slug(index: usize) -> StageSlug {
    StageSlug::parse(&format!("stage-{index}")).expect("フィクスチャの slug は文法内")
}

fn display(index: usize, phase: PhaseId) -> StageDisplay {
    StageDisplay::new(
        StageNumber::parse(&format!("{}.{}", phase.index(), index + 1))
            .expect("フィクスチャのステージ番号は文法内"),
        "Stage",
        "orchestrator",
    )
    .expect("単一行")
}

/// 合成計画の走査結果 (投影は見ないので固定値でよい)。
pub(crate) fn scan() -> WorkspaceScan {
    WorkspaceScan::new(
        BrownfieldGreenfield::Greenfield,
        "Unknown",
        "Unknown",
        "Unknown",
    )
    .expect("単一行")
}

/// フェーズと実効プラン・CONDITIONAL を名指しした合成計画で開始する。
pub(crate) fn start_from_plan(
    plan: &[(PhaseId, PlanAction, bool)],
) -> (Intent, IntentExecution, IntentExecutionEvent) {
    let stages = plan
        .iter()
        .enumerate()
        .map(|(index, (phase, action, conditional))| {
            StageEntry::new(
                slug(index),
                *phase,
                *action,
                *conditional,
                display(index, *phase),
            )
        })
        .collect();
    // 合成計画からの組み直しは完全コンストラクタ (IntentExecution::new) を通す — 検査点は genesis と
    // 同一である。誕生イベントを `store` する `IntentRepository` は U7 の課題である。
    let intent = Intent::from(Created::new(
        intent(),
        WorkflowDefinitionId::parse("claude").expect("フィクスチャの定義 id"),
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64)))
            .expect("フィクスチャの定義 revision"),
        StartRequest::new("classic", "report use case"),
        stages,
        scan(),
    ));
    let (execution, event) = IntentExecution::start(execution_id(), intent.clone(), at());
    (intent, execution, event)
}

/// 索引 0 = initialization (非ゲート)、索引 1 以降 = inception (ゲート付き) の合成計画。
pub(crate) fn genesis(stage_count: usize) -> (Intent, IntentExecution, IntentExecutionEvent) {
    let plan: Vec<(PhaseId, PlanAction, bool)> = (0..stage_count)
        .map(|index| {
            let phase = if index == 0 {
                PhaseId::Initialization
            } else {
                PhaseId::Inception
            };
            (phase, PlanAction::Execute, false)
        })
        .collect();
    start_from_plan(&plan)
}

/// [`IntentExecutionRepository`] のインメモリ実装。
///
/// 楽観 version は本家の実測どおり「新規作成は 0、1 件書くごとに 1 つ進む」で採番する。
/// レシーバは CQS どおり再構成が `&self`、永続化が `&mut self` である — 内部可変性で
/// `&self` に見せかけない (`coding-rules/interior-mutability.md`)。
///
/// # 応答をスクリプトできる
///
/// `Conflict` は「読んでから書くまでの間に別の書き手が入った」ときにしか起きないので、
/// 単一スレッドのテストからは自然には起こせない。そこで**割り込む書込**を台本として持たせる。
/// 台本が残っている間、`store` はストアの版を 1 つ進めて `Conflict` を返す。
///
/// 割り込みには 2 種類ある:
///
/// - [`holding_behind_concurrent_writes`](InMemoryIntentExecutionRepository::holding_behind_concurrent_writes)
///   — **版だけ**が進む。相手が書いた内容は模さない（`set-autonomy` のようにカーソルを
///   動かさない競合に相当する）。
/// - [`holding_behind_a_competing_commit`](InMemoryIntentExecutionRepository::holding_behind_a_competing_commit)
///   — 版に加えて**保持している集約も進む**。相手が先に同じゲートを承認してカーソルが動いた
///   状況に相当し、再構成し直した呼出は報告したステージが通過済みになっているのを見る。
#[derive(Debug)]
pub(crate) struct InMemoryIntentExecutionRepository {
    stored: Option<IntentExecution>,
    version: usize,
    interrupting_writes: usize,
    competing_commit: Option<IntentExecution>,
    store_attempts: usize,
    committed: Vec<IntentExecutionEvent>,
}

impl InMemoryIntentExecutionRepository {
    /// 基本コンストラクタ — 構築経路はここ 1 本に集約する
    /// (`coding-rules/factory-naming.md`)。
    fn new(
        stored: Option<IntentExecution>,
        version: usize,
        interrupting_writes: usize,
        competing_commit: Option<IntentExecution>,
    ) -> InMemoryIntentExecutionRepository {
        InMemoryIntentExecutionRepository {
            stored,
            version,
            interrupting_writes,
            competing_commit,
            store_attempts: 0,
            committed: Vec::new(),
        }
    }

    /// 何も入っていないストア — `find_by_id` は `NotFound` を返す。
    pub(crate) fn empty() -> InMemoryIntentExecutionRepository {
        InMemoryIntentExecutionRepository::new(None, 0, 0, None)
    }

    /// 集約 1 つを版 `version` で保持するストア。
    pub(crate) fn holding(
        aggregate: IntentExecution,
        version: usize,
    ) -> InMemoryIntentExecutionRepository {
        InMemoryIntentExecutionRepository::new(Some(aggregate), version, 0, None)
    }

    /// 最初の `writes` 回の `store` に、別の書き手の書込が割り込むストア。
    ///
    /// 割り込みが起きた回は版だけが 1 つ進み、提示された版は古くなるので `Conflict` になる。
    /// 集約は動かないので、再構成し直してもカーソルは同じ位置にある。台本を使い切ると通常の
    /// 楽観判定へ戻るので、**再構成からやり直した呼出だけが**新しい版を提示でき、書込に成功する。
    pub(crate) fn holding_behind_concurrent_writes(
        aggregate: IntentExecution,
        version: usize,
        writes: usize,
    ) -> InMemoryIntentExecutionRepository {
        InMemoryIntentExecutionRepository::new(Some(aggregate), version, writes, None)
    }

    /// 最初の `store` に、**集約を前進させる**別の書き手の書込が割り込むストア。
    ///
    /// 版が 1 つ進むと同時に、保持している集約が `advanced` へ置き換わる — 競合相手が先に
    /// 同じゲートを承認してカーソルが動いた状況である。再構成し直した呼出は、報告した
    /// ステージが既に通過済み（`[x]` かつカーソルより手前）になっているのを見る。
    pub(crate) fn holding_behind_a_competing_commit(
        aggregate: IntentExecution,
        advanced: IntentExecution,
        version: usize,
    ) -> InMemoryIntentExecutionRepository {
        InMemoryIntentExecutionRepository::new(Some(aggregate), version, 1, Some(advanced))
    }

    /// このストアが受理したイベント列 (コミットの有無を見るテスト用)。
    pub(crate) fn committed(&self) -> &[IntentExecutionEvent] {
        &self.committed
    }

    /// `store` が呼ばれた回数 (再試行が 1 回だけであることを見るテスト用)。
    pub(crate) const fn store_attempts(&self) -> usize {
        self.store_attempts
    }

    /// ストアが採番している現在の版。
    pub(crate) const fn version(&self) -> usize {
        self.version
    }
}

impl IntentExecutionRepository for InMemoryIntentExecutionRepository {
    async fn find_by_id(
        &self,
        id: &IntentExecutionId,
    ) -> Result<IntentExecution, RepositoryError<IntentExecutionId>> {
        // 識別子検索なので、保持している集約の識別子と一致するときだけ返す（ポート契約）。
        // 返す集約にはストアが採番した版を刻む — 呼出側はそれをそのまま書込へ提示する。
        self.stored
            .clone()
            .filter(|aggregate| aggregate.id() == id)
            .map(|aggregate| aggregate.with_version(self.version))
            .ok_or_else(|| RepositoryError::NotFound { id: id.clone() })
    }

    async fn store(
        &mut self,
        event: &IntentExecutionEvent,
        aggregate: &IntentExecution,
    ) -> Result<(), RepositoryError<IntentExecutionId>> {
        // 提示される版は集約が運んできたもの — 生値へ戻すのはストア境界を組む側だけである。
        let expected_version = aggregate.version();
        self.store_attempts += 1;
        if self.interrupting_writes > 0 {
            // 別の書き手が先に書いた — ストアの版が進み、提示された版が古くなる。
            self.interrupting_writes -= 1;
            self.version += 1;
            // カーソルを動かす競合なら、保持している集約も相手の書込後の状態へ差し替える。
            if let Some(advanced) = self.competing_commit.take() {
                self.stored = Some(advanced);
            }
            return Err(RepositoryError::Conflict {
                expected: expected_version,
                actual: self.version,
            });
        }
        if expected_version != self.version {
            return Err(RepositoryError::Conflict {
                expected: expected_version,
                actual: self.version,
            });
        }
        self.version = expected_version + 1;
        self.stored = Some(aggregate.clone());
        self.committed.push(event.clone());
        Ok(())
    }
}

/// ユースケースのテストが使う [`IntentRepository`] のダブル。
///
/// 実物の実装はまだ無い（読み先の設計ごと U7 の課題 — ポート doc を参照）ので、ここでは
/// 「保持している intent を返す / 無ければ `NotFound`」だけを模す。intent は不変なので、
/// 呼ばれるたびに同じ値が返る。
#[derive(Debug)]
pub(crate) struct InMemoryIntentRepository {
    held: Option<Intent>,
    lookups: std::cell::Cell<usize>,
}

impl InMemoryIntentRepository {
    /// 1 つの intent を保持する（この intent の識別子で引けば返る）。
    pub(crate) const fn holding(intent: Intent) -> InMemoryIntentRepository {
        InMemoryIntentRepository {
            held: Some(intent),
            lookups: std::cell::Cell::new(0),
        }
    }

    /// 何も保持しない（どの識別子で引いても `NotFound`）。
    pub(crate) const fn empty() -> InMemoryIntentRepository {
        InMemoryIntentRepository {
            held: None,
            lookups: std::cell::Cell::new(0),
        }
    }

    /// これまでに引かれた回数（再試行が intent を取り直すことの観測点）。
    pub(crate) fn lookups(&self) -> usize {
        self.lookups.get()
    }
}

impl IntentRepository for InMemoryIntentRepository {
    async fn find_by_id(&self, id: &IntentId) -> Result<Intent, RepositoryError<IntentId>> {
        self.lookups.set(self.lookups.get() + 1);
        match &self.held {
            Some(held) if held.id() == id => Ok(held.clone()),
            _ => Err(RepositoryError::NotFound { id: id.clone() }),
        }
    }
}
