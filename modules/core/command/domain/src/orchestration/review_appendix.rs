//! reviewerが書く末尾Review節の証拠。
use super::{ReviewEvidenceError, ReviewFindings, ReviewVerdict};
use core_infrastructure::hash::sha256_hex;
/// 生バイトを保ち、表示される証跡だけを照合する。
#[derive(Debug)]
pub(super) struct ReviewAppendix {
    bytes: Vec<u8>,
}
impl ReviewAppendix {
    pub(super) const fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
    pub(super) const fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    pub(super) fn evidence(&self) -> &[u8] {
        let mut offset = 0;
        loop {
            let start = offset;
            while self
                .bytes
                .get(offset)
                .is_some_and(|b| matches!(b, b' ' | b'\t'))
            {
                offset += 1;
            }
            match self.bytes.get(offset) {
                Some(b'\r') => {
                    offset += 1;
                    if self.bytes.get(offset) == Some(&b'\n') {
                        offset += 1;
                    }
                }
                Some(b'\n') => offset += 1,
                Some(_) => return self.bytes.get(start..).unwrap_or_default(),
                None => return self.bytes.get(offset..).unwrap_or_default(),
            }
        }
    }
    pub(super) fn digest(&self) -> String {
        if self.evidence().is_empty() {
            "none".into()
        } else {
            format!("sha256:{}", sha256_hex(self.evidence()))
        }
    }
    /// 末尾の `## Review` 節より前の本文（節が無ければ全文 — 2.8.2
    /// `contentBeforeTerminalReviewAppendix`）。
    pub(super) fn content_before(text: &str) -> &str {
        Self::existing_offset(text.as_bytes())
            .and_then(|offset| text.get(..offset))
            .unwrap_or(text)
    }
    pub(super) fn existing_offset(body: &[u8]) -> Option<usize> {
        let text = std::str::from_utf8(body).ok()?;
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let visible = super::summary_questions::visible_markdown_lines(&normalized);
        let mut candidate = None;
        let mut start = 0;
        for (line, raw) in text.split_inclusive('\n').enumerate() {
            let rendered = visible.get(line).map_or("", String::as_str);
            if rendered.starts_with("# ") || rendered.starts_with("## ") {
                candidate = if rendered.trim_end_matches([' ', '\t', '\r', '\n']) == "## Review"
                    && raw.trim_end_matches([' ', '\t', '\r', '\n']) == "## Review"
                {
                    Some(start)
                } else {
                    None
                };
            }
            start += raw.len();
        }
        let heading = candidate?;
        let prefix = text.get(..heading)?;
        let trimmed = prefix.trim_end();
        let trailing = prefix.get(trimmed.len()..)?;
        let retained = trailing.find(['\r', '\n']).map_or(0, |i| {
            i + if trailing.get(i..).is_some_and(|s| s.starts_with("\r\n")) {
                2
            } else {
                1
            }
        });
        Some(trimmed.len() + retained)
    }
    /// 判定の証拠として受理できるかを確かめる（upstream `validateReviewAppendix` と、
    /// 記録の直前に置かれた所見表の解析 `aidlc-log.ts:2403-2415`）。
    ///
    /// `artifact` は所見表の拒否で成果物を名指す綴り（依頼が固定した追記先）である。
    pub(super) fn validate(
        &self,
        artifact: &str,
        reviewer: &str,
        iteration: u32,
        verdict: ReviewVerdict,
        challenge: Option<&str>,
        standalone: bool,
    ) -> Result<(), ReviewEvidenceError> {
        let invalid = |reason: &str| ReviewEvidenceError::InvalidAppendix(reason.to_string());
        let text = std::str::from_utf8(self.evidence())
            .map_err(|_| invalid("the reviewer appendix is not valid UTF-8"))?;
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        // 単独のレビューファイルは `## Review` 見出しで始めてもよい（テンプレートどおり）が、
        // 必須ではない（upstream `validateReviewAppendix` の `standalone`）。追記は必須である。
        let opened = normalized
            .split_once('\n')
            .filter(|(heading, _)| heading.trim_end_matches([' ', '\t']) == "## Review");
        let section = match opened {
            Some((_, section)) => section,
            None if standalone => normalized.as_str(),
            None => {
                return Err(invalid(
                    "the appended bytes must begin with only blank lines followed by an exact `## Review` heading",
                ));
            }
        };
        let lines = super::summary_questions::visible_markdown_lines(section);
        if lines
            .iter()
            .any(|s| s.trim_start().starts_with("# ") || s.trim_start().starts_with("## "))
        {
            return Err(invalid(
                "the reviewer appendix must be terminal and contain no later rendered H1 or H2 heading",
            ));
        }
        for (label, expected, reason) in [
            (
                "Verdict",
                verdict.as_str().to_string(),
                "the reviewer appendix must contain exactly one canonical verdict line matching --verdict",
            ),
            (
                "Reviewer",
                reviewer.to_string(),
                "the reviewer appendix must contain exactly one Reviewer line matching the requested reviewer",
            ),
            (
                "Iteration",
                iteration.to_string(),
                "the reviewer appendix must contain exactly one Iteration line matching the request",
            ),
        ] {
            let prefix = format!("**{label}:** ");
            let values: Vec<_> = lines
                .iter()
                .filter_map(|s| s.strip_prefix(&prefix).map(str::trim))
                .collect();
            if values.as_slice() != [expected.as_str()] {
                return Err(invalid(reason));
            }
        }
        let values: Vec<_> = lines
            .iter()
            .filter_map(|s| s.strip_prefix("**Request Challenge:** ").map(str::trim))
            .collect();
        match challenge {
            None if !values.is_empty() => {
                return Err(invalid(
                    "the reviewer appendix must omit Request Challenge when the request did not issue one",
                ));
            }
            Some(expected) if values.as_slice() != [expected] => {
                return Err(invalid(
                    "the reviewer appendix must contain exactly one Request Challenge line matching the request",
                ));
            }
            _ => (),
        }
        // 所見表はレビュー記録の `findings` 欄になる。upstream 2.8.2 は記録を書く前に表を
        // 解析し、読めなければ REVIEW_COMPLETED を拒否する（`aidlc-log.ts:2403-2415`）。
        // 受理（イベントの確定）より後で読むと、「判定は確定したが記録は無く、打ち直しは
        // 依頼が無いと断られる」状態が残るので、受理の検査の中で読む。読むのは記録が
        // 本文として持つのと同じバイト（先頭の空行だけを除いた証跡）である。
        ReviewFindings::parse(text, artifact)?;
        Ok(())
    }
}
