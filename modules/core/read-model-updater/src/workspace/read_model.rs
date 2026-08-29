//! リードモデル — 投影が読み書きする 2 つの面（状態ファイルと監査シャード）の**メモリ上の姿**。
//!
//! ファイルではない。投影核は純粋であり、ストレージも接続もチェックポイントも知らない
//! （`coding-rules/cqrs-boundaries.md` 二層構造）。ディスクへ落とすのは取得ループの仕事である。
//!
//! # 2 つの面は対称ではない
//!
//! 状態ファイルは**置換**される（現在値そのものを持つので、投影は読んで書き換える）。監査
//! シャードは**追記**される（台帳なので既存の行は読まない — 投影が持つのはこの回に足すバイト
//! 列だけである）。この非対称がそのまま型に出ている。

/// 投影の書込先 2 面。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadModel {
    state: String,
    appended_audit: String,
}

impl ReadModel {
    /// いまの状態ファイル本文から投影の作業面を作る。
    ///
    /// 監査側は空で始まる — 追記する分だけを持つからである。
    #[must_use]
    pub fn new(state: impl Into<String>) -> ReadModel {
        ReadModel {
            state: state.into(),
            appended_audit: String::new(),
        }
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

    /// 状態ファイル本文を差し替える（writer 4 種の結果を受け取る口）。
    pub(crate) fn replace_state(&mut self, next: String) {
        self.state = next;
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
    fn replacing_the_state_does_not_touch_the_audit_side() {
        let mut model = ReadModel::new(SAMPLE);
        model.append_audit("\n## A\n\n---\n");
        model.replace_state("changed".to_string());
        assert_eq!(model.state(), "changed");
        assert_eq!(model.appended_audit(), "\n## A\n\n---\n");
    }
}
