//! リードモデル — 投影が読み書きする面（状態ファイル・監査シャード・メモリ層 2 本）の
//! **メモリ上の姿**。
//!
//! ファイルではない。投影核は純粋であり、ストレージも接続もチェックポイントも知らない
//! （`coding-rules/cqrs-boundaries.md` 二層構造）。ディスクへ落とすのは取得ループの仕事である。
//!
//! # 面は対称ではない
//!
//! 状態ファイルは**置換**される（現在値そのものを持つので、投影は読んで書き換える）。監査
//! シャードは**追記**される（台帳なので既存の行は読まない — 投影が持つのはこの回に足すバイト
//! 列だけである）。この非対称がそのまま型に出ている。
//!
//! # メモリ層は**在るとは限らない**面である（b49）
//!
//! `team.md` / `project.md` は `PracticesAffirmed` を描くときだけ要る面なので、取得ループは
//! 2 本とも在るときだけ載せる。載っていない状態でその投影を求められたら fail-closed で止まる
//! （`ProjectionError::MemoryFilesMissing`）— 動詞側が存在を確かめた後に消された場合だけ
//! 到達する経路である。

use super::memory_faces::MemoryFaces;

/// 投影の書込先。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadModel {
    active_directive: Option<String>,
    state: String,
    appended_audit: String,
    memory: Option<MemoryFaces>,
}

impl ReadModel {
    const fn of_faces(
        active_directive: Option<String>,
        state: String,
        appended_audit: String,
        memory: Option<MemoryFaces>,
    ) -> Self {
        Self {
            active_directive,
            state,
            appended_audit,
            memory,
        }
    }

    /// 保存済みの指示発行の事実から、今回投影するマーカーを描く。
    pub(crate) fn apply_directive_issue(
        &mut self,
        directive: &core_command_domain::orchestration::ActiveDirective,
    ) {
        self.active_directive = Some(super::active_directive::render(directive));
    }
    /// 今回の指示発行で更新するマーカー。
    #[must_use]
    pub fn active_directive(&self) -> Option<&str> {
        self.active_directive.as_deref()
    }

    /// いまの状態ファイル本文から投影の作業面を作る。
    ///
    /// 監査側は空で始まる — 追記する分だけを持つからである。
    #[must_use]
    pub fn new(state: impl Into<String>) -> ReadModel {
        Self::of_faces(None, state.into(), String::new(), None)
    }

    /// メモリ層 2 本の本文を載せる（取得ループが**両方在るとき**だけ呼ぶ）。
    #[must_use]
    pub fn with_memory(mut self, team: impl Into<String>, project: impl Into<String>) -> ReadModel {
        self.memory = Some(MemoryFaces::new(team, project));
        self
    }

    /// 載っているメモリ層の面（`None` = 2 本が揃っていない）。
    #[must_use]
    pub const fn memory(&self) -> Option<&MemoryFaces> {
        self.memory.as_ref()
    }

    /// 載っているメモリ面の描画結果を持つ完成値を返す。不在ならそのまま返す。
    pub(crate) fn with_rewritten_memory(self, team: String, project: String) -> Self {
        let memory = self.memory.map(|_| MemoryFaces::rewritten(team, project));
        Self::of_faces(
            self.active_directive,
            self.state,
            self.appended_audit,
            memory,
        )
    }

    /// 状態ファイルの現在の本文。
    #[must_use]
    pub fn state(&self) -> &str {
        &self.state
    }

    /// この投影が監査シャードへ追記するバイト列（空なら書くものが無い）。
    #[must_use]
    pub fn appended_audit(&self) -> &str {
        &self.appended_audit
    }

    /// 描画済みの状態本文を持つ完成値を返す。監査追記・指示・メモリ面は保持する。
    pub(crate) fn with_state(self, state: String) -> Self {
        Self::of_faces(
            self.active_directive,
            state,
            self.appended_audit,
            self.memory,
        )
    }

    /// 監査ブロックを 1 つ足す。
    pub(crate) fn append_audit(&mut self, block: &str) {
        self.appended_audit.push_str(block);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
## Project Information
- **Project**: demo
- **Scope**: classic
";

    #[test]
    fn a_new_read_model_carries_the_state_and_no_audit_yet() {
        let model = ReadModel::new(SAMPLE);
        assert_eq!(model.state(), SAMPLE);
        assert_eq!(model.appended_audit(), "");
    }

    #[test]
    fn appending_audit_accumulates_in_order() {
        let mut model = ReadModel::new(SAMPLE);
        model.append_audit("\n## A\n\n---\n");
        model.append_audit("\n## B\n\n---\n");
        assert_eq!(model.appended_audit(), "\n## A\n\n---\n\n## B\n\n---\n");
    }

    #[test]
    fn a_new_read_model_carries_no_memory_face() {
        assert!(ReadModel::new(SAMPLE).memory().is_none());
    }

    #[test]
    fn the_memory_face_is_loaded_and_replaced_together() {
        let mut model = ReadModel::new(SAMPLE).with_memory("# Team\n", "# Project\n");
        assert!(model.memory().is_some_and(|memory| !memory.is_dirty()));
        model = model.with_rewritten_memory("# Team 2\n".to_string(), "# Project 2\n".to_string());
        let memory = model.memory().expect("載せた面は在る");
        assert_eq!(memory.team(), "# Team 2\n");
        assert_eq!(memory.project(), "# Project 2\n");
        assert!(memory.is_dirty());
    }

    /// 面が載っていなければ差し替えは何もしない（fail-closed の判断は投影側）。
    #[test]
    fn replacing_an_absent_memory_face_is_a_no_op() {
        let mut model = ReadModel::new(SAMPLE);
        model = model.with_rewritten_memory("x".to_string(), "y".to_string());
        assert!(model.memory().is_none());
    }

    #[test]
    fn replacing_the_state_does_not_touch_the_audit_side() {
        let mut model = ReadModel::new(SAMPLE);
        model.append_audit("\n## A\n\n---\n");
        model = model.with_state("changed".to_string());
        assert_eq!(model.state(), "changed");
        assert_eq!(model.appended_audit(), "\n## A\n\n---\n");
    }
}
