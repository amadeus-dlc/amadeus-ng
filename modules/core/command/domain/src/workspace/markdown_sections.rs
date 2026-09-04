//! Markdown の `## ` 節を読む・置く・足す 3 つの純関数 (upstream `aidlc-lib.ts` の写し —
//! `extractMarkdownSection` `:10105` / `appendUnderHeading` `:10150` / `replaceSection` `:10175`)。
//!
//! # なぜ自由関数なのか
//!
//! 対象は「Markdown 本文という裸の文字列」であって、どのドメイン型もそれを所有しない —
//! 昇格の計画 ([`super::PracticesPromotion`]) はこの 3 つを**使う**が、状態ファイルの
//! フィールド行を書く投影 (RMU の `state_writers`) も同じ機構を使う。所有者が居ないので
//! 自由関数に置く (`coding-rules/domain-services.md` の但し書き)。
//!
//! # 見出しの一致規則 (upstream `:10098-10103` の逐語)
//!
//! - `heading` は**完全形**で渡す (`## Walking Skeleton`)。
//! - 実際の見出し行の**末尾空白は許容**する (`^<heading>[ \t]*$`)。
//! - 下位見出し (`### Walking Skeleton`) は `## Walking Skeleton` に一致しない。
//! - 同じ見出しが複数あるときは**最初が勝つ**。
//! - 節の終端は次の `## ` 見出し (行頭・`## ` の後に何か、行末まで) か本文末である。
//! - 見出しが無いとき [`extract_section`] は `None`、他の 2 つは `Err` を返す。

use super::heading_not_found::HeadingNotFound;

/// 節の本文を取り出す (`extractMarkdownSection`)。見出し行そのものは含まない。
///
/// **fenced code block (```` ``` ````) の中の見出しは無視する** — 教材の例に
/// `## Walking Skeleton` があっても本物の節と取り違えない。upstream は fence の中身を
/// 同数の空行へ潰してから走査するので、返る本文も**潰したあとのバイト**である
/// (`stripFencedCodeBlocks` `:10136`)。
///
/// 見出しが無ければ `None`。upstream は `""` を返して「不在」と「空節」を畳むが、
/// 呼出側の分岐は `=== ""` なので `None` と `Some("")` を同じに扱えばよい。
#[must_use]
pub fn extract_section(content: &str, heading: &str) -> Option<String> {
    let stripped = strip_fenced_code_blocks(content);
    let lines: Vec<&str> = stripped.split('\n').collect();
    let start = heading_line(&lines, heading)?;
    let (body_start, body_end) = section_bounds(&lines, start);
    Some(segment(&lines, body_start, body_end))
}

/// 節の本文を丸ごと差し替える (`replaceSection`)。見出し行は残る。
///
/// fence は**潰さない** — 置換は本文そのものを書き換えるので、upstream も生の本文に対して
/// 走査する (`:10184`)。
///
/// # Errors
///
/// 見出しが本文に無ければ [`HeadingNotFound`]。
pub fn replace_section(
    content: &str,
    heading: &str,
    body: &str,
) -> Result<String, HeadingNotFound> {
    let lines: Vec<&str> = content.split('\n').collect();
    let start = heading_line(&lines, heading).ok_or_else(|| HeadingNotFound::new(heading))?;
    let (body_start, body_end) = section_bounds(&lines, start);
    Ok(format!(
        "{}{body}{}",
        prefix(&lines, body_start),
        suffix(&lines, body_end)
    ))
}

/// 節の**末尾** (次の `## ` 見出しの直前、無ければ本文末) にテキストを差し込む
/// (`appendUnderHeading`)。
///
/// # Errors
///
/// 見出しが本文に無ければ [`HeadingNotFound`]。
pub fn append_under_heading(
    content: &str,
    heading: &str,
    text: &str,
) -> Result<String, HeadingNotFound> {
    let lines: Vec<&str> = content.split('\n').collect();
    let start = heading_line(&lines, heading).ok_or_else(|| HeadingNotFound::new(heading))?;
    let (_, insert_at) = section_bounds(&lines, start);
    Ok(format!(
        "{}{text}{}",
        prefix(&lines, insert_at),
        suffix(&lines, insert_at)
    ))
}

/// 見出し行の位置 (`^<heading>[ \t]*$` の最初の一致)。
fn heading_line(lines: &[&str], heading: &str) -> Option<usize> {
    lines
        .iter()
        .position(|line| line.trim_end_matches([' ', '\t']) == heading)
}

/// 節の本文の範囲 (行番号の対) — 見出し行の次から、次の `## ` 見出し行まで。
///
/// 行番号で持つのはバイト位置を扱わないためである — 本文は改行で綴じ直せるので、境界の
/// 計算にバイト位置は要らない (多バイト文字の途中で割る余地を構造的に無くす)。
fn section_bounds(lines: &[&str], start: usize) -> (usize, usize) {
    let body_start = start.saturating_add(1).min(lines.len());
    let end = lines
        .iter()
        .enumerate()
        .skip(body_start)
        .find(|(_, line)| line.starts_with("## "))
        .map_or(lines.len(), |(index, _)| index);
    (body_start, end)
}

/// `index` 行目の**手前**までの本文 (行を改行で綴じ直したもの)。
fn prefix(lines: &[&str], index: usize) -> String {
    let mut out = lines
        .iter()
        .take(index)
        .copied()
        .collect::<Vec<&str>>()
        .join("\n");
    if index > 0 && index < lines.len() {
        out.push('\n');
    }
    out
}

/// `index` 行目から本文末までの本文。
fn suffix(lines: &[&str], index: usize) -> String {
    lines
        .iter()
        .skip(index)
        .copied()
        .collect::<Vec<&str>>()
        .join("\n")
}

/// `from` 行目から `to` 行目の手前までの本文。
fn segment(lines: &[&str], from: usize, to: usize) -> String {
    let mut out = lines
        .iter()
        .skip(from)
        .take(to.saturating_sub(from))
        .copied()
        .collect::<Vec<&str>>()
        .join("\n");
    if to > from && to < lines.len() {
        out.push('\n');
    }
    out
}

/// fenced code block の中身を同数の空行へ潰す (`stripFencedCodeBlocks` `:10136` の写し)。
///
/// 行番号を保つので、潰したあとの本文に対する走査結果をそのまま返せる。
fn strip_fenced_code_blocks(content: &str) -> String {
    let mut in_fence = false;
    let mut out: Vec<&str> = Vec::new();
    for line in content.split('\n') {
        if line.starts_with("```") {
            in_fence = !in_fence;
            out.push("");
            continue;
        }
        out.push(if in_fence { "" } else { line });
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# Title

## Way of Working
trunk-based.

## Walking Skeleton

skeleton: off

## Code Style
rustfmt.
";

    #[test]
    fn a_section_body_stops_at_the_next_h2() {
        assert_eq!(
            extract_section(SAMPLE, "## Way of Working").as_deref(),
            Some("trunk-based.\n\n")
        );
        assert_eq!(
            extract_section(SAMPLE, "## Walking Skeleton").as_deref(),
            Some("\nskeleton: off\n\n")
        );
    }

    /// 本文末の節は最後まで取れる (次の見出しが無い)。
    #[test]
    fn the_last_section_runs_to_the_end_of_the_file() {
        assert_eq!(
            extract_section(SAMPLE, "## Code Style").as_deref(),
            Some("rustfmt.\n")
        );
    }

    /// 見出し行の末尾空白は許容し、下位見出しは `## ` の節を終わらせない。
    ///
    /// 一致は**渡された綴りとの完全一致**なので、`### Deployment` を渡せばその行に当たる
    /// (節の終端は依然として次の `## ` 行か本文末である)。
    #[test]
    fn trailing_whitespace_is_tolerated_and_a_sub_heading_does_not_close_the_section() {
        let content = "## Deployment  \nnone.\n\n### Deployment\ndeeper.\n";
        assert_eq!(
            extract_section(content, "## Deployment").as_deref(),
            Some("none.\n\n### Deployment\ndeeper.\n")
        );
        assert_eq!(
            extract_section(content, "### Deployment").as_deref(),
            Some("deeper.\n")
        );
    }

    /// 同じ見出しが 2 つあれば最初が勝つ。
    #[test]
    fn the_first_of_two_identical_headings_wins() {
        let content = "## A\nfirst.\n\n## B\nb.\n\n## A\nsecond.\n";
        assert_eq!(
            extract_section(content, "## A").as_deref(),
            Some("first.\n\n")
        );
    }

    /// fence の中の見出しは節として拾わない (中身は空行へ潰れる)。
    #[test]
    fn a_heading_inside_a_fence_is_not_a_section() {
        let content = "## Real\nprose.\n\n```md\n## Fake\nexample.\n```\n\n## Next\nx.\n";
        assert_eq!(extract_section(content, "## Fake"), None);
        assert_eq!(
            extract_section(content, "## Real").as_deref(),
            Some("prose.\n\n\n\n\n\n\n")
        );
    }

    /// 見出しが無ければ `None`、空の節は `Some("")`。
    #[test]
    fn an_absent_heading_is_none_and_an_empty_section_is_an_empty_body() {
        assert_eq!(extract_section(SAMPLE, "## Nowhere"), None);
        assert_eq!(
            extract_section("## Empty\n## Next\nx.\n", "## Empty").as_deref(),
            Some("")
        );
    }

    /// 見出しが本文の最終行 (末尾改行なし) でも落ちない。
    #[test]
    fn a_heading_on_the_last_line_yields_an_empty_body() {
        assert_eq!(
            extract_section("intro\n## Tail", "## Tail").as_deref(),
            Some("")
        );
    }

    #[test]
    fn a_replacement_keeps_the_heading_and_the_rest_of_the_file() {
        let out = replace_section(SAMPLE, "## Walking Skeleton", "skeleton: on\n\n").unwrap();
        assert_eq!(
            out,
            "\
# Title

## Way of Working
trunk-based.

## Walking Skeleton
skeleton: on

## Code Style
rustfmt.
"
        );
    }

    #[test]
    fn a_replacement_of_the_last_section_runs_to_the_end() {
        let out = replace_section(SAMPLE, "## Code Style", "black.\n").unwrap();
        assert!(out.ends_with("## Code Style\nblack.\n"), "{out}");
    }

    #[test]
    fn an_append_lands_immediately_before_the_next_h2() {
        let out = append_under_heading(SAMPLE, "## Way of Working", "- new rule\n").unwrap();
        assert_eq!(
            out,
            "\
# Title

## Way of Working
trunk-based.

- new rule
## Walking Skeleton

skeleton: off

## Code Style
rustfmt.
"
        );
    }

    #[test]
    fn an_append_to_the_last_section_lands_at_the_end_of_the_file() {
        let out = append_under_heading(SAMPLE, "## Code Style", "- black\n").unwrap();
        assert!(out.ends_with("rustfmt.\n- black\n"), "{out}");
    }

    #[test]
    fn a_missing_heading_is_refused_with_its_spelling() {
        assert_eq!(
            replace_section(SAMPLE, "## Nowhere", "x").unwrap_err(),
            HeadingNotFound::new("## Nowhere")
        );
        assert_eq!(
            append_under_heading(SAMPLE, "## Nowhere", "x").unwrap_err(),
            HeadingNotFound::new("## Nowhere")
        );
    }

    /// 多バイト文字を含む本文でも行頭は文字境界なので割れる。
    #[test]
    fn a_multibyte_body_is_sliced_on_line_boundaries() {
        let content = "## 見出し\n本文。\n\n## 次\nx\n";
        assert_eq!(
            extract_section(content, "## 見出し").as_deref(),
            Some("本文。\n\n")
        );
        let out = replace_section(content, "## 見出し", "置換。\n").unwrap();
        assert_eq!(out, "## 見出し\n置換。\n## 次\nx\n");
    }
}
