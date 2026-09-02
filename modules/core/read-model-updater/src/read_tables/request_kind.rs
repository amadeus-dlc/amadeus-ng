//! `RequestKind` — `next` の**要求の観測**の閉集合 (行のキーになる 4 値)。

use core_command_domain::orchestration::NextRequest;

/// `next` が受け取りうる要求の形。
///
/// 集約のクエリ [`IntentExecution::next_decision`] は要求の観測
/// ([`NextRequest`]) を引数に取るので、答えを行に焼き込むには**要求の側を列挙**しなければ
/// ならない。`NextRequest` は 3 つの真偽値だが、`next` が実際に組む組合せは 4 つだけで
/// ある (排他的な 3 フラグ + 素の要求) ので、行のキーはその 4 値にする。
///
/// 綴りは kebab-case。これは行の値であって upstream の逐語ではない (要求の綴りは CLI の
/// 引数が持つ)。
///
/// [`IntentExecution::next_decision`]: core_command_domain::orchestration::IntentExecution::next_decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RequestKind {
    /// 素の `next` (フラグなし)。
    Bare,
    /// `--resume` — 再開メニューを求める。
    Resume,
    /// 自由記述が添えられている — 新規作業のルーティングへ。
    FreeText,
    /// 再入 (park 中でも park 分岐を発火させない読み)。
    Reentry,
}

impl RequestKind {
    /// 行のキーになる 4 値 (この順で行を並べる)。
    pub const ALL: [RequestKind; 4] = [
        RequestKind::Bare,
        RequestKind::Resume,
        RequestKind::FreeText,
        RequestKind::Reentry,
    ];

    /// 行に書く綴り (kebab-case)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            RequestKind::Bare => "bare",
            RequestKind::Resume => "resume",
            RequestKind::FreeText => "free-text",
            RequestKind::Reentry => "reentry",
        }
    }

    /// 集約のクエリに渡す要求の観測へ写す。
    #[must_use]
    pub const fn to_request(self) -> NextRequest {
        match self {
            RequestKind::Bare => NextRequest::new(false, false, false),
            RequestKind::Resume => NextRequest::new(true, false, false),
            RequestKind::FreeText => NextRequest::new(false, false, true),
            RequestKind::Reentry => NextRequest::new(false, true, false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_kinds_spell_themselves_in_kebab_case() {
        let spelled: Vec<&str> = RequestKind::ALL.iter().map(|kind| kind.as_str()).collect();
        assert_eq!(spelled, ["bare", "resume", "free-text", "reentry"]);
    }

    #[test]
    fn each_kind_maps_to_the_flag_triple_that_next_would_observe() {
        let request = RequestKind::Bare.to_request();
        assert!(!request.is_resume() && !request.is_reentry() && !request.is_free_text());

        let resume = RequestKind::Resume.to_request();
        assert!(resume.is_resume() && !resume.is_reentry() && !resume.is_free_text());

        let free_text = RequestKind::FreeText.to_request();
        assert!(!free_text.is_resume() && !free_text.is_reentry() && free_text.is_free_text());

        let reentry = RequestKind::Reentry.to_request();
        assert!(!reentry.is_resume() && reentry.is_reentry() && !reentry.is_free_text());
    }

    #[test]
    fn the_four_requests_are_pairwise_distinct() {
        let requests: Vec<NextRequest> = RequestKind::ALL
            .iter()
            .map(|kind| kind.to_request())
            .collect();
        for (left, kind) in requests.iter().zip(RequestKind::ALL) {
            let matching = requests
                .iter()
                .zip(RequestKind::ALL)
                .filter(|(right, _)| {
                    right.is_resume() == left.is_resume()
                        && right.is_reentry() == left.is_reentry()
                        && right.is_free_text() == left.is_free_text()
                })
                .count();
            assert_eq!(matching, 1, "{} が他の kind と重なる", kind.as_str());
        }
    }
}
