//! `ReviewFinding` — レビューの `### Findings` 表の 1 行。
use super::ReviewEvidenceError;
use core_infrastructure::ecmascript::trim;

/// 所見表の 1 行（upstream 2.8.2 `ReviewFinding` のうち表から読む 6 欄）。
///
/// 表の行からしか作れない。ID は `R-<数字>`、状態は閉じた語彙（`Rejected: <理由>` だけが
/// 自由文を伴う）であることを構築時に確かめる — 形の崩れた行を所見として持ち回らない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewFinding {
    id: String,
    severity: String,
    location: String,
    finding: String,
    required_action: String,
    status: String,
}

impl ReviewFinding {
    /// 表の 1 行を所見へ写す（upstream `parseReviewSection` の行ごとの検査と同じ順）。
    ///
    /// セル数を先に確かめる — 欠けたセルは後ろの値を 1 つずつ左へずらすので、どの列の
    /// つもりだったかを取り戻せないからである。
    pub(super) fn parse_row(
        cells: &[String],
        headers: &[String],
        artifact: &str,
    ) -> Result<ReviewFinding, ReviewEvidenceError> {
        let at = |name: &str| {
            headers
                .iter()
                .position(|header| header == name)
                .and_then(|index| cells.get(index))
                .map_or_else(String::new, |value| trim(value).to_string())
        };
        let invalid = ReviewEvidenceError::InvalidAppendix;
        if cells.len() != headers.len() {
            let named = at("ID");
            let named = if named.is_empty() {
                "?".to_string()
            } else {
                named
            };
            if cells.len() > headers.len() {
                return Err(invalid(extra_cells(
                    artifact,
                    &named,
                    cells.len(),
                    headers.len(),
                )));
            }
            let last = cells.last().map_or("", String::as_str);
            let hint = if headers.last().is_some_and(|name| name == "Status") && is_status(last) {
                status_hint(last)
            } else {
                MISSING_HINT.to_string()
            };
            return Err(invalid(missing_cells(
                artifact,
                &named,
                cells.len(),
                headers,
                &hint,
            )));
        }
        let id = at("ID");
        if !is_id(&id) {
            return Err(invalid(invalid_id(artifact, &id)));
        }
        let status = at("Status");
        if !is_status(&status) {
            return Err(invalid(invalid_status(artifact, &id, &status)));
        }
        Ok(ReviewFinding {
            severity: at("Severity"),
            location: at("Location"),
            finding: at("Finding"),
            required_action: at("Required action"),
            id,
            status,
        })
    }

    /// 所見 ID（`R-01` など）。記録の直列化とゲート文脈の描画へ写すための境界の変換。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 重大度の欄。記録の直列化とゲート文脈の描画へ写すための境界の変換。
    #[must_use]
    pub fn severity(&self) -> &str {
        &self.severity
    }

    /// 場所の欄。記録の直列化とゲート文脈の描画へ写すための境界の変換。
    #[must_use]
    pub fn location(&self) -> &str {
        &self.location
    }

    /// 所見の本文の欄。記録の直列化とゲート文脈の描画へ写すための境界の変換。
    #[must_use]
    pub fn finding(&self) -> &str {
        &self.finding
    }

    /// 求める対応の欄。記録の直列化とゲート文脈の描画へ写すための境界の変換。
    #[must_use]
    pub fn required_action(&self) -> &str {
        &self.required_action
    }

    /// 状態の欄。記録の直列化とゲート文脈の描画へ写すための境界の変換。
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }
}

/// `R-` と 1 桁以上の数字だけ（upstream `/^R-[0-9]+$/`）。
fn is_id(value: &str) -> bool {
    value.strip_prefix("R-").is_some_and(|digits| {
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    })
}

/// 閉じた状態語彙（upstream `validReviewFindingStatus`）。
fn is_status(value: &str) -> bool {
    matches!(value, "New" | "Unresolved" | "Resolved" | "Accepted risk")
        || value
            .strip_prefix("Rejected: ")
            .is_some_and(|reason| reason.starts_with(|char: char| !char.is_whitespace()))
}

/// セルが足りない行（upstream 逐語、`aidlc-lib.ts:10753-10756`）。
fn missing_cells(artifact: &str, id: &str, cells: usize, headers: &[String], hint: &str) -> String {
    format!(
        "{artifact}#{id}: row has {cells} cells, header declares {}. Expected columns: {}. {hint}",
        headers.len(),
        headers.join(" | ")
    )
}

/// 末尾のセルが状態らしいときの助言（upstream 逐語、同 `:10751`）。
fn status_hint(last: &str) -> String {
    format!(
        "The last cell {last:?} looks like Status; check earlier cells for a missing value or \"|\" separator"
    )
}

/// 欠けた列を言い当てられないときの助言（upstream 逐語、同 `:10752`）。
const MISSING_HINT: &str = "Check for a missing cell or \"|\" separator";

/// セルが多い行（upstream 逐語、同 `:10758-10762`）。
fn extra_cells(artifact: &str, id: &str, cells: usize, declared: usize) -> String {
    format!(
        "{artifact}#{id}: row has {cells} cells, header declares {declared}: {} unexpected extra cell(s)",
        cells.saturating_sub(declared)
    )
}

/// 所見 ID が形を満たさない（upstream 逐語、同 `:10768`）。
fn invalid_id(artifact: &str, id: &str) -> String {
    format!("{artifact}: invalid finding ID {id:?}")
}

/// 所見の状態が語彙の外（upstream 逐語、同 `:10773`）。
fn invalid_status(artifact: &str, id: &str, status: &str) -> String {
    format!("{artifact}#{id}: invalid finding status {status:?}")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    fn headers() -> Vec<String> {
        [
            "ID",
            "Severity",
            "Location",
            "Finding",
            "Required action",
            "Status",
        ]
        .iter()
        .map(|name| (*name).to_string())
        .collect()
    }

    /// 表の行が宣言と食い違ったときの 4 形は、行と成果物を名指す（upstream 逐語）。
    #[test]
    fn a_malformed_findings_row_is_named_by_artifact_and_row() {
        assert_eq!(
            missing_cells("a.md", "R-01", 5, &headers(), MISSING_HINT),
            "a.md#R-01: row has 5 cells, header declares 6. Expected columns: ID | Severity | Location | Finding | Required action | Status. Check for a missing cell or \"|\" separator"
        );
        assert_eq!(
            status_hint("New"),
            "The last cell \"New\" looks like Status; check earlier cells for a missing value or \"|\" separator"
        );
        assert_eq!(
            extra_cells("a.md", "?", 8, 6),
            "a.md#?: row has 8 cells, header declares 6: 2 unexpected extra cell(s)"
        );
        assert_eq!(invalid_id("a.md", "1"), "a.md: invalid finding ID \"1\"");
        assert_eq!(
            invalid_status("a.md", "R-01", "Maybe"),
            "a.md#R-01: invalid finding status \"Maybe\""
        );
    }

    /// 状態語彙は閉じている。`Rejected:` だけが自由文を伴う。
    #[test]
    fn the_status_vocabulary_is_closed_except_for_a_stated_rejection() {
        for value in [
            "New",
            "Unresolved",
            "Resolved",
            "Accepted risk",
            "Rejected: 別案を採る",
        ] {
            assert!(is_status(value), "{value}");
        }
        for value in ["", "Rejected", "Rejected: ", "rejected: x", "Done"] {
            assert!(!is_status(value), "{value}");
        }
        assert!(is_id("R-07") && !is_id("R-") && !is_id("X-1"));
    }

    /// 末尾が状態らしい短い行には、どこが欠けたかの助言が付く。
    #[test]
    fn a_short_row_ending_in_a_status_gets_the_status_hint() {
        let cells: Vec<String> = ["R-01", "Major", "a.md", "欠落", "New"]
            .iter()
            .map(|cell| (*cell).to_string())
            .collect();
        let error = ReviewFinding::parse_row(&cells, &headers(), "a.md").expect_err("短い");
        assert!(error.to_string().ends_with(&status_hint("New")), "{error}");
    }
}
