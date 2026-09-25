//! `ProjectionError` — 投影 ([`super::project`]) の失敗（材料のみ — 文言はアダプタ層）。

use core_command_domain::workspace::{AuditFieldKeyError, CheckboxUpdateError};

use super::state_writers::FieldNotFound;

/// 投影の失敗（材料のみ — 文言はアダプタ層）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionError {
    /// 状態ファイルに書き換え先のフィールド行が無い。
    ///
    /// 無言 no-op は検出不能なドリフトなので、upstream 逐語の拒否文言を添えて止める。
    StateField(FieldNotFound),
    /// 状態ファイルに対象ステージのチェックボックス行が無い、または行末トークンが無い。
    Checkbox(CheckboxUpdateError),
    /// 監査行のフィールドキーが文法外だった（材料の綴りの誤り）。
    AuditFieldKey(AuditFieldKeyError),
    /// イベントが名指したステージが解決済み計画に無い。
    ///
    /// 計画とジャーナルが同じワークフローのものでなければ起きる — 読み替えずに止める。
    UnknownStage {
        /// 計画に無かったステージ。
        stage: String,
    },
    /// 状態ファイルに park マーカーの置き場（`## Runtime State`）が無い。
    ParkSectionMissing,
    /// メモリ層の 2 ファイルが載っていないのに `PracticesAffirmed` を描けと言われた。
    ///
    /// fail-closed である — 動詞側が 2 本の存在を確かめた後に消された場合だけ到達する。
    /// 読めないものを黙って飛ばすと、受領証だけが立って正本が古いままになる。
    MemoryFilesMissing,
    /// メモリ層のファイルに置換先・追記先の見出しが無い。
    MemoryHeadingMissing {
        /// 見出しが無かったファイル（`team.md` / `project.md`）。
        file: &'static str,
        /// 見つからなかった見出しの綴り（`## ` を含む完全形）。
        heading: String,
    },
    /// 状態ファイルの**骨格が無い** — 投影の前提違反である。
    ///
    /// # 骨格を書くのは投影ではない（オーナー裁定 2026-08-29）
    ///
    /// 投影の責務は**既存本文への差分適用に徹する**ことであり、本文そのもの — 9 セクションの
    /// 骨格と 31 のフィールド行 — を起こすことは含まれない。骨格は intent-create の時点で
    /// **合成ルート**が書く（環境と両側を知ってよい唯一の場所。実装は U7）。
    ///
    /// これは「導出の工夫が足りない」のではなく、**構造から従う**裁定である。骨格には
    /// `- **Project Root**:` があり、これはワークツリーの絶対パス — すなわち**環境の値**で、
    /// ジャーナルに存在しない。投影がこれを書けるようになる道は「環境を読む」か「環境パスを
    /// ドメインイベントへ載せる」かの 2 つしかなく、前者は投影核の定義を壊し、後者は ADR-008
    /// と NFR3 の趣旨に反する。書けないのではなく、**書く場所がここではない**。
    ///
    /// # NFR3 の適用範囲
    ///
    /// 冪等な再構成が保証するのは**差分適用**である — 同じジャーナルを同じ本文へ当てれば
    /// 常に同じバイトが出る。骨格はその保証の対象ではなく**環境成果物**であり、全損したら
    /// 再生成ではなく upstream 同様 archive & recreate で復旧する運用に載る。
    ///
    /// # 骨格の実バイトはある
    ///
    /// `cli/intent-create/classic-scope/state-full.md` が全文（102 行）で、U7 が骨格を書く
    /// ときの正本になる。upstream 側の正本は `aidlc-utility.ts` の template literal である
    /// （`knowledge/aidlc-shared/state-template.md` は LLM 向けの契約文書でツールは読まない）。
    ScaffoldMissing,
}

impl core::fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProjectionError::StateField(inner) => write!(f, "state field: {}", inner.message()),
            ProjectionError::Checkbox(CheckboxUpdateError::MissingStage(slug)) => {
                write!(f, "checkbox: missing stage {slug}")
            }
            ProjectionError::Checkbox(CheckboxUpdateError::MissingSuffix(slug)) => {
                write!(f, "checkbox: missing suffix {slug}")
            }
            ProjectionError::AuditFieldKey(inner) => write!(f, "audit field key: {inner}"),
            ProjectionError::UnknownStage { stage } => write!(f, "unknown stage: {stage}"),
            ProjectionError::ParkSectionMissing => f.write_str("park section missing"),
            ProjectionError::MemoryFilesMissing => f.write_str("memory files missing"),
            ProjectionError::MemoryHeadingMissing { file, heading } => {
                write!(f, "memory heading missing: {heading} in {file}")
            }
            ProjectionError::ScaffoldMissing => f.write_str("scaffold missing"),
        }
    }
}

impl std::error::Error for ProjectionError {}

impl From<FieldNotFound> for ProjectionError {
    fn from(inner: FieldNotFound) -> ProjectionError {
        ProjectionError::StateField(inner)
    }
}

impl From<CheckboxUpdateError> for ProjectionError {
    fn from(inner: CheckboxUpdateError) -> ProjectionError {
        ProjectionError::Checkbox(inner)
    }
}

impl From<AuditFieldKeyError> for ProjectionError {
    fn from(inner: AuditFieldKeyError) -> ProjectionError {
        ProjectionError::AuditFieldKey(inner)
    }
}
