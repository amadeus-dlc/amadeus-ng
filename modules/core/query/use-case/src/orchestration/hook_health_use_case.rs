//! HookHealth投影をIDで返すQuery。
use super::{HookHealthDao, HookHealthView, ReadModelReadError};
/// HookHealth投影を単純なID読取へ変換するQuery。
#[derive(Debug)]
pub struct HookHealthUseCase<D> {
    dao: D,
}
impl<D: HookHealthDao> HookHealthUseCase<D> {
    /// DAOを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }
    /// 指定IDの投影行を読む。
    /// # Errors
    /// リードモデルを読めない場合。
    pub fn execute(&self, id: &str) -> Result<Option<HookHealthView>, ReadModelReadError> {
        self.dao.find(id)
    }
}
