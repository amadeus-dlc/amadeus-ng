//! 読み取り範囲の照合が使う経路の読み方 (crate 内部・公開型なし)。
//!
//! upstream `aidlc-reviewer-scope.ts` の `normalizedComps` / `fold` /
//! `globComponentMatchesConstruction` / `canonicalSuffix` / `normalizeForCompare` /
//! `resolvePathStrings` / `containsPath` に対応する。
//!
//! **区切りは `/` だけを見る。** upstream の `toPosix` は POSIX ホストでは恒等写像なので、
//! `\` は経路区切りではなく普通の文字である ([`super::ScopeToken`] の doc を参照)。

/// 経路にワイルドカードとして現れる 5 文字 (upstream `WILDCARD_RE`)。
const WILDCARD: [char; 6] = ['*', '?', '[', ']', '{', '}'];

/// グロブから落としても意味が残らない文字 (upstream の `c.replace(/[*?[\]{}!,]/g, "")`)。
const GLOB_ONLY: [char; 8] = ['*', '?', '[', ']', '{', '}', '!', ','];

/// Construction の作業単位を束ねるディレクトリ名。
const CONSTRUCTION: &str = "construction";

/// 大小を畳む (upstream `toLowerCase()`)。
pub(super) fn fold(text: &str) -> String {
    text.to_lowercase()
}

/// ワイルドカードを含むか。
pub(super) fn has_wildcard(text: &str) -> bool {
    text.chars().any(|character| WILDCARD.contains(&character))
}

/// 経路を成分へ分け、空と `.` を落として `..` を親へ畳む。
///
/// 畳まないと `construction/u3/../u1/design.md` が「最初の成分 u3」で判定され、実際には
/// 兄弟 u1 へ解決するのに通ってしまう。消す親が無い先頭の `..` はそのまま残す。
pub(super) fn normalized_comps(path: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for component in path.split('/') {
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".." && out.last().is_some_and(|last| last != "..") {
            out.pop();
            continue;
        }
        out.push(component.to_string());
    }
    out
}

/// `*` と `?` だけを特別扱いする照合 (upstream は同じ規則を正規表現へ組み立てる)。
fn glob_like_matches(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let (mut p, mut t) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut mark = 0usize;
    while t < text.len() {
        let head = pattern.get(p).copied();
        if head == Some('?') || (head.is_some() && head == text.get(t).copied()) {
            p += 1;
            t += 1;
        } else if head == Some('*') {
            star = Some(p);
            p += 1;
            mark = t;
        } else if let Some(position) = star {
            p = position + 1;
            mark += 1;
            t = mark;
        } else {
            return false;
        }
    }
    while pattern.get(p) == Some(&'*') {
        p += 1;
    }
    p == pattern.len()
}

/// この成分が `construction` を名指すか (グロブを許すかは呼出側が決める)。
pub(super) fn is_construction_component(component: &str, allow_glob: bool) -> bool {
    let folded = fold(component);
    if folded == CONSTRUCTION {
        return true;
    }
    if !allow_glob || !has_wildcard(&folded) {
        return false;
    }
    // グロブ記号しか無い成分 (`*` など) は「construction を名指した」とは数えない。
    if !folded
        .chars()
        .any(|character| !GLOB_ONLY.contains(&character))
    {
        return false;
    }
    glob_like_matches(&folded, CONSTRUCTION)
}

/// 成分列を照合用の 1 本の綴りへ畳む。
pub(super) fn canonical_suffix(comps: &[String]) -> String {
    comps
        .iter()
        .map(|component| fold(component))
        .collect::<Vec<String>>()
        .join("/")
}

/// 許可経路の `construction/` 以降。`construction/` を通らない項目は `None`。
pub(super) fn exempt_suffix_of(entry: &str) -> Option<String> {
    let comps = normalized_comps(entry);
    let index = comps
        .iter()
        .position(|component| is_construction_component(component, false))?;
    comps.get(index..).map(canonical_suffix)
}

/// 絶対・相対を保ったまま成分を畳み、大小を落とした照合用の綴り。
pub(super) fn normalize_for_compare(path: &str) -> String {
    let absolute = path.starts_with('/');
    let body = canonical_suffix(&normalized_comps(path));
    if absolute { format!("/{body}") } else { body }
}

/// 基点に対して 1 つ解決する (Node `path.resolve` の字句部分)。
///
/// **相対の基点はプロセスの作業ディレクトリで補わない** — フックが受け取る基点は常に
/// 絶対であり、補うと試験の入力によって結果が変わる。絶対経路では根の上へ登らない。
pub(super) fn resolve_lexical(base: &str, text: &str) -> String {
    let joined = if text.starts_with('/') || base.is_empty() {
        text.to_string()
    } else {
        format!("{base}/{text}")
    };
    let absolute = joined.starts_with('/');
    let mut out: Vec<&str> = Vec::new();
    for component in joined.split('/') {
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".." {
            if out.last().is_some_and(|last| *last != "..") {
                out.pop();
                continue;
            }
            if absolute {
                continue;
            }
        }
        out.push(component);
    }
    let body = out.join("/");
    if absolute { format!("/{body}") } else { body }
}

/// 空を落として出現順に重複を除く (upstream `uniqueStrings`)。
pub(super) fn unique_strings(values: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for value in values {
        if !value.is_empty() && !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

/// 綴りを基点ごとに解決する。絶対経路は基点を見ない。
pub(super) fn resolve_path_strings(text: &str, bases: &[String]) -> Vec<String> {
    if text.starts_with('/') {
        return vec![resolve_lexical("", text)];
    }
    bases
        .iter()
        .map(|base| resolve_lexical(base, text))
        .collect()
}

/// 解決したうえで照合用に畳んだ綴りの列。
pub(super) fn normalize_resolved(text: &str, bases: &[String]) -> Vec<String> {
    unique_strings(
        resolve_path_strings(text, bases)
            .iter()
            .map(|path| normalize_for_compare(path))
            .collect(),
    )
}

/// `candidate` が `root` 自身か、その下にあるか。
pub(super) fn contains_path(root: &str, candidate: &str) -> bool {
    candidate == root || candidate.starts_with(&format!("{root}/"))
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_suffix, contains_path, exempt_suffix_of, has_wildcard, is_construction_component,
        normalize_for_compare, normalized_comps, resolve_lexical, unique_strings,
    };

    #[test]
    fn a_parent_step_is_folded_against_the_component_before_it() {
        assert_eq!(
            normalized_comps("construction/u3/../u1/a.md"),
            ["construction", "u1", "a.md"]
        );
        assert_eq!(normalized_comps("./a//b/."), ["a", "b"]);
    }

    #[test]
    fn a_leading_parent_step_has_nothing_to_consume_and_stays() {
        assert_eq!(
            normalized_comps("../construction/u2/a.md"),
            ["..", "construction", "u2", "a.md"]
        );
        assert_eq!(normalized_comps("../.."), ["..", ".."]);
    }

    #[test]
    fn a_glob_component_names_construction_only_when_it_can_spell_it() {
        assert!(is_construction_component("construction", false));
        assert!(is_construction_component("CONSTRUCTION", false));
        assert!(is_construction_component("constr*", true));
        assert!(is_construction_component("c*n", true));
        assert!(
            !is_construction_component("constr*", false),
            "許さなければ見ない"
        );
        assert!(
            !is_construction_component("*", true),
            "グロブ記号だけの成分は construction を名指していない"
        );
        assert!(!is_construction_component("inception", true));
        assert!(!is_construction_component("constructionx", true));
    }

    #[test]
    fn the_wildcard_set_is_the_five_upstream_characters() {
        for spelling in ["a*", "a?", "a[", "a]", "a{", "a}"] {
            assert!(has_wildcard(spelling), "{spelling}");
        }
        assert!(!has_wildcard("a-b_c.d"));
    }

    #[test]
    fn an_exempt_entry_outside_construction_never_constrains_the_matcher() {
        assert_eq!(
            exempt_suffix_of("/r/construction/u3/contract.md"),
            Some("construction/u3/contract.md".to_string())
        );
        assert_eq!(exempt_suffix_of("/r/inception/requirements.md"), None);
    }

    #[test]
    fn comparison_keeps_the_leading_slash_and_drops_the_case() {
        assert_eq!(
            normalize_for_compare("/R/Construction/U1/"),
            "/r/construction/u1"
        );
        assert_eq!(normalize_for_compare("R/Construction"), "r/construction");
        assert_eq!(normalize_for_compare("/"), "/");
        assert_eq!(canonical_suffix(&["A".to_string(), "b".to_string()]), "a/b");
    }

    #[test]
    fn resolution_never_climbs_above_an_absolute_root() {
        assert_eq!(resolve_lexical("/w", ".."), "/");
        assert_eq!(resolve_lexical("/", ".."), "/");
        assert_eq!(resolve_lexical("/w", "/r/a"), "/r/a");
        assert_eq!(resolve_lexical("/w", "a/../b"), "/w/b");
        assert_eq!(resolve_lexical("", "a/b"), "a/b");
    }

    #[test]
    fn containment_needs_a_separator_so_a_sibling_prefix_never_matches() {
        assert!(contains_path("/r/construction", "/r/construction"));
        assert!(contains_path("/r/construction", "/r/construction/u1"));
        assert!(!contains_path("/r/construction", "/r/constructionx"));
    }

    #[test]
    fn uniqueness_drops_the_empty_string_and_keeps_the_first_occurrence() {
        assert_eq!(
            unique_strings(vec![
                String::new(),
                "b".to_string(),
                "a".to_string(),
                "b".to_string()
            ]),
            ["b", "a"]
        );
    }
}
