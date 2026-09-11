//! `MemoryEntry` — ステージ日誌の 1 行が運ぶ記録。

use super::memory_entry_heading::MemoryEntryHeading;

/// 日誌 `memory.md` の**数えられた 1 行**を、見出し・時刻・要約・文脈へ割ったもの。
///
/// 正典の綴りは `- <ISO> — <要約>; <文脈>` である
/// (`aidlc-common/protocols/stage-protocol.md` §13 の 1)。綴りから外れた行は
/// **前の行へ併合せず**、要約だけを持つ退化した記録として 1 件に数える — 件数の不変条件
/// (`MemoryEntries::parse(raw).len() == MemoryJournal::parse(raw).total()`) を保つためである
/// (固定本家 2.7.1 `a277af21` `aidlc-lib.ts:21481-21491` の逐語注記)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEntry {
    heading: MemoryEntryHeading,
    timestamp: String,
    summary: String,
    context: String,
    line: String,
}

impl MemoryEntry {
    /// 割り済みの材料を束ねる完全コンストラクタ。
    #[must_use]
    pub fn new(
        heading: MemoryEntryHeading,
        timestamp: impl Into<String>,
        summary: impl Into<String>,
        context: impl Into<String>,
        line: impl Into<String>,
    ) -> MemoryEntry {
        MemoryEntry {
            heading,
            timestamp: timestamp.into(),
            summary: summary.into(),
            context: context.into(),
            line: line.into(),
        }
    }

    /// 前後の空白を落とした 1 行を割る (**この型の読取構築口**)。
    ///
    /// 綴りから外れた行は投げずに退化させる — 日誌は人が書く面であり、綴り違反 1 行で
    /// ゲートを落とすのは本家も採らない扱いである。
    #[must_use]
    pub fn parse(heading: MemoryEntryHeading, trimmed: &str) -> MemoryEntry {
        let body = strip_bullet(trimmed);
        let Some((timestamp, rest)) = split_timestamp(body) else {
            return MemoryEntry::new(heading, "", body, "", trimmed);
        };
        match rest.find(CONTEXT_SEPARATOR) {
            None => MemoryEntry::new(heading, timestamp, rest.trim(), "", trimmed),
            Some(at) => MemoryEntry::new(
                heading,
                timestamp,
                rest.get(..at).unwrap_or_default().trim(),
                rest.get(at.saturating_add(1)..).unwrap_or_default().trim(),
                trimmed,
            ),
        }
    }

    /// 記録が属する見出し。
    #[must_use]
    pub const fn heading(&self) -> MemoryEntryHeading {
        self.heading
    }

    /// 行頭の ISO 8601 時刻 (綴りから外れた行では空)。
    #[must_use]
    pub fn timestamp(&self) -> &str {
        &self.timestamp
    }

    /// 1 行要約。
    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// `;` より後ろの文脈 (無ければ空)。
    #[must_use]
    pub fn context(&self) -> &str {
        &self.context
    }

    /// 数えた行そのもの (前後の空白を落とした綴り)。
    #[must_use]
    pub fn line(&self) -> &str {
        &self.line
    }

    /// 学びの候補へ昇格しない記録か。
    #[must_use]
    pub const fn is_parked(&self) -> bool {
        self.heading.is_parked()
    }
}

/// 要約と文脈を分ける区切り。
const CONTEXT_SEPARATOR: char = ';';
/// 時刻と要約を分ける区切り (全角ダッシュ)。
const SUMMARY_SEPARATOR: char = '\u{2014}';

/// 行頭の箇条書き記号を落とす (`^[-*]\s+`)。記号の後に空白が無ければ箇条書きではない。
fn strip_bullet(trimmed: &str) -> &str {
    let Some(rest) = trimmed
        .strip_prefix('-')
        .or_else(|| trimmed.strip_prefix('*'))
    else {
        return trimmed;
    };
    let stripped = rest.trim_start_matches(char::is_whitespace);
    if stripped.len() == rest.len() {
        trimmed
    } else {
        stripped
    }
}

/// `^(\S+)\s+\u{2014}\s+(.*)$` — 先頭の 1 語と、ダッシュより後ろ。
///
/// `\S+` は空白で必ず止まるので、先頭の 1 語がそのまま時刻の候補になる。
fn split_timestamp(body: &str) -> Option<(&str, &str)> {
    let token_end = body.find(char::is_whitespace)?;
    let token = body.get(..token_end)?;
    if token.is_empty() {
        return None;
    }
    let after = body.get(token_end..)?;
    let dashed = after.trim_start_matches(char::is_whitespace);
    if dashed.len() == after.len() {
        return None;
    }
    let tail = dashed.strip_prefix(SUMMARY_SEPARATOR)?;
    let rest = tail.trim_start_matches(char::is_whitespace);
    if rest.len() == tail.len() {
        return None;
    }
    Some((token, rest))
}

#[cfg(test)]
mod tests {
    use super::{MemoryEntry, MemoryEntryHeading};

    #[test]
    fn the_canonical_bullet_splits_into_timestamp_summary_and_context() {
        let entry = MemoryEntry::parse(
            MemoryEntryHeading::Interpretations,
            "- 2026-05-20T10:14:32Z — REST を選んだ; 消費側は CRUD しか要らない",
        );
        assert_eq!(entry.heading(), MemoryEntryHeading::Interpretations);
        assert_eq!(entry.timestamp(), "2026-05-20T10:14:32Z");
        assert_eq!(entry.summary(), "REST を選んだ");
        assert_eq!(entry.context(), "消費側は CRUD しか要らない");
        assert_eq!(
            entry.line(),
            "- 2026-05-20T10:14:32Z — REST を選んだ; 消費側は CRUD しか要らない"
        );
    }

    #[test]
    fn a_missing_semicolon_leaves_the_context_empty() {
        let entry = MemoryEntry::parse(
            MemoryEntryHeading::Deviations,
            "* 2026-05-20T10:14:32Z — キャッシュ層を省いた",
        );
        assert_eq!(entry.timestamp(), "2026-05-20T10:14:32Z");
        assert_eq!(entry.summary(), "キャッシュ層を省いた");
        assert_eq!(entry.context(), "");
    }

    #[test]
    fn a_line_without_the_em_dash_degrades_into_a_summary_only_entry() {
        let entry = MemoryEntry::parse(MemoryEntryHeading::Tradeoffs, "- 綴りから外れた行");
        assert_eq!(entry.timestamp(), "");
        assert_eq!(entry.summary(), "綴りから外れた行");
        assert_eq!(entry.context(), "");
        assert_eq!(entry.line(), "- 綴りから外れた行");
    }

    #[test]
    fn open_questions_are_parked() {
        assert!(MemoryEntry::parse(MemoryEntryHeading::OpenQuestions, "- 確認").is_parked());
        assert!(!MemoryEntry::parse(MemoryEntryHeading::Tradeoffs, "- 選択").is_parked());
    }
}
