//! reviewerが書く末尾Review節の証拠。
use super::{ReviewEvidenceError, ReviewVerdict};
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
    pub(super) fn validate(
        &self,
        reviewer: &str,
        iteration: u32,
        verdict: ReviewVerdict,
        challenge: Option<&str>,
    ) -> Result<(), ReviewEvidenceError> {
        let invalid = |reason: &str| ReviewEvidenceError::InvalidAppendix(reason.to_string());
        let text = std::str::from_utf8(self.evidence())
            .map_err(|_| invalid("the reviewer appendix is not valid UTF-8"))?;
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let Some((heading, section)) = normalized.split_once('\n') else {
            return Err(invalid(
                "the appended bytes must begin with only blank lines followed by an exact `## Review` heading",
            ));
        };
        if heading.trim_end_matches([' ', '\t']) != "## Review" {
            return Err(invalid(
                "the appended bytes must begin with only blank lines followed by an exact `## Review` heading",
            ));
        }
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
        Ok(())
    }
}
