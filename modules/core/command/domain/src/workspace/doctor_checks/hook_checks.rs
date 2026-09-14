//! D2.a〜D2.e — 設定ファイルが語るフック配線 (本家 2288–2447、2713–2720 の分岐)。

use crate::workspace::{
    DoctorCheck, DoctorCheckId, HookBindingDeclaration, HookBindingTarget, HookWiring,
};

const SETTINGS_RELATIVE: &str = ".claude/settings.json";
const DECLARATION_RELATIVE: &str = "scripts/aidlc-selfhost/hook-binding.json";
const MANAGED_LABEL: &str = "enterprise managed settings";
const BINDINGS_LABEL: &str = "Native hook bindings";

/// D2.a のフック別行、D2.b、D2.c (該当時)、D2.d、D2.e。
pub(super) fn evaluate(wiring: &HookWiring) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    match wiring.wired_hooks() {
        Err(_) => checks.push(DoctorCheck::failed(
            DoctorCheckId::D2a,
            "Hook contract: settings.json unreadable — cannot verify wired hooks".to_string(),
            "restore .claude/settings.json (copy from `dist/claude/.claude/settings.json`)"
                .to_string(),
        )),
        Ok(hooks) if hooks.is_empty() => checks.push(DoctorCheck::failed(
            DoctorCheckId::D2a,
            "Hook contract: settings.json wires no aidlc-*.ts hooks".to_string(),
            "restore the hooks block in .claude/settings.json (copy from `dist/claude/.claude/settings.json`)"
                .to_string(),
        )),
        Ok(hooks) => {
            for hook in hooks {
                let label = format!("{} present", hook.name());
                checks.push(if hook.present() {
                    DoctorCheck::passed(DoctorCheckId::D2a, label)
                } else {
                    DoctorCheck::failed(
                        DoctorCheckId::D2a,
                        label,
                        "verify file exists in .claude/hooks/".to_string(),
                    )
                });
            }
        }
    }
    checks.push(match wiring.hooks_disabled_by() {
        None => DoctorCheck::passed(
            DoctorCheckId::D2b,
            "Hooks enabled (resolved disableAllHooks is not true)".to_string(),
        ),
        Some(layer) => DoctorCheck::failed(
            DoctorCheckId::D2b,
            format!(
                "Hooks DISABLED via \"disableAllHooks\": true in {layer} — AI-DLC cannot run (audit, state sync, sensors, and stage-graph rebuild are all silently skipped even though the hook files are present)"
            ),
            if layer == MANAGED_LABEL {
                "\"disableAllHooks\": true is enforced by enterprise managed settings — the highest-precedence layer, which a project or user setting cannot override. IT policy must remove it (or set it to false) for AI-DLC to run. If policy mandates disabled hooks, AI-DLC v2 is not compatible with this environment — its workflow engine is hook-driven.".to_string()
            } else {
                format!(
                    "remove \"disableAllHooks\": true from {layer} (or set it to false in a higher-precedence layer such as .claude/settings.local.json) and restart the Claude Code session — AI-DLC's workflow engine is hook-driven and cannot advance while hooks are disabled."
                )
            },
        ),
    });
    if wiring.managed_hooks_only() {
        checks.push(DoctorCheck::failed(
            DoctorCheckId::D2c,
            "Claude managed hook policy: allowManagedHooksOnly=true".to_string(),
            "hooks from .claude/settings.json are blocked by organization policy (allowManagedHooksOnly); only the Claude Code administrator can lift it in managed-settings.json. Until then, the workflow's human-presence and summary-confirmation receipts cannot be minted; attended sessions can set AIDLC_SKIP_HUMAN_PRESENCE_GUARD=1 and AIDLC_SKIP_SUMMARY_CONFIRMATION_GUARD=1 in the environment that launches the CLI as a temporary bypass".to_string(),
        ));
    }
    checks.push(if wiring.settings_present() {
        DoctorCheck::passed(DoctorCheckId::D2d, "settings.json present".to_string())
    } else {
        DoctorCheck::failed(
            DoctorCheckId::D2d,
            "settings.json present".to_string(),
            "copy from `dist/claude/.claude/settings.json`".to_string(),
        )
    });
    checks.push(match binding_failure(wiring) {
        None => DoctorCheck::passed(DoctorCheckId::D2e, BINDINGS_LABEL.to_string()),
        Some(fix) => DoctorCheck::failed(DoctorCheckId::D2e, BINDINGS_LABEL.to_string(), fix),
    });
    checks
}

/// D2.e の失敗 (無ければ正常)。原因を、それが読める資料の相対パスとともに返す。
///
/// 見るのは 3 つ。**解決できない呼出し**、**この build のフック面に無い名前を native 形で
/// 書いた登録**、そして接続定義がある作業ツリーでの**宣言と登録の食い違い**である。
/// native と配布が混ざっていること自体は失敗ではない — 混ぜ方を語るのが接続定義であり、
/// 定義どおりの混在は正常な接続の姿である (U4 の接続定義、オーナー裁定 2026-09-12)。
fn binding_failure(wiring: &HookWiring) -> Option<String> {
    let bindings = match wiring.bindings() {
        Err(_) if !wiring.settings_present() => {
            return Some(format!("{SETTINGS_RELATIVE}: missing"));
        }
        Err(cause) if cause.cause().starts_with("invalid JSON") => {
            return Some(format!("{SETTINGS_RELATIVE}: {}", cause.cause()));
        }
        Err(cause) => return Some(format!("{SETTINGS_RELATIVE}: unreadable ({cause})")),
        Ok(bindings) => bindings,
    };
    if bindings.is_empty() {
        return Some(format!("{SETTINGS_RELATIVE}: no hook bindings"));
    }
    if let Some(unknown) = bindings
        .iter()
        .find(|binding| matches!(binding.target(), HookBindingTarget::Unknown))
    {
        return Some(format!(
            "{SETTINGS_RELATIVE}: binding mismatch ({}: {})",
            unknown.event(),
            unknown.command()
        ));
    }
    if let Some(name) = bindings
        .iter()
        .filter_map(|binding| match binding.target() {
            HookBindingTarget::Native(name) => Some(name.as_str()),
            _ => None,
        })
        .find(|name| !wiring.native_hook_names().iter().any(|known| known == name))
    {
        return Some(format!(
            "{SETTINGS_RELATIVE}: binding mismatch (unknown native hook {name})"
        ));
    }
    match wiring.declaration() {
        HookBindingDeclaration::Absent => None,
        HookBindingDeclaration::Unreadable(cause) => {
            Some(format!("{DECLARATION_RELATIVE}: unreadable ({cause})"))
        }
        HookBindingDeclaration::Declared {
            native,
            distributed,
        } => bindings
            .iter()
            .find_map(|binding| declared_mismatch(binding.target(), native, distributed))
            .map(|cause| format!("{SETTINGS_RELATIVE}: {cause}")),
    }
}

/// 登録 1 件が宣言どおりの面へ結ばれているか (食い違えばその原因)。
fn declared_mismatch(
    target: &HookBindingTarget,
    native: &[String],
    distributed: &[String],
) -> Option<String> {
    let declares = |names: &[String], name: &str| names.iter().any(|declared| declared == name);
    match target {
        HookBindingTarget::Native(name) if declares(distributed, name) => Some(format!(
            "binding mismatch (declared distributed, registered native: {name})"
        )),
        HookBindingTarget::Distributed(name) if declares(native, name) => Some(format!(
            "binding mismatch (declared native, registered distributed: {name})"
        )),
        HookBindingTarget::Native(name) | HookBindingTarget::Distributed(name)
            if !declares(native, name) && !declares(distributed, name) =>
        {
            Some(format!("binding mismatch (undeclared hook {name})"))
        }
        _ => None,
    }
}
