//! `FindProjectDescriptionUseCase` — 依頼原文の正本を引く (upstream
//! `readProjectDescriptionAuthority`)。

use core_infrastructure::canon_json::{JsonValue, parse};

use crate::orchestration::{
    ProjectDescriptionDao, ProjectDescriptionError, ProjectDescriptionView, StateFileDao,
};

/// サイドカーを名指す `Project Description Source` の唯一の綴り (upstream 逐語)。
const PROJECT_DESCRIPTION_FILE: &str = "project-description.json";

/// 名指しの無い legacy record が答える出所の綴り (upstream 逐語)。
const LEGACY_PROJECT_DESCRIPTION_SOURCE: &str = "aidlc-state.md#Project";

/// 状態ファイルの名指しに従って依頼原文の正本を引く。
///
/// **名指しがサイドカーを必須にする**のがこの引当の要である — `Project Description Source` が
/// 在る record は必ずサイドカーから読み、不在なら黙って `Project` 欄の**プレビュー**へ
/// 退かずに拒否する (upstream の doc コメント: "a malformed marked record never silently
/// degrades to the preview")。名指しの無い record だけが legacy の `Project` 欄へ後退する。
///
/// 2 つの読取源を `Project Description Source` の綴りで**選び分けて**組むのは、規則 6
/// (2026-09-03) がクエリ側ユースケースに認めた「鍵をたどって面ごとに引き、View を組む」に
/// 当たる (`coding-rules/cqrs-boundaries.md`)。逐語文言は組まない — 拒否は材料だけを運ぶ
/// [`ProjectDescriptionError`] で返し、文言は出す側が組む。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindProjectDescriptionUseCase<S: StateFileDao, D: ProjectDescriptionDao> {
    state_file_dao: S,
    project_description_dao: D,
}

impl<S: StateFileDao, D: ProjectDescriptionDao> FindProjectDescriptionUseCase<S, D> {
    /// 2 つの引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        state_file_dao: S,
        project_description_dao: D,
    ) -> FindProjectDescriptionUseCase<S, D> {
        FindProjectDescriptionUseCase {
            state_file_dao,
            project_description_dao,
        }
    }

    /// 依頼原文の正本と出所を引く。
    ///
    /// # Errors
    ///
    /// 状態ファイル・サイドカーを引けない、または 2 つの面の取り合わせが成立しない
    /// ([`ProjectDescriptionError`])。
    pub fn execute(&self) -> Result<ProjectDescriptionView, ProjectDescriptionError> {
        let state = self
            .state_file_dao
            .find()
            .map_err(ProjectDescriptionError::StateFileUnreadable)?
            .ok_or(ProjectDescriptionError::StateFileAbsent)?;

        // upstream の `getField(state, "Project Description Source") ?? ""` — 欄が無いことと
        // 欄が空であることを同じ「名指し無し」に畳む。
        let source = field(&state, "Project Description Source").unwrap_or_default();
        if source.is_empty() {
            let description =
                field(&state, "Project").ok_or(ProjectDescriptionError::MissingProjectField)?;
            return Ok(ProjectDescriptionView::new(
                description,
                LEGACY_PROJECT_DESCRIPTION_SOURCE.to_string(),
            ));
        }
        if source != PROJECT_DESCRIPTION_FILE {
            return Err(ProjectDescriptionError::UnsupportedSource(source));
        }

        let raw = self
            .project_description_dao
            .find()
            .map_err(ProjectDescriptionError::SidecarUnreadable)?
            .ok_or(ProjectDescriptionError::SidecarMissing)?;
        // 契約 JSON の読取は canon-json の 1 経路に固定されている (BR1.7)。
        let parsed = parse(&raw)
            .map_err(|error| ProjectDescriptionError::SidecarNotJson(error.to_string()))?;
        match parsed {
            JsonValue::String(description) => Ok(ProjectDescriptionView::new(
                description,
                PROJECT_DESCRIPTION_FILE.to_string(),
            )),
            _ => Err(ProjectDescriptionError::SidecarNotAString),
        }
    }
}

/// 状態ファイルの `- **<name>**: <value>` 欄を読む (upstream `getField`)。
///
/// 最初に一致した行が勝ち、値は前後の空白を落とす。**欄が在って値が空**なのは `Some("")` で
/// あり、欄が無い `None` とは区別する — upstream の正規表現 `[ \t]*(.*)` が空文字を捕まえる
/// のと同じ意味論で、legacy record の `Project` が空でも「欄が無い」拒否には落ちない。
fn field(content: &str, name: &str) -> Option<String> {
    let prefix = format!("- **{name}**:");
    content
        .lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()))
        .map(|value| value.trim().to_string())
}
