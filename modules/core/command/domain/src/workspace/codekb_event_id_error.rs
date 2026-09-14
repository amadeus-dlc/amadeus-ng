//! `CodekbEventIdError` — [`CodekbEventId::parse`](super::CodekbEventId::parse) の拒否理由。

/// codekb のドメインイベント識別子として受理できなかった理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodekbEventIdError {
    /// UUIDv7 の正準表記ではない。
    NotCanonicalUuidV7,
}

impl std::fmt::Display for CodekbEventIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("not a canonical UUIDv7 codekb event id")
    }
}

impl std::error::Error for CodekbEventIdError {}
