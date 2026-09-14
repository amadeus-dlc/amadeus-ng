//! D2.a〜D2.e の観測 — Claude 設定ファイルとその優先層 (本家 `handleDoctor` 2288–2447 の読取)。

use std::path::{Path, PathBuf};

use core_query_use_case::orchestration::{
    HookBindingDeclaration, HookBindingTarget, HookBindingView, HookWiringView, ObservationFailure,
    WiredHookView,
};

use super::super::doctor_environment::DoctorEnvironment;
use super::super::doctor_paths::DoctorPaths;
use super::super::native_doctor_facts::NativeDoctorFacts;

/// 管理設定が最上位の層であることを示す本家のラベル。
const MANAGED_LABEL: &str = "enterprise managed settings";

/// このリポジトリ固有の接続定義の置き場 (無い作業ツリーもある)。
const DECLARATION_RELATIVE: &str = "scripts/aidlc-selfhost/hook-binding.json";

/// 設定の観測を束ねる。
pub(super) fn observe(
    paths: &DoctorPaths,
    environment: &DoctorEnvironment,
    facts: &NativeDoctorFacts,
) -> HookWiringView {
    let harness = paths.harness_dir();
    let settings = harness.join("settings.json");
    let raw = std::fs::read_to_string(&settings)
        .map_err(|error| ObservationFailure::new(error.to_string()));
    let wired_hooks = raw.as_ref().map(|raw| {
        let mut names = wired_hook_names(raw);
        names.sort();
        names.dedup();
        names
            .into_iter()
            .map(|name| {
                let present = harness.join("hooks").join(&name).exists();
                WiredHookView::new(name, present)
            })
            .collect()
    });
    let bindings = raw
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|raw| bindings_of(raw));
    HookWiringView::new(
        settings.exists(),
        wired_hooks.map_err(Clone::clone),
        hooks_disabled_by(paths, environment),
        managed_boolean("allowManagedHooksOnly", environment) == Some(true),
        bindings,
        facts.native_hook_names().to_vec(),
        declaration_of(paths),
    )
}

/// 接続定義を読む。無ければ [`HookBindingDeclaration::Absent`]、壊れていれば原因を運ぶ。
fn declaration_of(paths: &DoctorPaths) -> HookBindingDeclaration {
    let path = paths.project_dir().join(DECLARATION_RELATIVE);
    if !path.exists() {
        return HookBindingDeclaration::Absent;
    }
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) => {
            return HookBindingDeclaration::Unreadable(ObservationFailure::new(error.to_string()));
        }
    };
    let parsed: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(parsed) => parsed,
        Err(error) => {
            return HookBindingDeclaration::Unreadable(ObservationFailure::new(format!(
                "invalid JSON: {error}"
            )));
        }
    };
    let Some(native) = names_of(parsed.get("native_hooks")) else {
        return HookBindingDeclaration::Unreadable(ObservationFailure::new(
            "native_hooks is not a list of names".to_string(),
        ));
    };
    let Some(distributed) = names_of(parsed.get("distributed_hooks")) else {
        return HookBindingDeclaration::Unreadable(ObservationFailure::new(
            "distributed_hooks is not a list of names".to_string(),
        ));
    };
    HookBindingDeclaration::Declared {
        native,
        distributed,
    }
}

/// 文字列そのもの、またはオブジェクトの `name` を並べる (どちらか 1 つでも崩れていれば `None`)。
fn names_of(value: Option<&serde_json::Value>) -> Option<Vec<String>> {
    value?
        .as_array()?
        .iter()
        .map(|entry| match entry {
            serde_json::Value::String(name) => Some(name.clone()),
            other => other.get("name")?.as_str().map(str::to_string),
        })
        .collect()
}

/// 設定本文中の `aidlc-[A-Za-z0-9_-]+\.ts` をすべて拾う (本家の正規表現の写し)。
fn wired_hook_names(raw: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = raw;
    while let Some(start) = rest.find("aidlc-") {
        let tail = rest.get(start..).unwrap_or_default();
        let body_len = tail
            .get(6..)
            .unwrap_or_default()
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            .map_or(tail.len(), |offset| 6 + offset);
        let body = tail.get(..body_len).unwrap_or_default();
        if body.len() > 6 && tail.get(body_len..).unwrap_or_default().starts_with(".ts") {
            names.push(format!("{body}.ts"));
        }
        rest = tail.get(1..).unwrap_or_default();
    }
    names
}

/// `disableAllHooks: true` を明示した最上位の層 (本家 2379–2431)。
fn hooks_disabled_by(paths: &DoctorPaths, environment: &DoctorEnvironment) -> Option<String> {
    match managed_boolean("disableAllHooks", environment) {
        Some(true) => return Some(MANAGED_LABEL.to_string()),
        Some(false) => return None,
        None => {}
    }
    let harness = paths.harness_dir();
    let mut layers: Vec<(PathBuf, &str)> = vec![
        (
            harness.join("settings.local.json"),
            ".claude/settings.local.json",
        ),
        (harness.join("settings.json"), ".claude/settings.json"),
    ];
    if let Some(home) = environment.home() {
        layers.push((
            home.join(".claude").join("settings.json"),
            "~/.claude/settings.json",
        ));
    }
    for (path, label) in layers {
        // 明示的な boolean を持つ層だけが解決する。無い・読めない・壊れている層は次へ。
        if let Some(value) = boolean_field(&path, "disableAllHooks") {
            return value.then(|| label.to_string());
        }
    }
    None
}

/// 管理設定 (候補 + `managed-settings.d/*.json`) の boolean を後勝ちで解決する
/// (本家 `resolveManagedBooleanSetting`)。
fn managed_boolean(key: &str, environment: &DoctorEnvironment) -> Option<bool> {
    for candidate in managed_candidates(environment) {
        let mut effective = None;
        for path in managed_files(&candidate) {
            if let Some(value) = boolean_field(&path, key) {
                effective = Some(value);
            }
        }
        if effective.is_some() {
            return effective;
        }
    }
    None
}

/// 管理設定ファイルの候補 (本家 `resolveManagedSettingsCandidates`)。
fn managed_candidates(environment: &DoctorEnvironment) -> Vec<PathBuf> {
    if let Some(explicit) = environment.managed_settings_path() {
        return vec![explicit.clone()];
    }
    if cfg!(target_os = "macos") {
        vec![PathBuf::from(
            "/Library/Application Support/ClaudeCode/managed-settings.json",
        )]
    } else if cfg!(windows) {
        vec![
            PathBuf::from("C:\\Program Files\\ClaudeCode\\managed-settings.json"),
            PathBuf::from("C:\\ProgramData\\ClaudeCode\\managed-settings.json"),
        ]
    } else {
        vec![PathBuf::from("/etc/claude-code/managed-settings.json")]
    }
}

/// 候補と、その隣の `managed-settings.d/*.json` (名前順)。
fn managed_files(candidate: &Path) -> Vec<PathBuf> {
    let mut files = vec![candidate.to_path_buf()];
    let fragments = candidate.parent().map(|dir| dir.join("managed-settings.d"));
    if let Some(entries) = fragments.and_then(|dir| std::fs::read_dir(dir).ok()) {
        let mut names: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json") && path.is_file())
            .collect();
        names.sort();
        files.extend(names);
    }
    files
}

/// JSON ファイルの最上位キーが boolean ならその値 (無い・読めない・壊れているは `None`)。
fn boolean_field(path: &Path, key: &str) -> Option<bool> {
    let bytes = std::fs::read(path).ok()?;
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    parsed.get(key)?.as_bool()
}

/// `hooks` ブロックの登録を順に写す (JSON として読めなければ原因)。
fn bindings_of(raw: &str) -> Result<Vec<HookBindingView>, ObservationFailure> {
    let parsed: serde_json::Value = serde_json::from_str(raw)
        .map_err(|error| ObservationFailure::new(format!("invalid JSON: {error}")))?;
    let mut bindings = Vec::new();
    let Some(events) = parsed.get("hooks").and_then(serde_json::Value::as_object) else {
        return Ok(bindings);
    };
    for (event, groups) in events {
        for group in groups.as_array().into_iter().flatten() {
            let matcher = group
                .get("matcher")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            for hook in group
                .get("hooks")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                let command = hook
                    .get("command")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                bindings.push(HookBindingView::new(
                    event.clone(),
                    matcher.to_string(),
                    command.to_string(),
                    classify(command),
                ));
            }
        }
    }
    Ok(bindings)
}

/// コマンド行の呼出し先を識別する — 配布 `.ts` か、この build のフック面か。
fn classify(command: &str) -> HookBindingTarget {
    let words = shell_words(command);
    for word in &words {
        let base = word.rsplit(['/', '\\']).next().unwrap_or(word);
        if let Some(stem) = base
            .strip_prefix("aidlc-")
            .and_then(|rest| rest.strip_suffix(".ts"))
        {
            return HookBindingTarget::Distributed(stem.to_string());
        }
    }
    for (index, word) in words.iter().enumerate() {
        if word != "hook" || index == 0 {
            continue;
        }
        let Some(program) = words.get(index - 1) else {
            continue;
        };
        let base = program.rsplit(['/', '\\']).next().unwrap_or(program);
        let base = base.strip_suffix(".exe").unwrap_or(base);
        if matches!(base, "aidlc" | "aidlc-orchestrate")
            && let Some(name) = words.get(index + 1)
        {
            return HookBindingTarget::Native(name.clone());
        }
    }
    HookBindingTarget::Unknown
}

/// 空白区切り + 引用符剥がしの最小の語分割 (設定の command は展開しない)。
fn shell_words(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut pending = false;
    for c in command.chars() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => current.push(c),
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                pending = true;
            }
            None if c.is_whitespace() => {
                if pending || !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                    pending = false;
                }
            }
            None => current.push(c),
        }
    }
    if pending || !current.is_empty() {
        words.push(current);
    }
    words
}
