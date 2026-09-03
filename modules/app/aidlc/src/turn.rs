//! `next` / `continue` の**構文的ルーティング** — どの引当をどの鍵で呼ぶか。
//!
//! # 状態の値では分岐しない
//!
//! ここが見るのは**要求の形**だけである — フラグの有無・本文の語数・token が何を運ぶか。
//! 状態の値で決まる答え (どのステージを走らせるか・park しているか・完了したか) は RMU が
//! 集約のクエリを呼んで `read_next_answer.decision_kind` に書いてあり、こちらはその綴りに
//! 従って [`crate::directive_drawing`] に描かせるだけである
//! (`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記)。
//!
//! # 引当は 1 要求 1 接続
//!
//! [`ReadModelDaos`] を 1 度だけ開き、12 実装がそれを分け合う。多段の引当 (`next` は最大
//! 5 表) が同じスナップショットを見るためである。

use core_command_domain::workspace::{SpaceName, StorePath};
use core_query_interface_adapter::ReadModelDaos;
use core_query_use_case::orchestration::{
    AskDirective, AskKind, ContinueToken, Directive, EngineCommand, FindContinuationUseCase,
    FindDefinitionUseCase, FindExecutionUseCase, FindJumpUseCase, FindNextAnswerUseCase,
    FindPhaseEntryUseCase, FindRunStageUseCase, FindScopeChangeUseCase, FindScopeKeywordUseCase,
    FindScopeUseCase, FindSteeringUseCase, GateField, JumpView, NextTurnInput, NextTurnView,
    ReadModelReadError, RunStageView, ScopeDao, ScopeSlugView, ScopeView, StageSlugView,
    SteeringDeliveryView,
};

use crate::directive_drawing;
use crate::execution_cursor::ExecutionCursor;
use crate::layout::Layout;
use crate::wording;

/// デフォルト scope (`export const DEFAULT_SCOPE = "classic";`)。
const DEFAULT_SCOPE: &str = "classic";

/// キーワード推論を抑止する語数 (upstream `:5586-5594` — 「語が多いのは説明文」)。
const KEYWORD_WORD_LIMIT: usize = 5;

/// 定義の系譜名 — `harness.json` の `name` (ADR-008)。出荷ハーネスは `claude`。
const DEFINITION_ID: &str = "claude";

/// `next` 1 回を directive ちょうど 1 つに写す。
///
/// 失敗も `Directive::Error` になる — エンジンの契約は「stdout に directive ちょうど 1 つ」
/// である (§3.2)。
#[must_use]
pub(crate) fn next(layout: &Layout, input: &NextTurnInput) -> Directive {
    if let Some(directive) = pre_guards(input) {
        return directive;
    }
    match Turn::open(layout) {
        Ok(turn) => turn.next(input),
        Err(directive) => *directive,
    }
}

/// `continue` 1 回を directive ちょうど 1 つに写す (開封は呼出側 — 失敗は `None`)。
#[must_use]
pub(crate) fn resume(layout: &Layout, token: Option<&ContinueToken>) -> Directive {
    let Some(token) = token else {
        return Directive::Error {
            message: wording::INVALID_CONTINUATION_TOKEN.to_string(),
        };
    };
    match Turn::open(layout) {
        Ok(turn) => turn.resume(token),
        Err(directive) => *directive,
    }
}

/// リードモデルを 1 度も読まずに答えが決まる前置ガード。
fn pre_guards(input: &NextTurnInput) -> Option<Directive> {
    if let Some(message) = input.parse_error() {
        return Some(Directive::Error {
            message: message.to_string(),
        });
    }
    if input.review().is_some()
        && (input.read_only().is_some()
            || input.noun_token().is_some()
            || input.is_compose()
            || input.is_single()
            || input.stage().is_some()
            || input.phase().is_some()
            || input.is_resume())
    {
        return Some(Directive::Error {
            message: wording::REVIEW_COMBINATION.to_string(),
        });
    }
    // 分岐 0: Kiro roll-forward ラッチ (advisory, fail-open)。
    if input.is_kiro_latch_bare_next() {
        return Some(Directive::Done { reason: None });
    }
    // 分岐 1: 読み取り専用ユーティリティ。
    if let Some(verb) = input.read_only() {
        return Some(Directive::Print {
            message: wording::read_only(&EngineCommand::ReadOnlyUtility(verb).cli_spelling()),
        });
    }
    // 分岐 1b/1c/1d: 名詞トークン (先頭トークン意味論のみ)。
    if let Some(token) = input.noun_token() {
        return Some(Directive::Print {
            message: wording::terminal_utility(
                &EngineCommand::NounTokens(token.tokens().to_vec()).cli_spelling(),
            ),
        });
    }
    // 分岐 2: --stage と --phase の併用。
    if input.stage().is_some() && input.phase().is_some() {
        return Some(Directive::Error {
            message: wording::STAGE_AND_PHASE.to_string(),
        });
    }
    None
}

/// 1 要求ぶんの引当の口 (12 の実装が 1 接続を分け合う)。
struct Turn<'a> {
    layout: &'a Layout,
    daos: ReadModelDaos,
}

impl<'a> Turn<'a> {
    /// 構造化リードモデルを読取専用で開く。開けなければ描く directive を返す。
    fn open(layout: &'a Layout) -> Result<Turn<'a>, Box<Directive>> {
        let space = SpaceName::parse(layout.space()).map_err(|_| {
            Box::new(Directive::Error {
                message: wording::invalid_active_space(layout.space()),
            })
        })?;
        let store = StorePath::for_space(&layout.aidlc_root(), &space);
        ReadModelDaos::open(store.as_path())
            .map(|daos| Turn { layout, daos })
            .map_err(|error| {
                Box::new(Directive::Error {
                    message: wording::read_model_unreadable(
                        &store.as_path().to_string_lossy(),
                        &error.kind().to_string(),
                    ),
                })
            })
    }

    /// 引当の失敗を逐語へ写す (材料は分類と所在だけ)。
    fn unreadable(error: &ReadModelReadError) -> Directive {
        let path = error
            .path()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default();
        Directive::Error {
            message: wording::read_model_unreadable(&path, &error.kind().to_string()),
        }
    }

    /// record が指す実行の識別子 (カーソルが無い・読めないなら `None`)。
    fn execution_id(&self) -> Option<String> {
        let record = self.layout.record_dir()?;
        ExecutionCursor::read(record)
            .ok()
            .flatten()
            .map(|cursor| cursor.execution_id().as_str().to_string())
    }

    /// `next` のラダー (state の有無で 2 群に割れる)。
    fn next(&self, input: &NextTurnInput) -> Directive {
        // 定義が取り込まれていなければ、どの引当も答えを持たない。
        let definition = FindDefinitionUseCase::new(self.daos.definition()).execute(DEFINITION_ID);
        let definition = match definition {
            Ok(found) => found,
            Err(error) => return Turn::unreadable(&error),
        };
        let execution = self.execution_id();
        if definition.is_none() {
            // 定義もカーソルも無いのは fresh なワークスペースの正常な姿である。
            return Directive::Error {
                message: if execution.is_none() {
                    wording::NO_STATE.to_string()
                } else {
                    wording::stage_graph_not_readable(
                        &self
                            .layout
                            .definition_data_dir()
                            .join("stage-graph.json")
                            .to_string_lossy(),
                        "the workflow definition has not been projected into the read model",
                    )
                },
            };
        }
        let answer = match self.answer(execution.as_deref(), input) {
            Ok(answer) => answer,
            Err(directive) => return *directive,
        };
        // 分岐 2.5 / 2.6: park (答えの綴りがそのまま行き先である)。
        if let Some(view) = answer.as_ref() {
            match view.answer().decision_kind() {
                "parked" => return Turn::parked(view),
                "unpark-then-resume" => {
                    return Directive::Print {
                        message: wording::unpark_then_resume(&EngineCommand::Unpark.cli_spelling()),
                    };
                }
                _ => {}
            }
        }
        // 分岐 3b / 4 / 解決不能: scope 解決ラダー。
        let state_scope = answer
            .as_ref()
            .map(|view| view.execution().scope().to_string());
        let resolved = match self.resolve_scope(state_scope.as_deref(), input) {
            Ok(resolved) => resolved,
            Err(directive) => return *directive,
        };
        if let Some(directive) = self.request_shaped_branches(input, &resolved, answer.as_ref()) {
            return directive;
        }
        // ---- state なしの群 (7b / 8 / 9a / 9b) ----
        let Some(view) = answer.as_ref() else {
            return self.birth_group(input);
        };
        self.happy_path(input, view, &resolved)
    }

    /// 要求の形で決まる分岐 (compose / new-intent / single / 設定変更 / resume / jump)。
    ///
    /// どれも**フラグの有無**だけで入口が決まる。中で引く行はその分岐の鍵で引いた 1 行で
    /// ある。
    fn request_shaped_branches(
        &self,
        input: &NextTurnInput,
        scope: &ScopeSlugView,
        answer: Option<&NextTurnView>,
    ) -> Option<Directive> {
        // 分岐 4c: compose。
        if input.is_compose() {
            if input.stage().is_some() || input.phase().is_some() {
                return Some(Directive::Error {
                    message: wording::COMPOSE_WITH_JUMP.to_string(),
                });
            }
            return Some(Directive::Print {
                message: wording::dispatch_composer(
                    &EngineCommand::DispatchComposer.cli_spelling(),
                ),
            });
        }
        // 分岐 4a: --new-intent。明示 `--scope` が勝ち、無ければ解決済み scope へ落ちる。
        if let Some(description) = input.new_intent() {
            let description = description.trim();
            if description.is_empty() {
                return Some(Directive::Error {
                    message: wording::NEW_INTENT_BLANK.to_string(),
                });
            }
            let named = input.scope().unwrap_or_else(|| scope.as_str()).to_string();
            return Some(self.mint_intent(&named, Some(description), input, true));
        }
        // 分岐 4b: --single (scope-change / jump より前)。
        if input.is_single() {
            return Some(self.single(input, scope));
        }
        // 分岐 5: state あり + 有効で異なる設定。
        if let Some(view) = answer
            && let Some(directive) = self.configuration_change(input, view)
        {
            return Some(directive);
        }
        // 分岐 6: state ありでの --resume。
        if let Some(view) = answer
            && view.answer().decision_kind() == "resume-menu"
        {
            let stage = view.answer().stage_slug().unwrap_or_default();
            return Some(Directive::Ask(AskDirective::new(
                AskKind::ResumeMenu,
                wording::resume_menu(stage),
            )));
        }
        // 分岐 7: --stage / --phase (jump)。
        if input.stage().is_some() || input.phase().is_some() {
            return Some(self.jump(input, scope, answer));
        }
        None
    }

    /// 分岐 2.5 — park している位置を名乗って止まる。
    fn parked(view: &NextTurnView) -> Directive {
        let slug = view.answer().stage_slug().unwrap_or_default();
        match StageSlugView::parse(slug) {
            Ok(stage) => Directive::Parked {
                message: wording::parked(stage.as_str()),
                stage,
            },
            Err(_) => Directive::Error {
                message: wording::unknown_stage(slug),
            },
        }
    }

    /// `read_next_answer` を要求の形で引く (4 綴りのどれで引くかがルーティング)。
    fn answer(
        &self,
        execution_id: Option<&str>,
        input: &NextTurnInput,
    ) -> Result<Option<NextTurnView>, Box<Directive>> {
        let Some(execution_id) = execution_id else {
            return Ok(None);
        };
        FindNextAnswerUseCase::new(
            self.daos.next_answer(),
            self.daos.execution(),
            self.daos.run_stage(),
            self.daos.steering_plan(),
            self.daos.steering_part(),
        )
        .execute(execution_id, request_kind(input))
        .map_err(|error| Box::new(Turn::unreadable(&error)))
    }

    /// scope 解決ラダー (`state > --scope > positional > env > default`)。
    ///
    /// **どの候補を先に試すかの順序**であり、状態の値を見る判断は含まない — 各候補の
    /// 有効性は `read_definition_scope` に行があるかどうかで決まる (行が返ること自体が
    /// 「その scope は有効」の答えである)。
    fn resolve_scope(
        &self,
        state_scope: Option<&str>,
        input: &NextTurnInput,
    ) -> Result<ScopeSlugView, Box<Directive>> {
        let scopes = FindScopeUseCase::new(self.daos.scope());
        let valid = |name: &str| -> Result<bool, Box<Directive>> {
            scopes
                .execute(DEFINITION_ID, name)
                .map(|found| found.is_some())
                .map_err(|error| Box::new(Turn::unreadable(&error)))
        };
        // 分岐 3b — 無効な明示 --scope は state が勝つ場合でも無条件に検証される。
        if let Some(named) = input.scope()
            && !valid(named)?
        {
            return Err(Box::new(self.unknown_scope(named)));
        }
        if let Some(named) = state_scope {
            return match (valid(named)?, ScopeSlugView::parse(named)) {
                (true, Ok(scope)) => Ok(scope),
                _ => Err(Box::new(self.unknown_scope(named))),
            };
        }
        if let Some(named) = input.scope() {
            return ScopeSlugView::parse(named).map_err(|_| Box::new(self.unknown_scope(named)));
        }
        if let Some(text) = input.freeform()
            && let Some(scope) = self.infer_scope(text)?
        {
            return Ok(scope);
        }
        if let Some(value) = input.env_default_scope() {
            return match (valid(value)?, ScopeSlugView::parse(value)) {
                (true, Ok(scope)) => Ok(scope),
                _ => Err(Box::new(self.invalid_env_scope(value))),
            };
        }
        ScopeSlugView::parse(DEFAULT_SCOPE).map_err(|_| Box::new(self.unknown_scope(DEFAULT_SCOPE)))
    }

    /// 定義が名乗る scope の綴りを綴り順で並べる (拒否文言の材料)。
    fn valid_scopes(&self) -> Vec<String> {
        self.daos
            .scope()
            .find_all(DEFINITION_ID)
            .unwrap_or_default()
            .iter()
            .map(|view| view.scope().to_string())
            .collect()
    }

    fn unknown_scope(&self, scope: &str) -> Directive {
        Directive::Error {
            message: wording::unknown_scope(scope, &self.valid_scopes()),
        }
    }

    fn invalid_env_scope(&self, value: &str) -> Directive {
        Directive::Error {
            message: wording::invalid_env_scope(value, &self.valid_scopes()),
        }
    }

    /// キーワード推論 — 本文を語に分け、**長い窓から**引く (5 語超は抑止)。
    ///
    /// 語への分割と窓の生成は要求本文の**構文的な扱い**であり、語からスコープへの写像は
    /// 行 (`read_definition_scope_keyword`) が持つ。同じ語を複数のスコープが宣言したときの
    /// 先着は RMU が畳んである。
    fn infer_scope(&self, text: &str) -> Result<Option<ScopeSlugView>, Box<Directive>> {
        if text.split_whitespace().count() > KEYWORD_WORD_LIMIT {
            return Ok(None);
        }
        let lowered = text.to_lowercase();
        let words: Vec<&str> = lowered
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
            .filter(|word| !word.is_empty())
            .collect();
        let keywords = FindScopeKeywordUseCase::new(self.daos.scope_keyword());
        for width in (1..=words.len()).rev() {
            for window in words.windows(width) {
                let candidate = window.join(" ");
                let found = keywords
                    .execute(DEFINITION_ID, &candidate)
                    .map_err(|error| Box::new(Turn::unreadable(&error)))?;
                if let Some(scope) = found
                    && let Ok(parsed) = ScopeSlugView::parse(&scope)
                {
                    return Ok(Some(parsed));
                }
            }
        }
        Ok(None)
    }

    /// 分岐 5 — scope 変更 / 設定変更 (どちらも命令 1 本に修飾子をまとめる)。
    fn configuration_change(
        &self,
        input: &NextTurnInput,
        view: &NextTurnView,
    ) -> Option<Directive> {
        let depth = input.depth().map(str::to_string);
        let test_strategy = input.test_strategy().map(str::to_string);
        let review = input.review().map(str::to_string);
        if let Some(named) = input.scope() {
            let changed = FindScopeChangeUseCase::new(self.daos.scope_change())
                .execute(view.execution().execution_id(), named);
            match changed {
                Err(error) => return Some(Turn::unreadable(&error)),
                Ok(Some(change)) if change.kind() == "scope-change" => {
                    let Ok(scope) = ScopeSlugView::parse(named) else {
                        return Some(self.unknown_scope(named));
                    };
                    return Some(Directive::Print {
                        message: wording::scope_change(
                            &EngineCommand::ChangeScope {
                                scope,
                                depth,
                                test_strategy,
                                review,
                            }
                            .cli_spelling(),
                        ),
                    });
                }
                Ok(_) => {}
            }
        }
        if depth.is_some() || test_strategy.is_some() || review.is_some() {
            return Some(Directive::Print {
                message: wording::config_change(
                    &EngineCommand::ChangeConfig {
                        depth,
                        test_strategy,
                        review,
                    }
                    .cli_spelling(),
                ),
            });
        }
        None
    }

    /// 分岐 4b — 単一ステージ隔離モード。
    fn single(&self, input: &NextTurnInput, scope: &ScopeSlugView) -> Directive {
        let Some(stage) = input.stage() else {
            return Directive::Error {
                message: wording::SINGLE_REQUIRES_STAGE.to_string(),
            };
        };
        match FindRunStageUseCase::new(self.daos.run_stage()).execute(
            DEFINITION_ID,
            scope.as_str(),
            stage,
        ) {
            Err(error) => Turn::unreadable(&error),
            Ok(None) => Directive::Error {
                message: wording::unknown_stage(stage),
            },
            Ok(Some(row)) => {
                self.deliver(&row, scope, gate_of(row.gate_default()), true, None, None)
            }
        }
    }

    /// 分岐 7 — jump (state ありは解決命令、state なしは孤立 run-stage)。
    fn jump(
        &self,
        input: &NextTurnInput,
        scope: &ScopeSlugView,
        answer: Option<&NextTurnView>,
    ) -> Directive {
        match (input.stage(), input.phase(), answer) {
            // state あり: 受理判定は行が持つ (`read_next_jump` / `read_next_jump_phase`)。
            (Some(stage), _, Some(view)) => {
                let found = FindJumpUseCase::new(self.daos.jump(), self.daos.jump_phase())
                    .execute(view.execution().execution_id(), stage);
                Turn::jump_command(found, &wording::unknown_stage(stage))
            }
            (None, Some(phase), Some(view)) => {
                let lowered = phase.to_lowercase();
                let found = FindJumpUseCase::new(self.daos.jump(), self.daos.jump_phase())
                    .execute_phase(view.execution().execution_id(), &lowered);
                Turn::jump_command(found, &wording::unknown_phase(phase))
            }
            // state なし: 定義側の入口を引いて孤立 run-stage を届ける。
            (Some(stage), _, None) => self.isolated_run_stage(scope, stage),
            (None, Some(phase), None) => {
                let lowered = phase.to_lowercase();
                match FindPhaseEntryUseCase::new(self.daos.phase_entry()).execute(
                    DEFINITION_ID,
                    scope.as_str(),
                    &lowered,
                ) {
                    Err(error) => Turn::unreadable(&error),
                    Ok(None) => Directive::Error {
                        message: wording::no_stage_in_phase(&lowered),
                    },
                    Ok(Some(entry)) => {
                        let slug = entry.first_stage_slug().to_string();
                        self.isolated_run_stage(scope, &slug)
                    }
                }
            }
            // 分岐 7 は --stage / --phase のいずれかが前提 (防御的)。
            (None, None, _) => Directive::Error {
                message: wording::STAGE_AND_PHASE.to_string(),
            },
        }
    }

    /// jump の受理判定を命令 / 拒否へ写す。
    ///
    /// 行が無いのは「その名前の目的地が定義に無い」であり、文言は名指しの種類 (ステージか
    /// フェーズか) で決まる — **要求の形**なので呼出側が渡す。行が在って `outcome` が
    /// `refused` なら、拒否理由の綴りがそのまま行き先である。
    fn jump_command(
        found: Result<Option<JumpView>, ReadModelReadError>,
        absent: &str,
    ) -> Directive {
        match found {
            Err(error) => Turn::unreadable(&error),
            Ok(None) => Directive::Error {
                message: absent.to_string(),
            },
            Ok(Some(jump)) if jump.outcome() == "refused" => Directive::Error {
                message: match jump.refusal() {
                    Some("invalid-target") => wording::INIT_JUMP.to_string(),
                    _ => wording::unknown_stage(jump.target_slug()),
                },
            },
            Ok(Some(jump)) => match StageSlugView::parse(jump.target_slug()) {
                Ok(stage) => Directive::Print {
                    message: wording::resolve_jump(
                        &EngineCommand::ResolveJump { stage }.cli_spelling(),
                    ),
                },
                Err(_) => Directive::Error {
                    message: wording::unknown_stage(jump.target_slug()),
                },
            },
        }
    }

    /// state なしの jump — 定義 × scope の run-stage を孤立して届ける。
    fn isolated_run_stage(&self, scope: &ScopeSlugView, stage: &str) -> Directive {
        match FindRunStageUseCase::new(self.daos.run_stage()).execute(
            DEFINITION_ID,
            scope.as_str(),
            stage,
        ) {
            Err(error) => Turn::unreadable(&error),
            Ok(None) => Directive::Error {
                message: wording::unknown_stage(stage),
            },
            Ok(Some(row)) if row.phase() == "initialization" => Directive::Error {
                message: wording::INIT_JUMP.to_string(),
            },
            Ok(Some(row)) => {
                self.deliver(&row, scope, gate_of(row.gate_default()), false, None, None)
            }
        }
    }

    /// state なしの群 (7b / 8 / 9a / 9b)。
    fn birth_group(&self, input: &NextTurnInput) -> Directive {
        // 分岐 9a: 明示 --scope (有効性はラダーが検証済み)。
        if let Some(named) = input.scope() {
            return self.mint_intent(named, input.freeform(), input, false);
        }
        let Some(text) = input.freeform() else {
            // 分岐 9b: 何も名指しされていない。
            return Directive::Error {
                message: wording::NO_STATE.to_string(),
            };
        };
        // 分岐 7b: 位置引数が scope 名そのもの。
        let named = text.trim();
        match FindScopeUseCase::new(self.daos.scope()).execute(DEFINITION_ID, named) {
            Err(error) => return Turn::unreadable(&error),
            Ok(Some(_)) => {
                if input.records_exist_without_cursor() {
                    return Directive::Ask(AskDirective::new(
                        AskKind::IntentPick,
                        wording::INTENT_PICK.to_string(),
                    ));
                }
                return self.mint_intent(named, None, input, false);
            }
            Ok(None) => {}
        }
        // 分岐 8: キーワードヒット → scope 確認 / 非ヒット → compose 提案。
        match self.infer_scope(text) {
            Err(directive) => *directive,
            Ok(Some(inferred)) => {
                let cost = match self.scope_row(inferred.as_str()) {
                    Err(directive) => return *directive,
                    Ok(found) => found.map_or_else(String::new, |view| {
                        cost_clause(&view).map_or_else(String::new, |clause| format!(" - {clause}"))
                    }),
                };
                Directive::Ask(AskDirective::new(
                    AskKind::ScopeConfirm,
                    wording::scope_confirm(inferred.as_str(), text, &cost),
                ))
            }
            Ok(None) => match self.stock_examples() {
                Err(directive) => *directive,
                Ok(examples) => Directive::Ask(AskDirective::new(
                    AskKind::ComposeOffer,
                    wording::compose_offer(text, &examples),
                )),
            },
        }
    }

    /// intent 鋳造の名指し (分岐 4a / 7b / 9a)。
    fn mint_intent(
        &self,
        scope: &str,
        description: Option<&str>,
        input: &NextTurnInput,
        new_intent: bool,
    ) -> Directive {
        let Ok(parsed) = ScopeSlugView::parse(scope) else {
            return self.unknown_scope(scope);
        };
        let cost = match self.scope_row(scope) {
            Err(directive) => return *directive,
            Ok(found) => found.and_then(|view| cost_clause(&view)),
        };
        let description = description.map(str::trim).filter(|text| !text.is_empty());
        let spelled = EngineCommand::MintIntent {
            scope: parsed,
            description: description.map(str::to_string),
            depth: input.depth().map(str::to_string),
            test_strategy: input.test_strategy().map(str::to_string),
            review: input.review().map(str::to_string),
        }
        .cli_spelling();
        Directive::Print {
            message: wording::birth_print(
                &spelled,
                &cost.map_or_else(String::new, |clause| format!(" ({clause})")),
                description.is_some(),
                new_intent,
            ),
        }
    }

    /// compose 提案が例に挙げる既製 scope の綴り。
    fn stock_examples(&self) -> Result<String, Box<Directive>> {
        let stock = FindScopeUseCase::new(self.daos.scope())
            .execute_stock(DEFINITION_ID)
            .map_err(|error| Box::new(Turn::unreadable(&error)))?;
        Ok(stock
            .iter()
            .map(|view| format!("\"{}\"", view.scope()))
            .collect::<Vec<_>>()
            .join(", "))
    }

    /// scope カタログ 1 行。
    fn scope_row(&self, scope: &str) -> Result<Option<ScopeView>, Box<Directive>> {
        FindScopeUseCase::new(self.daos.scope())
            .execute(DEFINITION_ID, scope)
            .map_err(|error| Box::new(Turn::unreadable(&error)))
    }

    /// 分岐 10 — ハッピーパス。答えの綴りを directive に写すだけ。
    fn happy_path(
        &self,
        input: &NextTurnInput,
        view: &NextTurnView,
        scope: &ScopeSlugView,
    ) -> Directive {
        let answer = view.answer();
        match answer.decision_kind() {
            "run-stage" => match view.run_stage() {
                Some(row) => self.deliver(
                    row,
                    scope,
                    gate_of(answer.gated().unwrap_or(row.gate_default())),
                    false,
                    Some(view.execution().state_binding()),
                    view.plan().map(|plan| {
                        SteeringDeliveryView::new(plan.clone(), view.first_part().cloned())
                    }),
                ),
                None => Directive::Error {
                    message: wording::unknown_stage(answer.stage_slug().unwrap_or_default()),
                },
            },
            "done" => Directive::Done {
                reason: Some(wording::workflow_complete(
                    view.execution().cursor_slug().unwrap_or_default(),
                    view.execution().scope(),
                )),
            },
            "recover-skip-inconsistency" => {
                let slug = answer.stage_slug().unwrap_or_default();
                match StageSlugView::parse(slug) {
                    Ok(stage) => Directive::Print {
                        message: wording::recover_skip(
                            slug,
                            &EngineCommand::ReportSkipped { stage }.cli_spelling(),
                        ),
                    },
                    Err(_) => Directive::Error {
                        message: wording::unknown_stage(slug),
                    },
                }
            }
            "inconsistent-skip" => Directive::Error {
                message: wording::inconsistent_skip(
                    answer.stage_slug().unwrap_or_default(),
                    answer.checkbox().unwrap_or_default(),
                ),
            },
            // 分岐 9c — 稼働中の自由記述。提案 scope は本文からの推論で、当たらなければ
            // 解決済み scope に落ちる (どちらも行の値であって判断ではない)。
            "new-work-routing" => {
                let description = input.freeform().unwrap_or_default().to_string();
                let proposed = match self.infer_scope(&description) {
                    Err(directive) => return *directive,
                    Ok(inferred) => inferred.unwrap_or_else(|| scope.clone()),
                };
                Directive::Ask(
                    AskDirective::new(
                        AskKind::NewWorkRouting,
                        wording::NEW_WORK_ROUTING.to_string(),
                    )
                    .with_new_work(proposed.as_str(), description),
                )
            }
            // 先行分岐で消費済みの綴り (防御的 — 行き先はラダーが手前で決めている)。
            other => Directive::Error {
                message: format!("internal: a routing decision reached the happy path ({other})"),
            },
        }
    }

    /// run-stage を steering 連鎖経由で届ける (空計画なら bare run-stage)。
    ///
    /// 配信計画の 2 面 (計画とその第 1 部) は、答えの行を持つ分岐ならそれが運んでいるので
    /// 引き直さない。持たない分岐 (`--single` / state なし jump) だけが run-stage の FK から
    /// たどる。
    fn deliver(
        &self,
        row: &RunStageView,
        scope: &ScopeSlugView,
        gate: GateField,
        single: bool,
        state: Option<&str>,
        delivery: Option<SteeringDeliveryView>,
    ) -> Directive {
        let directive = match directive_drawing::run_stage(row, self.layout, gate, single) {
            Ok(directive) => directive,
            Err(message) => return Directive::Error { message },
        };
        let delivery = match delivery {
            Some(delivery) => Some(delivery),
            None => {
                match FindSteeringUseCase::new(self.daos.steering_plan(), self.daos.steering_part())
                    .execute(row.steering_plan_id())
                {
                    Ok(found) => found,
                    Err(error) => return Turn::unreadable(&error),
                }
            }
        };
        // 未パック — 台帳が無いので bare run-stage (別トランザクションなので不在は正常)。
        let Some(delivery) = delivery else {
            return Directive::RunStage(directive);
        };
        let bindings = directive_drawing::bindings(row, delivery.plan(), state);
        match delivery.first_part() {
            // 空計画 — 台帳だけを添えた bare run-stage。
            None => match directive_drawing::delivered_paths(delivery.plan()) {
                Ok(paths) => Directive::RunStage(directive.with_rules_in_context(paths)),
                Err(message) => Directive::Error { message },
            },
            Some(part) => match directive_drawing::load_steering(
                &directive,
                scope,
                delivery.plan(),
                part,
                &bindings,
            ) {
                Ok(directive) => directive,
                Err(message) => Directive::Error { message },
            },
        }
    }

    /// `continue` — token が運ぶ鍵で 3 表を引き、続きの部か終端 run-stage を描く。
    fn resume(&self, token: &ContinueToken) -> Directive {
        // state 束縛 — token が運ぶときだけ照合する (要求の形の分岐)。
        let state = token.bindings().state().map(|binding| binding.as_str());
        if let Some(binding) = state {
            match FindExecutionUseCase::new(self.daos.execution()).execute_by_state_binding(binding)
            {
                Err(error) => return Turn::unreadable(&error),
                Ok(None) => {
                    return Directive::Error {
                        message: wording::STATE_MOVED_ON.to_string(),
                    };
                }
                Ok(Some(_)) => {}
            }
        }
        let delivered = token.next_part_index();
        let wanted = delivered.next();
        let found = FindContinuationUseCase::new(
            self.daos.run_stage(),
            self.daos.steering_plan(),
            self.daos.steering_part(),
        )
        .execute(
            DEFINITION_ID,
            token.scope().as_str(),
            token.stage().as_str(),
            token.bindings().route().as_str(),
            token.bindings().directive().as_str(),
            token.bindings().bundle().as_str(),
            wanted.as_u32(),
        );
        let continuation = match found {
            Err(error) => return Turn::unreadable(&error),
            // 束縛が 1 つでもずれれば行に当たらない (fail-closed — 原因は区別しない)。
            Ok(None) => {
                return Directive::Error {
                    message: wording::STALE_CONTINUATION.to_string(),
                };
            }
            Ok(Some(continuation)) => continuation,
        };
        let rebuilt = match directive_drawing::run_stage(
            continuation.run_stage(),
            self.layout,
            token.gate(),
            token.is_single(),
        ) {
            Ok(directive) => directive.with_pins(token),
            Err(message) => return Directive::Error { message },
        };
        let bindings =
            directive_drawing::bindings(continuation.run_stage(), continuation.plan(), state);
        match continuation.next_part() {
            Some(part) => match directive_drawing::load_steering(
                &rebuilt,
                token.scope(),
                continuation.plan(),
                part,
                &bindings,
            ) {
                Ok(directive) => directive,
                Err(message) => Directive::Error { message },
            },
            // 続きが無い — 全部届いていれば終端、届いていなければ範囲外の要求である。
            None if delivered.as_u32() == continuation.plan().part_count() => {
                match directive_drawing::delivered_paths(continuation.plan()) {
                    Ok(paths) => Directive::RunStage(rebuilt.with_rules_in_context(paths)),
                    Err(message) => Directive::Error { message },
                }
            }
            None => Directive::Error {
                message: wording::PART_NOT_EXIST.to_string(),
            },
        }
    }
}

/// 要求の形 → 行のキーになる 4 綴り (`read_next_answer.request_kind`)。
///
/// 3 つの排他フラグと素の要求の 4 通りで、**フラグの有無だけ**で決まる。
fn request_kind(input: &NextTurnInput) -> &'static str {
    if input.is_resume() {
        "resume"
    } else if input.stage().is_some()
        || input.phase().is_some()
        || input.review().is_some()
        || input.new_intent().is_some()
    {
        "reentry"
    } else if input.freeform().is_some() {
        "free-text"
    } else {
        "bare"
    }
}

/// 行の真偽値をワイヤの 3 値へ写す (`unresolved` は行が表現しない)。
const fn gate_of(gated: bool) -> GateField {
    if gated {
        GateField::Gated
    } else {
        GateField::Ungated
    }
}

/// コスト節 (4 列が揃っている scope だけが持つ)。
fn cost_clause(view: &ScopeView) -> Option<String> {
    Some(wording::cost_clause(
        view.cost_total()?,
        view.cost_execute()?,
        view.cost_gates()?,
        view.cost_per_unit_stages()?,
    ))
}
