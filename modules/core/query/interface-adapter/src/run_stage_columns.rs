//! `read_run_stage` の 23 列の選択句と行写像 (公開型ゼロの内部モジュール)。
//!
//! [`RunStageDaoImpl`] の 3 つの動詞が同じ列を同じ並びで読むので、**列の並びを 1 か所に
//! 閉じる**。並びが選択句と写像でずれると値が静かに入れ替わるので、両方をこのモジュールが
//! 一緒に持つ。
//!
//! 表別名は付けない — DAO は 1 表しか引かないので、修飾すべき同名列が無い
//! (オーナー裁定 2026-09-03 — `coding-rules/cqrs-boundaries.md` 規則 6)。
//!
//! [`RunStageDaoImpl`]: super::RunStageDaoImpl

use core_query_use_case::orchestration::RunStageView;
use rusqlite::Row;

/// `read_run_stage` を 1 表で引く SELECT を、`WHERE` 句の literal から組む。
///
/// 定数ではなくマクロなのは、呼び手が `concat!` で**コンパイル時に**文を組み立てられる
/// ようにするためである — 実行時に `format!` で組むと、引く表が 1 つであることを文字列
/// リテラルの検査 (レビューと lint) で確かめられなくなる。列は DDL と同じ並びで、この順で
/// [`run_stage_row`] が読む。
macro_rules! select_run_stage {
    ($where_clause:literal) => {
        concat!(
            "SELECT id, definition_id, scope, stage_slug, phase, steering_plan_id, \
             lead_agent, support_agents, mode, gate_default, inline_context_paths_rel, \
             stage_file_rel, memory_path_rel, consumes_rel, produces_rel, sensors_applicable, \
             reviewer, reviewer_max_iterations, review_class, protocol_modules, \
             next_stage_name, route_digest, directive_digest \
             FROM read_run_stage WHERE ",
            $where_clause
        )
    };
}

pub(crate) use select_run_stage;

/// 23 列を 1 行の写しへ写す。
pub(crate) fn run_stage_row(row: &Row<'_>) -> rusqlite::Result<RunStageView> {
    Ok(RunStageView::new(
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
        row.get(13)?,
        row.get(14)?,
        row.get(15)?,
        row.get(16)?,
        row.get(17)?,
        row.get(18)?,
        row.get(19)?,
        row.get(20)?,
        row.get(21)?,
        row.get(22)?,
    ))
}
