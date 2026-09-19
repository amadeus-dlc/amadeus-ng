//! 指揮役のシェルが、print directive の綴る命令行を argv へ割る動きを再現する補助。
//!
//! 名指し側（`next` の誕生 print）の綴りを書き換えずに受け口へ渡す契約テストが共有する。
//! 割り方を 1 か所に置くのは、片方だけ変えると通しの検査の前提がずれるためである。

/// print が名指しした命令行を、シェルと同じ規則で argv へ割る（テスト用の最小トークナイザ）。
///
/// 単一引用符（`shellArg` が出す形。内側は `'"'"'` で綴られる）と二重引用符（`--label`
/// のプレースホルダ）の両方を剥がす。エスケープ記号は upstream の綴りに現れないので扱わない。
pub(super) fn shell_split(command: &str) -> Vec<String> {
    #[derive(PartialEq, Eq)]
    enum Quote {
        None,
        Single,
        Double,
    }
    let mut argv = Vec::new();
    let mut token = String::new();
    let mut open = false;
    let mut quote = Quote::None;
    for character in command.chars() {
        match (&quote, character) {
            (Quote::None, ' ') => {
                if open {
                    argv.push(std::mem::take(&mut token));
                    open = false;
                }
            }
            (Quote::None, '\'') => {
                quote = Quote::Single;
                open = true;
            }
            (Quote::None, '"') => {
                quote = Quote::Double;
                open = true;
            }
            (Quote::Single, '\'') | (Quote::Double, '"') => quote = Quote::None,
            _ => {
                token.push(character);
                open = true;
            }
        }
    }
    if open {
        argv.push(token);
    }
    argv
}
