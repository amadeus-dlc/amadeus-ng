//! `CodekbRepository` ポート — codekb ストア集約の永続化契約。

use core_command_domain::workspace::{Codekb, CodekbEvent, CodekbRepoId};

use super::repository_error::RepositoryError;

/// リポジトリごとの codekb ストアの Repository。
///
/// 集約 [`Codekb`] は**ディスク上の 9 成果物**として住む。媒体がファイルであることは実装の
/// 内部詳細であり、ポート面には現れない (`coding-rules/gateway-taxonomy.md` §2 — 媒体名を
/// 契約に漏らさない)。集約 `CompiledDefinition` の Repository と同じ形である。
///
/// # 語彙は 2 つだけ
///
/// 集約を I/O する語彙しか置かない — 再構成する [`CodekbRepository::find_by_id`] と、
/// 起きた事実 1 件と適用後の集約を永続化する [`CodekbRepository::store`] である
/// (`coding-rules/gateway-taxonomy.md` §2b の ES Repository 拡張語彙)。使い方も 1 つで、
/// **再構成 → 集約が判断 → `store`** の 3 手に収まる
/// (`coding-rules/cqrs-boundaries.md` 追補「リポジトリの使い方」)。歴史を消す動詞は無い
/// (ES リポジトリなので `delete_by_id` は要らない)。
///
/// # 中断した公開は集約が畳む
///
/// 公開は途中で落ちうるので、再構成の時点で**まだ決着していない公開**が残っていることが
/// ある。それは読取が観測する事実であり、畳むかどうかを決めるのは集約
/// ([`Codekb::settle_interrupted_publication`]) である。畳む手順そのものは決着の事実を受けた
/// `store` の内部詳細で、ポート面には現れない — 読取メソッドが黙って媒体を書き換えるのは
/// CQS 違反であり (`coding-rules/command-query-separation.md`)、外科的なライタを Repository の
/// メソッドに生やすのも同じく禁じられている (`coding-rules/gateway-taxonomy.md` §2b)。
///
/// 呼出側はロックを取った区間でこの 3 手を進める。upstream も同じ順序である —
/// `handleCodekbSnapshot` / `handleCodekbPublish` は `withCodekbLock` の下でまず
/// `recoverCodekbTransactions` を走らせ、そのあとで読んで出力する。
#[allow(
    async_fn_in_trait,
    reason = "既存Repositoryポートと同じcurrent_thread契約"
)]
pub trait CodekbRepository {
    /// リポジトリ識別子でストアを再構成する (**純粋な読取** — 何も書き換えない)。
    ///
    /// ストアがまだ無いことは失敗ではない — 不在も 1 つの世代
    /// ([`CodekbGeneration::absent`](core_command_domain::workspace::CodekbGeneration::absent))
    /// として再構成する。
    ///
    /// 中断した公開が残っていれば、**畳む前のディスクの姿**をそのまま観測し、集約はそれを
    /// 抱えたまま再構成される。畳んだあとの世代が要るなら、集約に決着をつけさせ、その事実を
    /// [`CodekbRepository::store`] で確定させてから改めて引く。
    ///
    /// # Errors
    ///
    /// OS 由来の読取失敗 (`Io`)、世代を畳めないほど壊れている (`Corrupt`)。
    async fn find_by_id(&self, id: &CodekbRepoId) -> Result<Codekb, RepositoryError<CodekbRepoId>>;

    /// 起きた事実 1 件と、適用後の集約を永続化する。
    ///
    /// 公開の事実なら、書き出すバイトは**イベントが運ぶ** 9 成果物である。置き換えは全体で
    /// 1 つの操作として見えなければならない (読み手が 9 枚のうち 3 枚だけ新しい状態を見ては
    /// ならない)。決着の事実なら、中断した公開を畳んでストアを「いま在る」姿へ確定させる。
    ///
    /// # Errors
    ///
    /// イベントと集約の対が食い違う書込契約違反 (`Corrupt`)、OS 由来の書込失敗 (`Io`)。
    async fn store(
        &mut self,
        event: &CodekbEvent,
        codekb: &Codekb,
    ) -> Result<(), RepositoryError<CodekbRepoId>>;
}
