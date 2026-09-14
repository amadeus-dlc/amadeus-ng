//! `Codekb` — リポジトリごとの durable な知識ストアの集約ルート。

use super::codekb_artifacts::CodekbArtifacts;
use super::codekb_candidate::CodekbCandidate;
use super::codekb_event::{CodekbEvent, CodekbInterruptedPublicationSettled, CodekbPublished};
use super::codekb_event_id::CodekbEventId;
use super::codekb_generation::CodekbGeneration;
use super::codekb_publish_refusal::CodekbPublishRefusal;
use super::codekb_repo_id::CodekbRepoId;
use super::codekb_scope_paths::CodekbScopePaths;
use super::codekb_snapshot::CodekbSnapshot;
use super::codekb_source_fingerprint::CodekbSourceFingerprint;

/// 再構成した時点でストアが抱えていた「中断した公開」と、その後の決着 (集約私有の状態)。
///
/// 3 つの状態を 1 つの型で持つので、「抱えていないのに畳んだ」という組み合わせが表せない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InterruptedPublication {
    /// 中断した公開は残っていない。
    Absent,
    /// 中断した公開が残っている (まだ畳んでいない)。
    Pending,
    /// 中断した公開を畳んだ。
    Settled,
}

/// リポジトリごとの codekb ストア。
///
/// 識別子はリポジトリを指す識別子である — ストアは space の中の全 intent が共有し、intent では鍵付か
/// ない。状態は**再構成した時点で観測した世代**であり、これが compare-and-swap の片側になる。
///
/// # 中断した公開は集約の状態である
///
/// 公開は途中で落ちうるので、再構成の時点で**まだ決着していない公開**が残っていることが
/// ある。それは観測できる事実なので、集約はその姿のまま
/// ([`Codekb::observed_with_interrupted_publication`]) 再構成され、畳むかどうかを
/// [`Codekb::settle_interrupted_publication`] で自分で決める。畳み方 (媒体の始末) は
/// Repository 実装の内部詳細であり、集約もポート面も知らない。
///
/// # 公開後の世代を持たない理由
///
/// 公開が済んだ時点の世代は、ディスクに置かれたバイトを畳み直して**観測**する値である。
/// 畳み方 (木のハッシュ) はインフラの持ち物なので、集約が自分で計算して名乗ることはできない。
/// したがって [`Codekb::publish`] は世代を進めず、公開した中身だけを覚える — 公開後の世代は
/// 呼出側が Gateway 経由で改めて観測する。中断した公開を畳んだあとの世代も同じである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Codekb {
    id: CodekbRepoId,
    generation: CodekbGeneration,
    interrupted_publication: InterruptedPublication,
    published: Option<CodekbArtifacts>,
}

impl Codekb {
    /// 実測した世代からストアを再構成する (中断した公開は残っていない)。
    ///
    /// ストアが 1 つも無いことも世代の 1 つ ([`CodekbGeneration::absent`]) なので、
    /// 「まだ無いストア」も同じ型で表せる。
    #[must_use]
    pub const fn observed(id: CodekbRepoId, generation: CodekbGeneration) -> Codekb {
        Codekb {
            id,
            generation,
            interrupted_publication: InterruptedPublication::Absent,
            published: None,
        }
    }

    /// **中断した公開を抱えたまま**、実測した世代からストアを再構成する。
    ///
    /// 世代は**畳む前のディスクの姿**である (Repository の読取は何も書き換えない)。畳んだ
    /// あとの世代が要るなら、決着を保存してから改めて観測する。
    #[must_use]
    pub const fn observed_with_interrupted_publication(
        id: CodekbRepoId,
        generation: CodekbGeneration,
    ) -> Codekb {
        Codekb {
            id,
            generation,
            interrupted_publication: InterruptedPublication::Pending,
            published: None,
        }
    }

    /// 走査の直前に 2 つの世代の写しを取る (状態を変えないクエリ)。
    #[must_use]
    pub fn take_snapshot(
        &self,
        paths: CodekbScopePaths,
        source_fingerprint: CodekbSourceFingerprint,
    ) -> CodekbSnapshot {
        CodekbSnapshot::new(
            self.id.clone(),
            paths,
            self.generation.clone(),
            source_fingerprint,
        )
    }

    /// 中断した公開に決着をつけ、その事実を 1 件返す。
    ///
    /// **畳むべきものが無ければ事実は起きない** (`None`) — 遷移していないのにイベントを
    /// 生むと、何も起きていない場面で Gateway が書込を通ることになる (空振りの書込)。
    /// 一度畳んだら二度目も `None` である (同じ決着を二度は語らない)。
    ///
    /// 世代は進めない: 畳んだあとの世代はディスクのバイトから観測する値であり、集約は
    /// 木を畳み直せない。
    pub fn settle_interrupted_publication(&mut self) -> Option<CodekbEvent> {
        if self.interrupted_publication != InterruptedPublication::Pending {
            return None;
        }
        self.interrupted_publication = InterruptedPublication::Settled;
        Some(CodekbEvent::InterruptedPublicationSettled(
            CodekbInterruptedPublicationSettled::new(CodekbEventId::generate(), self.id.clone()),
        ))
    }

    /// 候補を公開してよいかを決め、公開の事実を 1 件返す。
    ///
    /// 検査は upstream 逐語の順で行い、**最初に外れた 1 つだけ**を拒否として返す —
    /// ストアの世代 → 源の指紋 → 候補の鮮度印。`current_source` / `current_candidate_fingerprint`
    /// が `None` なのは「いま計算できない」という観測であり、呼出側 (Gateway) が実測して渡す。
    ///
    /// # Errors
    ///
    /// 3 つの compare-and-swap のいずれかが噛み合わなければ
    /// [`CodekbPublishRefusal`] を返す。
    pub fn publish(
        &mut self,
        candidate: CodekbCandidate,
        expected_store: &CodekbGeneration,
        expected_source: &CodekbSourceFingerprint,
        current_source: Option<&CodekbSourceFingerprint>,
        current_candidate_fingerprint: Option<&str>,
    ) -> Result<CodekbEvent, CodekbPublishRefusal> {
        if self.generation != *expected_store {
            return Err(CodekbPublishRefusal::StoreChanged {
                expected: expected_store.clone(),
                found: self.generation.clone(),
            });
        }
        if current_source != Some(expected_source) {
            return Err(CodekbPublishRefusal::SourceChanged {
                expected: expected_source.clone(),
                found: current_source.cloned(),
            });
        }
        // 記録が無い同士は食い違いではない (upstream の `null === null`)。
        if candidate.recorded_fingerprint() != current_candidate_fingerprint {
            return Err(CodekbPublishRefusal::CandidateStale {
                staged: candidate.recorded_fingerprint().map(str::to_string),
                current: current_candidate_fingerprint.map(str::to_string),
            });
        }
        // 候補はここで使い切る — 公開はその最後の使い道である。
        let artifacts = candidate.into_artifacts();
        let event = CodekbEvent::Published(CodekbPublished::new(
            CodekbEventId::generate(),
            self.id.clone(),
            artifacts.clone(),
        ));
        self.published = Some(artifacts);
        Ok(event)
    }

    /// 集約とイベントが同じ内容を語っているか (Gateway が対の取り違えを拒むための照合)。
    #[must_use]
    pub fn describes(&self, event: &CodekbEvent) -> bool {
        match event {
            CodekbEvent::Published(published) => {
                published.aggregate_id() == &self.id
                    && self.published.as_ref() == Some(published.artifacts())
            }
            CodekbEvent::InterruptedPublicationSettled(settled) => {
                settled.aggregate_id() == &self.id
                    && self.interrupted_publication == InterruptedPublication::Settled
            }
        }
    }

    /// codekb を鍵付けるリポジトリ識別子。
    #[must_use]
    pub const fn id(&self) -> &CodekbRepoId {
        &self.id
    }

    /// 公開した中身 (まだ公開していなければ `None`)。
    #[must_use]
    pub const fn published(&self) -> Option<&CodekbArtifacts> {
        self.published.as_ref()
    }
}
