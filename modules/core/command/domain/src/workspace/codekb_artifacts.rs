//! `CodekbArtifacts` — codekb を構成する 9 成果物ちょうどの集合。

use core_infrastructure::collections::{Collection, FirstClassCollection};

use super::codekb_artifact::CodekbArtifact;
use super::codekb_artifact_name::CodekbArtifactName;
use super::codekb_artifacts_error::CodekbArtifactsError;

/// codekb を構成する成果物の集合。
///
/// **9 つちょうど**であることがこの型の不変条件である (upstream `readCodekbCandidate` は
/// 過不足のある staged を受け取らない)。並びは常に辞書順なので、入力の順序が公開後の
/// ストアのバイトに影響しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodekbArtifacts(Vec<CodekbArtifact>);

impl CodekbArtifacts {
    /// 正準の 9 つちょうどであることを確かめて辞書順に束ねる
    /// (**この型の唯一の構築経路**)。
    ///
    /// # Errors
    ///
    /// 欠け・余分・重複のいずれかがあれば、実際に在った綴りを材料に添えて拒否する。
    pub fn of(artifacts: Vec<CodekbArtifact>) -> Result<CodekbArtifacts, CodekbArtifactsError> {
        let mut found: Vec<String> = artifacts
            .iter()
            .map(|artifact| artifact.name().as_str().to_string())
            .collect();
        found.sort();
        let canonical: Vec<String> = CodekbArtifactName::all()
            .iter()
            .map(|name| name.as_str().to_string())
            .collect();
        if found != canonical {
            return Err(CodekbArtifactsError::NotTheCanonicalNine { found });
        }
        let mut ordered = artifacts;
        ordered.sort_by(|left, right| left.name().cmp(right.name()));
        Ok(CodekbArtifacts(ordered))
    }
}

impl FirstClassCollection for CodekbArtifacts {
    type Item<'a> = &'a CodekbArtifact;
    /// 絞り込むと 9 つちょうどでなくなりうるので、結果は不変条件を持たない列で受ける。
    type Filtered = Collection<CodekbArtifact>;

    fn len(&self) -> usize {
        self.0.len()
    }

    fn at(&self, index: usize) -> Option<&CodekbArtifact> {
        self.0.get(index)
    }

    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a CodekbArtifact) -> A) -> A {
        self.0.iter().fold(initial, fold)
    }

    fn filter(
        &self,
        mut predicate: impl FnMut(&CodekbArtifact) -> bool,
    ) -> Collection<CodekbArtifact> {
        Collection::new(
            self.0
                .iter()
                .filter(|artifact| predicate(artifact))
                .cloned()
                .collect(),
        )
    }
}
