//! `continue` のカーソル照合 — 提示トークンが現行か、文脈が動いていないか。
//!
//! 継続トークンは**単回使用**である（upstream 2.6.51、`aidlc-orchestrate.ts` の
//! `handleContinue` → `advanceContinuationCursor`）。1 度後続を発行したトークンは
//! 現行ではなくなり、再提示は後続の繰り返しではなく拒否になる。
//!
//! 本家が 1 つのトランザクションの中で出す 3 つの結論をここへ写す。
//!
//! | 本家の結論 | ここでの [`Verdict`] | 逐語 |
//! | --- | --- | --- |
//! | `superseded` | [`Verdict::Superseded`] | [`crate::wording::CONTINUATION_TOKEN_SUPERSEDED`] |
//! | `drift` | [`Verdict::ContextChanged`] | [`crate::wording::CONTINUATION_CONTEXT_CHANGED`] |
//! | `ActiveDirectiveLockContendedError` | [`Verdict::Busy`] | [`crate::wording::CONTINUATION_COORDINATION_BUSY`] |
//!
//! `drift` は**準備中に文脈が入れ替わった**ときだけの結論である（本家は snapshot と
//! トランザクション内の再読取を突き合わせる）。したがってここでも「入口で採った
//! [`Snapshot`]」と「publish 直前の実測」を比べる。marker が別 intent を指しているだけの
//! ときは drift ではない — 本家はそれを 1 度だけの復旧継続として通すからである
//! （CHANGELOG 2.6.51「a mismatched token is stale, not a bootstrap opportunity」）。

use super::Layout;
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, parse};

/// この build が名乗るハーネス（marker の `cursor_harness`）。
const CURSOR_HARNESS: &str = "claude";

/// marker の読取上限。
const MARKER_MAX_BYTES: u64 = 65_536;

/// カーソル照合の結論。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Verdict {
    /// 提示トークンは現行。後続を発行してよい。
    Current,
    /// 消費済み、または後続が既に発行されている。
    Superseded,
    /// 準備中に作業文脈が入れ替わった。
    ContextChanged,
    /// 調整のロックが取れなかった（**カーソルは動かしていない**）。
    Busy,
}

impl Verdict {
    /// 拒否なら出す逐語。`Current` は拒否しないので `None`。
    pub(super) const fn refusal(self) -> Option<&'static str> {
        match self {
            Verdict::Current => None,
            Verdict::Superseded => Some(crate::wording::CONTINUATION_TOKEN_SUPERSEDED),
            Verdict::ContextChanged => Some(crate::wording::CONTINUATION_CONTEXT_CHANGED),
            Verdict::Busy => Some(crate::wording::CONTINUATION_COORDINATION_BUSY),
        }
    }
}

/// 入口で採る作業文脈の写し。publish 直前の実測とこれを比べて drift を見る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Snapshot {
    project_sha256: String,
    intent_uuid: Option<String>,
    state_sha256: Option<String>,
    harness: Option<String>,
}

impl Snapshot {
    /// いまのワークスペースから文脈を採る。
    pub(super) fn capture(layout: &Layout) -> Snapshot {
        Snapshot {
            project_sha256: project_sha256(layout),
            intent_uuid: intent_uuid(layout),
            state_sha256: state_sha256(layout),
            harness: Some(CURSOR_HARNESS.to_string()),
        }
    }

    /// 状態が「在る」ことを marker の `state_present` と同じ意味で答える。
    const fn state_present(&self) -> bool {
        self.state_sha256.is_some()
    }
}

/// 正準化したプロジェクト路の SHA-256（publish と同じ計算）。
fn project_sha256(layout: &Layout) -> String {
    let project = std::fs::canonicalize(layout.project_dir())
        .unwrap_or_else(|_| layout.project_dir().to_path_buf());
    core_infrastructure::hash::sha256_hex(project.to_string_lossy().as_bytes())
}

/// 実行カーソルが指す intent（marker の `intent_uuid` と同じ値）。
fn intent_uuid(layout: &Layout) -> Option<String> {
    let record = layout.record_dir()?;
    let cursor = crate::execution_cursor::ExecutionCursor::read(record).ok()??;
    Some(cursor.intent_id().as_str().to_string())
}

/// 状態本文の SHA-256（無ければ `None` = `state_present: false`）。
fn state_sha256(layout: &Layout) -> Option<String> {
    let path = layout.state_file()?;
    let body = std::fs::read(path).ok()?;
    Some(core_infrastructure::hash::sha256_hex(&body))
}

/// marker から照合に要る面だけを取り出したもの。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Marker {
    version: u64,
    kind: String,
    project_sha256: String,
    intent_uuid: String,
    state_present: bool,
    state_sha256: String,
    continue_token_sha256: String,
}

impl Marker {
    /// 公開互換 marker の JSON を読む。形が違えば `None`（= 復旧継続を許す）。
    fn parse(body: &str) -> Option<Marker> {
        let JsonValue::Object(fields) = parse(body).ok()? else {
            return None;
        };
        Some(Marker {
            version: match fields.get("version") {
                Some(&JsonValue::Number(core_infrastructure::canon_json::Number::PosInt(n))) => n,
                _ => 0,
            },
            kind: text(&fields, "kind").to_string(),
            project_sha256: text(&fields, "project_sha256").to_string(),
            intent_uuid: text(&fields, "intent_uuid").to_string(),
            state_present: matches!(fields.get("state_present"), Some(&JsonValue::Bool(true))),
            state_sha256: text(&fields, "state_sha256").to_string(),
            continue_token_sha256: text(&fields, "continue_token_sha256").to_string(),
        })
    }

    /// 本家 `exactContext` — marker が**いまの文脈そのもの**を指しているか。
    fn is_exact_context(&self, live: &Snapshot) -> bool {
        self.version == 2
            && self.project_sha256 == live.project_sha256
            && Some(self.intent_uuid.as_str()) == live.intent_uuid.as_deref()
            && self.state_present == live.state_present()
            && Some(self.state_sha256.as_str()) == live.state_sha256.as_deref()
    }
}

fn text<'a>(fields: &'a ObjectMembers, key: &str) -> &'a str {
    match fields.get(key) {
        Some(JsonValue::String(value)) => core_infrastructure::ecmascript::trim(value),
        _ => "",
    }
}

/// 入口の写しと publish 直前の実測を突き合わせて結論を出す（純粋 — I/O なし）。
///
/// 本家の順序をそのまま保つ。
/// 1. 文脈が動いていれば `drift`。
/// 2. marker がいまの文脈そのもので、かつ提示トークンが現行でなければ `superseded`。
/// 3. それ以外は現行（marker 不在・別文脈は 1 度だけの復旧継続として通す）。
pub(super) fn verdict(
    snapshot: &Snapshot,
    live: &Snapshot,
    marker: Option<&Marker>,
    token_sha256: &str,
) -> Verdict {
    if snapshot != live {
        return Verdict::ContextChanged;
    }
    let Some(marker) = marker else {
        return Verdict::Current;
    };
    if marker.is_exact_context(live)
        && (marker.kind != "load-steering" || marker.continue_token_sha256 != token_sha256)
    {
        return Verdict::Superseded;
    }
    Verdict::Current
}

/// ワークスペースを実測して結論を出す。
///
/// ロックは本家の active-directive トランザクションに対応する。取れなければ
/// [`Verdict::Busy`] を返し、**呼出側は publish を行わない** — 逐語が
/// 「この呼び出しはカーソルを動かしていない」と主張しているからである。
pub(super) fn inspect(layout: &Layout, snapshot: &Snapshot, token: &str) -> Verdict {
    let Some(record) = layout.record_dir() else {
        return Verdict::Current;
    };
    let live = Snapshot::capture(layout);
    if live.harness.as_deref() != Some(CURSOR_HARNESS) {
        return Verdict::ContextChanged;
    }
    // ロックの土台が開けない = 調整できない。カーソルは動かさない。
    let Ok(lock) = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(layout.aidlc_root().join(".aidlc-runtime.lock"))
    else {
        return Verdict::Busy;
    };
    let Ok(_guard) =
        core_infrastructure::ExclusiveFileLock::acquire(lock, std::time::Duration::from_secs(1))
    else {
        return Verdict::Busy;
    };
    let marker = read_marker(&record.join(".aidlc-active-directive.json"));
    let token_sha256 = core_infrastructure::hash::sha256_hex(token.as_bytes());
    verdict(snapshot, &live, marker.as_ref(), &token_sha256)
}

/// marker を上限付きで読む。無ければ・読めなければ・形が違えば `None`。
fn read_marker(path: &std::path::Path) -> Option<Marker> {
    use std::io::Read as _;
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(MARKER_MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MARKER_MAX_BYTES {
        return None;
    }
    Marker::parse(&String::from_utf8_lossy(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        Snapshot {
            project_sha256: "p".to_string(),
            intent_uuid: Some("i".to_string()),
            state_sha256: Some("s".to_string()),
            harness: Some(CURSOR_HARNESS.to_string()),
        }
    }

    fn marker(kind: &str, token_sha256: &str) -> Marker {
        Marker {
            version: 2,
            kind: kind.to_string(),
            project_sha256: "p".to_string(),
            intent_uuid: "i".to_string(),
            state_present: true,
            state_sha256: "s".to_string(),
            continue_token_sha256: token_sha256.to_string(),
        }
    }

    /// 現行トークンは通る。
    #[test]
    fn the_current_token_is_current() {
        let live = snapshot();
        let marker = marker("load-steering", "t");
        assert_eq!(
            verdict(&snapshot(), &live, Some(&marker), "t"),
            Verdict::Current
        );
    }

    /// 別のトークンが現行になっていれば、前のトークンは superseded。
    #[test]
    fn a_token_that_is_no_longer_current_is_superseded() {
        let live = snapshot();
        let marker = marker("load-steering", "newer");
        assert_eq!(
            verdict(&snapshot(), &live, Some(&marker), "older"),
            Verdict::Superseded
        );
    }

    /// 後続が run-stage で着地していれば、継続はもう現行ではない。
    #[test]
    fn a_settled_run_stage_supersedes_the_token() {
        let live = snapshot();
        let marker = marker("run-stage", "t");
        assert_eq!(
            verdict(&snapshot(), &live, Some(&marker), "t"),
            Verdict::Superseded
        );
    }

    /// 準備中に状態が動けば drift。
    #[test]
    fn a_state_that_moved_while_preparing_is_context_changed() {
        let mut live = snapshot();
        live.state_sha256 = Some("moved".to_string());
        let marker = marker("load-steering", "t");
        assert_eq!(
            verdict(&snapshot(), &live, Some(&marker), "t"),
            Verdict::ContextChanged
        );
    }

    /// 準備中に intent が入れ替われば drift。
    #[test]
    fn an_intent_that_moved_while_preparing_is_context_changed() {
        let mut live = snapshot();
        live.intent_uuid = Some("other".to_string());
        let marker = marker("load-steering", "t");
        assert_eq!(
            verdict(&snapshot(), &live, Some(&marker), "t"),
            Verdict::ContextChanged
        );
    }

    /// 準備中に状態が消えれば drift。
    #[test]
    fn a_state_that_disappeared_while_preparing_is_context_changed() {
        let mut live = snapshot();
        live.state_sha256 = None;
        let marker = marker("load-steering", "t");
        assert_eq!(
            verdict(&snapshot(), &live, Some(&marker), "t"),
            Verdict::ContextChanged
        );
    }

    /// marker が無ければ 1 度だけの復旧継続を許す（bootstrap — 本家と同じ）。
    #[test]
    fn a_missing_marker_permits_one_recovery_continuation() {
        let live = snapshot();
        assert_eq!(verdict(&snapshot(), &live, None, "t"), Verdict::Current);
    }

    /// marker が別文脈を指しているだけなら superseded ではない（bootstrap 扱い）。
    #[test]
    fn a_marker_from_another_context_is_not_superseded() {
        let live = snapshot();
        let mut other = marker("load-steering", "newer");
        other.intent_uuid = "another".to_string();
        assert_eq!(
            verdict(&snapshot(), &live, Some(&other), "older"),
            Verdict::Current
        );
    }

    /// v1 marker は照合対象にならない（本家は version 2 だけを exactContext とする）。
    #[test]
    fn a_v1_marker_is_not_an_exact_context() {
        let live = snapshot();
        let mut legacy = marker("load-steering", "newer");
        legacy.version = 1;
        assert_eq!(
            verdict(&snapshot(), &live, Some(&legacy), "older"),
            Verdict::Current
        );
    }

    /// 拒否の逐語は本家とバイト一致である。
    #[test]
    fn every_refusal_carries_its_upstream_wording() {
        assert_eq!(Verdict::Current.refusal(), None);
        assert_eq!(
            Verdict::Superseded.refusal(),
            Some(
                "This continuation token is no longer current for this workflow. Run a fresh `next`; do not reuse an earlier token."
            )
        );
        assert_eq!(
            Verdict::ContextChanged.refusal(),
            Some(
                "The active workflow context changed while this continuation was prepared. Run a fresh `next`; do not use the prepared result."
            )
        );
        assert_eq!(
            Verdict::Busy.refusal(),
            Some(
                "Continuation coordination is busy. This call did not commit a cursor change. Retry the current token; if it is reported superseded, run a fresh `next`."
            )
        );
    }

    /// 壊れた marker は照合対象にならない。
    #[test]
    fn a_malformed_marker_is_not_parsed() {
        assert_eq!(Marker::parse("not json"), None);
        assert_eq!(Marker::parse("[]"), None);
    }

    /// 版が数値でない・欄が文字列でない marker は、その欄を空（版 0）として読む。
    #[test]
    fn non_numeric_versions_and_non_text_fields_read_as_empty() {
        let marker = Marker::parse(
            r#"{"version":"2","kind":7,"project_sha256":"p","intent_uuid":"i","state_present":"yes","state_sha256":"s","continue_token_sha256":"t"}"#,
        )
        .expect("object は読める");
        assert_eq!(marker.version, 0);
        assert_eq!(marker.kind, "");
        assert!(!marker.state_present);
        assert_eq!(marker.project_sha256, "p");
    }

    /// 上限を超える marker は読まず、記録が無ければ照合は常に「現行」である。
    #[test]
    fn oversized_markers_are_ignored_and_a_recordless_layout_is_current() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let path = root.path().join(".aidlc-active-directive.json");
        std::fs::write(&path, "x".repeat((MARKER_MAX_BYTES + 1) as usize)).expect("大きな marker");
        assert_eq!(read_marker(&path), None);
        assert_eq!(read_marker(&root.path().join("absent.json")), None);
        let layout = Layout::resolve(root.path());
        assert_eq!(inspect(&layout, &snapshot(), "token"), Verdict::Current);
    }

    /// ロックの土台が開けなければ Busy（カーソルは動かさない）。
    #[test]
    fn an_unopenable_lock_file_is_busy() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let intents = root.path().join("aidlc/spaces/default/intents");
        let record = intents.join("260101-lock-aaaaaaaa");
        std::fs::create_dir_all(&record).expect("記録");
        std::fs::write(record.join("aidlc-state.md"), "# state\n").expect("状態");
        std::fs::write(intents.join("active-intent"), "260101-lock-aaaaaaaa\n").expect("カーソル");
        std::fs::create_dir_all(root.path().join("aidlc/.aidlc-runtime.lock")).expect("塞ぐ");
        let layout = Layout::resolve(root.path());
        assert!(layout.record_dir().is_some());
        assert_eq!(
            inspect(&layout, &Snapshot::capture(&layout), "token"),
            Verdict::Busy
        );
    }
}
