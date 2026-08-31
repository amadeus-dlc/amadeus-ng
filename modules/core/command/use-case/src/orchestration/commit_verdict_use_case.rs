//! `CommitVerdictUseCase` — 報告された結末を 1 つの遷移としてコミットする（FR2.1）。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    Intent, IntentExecution, IntentExecutionEvent, IntentExecutionId, StageIndex,
};
use core_command_domain::workflow_definition::StageSlug;
use core_command_domain::workspace::CheckboxState;

use super::commit_error::CommitError;
use super::port::IntentExecutionRepository;
use super::port::IntentRepository;
use super::port::RepositoryError;
use super::reported_transition::ReportedTransition;

/// コンダクタが報告した結末（[`ReportedTransition`]）を 1 つの遷移としてコミットする。
///
/// 定型は 3 手である: **`find_by_id` で集約を再構成 → 集約コマンドで判断 → `store` で保存**。
/// 以降の U6・U7 もこの形に乗る。
///
/// # 名前について
///
/// upstream の CLI 動詞は `report` だが、その綴りをそのまま型名にすると「レポート（帳票）を
/// 作る／読むユースケース」と読める（オーナー裁定 2026-08-29 — 実際に誤読された）。型名は
/// **更新の意図を先頭に置く**ものへ改めた。CLI 動詞と型の対応は U7 の ROUTES 表が持つので、
/// 型名が upstream の綴りに縛られる必要はない。
///
/// # ここに無いもの
///
/// - **業務判断**。前提の検査（受理状態・ゲートの有無・checkbox の前提集合・読み飛ばし可否・
///   通過済み判定）はすべて集約が持つ。ここにあるのは「どの集約コマンドを打つか」を集約の
///   クエリに訊いて決めるフロー制御だけである（`coding-rules/tell-dont-ask.md` — 判断は
///   状態の所有者へ）。
/// - **何が起きたかの読取チャネル**。[`CommitVerdictUseCase::execute`] は成功しても値を返さない
///   （下記「戻り値を持たない理由」）。
/// - **文言**。「Committed approve for "..."」のような逐語は合成ルート（U7）の Presenter が組む。
/// - **リードモデルの更新**。`aidlc-state.md` と監査シャードを最新化する `ReadModelUpdater` を
///   起動するのは合成ルート（U7）である。コマンド側のユースケースはクエリ側を知らない
///   （`coding-rules/cqrs-boundaries.md` — 境界はクレート分離で物理強制されている）。
/// - **`resume` のルーティング**。遷移をコミットしないので入力型に無く、Controller（U7）が
///   手前で分岐する（`coding-rules/use-case-rules.md` §3）。
///
/// # 束縛はスタティック
///
/// `dyn` は使わない（`coding-rules/use-case-rules.md` §2）。結線（実物 / インメモリの選択）は
/// 合成ルートだけが行い、ユースケースはポートの trait しか知らない。
#[derive(Debug)]
pub struct CommitVerdictUseCase<E: IntentExecutionRepository, I: IntentRepository> {
    execution_repository: E,
    intent_repository: I,
}

/// [`CommitVerdictUseCase::attempt`] 1 回分の結末。
///
/// 楽観 version の競合だけを `Err` から切り出しているのは、**再試行の対象を名指しする**ため
/// である。競合したときにその試行が対象にしたステージを持ち帰らないと、2 回目が「そのときの
/// カーソル」へ再解決してしまい、競合相手が先に承認していた場合に報告されていない次ステージを
/// コミットしうる。
#[derive(Debug)]
enum AttemptOutcome {
    /// 決着した — コミットしたか、何もコミットしない成功（ゲート既開・BR1.9）だった。
    Settled,
    /// 楽観 version が競合した。
    Conflicted {
        /// この試行が対象にしたステージ（再試行はこれを名指しする）。
        target: StageSlug,
        /// ストアが返した競合そのもの（2 回目も競合したらこれを伝播する）。
        conflict: RepositoryError<IntentExecutionId>,
    },
}

impl<E: IntentExecutionRepository, I: IntentRepository> CommitVerdictUseCase<E, I> {
    /// ポートの実装を 2 つ注入する。
    ///
    /// **ユースケースはリポジトリを保持し、`execute` の内部で使う** (改訂 10 のオーナー裁定)。
    /// 以前は Controller が `&Intent` を読んで渡していたが、あれは I8 — 読取専用ユースケース
    /// (`Next`) 専用のパターン — の誤適用だった (`coding-rules/use-case-rules.md` §4 の射程)。
    #[must_use]
    pub const fn new(execution_repository: E, intent_repository: I) -> CommitVerdictUseCase<E, I> {
        CommitVerdictUseCase {
            execution_repository,
            intent_repository,
        }
    }

    /// 報告された結末を 1 つの遷移としてコミットする。
    ///
    /// `stage` は報告が名指ししたステージ（`None` はカーソル）。カーソル以外を名指しした報告は
    /// **通過済み completed への再報告**としてのみ受理し、集約の `stale_report` に判断を委ねる
    /// （BR1.9）。
    ///
    /// `occurred_at` は呼出側が持つ時計の読みである — 集約は時計を持たない（NFR3.1）。
    ///
    /// # 引数は集約 ID と値オブジェクトだけである
    ///
    /// ユースケースの `execute` に**集約を渡さない** — 渡してよいのは集約 ID と値オブジェクト
    /// だけで、集約は保持するリポジトリから内部で取る
    /// （`coding-rules/use-case-rules.md` §2b、改訂 10 のオーナー裁定）。したがって
    /// 引数は `IntentExecutionId`（集約 ID）・`StageSlug`（値オブジェクト）・
    /// `ReportedTransition`（値オブジェクト）・`DateTime<Utc>`（値）である。
    ///
    /// 内部フローは ① 実行を再構成 → ② その `intent_id` を読む → ③ `IntentRepository` で
    /// 計画を引く → ④ `&Intent` を集約コマンドへ → ⑤ `store`。以前は Controller が
    /// `&Intent` を読んで渡していたが、あれは I8 — **読取専用**ユースケース専用のパターン —
    /// の誤適用だった（§4 の射程）。
    ///
    /// # 戻り値を持たない理由（CQS）
    ///
    /// 状態を変えるので Command であり、CQS が定める Command の形（`&mut self` +
    /// `Result<(), E>`）をそのまま採る（`coding-rules/command-query-separation.md`）。
    ///
    /// 「何をコミットしたか」を返さなくても呼出側は困らない。**この CLI では `ReadModelUpdater`
    /// の catch_up が同一プロセス内で同期実行される**ので、コミット直後にリードモデルは最新に
    /// なる。CLI サブコマンドの出力データはそこから、**コマンドユースケース → RMU（投影）→
    /// クエリユースケース**の経路で得る（オーナー裁定 2026-08-29 — 「イベントを基に RMU で
    /// 作ったリードモデルを読めばよい」）。クエリユースケースはクエリ側が所有し、ドメインには
    /// 依存しない（`coding-rules/cqrs-boundaries.md`）ので、境界違反にはならない。
    ///
    /// その読み先は `aidlc-state.md` や監査シャードとは限らない。あれは「upstream 互換・
    /// 人間可読・git 交換用」のリードモデルであってバイト逐語が契約であり、逆パースする設計には
    /// しない。RMU は同じイベント列からクエリ向きの投影（SQLite テーブル）を別に作ってよく、
    /// チェックポイント表は投影名をキーに持つので既に複数投影を許す形になっている。
    ///
    /// 集約コマンドが `&mut self -> Result<IntentExecutionEvent, _>` である事実は先例に
    /// ならない。あれは `store()` へ渡す**書込パイプラインの配管**であって、Presenter に
    /// 読ませる読取チャネルではない。
    ///
    /// **何もコミットしない成功が 2 つある** — 既に開いているゲートへの再報告
    /// （golden `awaiting-approval-repeat` は監査行も状態差分も空）と、カーソル通過済み
    /// ステージへの冪等な再報告（BR1.9）である。どちらも `Ok(())` であり、区別は戻り値では
    /// 外へ出さない。呼出側が区別を要するなら、それもクエリ側から得る。
    ///
    /// # `Conflict` は 1 回だけ再試行する
    ///
    /// 楽観 version の競合だけがこのユースケースの持つ唯一の再試行政策である
    /// （contract-design Q6 = A — 「楽観 version 競合は即 `Err`、ユースケースが 1 回だけ再構成
    /// して再試行、それでも競合なら CLI がエラー終了」）。ポート doc の C3 ③「`Conflict` 以外は
    /// 再試行しない」は、`Conflict` **だけ**が再試行の対象であり、その政策の持ち主が
    /// ユースケースであることを言っている。
    ///
    /// 再試行は**再構成からやり直す** — 古い集約に `store` だけ打ち直すのは、読んだ時点の版で
    /// 書くという楽観ロックの意味そのものを壊す。したがって 2 回目は `find_by_id` から始め、
    /// 新しい版の集約に改めてコマンドを打つ。2 回目も `Conflict` なら伝播する。
    ///
    /// **対象ステージは 1 回目が解決したものを名指しで引き継ぐ。** `stage` に `None` を受けた
    /// まま再試行すると対象が「そのときのカーソル」に再解決されてしまい、競合相手が先に同じ
    /// ゲートを承認していた場合、**報告されていない次のステージ**へ `Forward` を打つ
    /// （次ステージは `[-]` なので BR1.3 により承認が通ってしまう）。名指しすれば、その状況は
    /// [`is_stale_re_report`](CommitVerdictUseCase::is_stale_re_report) が通過済み no-op
    /// （BR1.9）に畳み、カーソルが動いていない競合（`set-autonomy` 等）は名指し == カーソルで
    /// 通常経路に入る。再試行の意味論が「**同じ報告をもう一度**」に固定される。
    ///
    /// **再試行は attempt 全体をやり直す**ので、計画も引き直す。`Intent` は不変なので再取得は
    /// 無害である（改訂 10）。
    ///
    /// 再試行後に集約がコマンドを拒否した場合も、そのまま伝播する。
    ///
    /// # Errors
    ///
    /// 実行の再構成・永続化の失敗（`Repository`）、計画の取得の失敗（`IntentRepository`）、
    /// 集約による拒否（`Command`）、計画に無いステージの名指し（`UnknownStage`）を返す。集約とポートの失敗は**そのまま伝播**する — 握り潰しも
    /// 言い換えもしない。再試行するのは上記の `Conflict` 1 回だけである。
    pub async fn execute(
        &mut self,
        execution_id: &IntentExecutionId,
        stage: Option<&StageSlug>,
        transition: ReportedTransition,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), CommitError> {
        let AttemptOutcome::Conflicted { target, .. } = self
            .attempt(execution_id, stage, transition.clone(), occurred_at)
            .await?
        else {
            return Ok(());
        };
        // 再試行は 1 回目が解決した対象を**名指しで**引き継ぐ（doc「対象ステージは…」を参照）。
        match self
            .attempt(execution_id, Some(&target), transition, occurred_at)
            .await?
        {
            AttemptOutcome::Settled => Ok(()),
            AttemptOutcome::Conflicted { conflict, .. } => Err(CommitError::Repository(conflict)),
        }
    }

    /// 再構成からコミットまでの 1 回分。競合したときはこれをもう 1 度だけ通す。
    ///
    /// # Errors
    ///
    /// 競合**以外**の失敗（再構成・集約の拒否・計画に無いステージ・符号化の失敗など）。
    /// 楽観 version の競合だけは `Err` ではなく [`AttemptOutcome::Conflicted`] で返す —
    /// 呼出側が再試行の対象を名指しできるように、この試行が対象にしたステージを添えるためで
    /// ある。
    async fn attempt(
        &mut self,
        execution_id: &IntentExecutionId,
        stage: Option<&StageSlug>,
        transition: ReportedTransition,
        occurred_at: DateTime<Utc>,
    ) -> Result<AttemptOutcome, CommitError> {
        // 再構成した集約は**ストアが刻んだ版を運んでいる**ので、書込へはそれをそのまま提示
        // する（ポート doc「楽観 version は集約が運ぶ」— 版は不透明なトークンであり
        // `aggregate.seq_nr()` から導いてはならない）。
        let mut aggregate = self.execution_repository.find_by_id(execution_id).await?;
        // 計画は**保持しているリポジトリから内部で取る**（改訂 10）。実行は intent を ID で
        // 参照するだけなので（`coding-rules/aggregate-references.md`）、その ID で引く。
        // 取り違えのガードは従来どおり集約側で発火する — ここでは構成上一致する。
        let intent = self
            .intent_repository
            .find_by_id(aggregate.intent_id())
            .await?;
        let intent = &intent;

        // 何もコミットしない 2 経路。どちらも成功であり、区別は戻り値に出さない。
        if let Some(named) = stage
            && Self::is_stale_re_report(intent, &aggregate, named)?
        {
            return Ok(AttemptOutcome::Settled);
        }
        let cursor = aggregate.cursor();
        if Self::gate_is_already_open(&aggregate, cursor, &transition) {
            return Ok(AttemptOutcome::Settled);
        }

        // ここまで来たら対象は必ずカーソルである — `stage` が名指ししていた場合、カーソルより
        // 手前のステージは既に上の no-op で返しているからである。
        let event = Self::command(intent, &mut aggregate, cursor, transition, occurred_at)?;
        match self.execution_repository.store(&event, &aggregate).await {
            Ok(()) => Ok(AttemptOutcome::Settled),
            Err(conflict @ RepositoryError::Conflict { .. }) => {
                match Self::slug_at(intent, cursor) {
                    Some(target) => Ok(AttemptOutcome::Conflicted { target, conflict }),
                    // カーソルは不変条件により常に範囲内なので到達しない。万一名指しできない
                    // なら盲目的にやり直さず伝播する — 誤ったステージを打つより安全である。
                    None => Err(CommitError::Repository(conflict)),
                }
            }
            Err(other) => Err(CommitError::Repository(other)),
        }
    }

    /// 名指しに使うステージ slug（範囲外は `None`）。
    ///
    /// 計画は intent の持ち物なので、集約ではなく `&Intent` から引く
    /// （`coding-rules/aggregate-references.md`）。
    fn slug_at(intent: &Intent, stage: StageIndex) -> Option<StageSlug> {
        intent
            .stages()
            .get(stage.to_usize())
            .map(|entry| entry.slug().clone())
    }

    /// 名指しされたステージがカーソルの手前なら、集約に通過済み判定を委ねる（BR1.9）。
    ///
    /// カーソル自身を名指しした報告は通常経路なので偽を返す。判断は集約の `stale_report` が
    /// 持ち、ここがしているのは slug から位置への解決だけである。集約のガードが答えるのは
    /// **受理してよいか**だけで、それは `Err` になるかどうかで既に済んでいる — 「次に何を
    /// すべきか」を呼出側へ渡す読取チャネルは作らない（判断はクエリ側の持ち物である）。
    ///
    /// # Errors
    ///
    /// 計画に無いステージ（`UnknownStage`）、通過済み completed でない対象（集約の
    /// `NotStale` をそのまま伝播）。
    fn is_stale_re_report(
        intent: &Intent,
        aggregate: &IntentExecution,
        named: &StageSlug,
    ) -> Result<bool, CommitError> {
        let target =
            Self::locate(intent, aggregate, named).ok_or_else(|| CommitError::UnknownStage {
                stage: named.clone(),
            })?;
        if target == aggregate.cursor() {
            return Ok(false);
        }
        aggregate.stale_report(target)?;
        Ok(true)
    }

    /// 解決済み計画の中での位置（計画に無ければ `None`）。
    ///
    /// 集約の読取モデル（`stages` / `stage_index`）だけで完結する**参照**であって判断ではない
    /// — 前提の判定（通過済み completed か）は集約の `stale_report` が持つ。
    fn locate(
        intent: &Intent,
        aggregate: &IntentExecution,
        named: &StageSlug,
    ) -> Option<StageIndex> {
        let position = intent
            .stages()
            .iter()
            .position(|entry| entry.slug() == named)?;
        aggregate.stage_index(position)
    }

    /// 既に開いているゲートへの `awaiting-approval` 再報告か。
    ///
    /// upstream の `cli/report/awaiting-approval-repeat` は監査行も状態差分も空である。集約に
    /// 打てば `CheckboxPrecondition` で拒否されるが、それは**失敗ではない**ので、報告された語と
    /// 現在の印を突き合わせるフロー制御でここを分ける。
    fn gate_is_already_open(
        aggregate: &IntentExecution,
        cursor: StageIndex,
        transition: &ReportedTransition,
    ) -> bool {
        matches!(transition, ReportedTransition::AwaitingApproval { .. })
            && aggregate.checkbox(cursor) == Some(CheckboxState::AwaitingApproval)
    }

    /// 報告された結末に対応する集約コマンドを 1 つ打つ。
    ///
    /// `Forward` がどちらのコマンドになるかは、報告された語ではなく**ステージの性質**で決まる
    /// — ゲート付きなら承認、非ゲート（initialization）なら完了である（BR1.3）。どちらを打つかを
    /// 集約の `gated` クエリに訊いて決めるのはフロー制御であって、業務判断の複製ではない。
    fn command(
        intent: &Intent,
        aggregate: &mut IntentExecution,
        cursor: StageIndex,
        transition: ReportedTransition,
        occurred_at: DateTime<Utc>,
    ) -> Result<IntentExecutionEvent, CommitError> {
        let event = match transition {
            ReportedTransition::AwaitingApproval { artifacts } => {
                aggregate.open_gate(intent, artifacts, occurred_at)
            }
            ReportedTransition::Forward { user_input } => {
                // カーソルは不変条件により常に範囲内なので `None` は起きない。起きたとしても
                // 非ゲート扱いに畳めば `complete_stage` が `InvalidTarget` で拒否するので、
                // ここで panic する理由はない（NFR4.3 — 集約の `commit` と同じ作法）。
                if aggregate.gated(intent, cursor).unwrap_or(false) {
                    aggregate.approve_gate(intent, user_input, occurred_at)
                } else {
                    aggregate.complete_stage(intent, occurred_at)
                }
            }
            ReportedTransition::Rejected { feedback } => {
                aggregate.reject_gate(intent, feedback, occurred_at)
            }
            ReportedTransition::Revised => aggregate.revise_stage(intent, occurred_at),
            ReportedTransition::Skipped { reason } => {
                aggregate.skip_stage(intent, reason, occurred_at)
            }
        };
        Ok(event?)
    }

    /// 注入された実行ポートの実装（テストが**効果**を観測するための継ぎ目）。
    #[cfg(test)]
    pub(crate) const fn execution_repository(&self) -> &E {
        &self.execution_repository
    }

    /// 注入された intent ポートの実装（テストが取得回数を観測するための継ぎ目）。
    #[cfg(test)]
    pub(crate) const fn intent_repository(&self) -> &I {
        &self.intent_repository
    }
}

#[cfg(test)]
mod tests {
    // panic! は「想定した変種でなければ即失敗」という検証用途で使っており、テスト失敗の
    // シグナルとして妥当なので許容する（集約のテストモジュールと同じ作法）。
    #![allow(clippy::panic)]

    use super::super::commit_error::CommitError;
    use super::super::commit_verdict_use_case::CommitVerdictUseCase;
    use super::super::port::RepositoryError;
    use super::super::reported_transition::ReportedTransition;
    use super::super::test_support::{
        InMemoryIntentExecutionRepository, InMemoryIntentRepository, absent_execution, at,
        execution_id, genesis, slug, start_from_plan,
    };
    use chrono::{DateTime, Utc};
    use core_command_domain::orchestration::{
        CommandError, Intent, IntentExecution, IntentExecutionEvent, Verdict,
    };
    use core_command_domain::workflow_definition::{PhaseId, PlanAction, StageSlug};
    use core_command_domain::workspace::CheckboxState;

    /// 索引 0（initialization）を完了させ、カーソルを最初のゲート付きステージへ進めた実行。
    fn at_the_first_gate(stage_count: usize) -> (Intent, IntentExecution) {
        let (intent, mut aggregate, _) = genesis(stage_count);
        aggregate
            .complete_stage(&intent, at())
            .expect("初期化ステージは非ゲートなので完了できる");
        (intent, aggregate)
    }

    /// テストの主体 — 2 本のポートを注入したユースケース。
    ///
    /// **ユースケースは計画を自分で取る**（改訂 10）ので、テストが `&Intent` を持ち回る必要は
    /// もう無い。intent は `InMemoryIntentRepository` に預けてある。
    struct Subject {
        use_case: CommitVerdictUseCase<InMemoryIntentExecutionRepository, InMemoryIntentRepository>,
    }

    impl Subject {
        async fn execute(
            &mut self,
            stage: Option<&StageSlug>,
            transition: ReportedTransition,
            occurred_at: DateTime<Utc>,
        ) -> Result<(), CommitError> {
            self.use_case
                .execute(&execution_id(), stage, transition, occurred_at)
                .await
        }

        const fn repository(&self) -> &InMemoryIntentExecutionRepository {
            self.use_case.execution_repository()
        }

        const fn intents(&self) -> &InMemoryIntentRepository {
            self.use_case.intent_repository()
        }
    }

    fn use_case(pair: (Intent, IntentExecution), version: usize) -> Subject {
        let (intent, aggregate) = pair;
        Subject {
            use_case: CommitVerdictUseCase::new(
                InMemoryIntentExecutionRepository::holding(aggregate, version),
                InMemoryIntentRepository::holding(intent),
            ),
        }
    }

    fn forward() -> ReportedTransition {
        ReportedTransition::Forward {
            user_input: Some("Approve".to_string()),
        }
    }

    /// **効果**の観測 — ストアが受理した唯一のイベントを取り出す。
    ///
    /// ユースケースは何が起きたかを返さないので、テストは戻り値ではなくストアに残った
    /// 痕跡でコミット内容を固定する。
    fn only_committed(repository: &InMemoryIntentExecutionRepository) -> &IntentExecutionEvent {
        let committed = repository.committed();
        assert_eq!(committed.len(), 1, "コミットは 1 件のはず");
        committed.first().expect("1 件ある")
    }

    // ---- 経路ごとの正常系（効果で観測する） ----

    #[tokio::test]
    async fn an_awaiting_approval_report_opens_the_gate() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        subject
            .execute(
                None,
                ReportedTransition::AwaitingApproval {
                    artifacts: vec!["intent.md".to_string()],
                },
                at(),
            )
            .await
            .expect("in-progress のゲート付きステージは開ける");
        let IntentExecutionEvent::GateOpened(opened) = only_committed(subject.repository()) else {
            panic!("GateOpened を期待した");
        };
        assert_eq!(opened.stage(), &slug(1));
        assert_eq!(opened.artifacts(), ["intent.md".to_string()]);
    }

    #[tokio::test]
    async fn a_repeated_awaiting_approval_report_commits_nothing() {
        // upstream の `cli/report/awaiting-approval-repeat` は監査行も状態差分も空である。
        let (intent, mut aggregate) = at_the_first_gate(3);
        aggregate
            .open_gate(&intent, vec!["intent.md".to_string()], at())
            .expect("最初の開放は通る");
        let mut subject = use_case((intent, aggregate), 2);
        subject
            .execute(
                None,
                ReportedTransition::AwaitingApproval {
                    artifacts: vec!["intent.md".to_string()],
                },
                at(),
            )
            .await
            .expect("既に開いているゲートへの再報告は成功扱い");
        assert!(subject.repository().committed().is_empty());
        assert_eq!(subject.repository().store_attempts(), 0, "書込を試みない");
        assert_eq!(
            subject.repository().version_of(&execution_id()),
            Some(2),
            "版も動かない"
        );
    }

    #[tokio::test]
    async fn a_forward_report_on_a_gated_stage_approves_the_gate() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        subject
            .execute(None, forward(), at())
            .await
            .expect("ゲート付きステージは承認できる");
        let IntentExecutionEvent::GateApproved(approved) = only_committed(subject.repository())
        else {
            panic!("GateApproved を期待した");
        };
        assert_eq!(approved.stage(), &slug(1));
        assert_eq!(approved.user_input(), Some("Approve"));
    }

    #[tokio::test]
    async fn a_forward_report_on_an_ungated_stage_completes_the_stage() {
        // カーソルは索引 0（initialization = 非ゲート）。どちらのコマンドを打つかは集約の
        // `gated` クエリで決まる。
        let (intent, aggregate, _) = genesis(3);
        let mut subject = use_case((intent, aggregate), 1);
        subject
            .execute(None, forward(), at())
            .await
            .expect("非ゲートステージは完了できる");
        let IntentExecutionEvent::StageCompleted(completed) = only_committed(subject.repository())
        else {
            panic!("StageCompleted を期待した");
        };
        assert_eq!(completed.stage(), &slug(0));
    }

    #[tokio::test]
    async fn a_rejected_report_carries_the_feedback() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        subject
            .execute(
                None,
                ReportedTransition::Rejected {
                    feedback: Some("Sharpen the testing posture.".to_string()),
                },
                at(),
            )
            .await
            .expect("ゲート付きステージは差し戻せる");
        let IntentExecutionEvent::GateRejected(rejected) = only_committed(subject.repository())
        else {
            panic!("GateRejected を期待した");
        };
        assert_eq!(rejected.feedback(), Some("Sharpen the testing posture."));
    }

    #[tokio::test]
    async fn a_revised_report_re_enters_the_gate() {
        let (intent, mut aggregate) = at_the_first_gate(3);
        aggregate
            .reject_gate(&intent, Some("直して".to_string()), at())
            .expect("差し戻しは通る");
        let mut subject = use_case((intent, aggregate), 2);
        subject
            .execute(None, ReportedTransition::Revised, at())
            .await
            .expect("revising のステージはゲートへ再入できる");
        let IntentExecutionEvent::StageRevised(revised) = only_committed(subject.repository())
        else {
            panic!("StageRevised を期待した");
        };
        assert_eq!(revised.stage(), &slug(1));
    }

    #[tokio::test]
    async fn a_skipped_report_carries_the_reason() {
        let (intent, mut aggregate, _) = start_from_plan(&[
            (PhaseId::Initialization, PlanAction::Execute, false),
            (PhaseId::Inception, PlanAction::Execute, true),
            (PhaseId::Inception, PlanAction::Execute, false),
        ]);
        aggregate
            .complete_stage(&intent, at())
            .expect("初期化は完了できる");
        let mut subject = use_case((intent, aggregate), 1);
        subject
            .execute(
                None,
                ReportedTransition::Skipped {
                    reason: "Not applicable".to_string(),
                },
                at(),
            )
            .await
            .expect("CONDITIONAL なステージは読み飛ばせる");
        let IntentExecutionEvent::StageSkipped(skipped) = only_committed(subject.repository())
        else {
            panic!("StageSkipped を期待した");
        };
        assert_eq!(skipped.reason(), "Not applicable");
    }

    // ---- 冪等・no-op ----

    #[tokio::test]
    async fn a_re_report_of_a_stage_the_cursor_has_passed_commits_nothing() {
        // BR1.9 — カーソル通過済み completed への再報告は冪等。判断は集約の `stale_report`。
        let mut subject = use_case(at_the_first_gate(3), 2);
        subject
            .execute(Some(&slug(0)), forward(), at())
            .await
            .expect("通過済み completed への再報告は冪等な成功");
        assert!(subject.repository().committed().is_empty());
        assert_eq!(subject.repository().store_attempts(), 0, "書込を試みない");
        assert_eq!(subject.repository().version_of(&execution_id()), Some(2));
    }

    #[tokio::test]
    async fn naming_the_cursor_explicitly_still_takes_the_normal_route() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        subject
            .execute(Some(&slug(1)), forward(), at())
            .await
            .expect("カーソル自身を名指しした報告は通常経路");
        assert!(matches!(
            only_committed(subject.repository()),
            IntentExecutionEvent::GateApproved(_)
        ));
    }

    #[tokio::test]
    async fn a_report_that_names_a_stage_outside_the_plan_is_refused() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        let unknown = StageSlug::parse("not-in-the-plan").expect("slug は文法内");
        let err = subject
            .execute(Some(&unknown), forward(), at())
            .await
            .expect_err("計画に無いステージは解決できない");
        assert!(matches!(err, CommitError::UnknownStage { stage } if stage == unknown));
        assert!(subject.repository().committed().is_empty());
    }

    #[tokio::test]
    async fn a_report_that_names_a_stage_the_cursor_has_not_reached_is_refused_by_the_aggregate() {
        let mut subject = use_case(at_the_first_gate(3), 1);
        let err = subject
            .execute(Some(&slug(2)), forward(), at())
            .await
            .expect_err("未着手のステージは通過済み completed ではない");
        let stage = at_the_first_gate(3)
            .1
            .stage_index(2)
            .expect("索引 2 は範囲内");
        assert!(matches!(
            err,
            CommitError::Command(CommandError::NotStale(inner)) if inner == stage
        ));
        assert!(subject.repository().committed().is_empty());
    }

    // ---- 楽観 version の往復 ----

    #[tokio::test]
    async fn the_write_presents_the_version_the_rehydration_returned() {
        // `aggregate.seq_nr()` から導かない — 再構成が返した版そのものを渡す（ポート doc C3）。
        let pair = at_the_first_gate(3);
        assert_eq!(pair.1.seq_nr(), 2, "通番と版はたまたま一致させない");
        let mut subject = use_case(pair, 7);
        subject
            .execute(None, forward(), at())
            .await
            .expect("承認は通る");
        assert_eq!(
            subject.repository().version_of(&execution_id()),
            Some(8),
            "版 7 を提示して書けたので、ストアは 8 を採番した"
        );
    }

    // ---- 異常系 ----

    #[tokio::test]
    async fn the_use_case_fetches_the_intent_itself_from_the_port() {
        // 改訂 10: `execute` は intent を受け取らない。実行を再構成し、その `intent_id` で
        // 保持しているリポジトリから引く。
        let mut subject = use_case(at_the_first_gate(3), 7);
        assert_eq!(subject.intents().lookups(), 0, "呼ぶ前は 0 回");
        subject
            .execute(None, forward(), at())
            .await
            .expect("ゲートは承認できる");
        assert_eq!(subject.intents().lookups(), 1, "1 試行につき 1 回引く");
    }

    #[tokio::test]
    async fn a_missing_intent_is_propagated_from_its_own_port() {
        // 実行は読めたが計画が引けない場合。ユースケースは握り潰さず、その面の失敗のまま
        // 伝播する（`RepositoryError` ではなく `IntentRepositoryError` である）。
        let (intent, aggregate) = at_the_first_gate(3);
        let mut use_case = CommitVerdictUseCase::new(
            InMemoryIntentExecutionRepository::holding(aggregate, 7),
            InMemoryIntentRepository::empty(),
        );
        let err = use_case
            .execute(&execution_id(), None, forward(), at())
            .await
            .expect_err("計画が無ければコミットできない");
        assert!(matches!(
            err,
            CommitError::IntentRepository(RepositoryError::NotFound { id }) if id == *intent.id()
        ));
    }

    #[tokio::test]
    async fn a_retry_fetches_the_intent_again() {
        // 再試行は attempt 全体をやり直すので intent も引き直す。Intent は不変なので
        // 再取得は無害である（改訂 10）。
        let (intent, aggregate) = at_the_first_gate(3);
        let mut subject = Subject {
            use_case: CommitVerdictUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                    aggregate, 7, 1,
                ),
                InMemoryIntentRepository::holding(intent),
            ),
        };
        subject
            .execute(None, forward(), at())
            .await
            .expect("1 回だけ再試行すれば通る");
        assert_eq!(subject.intents().lookups(), 2, "2 試行なので 2 回引く");
    }

    #[tokio::test]
    async fn a_missing_aggregate_is_reported_as_not_found() {
        let (intent, _, _) = genesis(3);
        let mut subject = CommitVerdictUseCase::new(
            InMemoryIntentExecutionRepository::empty(),
            InMemoryIntentRepository::holding(intent),
        );
        let err = subject
            .execute(&absent_execution(), None, forward(), at())
            .await
            .expect_err("ストアに無い集約は再構成できない");
        assert!(matches!(
            err,
            CommitError::Repository(RepositoryError::NotFound { id }) if id == absent_execution()
        ));
    }

    #[tokio::test]
    async fn a_first_conflict_is_retried_once_from_the_rehydration() {
        // 1 件の割り込み書込で 1 回目は競合し、2 回目で通る（contract-design Q6 = A）。
        let (intent, aggregate) = at_the_first_gate(3);
        let mut subject = Subject {
            use_case: CommitVerdictUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                    aggregate, 7, 1,
                ),
                InMemoryIntentRepository::holding(intent),
            ),
        };
        subject
            .execute(None, forward(), at())
            .await
            .expect("1 回だけ再試行すれば通る");
        assert!(matches!(
            only_committed(subject.repository()),
            IntentExecutionEvent::GateApproved(_)
        ));
        assert_eq!(
            subject.repository().store_attempts(),
            2,
            "再試行は 1 回だけ"
        );
        // 2 回目が通ったこと自体が「再構成からやり直した」証拠である — このストアは現在の版を
        // 提示した書込しか受理しないので、古い集約に `store` だけ打ち直していたら再び競合する。
        assert_eq!(subject.repository().version_of(&execution_id()), Some(9));
    }

    #[tokio::test]
    async fn a_retry_after_a_competitor_committed_the_same_gate_commits_nothing() {
        // 競合相手が先に同じゲートを承認してカーソルが動いたケース。再試行が対象を名指しし
        // 直さないと、報告されていない次ステージ（`[-]` なので BR1.3 で承認が通ってしまう）へ
        // `Forward` を打ってしまう。
        let (intent, held) = at_the_first_gate(3);
        let mut advanced = held.clone();
        advanced
            .approve_gate(&intent, Some("Approve".to_string()), at())
            .expect("競合相手の承認は通る");
        assert_ne!(
            advanced.cursor(),
            held.cursor(),
            "相手の承認でカーソルが動いている前提のテストである"
        );

        let mut subject = Subject {
            use_case: CommitVerdictUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_a_competing_commit(
                    held, advanced, 7,
                ),
                InMemoryIntentRepository::holding(intent),
            ),
        };
        subject
            .execute(None, forward(), at())
            .await
            .expect("通過済みになった報告は冪等な成功");

        assert!(
            subject.repository().committed().is_empty(),
            "次ステージを勝手にコミットしない"
        );
        assert_eq!(
            subject.repository().store_attempts(),
            1,
            "書込は 1 回目の失敗だけ — 再試行は BR1.9 の no-op に畳まれる"
        );
        assert_eq!(
            subject.repository().version_of(&execution_id()),
            Some(8),
            "版は相手の書込ぶんだけ進む"
        );
    }

    #[tokio::test]
    async fn a_second_conflict_is_propagated_without_a_further_retry() {
        // 2 件の割り込み書込。2 回目も競合したら伝播する — 3 回目は無い。
        let (intent, aggregate) = at_the_first_gate(3);
        let mut subject = Subject {
            use_case: CommitVerdictUseCase::new(
                InMemoryIntentExecutionRepository::holding_behind_concurrent_writes(
                    aggregate, 7, 2,
                ),
                InMemoryIntentRepository::holding(intent),
            ),
        };
        let err = subject
            .execute(None, forward(), at())
            .await
            .expect_err("2 回目も競合したら伝播する");
        assert!(matches!(
            err,
            CommitError::Repository(RepositoryError::Conflict {
                expected: 8,
                actual: 9,
            })
        ));
        assert_eq!(subject.repository().store_attempts(), 2, "3 回目は打たない");
        assert!(subject.repository().committed().is_empty());
    }

    #[tokio::test]
    async fn a_command_the_aggregate_refuses_is_propagated_verbatim() {
        // in-progress のステージは revise できない。ユースケースは言い換えも握り潰しもしない。
        let (intent, aggregate) = at_the_first_gate(3);
        let stage = aggregate.cursor();
        let mut subject = use_case((intent, aggregate), 1);
        let err = subject
            .execute(None, ReportedTransition::Revised, at())
            .await
            .expect_err("revising 以外はゲートへ再入できない");
        assert!(matches!(
            err,
            CommitError::Command(CommandError::CheckboxPrecondition {
                stage: inner,
                actual: CheckboxState::InProgress,
            }) if inner == stage
        ));
        assert!(
            subject.repository().committed().is_empty(),
            "拒否されたコマンドは 1 バイトも書かない"
        );
    }

    // ---- 入力の正規化 ----

    #[test]
    fn every_reported_transition_projects_onto_one_domain_verdict() {
        // `Verdict::Resume` はここに現れない — 再開は U7 が手前でルーティングする。
        let cases = [
            (
                ReportedTransition::AwaitingApproval {
                    artifacts: Vec::new(),
                },
                Verdict::AwaitingApproval,
            ),
            (
                ReportedTransition::Forward { user_input: None },
                Verdict::Forward,
            ),
            (
                ReportedTransition::Rejected { feedback: None },
                Verdict::Rejected,
            ),
            (ReportedTransition::Revised, Verdict::Revised),
            (
                ReportedTransition::Skipped {
                    reason: "x".to_string(),
                },
                Verdict::Skipped,
            ),
        ];
        for (transition, verdict) in cases {
            assert_eq!(transition.verdict(), verdict);
        }
    }
}
