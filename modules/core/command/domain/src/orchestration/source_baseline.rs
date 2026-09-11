//! ワークフロー開始時に確定したソースの比較基準。

use core_infrastructure::hash::sha256_hex;

/// 採取済みのソース一覧、または採取不能という事実。
/// 一覧の形式は公開契約のTSVであり、保存媒体への依存ではない。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceBaseline {
    listing: Option<String>,
}

impl SourceBaseline {
    /// 公開一覧の構文を検査して比較基準を組む。`None` は採取不能。
    ///
    /// # Errors
    /// 一覧の行・エスケープ・順序・重複が正規形でない場合。
    pub fn new(listing: Option<String>) -> Result<Self, super::SourceBaselineError> {
        if let Some(text) = &listing {
            validate(text)?;
        }
        Ok(Self { listing })
    }

    /// 監査へ描く比較基準。ソース指紋とは異なり一覧自体のSHA-256を使う。
    #[must_use]
    pub fn fingerprint(&self) -> String {
        self.listing.as_ref().map_or_else(
            || "unbindable".to_string(),
            |text| format!("sha256:{}", sha256_hex(text.as_bytes())),
        )
    }

    /// 保存・投影境界へ渡す、採取時の一覧の公開バイト。
    #[must_use]
    pub fn listing(&self) -> Option<&str> {
        self.listing.as_deref()
    }

    /// 公開スナップショットの名前。同じ一覧には同じアドレスを割り当てる。
    #[must_use]
    pub fn snapshot_name(&self) -> Option<String> {
        self.listing.as_ref().map(|text| {
            let hash = sha256_hex(text.as_bytes());
            format!("baseline-{}.tsv", hash.chars().take(12).collect::<String>())
        })
    }
}

fn validate(text: &str) -> Result<(), super::SourceBaselineError> {
    use super::SourceBaselineError as Error;
    if text.is_empty() {
        return Ok(());
    }
    if !text.ends_with('\n') {
        return Err(Error::MalformedListing);
    }
    let mut previous: Option<String> = None;
    for row in text
        .strip_suffix('\n')
        .ok_or(Error::MalformedListing)?
        .split('\n')
    {
        let fields: Vec<_> = row.split('\t').collect();
        let [repo, path, mode, hash] = fields.as_slice() else {
            return Err(Error::MalformedListing);
        };
        let repo = decode(repo)?;
        let path = decode(path)?;
        if path.is_empty()
            || !repo
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
            || mode.len() != 6
            || !mode.bytes().all(|c| c.is_ascii_digit())
            || !matches!(hash.len(), 40 | 64)
            || !hash
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(Error::MalformedListing);
        }
        let key = format!("{repo}\0{path}");
        if previous
            .as_ref()
            .is_some_and(|old| old.encode_utf16().cmp(key.encode_utf16()).is_ge())
        {
            return Err(Error::MalformedListing);
        }
        previous = Some(key);
    }
    Ok(())
}

fn decode(field: &str) -> Result<String, super::SourceBaselineError> {
    use super::SourceBaselineError as Error;
    let mut decoded = String::new();
    let mut chars = field.chars();
    while let Some(c) = chars.next() {
        decoded.push(match c {
            '\\' => match chars.next() {
                Some('\\') => '\\',
                Some('t') => '\t',
                Some('n') => '\n',
                Some('r') => '\r',
                _ => return Err(Error::MalformedListing),
            },
            '\0' | '\r' => return Err(Error::MalformedListing),
            c => c,
        });
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_public_listing_and_unbindable_fact_round_trip() {
        let corpus: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../tests/golden/selfhost-stage1/source-baseline.json"
        ))
        .unwrap();
        for observation in corpus.get("observations").unwrap().as_array().unwrap() {
            let baseline = SourceBaseline::new(
                observation
                    .get("listing")
                    .unwrap()
                    .as_str()
                    .map(str::to_string),
            )
            .unwrap();
            assert_eq!(
                baseline.listing(),
                observation.get("listing").unwrap().as_str()
            );
            assert_eq!(
                baseline.fingerprint(),
                observation
                    .get("fingerprint")
                    .unwrap()
                    .as_str()
                    .unwrap_or("unbindable")
            );
            assert_eq!(
                baseline.snapshot_name().is_some(),
                observation.get("listing").unwrap().is_string()
            );
        }
    }
    #[test]
    fn malformed_lists_are_refused_at_construction() {
        let digest = "a".repeat(64);
        let row = format!("\tsource\t100644\t{digest}\n");
        for malformed in [
            row.trim_end().to_string(),
            "\n".to_string(),
            format!("\t\t100644\t{digest}\n"),
            format!("bad/repo\tx\t100644\t{digest}\n"),
            format!("\tx\t10064x\t{digest}\n"),
            "\tx\t100644\twrong\n".to_string(),
            format!("\tx\t100644\t{}\n", "A".repeat(64)),
            format!("\tx\\q\t100644\t{digest}\n"),
            format!("\tx\\\t100644\t{digest}\n"),
            format!("\tx\0\t100644\t{digest}\n"),
            format!("\tx\r\t100644\t{digest}\n"),
            row.repeat(2),
            format!("\tz\t100644\t{digest}\n\ta\t100644\t{digest}\n"),
        ] {
            assert_eq!(
                SourceBaseline::new(Some(malformed)),
                Err(super::super::SourceBaselineError::MalformedListing)
            );
        }
        assert_eq!(
            super::super::SourceBaselineError::MalformedListing.to_string(),
            "malformed source listing"
        );
    }
}
