//! memory 層 (決定論的 steering) の loader — active-space のルールファイルを読む。
//!
//! 読み順は memory 層の解決順 `org → team → project → phases/<phase>` (strict-additive)。
//! ファイルが**無い**のは正常 (ルール未整備・initialization はフェーズルールを持たない)。
//! **在るのに読めない** (権限・UTF-8 破損) のは blocking で [`RuleUnreadable`] を返す
//! (02 §10)。分割とパックはドメイン (`SteeringPlan::pack`) の純計算であり、ここは読むだけで
//! ある (issue #46 — 旧 `RuleBundleSource` ポート実装の I/O 部分)。

use std::collections::BTreeMap;
use std::path::Path;

use core_command_domain::orchestration::{MemoryRules, RuleContent};
use core_command_domain::workflow_definition::PhaseId;
use core_command_use_case::orchestration::RuleUnreadable;

/// フェーズルールファイルを持つフェーズ (initialization は持たない唯一のフェーズ)。
const RULED_PHASES: [PhaseId; 4] = [
    PhaseId::Ideation,
    PhaseId::Inception,
    PhaseId::Construction,
    PhaseId::Operation,
];

/// active-space の memory ディレクトリ (`aidlc/spaces/<space>/memory`) を読む。
///
/// # Errors
///
/// 在るのに読めないルールファイル (`RuleUnreadable` — blocking)。
pub fn load_memory_rules(memory_dir: &Path) -> Result<MemoryRules, RuleUnreadable> {
    let mut base = Vec::new();
    for relative in ["org.md", "team.md", "project.md"] {
        if let Some(file) = read_if_present(memory_dir, relative)? {
            base.push(file);
        }
    }
    let mut phases = BTreeMap::new();
    for phase in RULED_PHASES {
        let relative = format!("phases/{}.md", phase.as_str());
        if let Some(file) = read_if_present(memory_dir, &relative)? {
            phases.insert(phase, file);
        }
    }
    Ok(MemoryRules::new(base, phases))
}

/// 存在するファイルだけを読む — 無いのは正常、在るのに読めないのは blocking。
fn read_if_present(
    memory_dir: &Path,
    relative: &str,
) -> Result<Option<RuleContent>, RuleUnreadable> {
    let path = memory_dir.join(relative);
    if !path.exists() {
        return Ok(None);
    }
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(Some(RuleContent::new(path.display().to_string(), text))),
        Err(error) => Err(RuleUnreadable::new(
            path.display().to_string(),
            error.to_string(),
        )),
    }
}
