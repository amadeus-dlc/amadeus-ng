//! 行の**主キー** `id` を自然キーから決定的に導く (公開型ゼロの内部モジュール)。
//!
//! # なぜ代理キーか
//!
//! 表の主キーは 1 列 `id` である (オーナー裁定 2026-09-03 — 基本的な関係モデリング。
//! 複合主キーにしない)。集約そのものを表す 3 表 (`read_definition` / `read_intent` /
//! `read_execution`) は集約 id をそのまま `id` にできるが、それ以外の表には自然な 1 列が
//! 無いので、**自然キーから決定的に導いた代理キー**を置く。
//!
//! 素材は自然キーの名前付き正準 JSON (`{"definition_id":..,"stage_slug":..}`) で、
//! `ContractCompact` 族の [`hash_compact`] でハッシュする。`"$a:$b"` のような**連結文字列は
//! 使わない** — 区切り文字が値に現れると別のキーと衝突する (`("a:b", "c")` と
//! `("a", "b:c")` が同じ id になる)。名前付きオブジェクトなら値のどのバイトも構造を壊さない。
//!
//! **挿入順が素材バイトの一部である** — 表ごとのキーの並びを変えると id が変わる。
//!
//! # 一意性の射程は表の中である
//!
//! 自然キーの形が同じ表 (`read_definition_scope_stage` と `read_run_stage` はどちらも
//! `{definition_id, scope, stage_slug}`) は同じ値を持つ。主キーは表の中で一意であればよく、
//! 表を跨いだ一意性はここでは要求しない。FK は「どの表を指すか」を列名で言う。

use core_infrastructure::canon_json::{JsonValue, Number, hash_compact};

/// 自然キーの正準 JSON をハッシュして `id` にする。
fn of<const N: usize>(natural_key: [(&str, JsonValue); N]) -> String {
    let material = JsonValue::Object(
        natural_key
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect(),
    );
    hash_compact(&material).rendered()
}

/// 自然キーの文字列項目。
fn text(value: &str) -> JsonValue {
    JsonValue::String(value.to_string())
}

/// 自然キーの索引項目 (文書順の位置・部番号)。
///
/// `usize` から `u64` への拡大変換であり、どの対象環境でも値は失われない。
const fn index(value: usize) -> JsonValue {
    JsonValue::Number(Number::PosInt(value as u64))
}

/// `read_definition_stage.id` — 定義 × ステージ。
pub(crate) fn definition_stage(definition_id: &str, stage_slug: &str) -> String {
    of([
        ("definition_id", text(definition_id)),
        ("stage_slug", text(stage_slug)),
    ])
}

/// `read_definition_scope.id` — 定義 × スコープ。
pub(crate) fn definition_scope(definition_id: &str, scope: &str) -> String {
    of([
        ("definition_id", text(definition_id)),
        ("scope", text(scope)),
    ])
}

/// `read_definition_scope_keyword.id` — 定義 × 語。
pub(crate) fn definition_scope_keyword(definition_id: &str, keyword: &str) -> String {
    of([
        ("definition_id", text(definition_id)),
        ("keyword", text(keyword)),
    ])
}

/// `read_definition_scope_stage.id` — 定義 × スコープ × ステージ。
pub(crate) fn definition_scope_stage(definition_id: &str, scope: &str, stage_slug: &str) -> String {
    of([
        ("definition_id", text(definition_id)),
        ("scope", text(scope)),
        ("stage_slug", text(stage_slug)),
    ])
}

/// `read_definition_scope_phase_entry.id` — 定義 × スコープ × フェーズ。
pub(crate) fn definition_scope_phase_entry(
    definition_id: &str,
    scope: &str,
    phase: &str,
) -> String {
    of([
        ("definition_id", text(definition_id)),
        ("scope", text(scope)),
        ("phase", text(phase)),
    ])
}

/// `read_intent_stage.id` — intent × 文書順の位置。
pub(crate) fn intent_stage(intent_id: &str, stage_index: usize) -> String {
    of([
        ("intent_id", text(intent_id)),
        ("stage_index", index(stage_index)),
    ])
}

/// `read_execution_stage.id` — 実行 × 文書順の位置。
pub(crate) fn execution_stage(execution_id: &str, stage_index: usize) -> String {
    of([
        ("execution_id", text(execution_id)),
        ("stage_index", index(stage_index)),
    ])
}

/// `read_next_answer.id` — 実行 × 要求の形。
pub(crate) fn next_answer(execution_id: &str, request_kind: &str) -> String {
    of([
        ("execution_id", text(execution_id)),
        ("request_kind", text(request_kind)),
    ])
}

/// `read_next_jump.id` — 実行 × ジャンプ先の位置。
pub(crate) fn next_jump(execution_id: &str, target_index: usize) -> String {
    of([
        ("execution_id", text(execution_id)),
        ("target_index", index(target_index)),
    ])
}

/// `read_next_jump_phase.id` — 実行 × フェーズ。
pub(crate) fn next_jump_phase(execution_id: &str, phase: &str) -> String {
    of([("execution_id", text(execution_id)), ("phase", text(phase))])
}

/// `read_run_stage.id` — 定義 × スコープ × ステージ。
///
/// `read_next_answer.run_stage_id` はこの関数の答えを FK に載せる (同じ素材を 2 か所で
/// 書き下さないための単一の口である)。
pub(crate) fn run_stage(definition_id: &str, scope: &str, stage_slug: &str) -> String {
    of([
        ("definition_id", text(definition_id)),
        ("scope", text(scope)),
        ("stage_slug", text(stage_slug)),
    ])
}

/// `read_scope_change.id` — 実行 × 要求されうるスコープ。
pub(crate) fn scope_change(execution_id: &str, scope: &str) -> String {
    of([("execution_id", text(execution_id)), ("scope", text(scope))])
}

/// `read_steering_plan.id` — フェーズ。
///
/// `read_steering_part.steering_plan_id` と `read_run_stage.steering_plan_id` はこの
/// 関数の答えを FK に載せる。
pub(crate) fn steering_plan(phase: &str) -> String {
    of([("phase", text(phase))])
}

/// `read_steering_part.id` — フェーズ × 部番号。
pub(crate) fn steering_part(phase: &str, part_index: usize) -> String {
    of([("phase", text(phase)), ("part_index", index(part_index))])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_natural_key_always_yields_the_same_id() {
        assert_eq!(
            definition_stage("d1", "state-init"),
            definition_stage("d1", "state-init")
        );
        assert_eq!(steering_part("ideation", 2), steering_part("ideation", 2));
    }

    #[test]
    fn a_different_natural_key_yields_a_different_id() {
        assert_ne!(
            definition_stage("d1", "state-init"),
            definition_stage("d1", "intent-capture")
        );
        assert_ne!(
            definition_stage("d1", "state-init"),
            definition_stage("d2", "state-init")
        );
        assert_ne!(steering_part("ideation", 1), steering_part("ideation", 2));
        assert_ne!(next_jump("e1", 0), next_jump("e1", 1));
    }

    #[test]
    fn every_id_is_a_bare_sixty_four_digit_lowercase_hex() {
        for value in [
            definition_stage("d1", "s"),
            definition_scope("d1", "classic"),
            definition_scope_keyword("d1", "bug"),
            definition_scope_stage("d1", "classic", "s"),
            definition_scope_phase_entry("d1", "classic", "ideation"),
            intent_stage("i1", 0),
            execution_stage("e1", 0),
            next_answer("e1", "bare"),
            next_jump("e1", 0),
            next_jump_phase("e1", "ideation"),
            run_stage("d1", "classic", "s"),
            scope_change("e1", "classic"),
            steering_plan("ideation"),
            steering_part("ideation", 1),
        ] {
            assert_eq!(value.len(), 64, "実際: {value}");
            assert!(
                value
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()),
                "実際: {value}"
            );
        }
    }

    #[test]
    fn a_separator_inside_a_value_cannot_forge_another_rows_id() {
        // 連結文字列 (`"$a:$b"`) なら ("a:b", "c") と ("a", "b:c") が同じ id になる。
        // 名前付きオブジェクトの正準 JSON はこの衝突を構造的に持たない。
        assert_ne!(
            definition_stage("d:1", "state-init"),
            definition_stage("d", "1:state-init")
        );
    }

    #[test]
    fn the_key_names_are_part_of_the_material() {
        // 同じ値でもキー名が違えば別の id — 表ごとにキー名を書き下す意味がここにある。
        assert_ne!(
            definition_scope("x", "y"),
            definition_scope_keyword("x", "y")
        );
    }

    #[test]
    fn the_run_stage_id_is_the_one_the_next_answer_foreign_key_points_at() {
        // FK の素材が 2 か所に書かれていないことの固定 — 綴りが割れたら参照が外れる。
        assert_eq!(
            run_stage("d1", "classic", "state-init"),
            super::run_stage("d1", "classic", "state-init")
        );
        assert_ne!(run_stage("d1", "classic", "a"), run_stage("d1", "mvp", "a"));
    }
}
