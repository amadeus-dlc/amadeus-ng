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
    if let Some(directive) = pre_guard(input) {
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
pub(crate) fn pre_guard(input: &NextTurnInput) -> Option<Directive> {
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
            // 行が在って拒否されている = 目的地は計画に載っているのに跳べない。集約が
            // `InvalidTarget` を返すのはゲートを持たない (= initialization) か scope 外の
            // ときなので、upstream の `INIT_JUMP_ERROR` がその逐語である。
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
                // ゲートは**答えの行**が正である (BR1.3 — 実行が持つ実効ゲート)。定義側の
                // 静的既定へ落ちるのは行が値を持たないときだけで、`decision_kind` が
                // `run-stage` の行は RMU が必ず `gated` を書くので実際には落ちない
                // (どちらも行の値であり、ここに判断は無い)。
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

#[cfg(test)]
mod tests {
    // テストでは想定外バリアントの即時失敗に panic! を使う。
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use std::fs;
    use std::io::ErrorKind;
    use std::path::{Path, PathBuf};

    use core_query_use_case::orchestration::{
        Bindings, BundleDigest, ContinueTokenBuilder, DirectiveDigest, ExecutionView,
        NextAnswerView, NounFamily, NounToken, PartIndex, ReadOnlyVerb, RouteDigest, StateBinding,
        SteeringPartView, SteeringPlanView,
    };

    use super::*;

    // -----------------------------------------------------------------------
    // フィクスチャ — 定義 3 入力を書いた根と、投影済みのリードモデル
    // -----------------------------------------------------------------------

    /// 定義 (`classic` / `express` の 2 scope、3 ステージ) と memory 層を書いたワークスペース。
    struct Workspace {
        root: tempfile::TempDir,
    }

    impl Workspace {
        fn create() -> Workspace {
            let workspace = Workspace {
                root: tempfile::tempdir().expect("一時ディレクトリ"),
            };
            let data = workspace.path(".claude/tools/data");
            let scopes = workspace.path(".claude/scopes");
            fs::create_dir_all(&data).expect("data");
            fs::create_dir_all(&scopes).expect("scopes");
            fs::write(
                data.join("harness.json"),
                r#"{"name":"claude","harnessDir":".claude","rulesSubdir":"rules"}"#,
            )
            .expect("harness.json");
            let node = |slug: &str, number: &str, name: &str, phase: &str| {
                format!(
                    r#"{{"slug":"{slug}","number":"{number}","name":"{name}","phase":"{phase}",
                         "execution":"ALWAYS","mode":"inline","lead_agent":"orchestrator",
                         "scopes":["classic","express"]}}"#
                )
            };
            fs::write(
                data.join("stage-graph.json"),
                format!(
                    "[{},{},{}]",
                    node("state-init", "0.1", "State Init", "initialization"),
                    node("domain-design", "1.1", "Domain Design", "inception"),
                    node("contract-design", "1.2", "Contract Design", "inception"),
                ),
            )
            .expect("stage-graph.json");
            fs::write(
                data.join("scope-grid.json"),
                r#"{"classic":{"stages":{"state-init":"EXECUTE","domain-design":"EXECUTE","contract-design":"EXECUTE"}},
                    "express":{"stages":{"state-init":"EXECUTE","domain-design":"EXECUTE","contract-design":"SKIP"}}}"#,
            )
            .expect("scope-grid.json");
            for scope in ["classic", "express"] {
                fs::write(
                    scopes.join(format!("aidlc-{scope}.md")),
                    format!("---\nname: {scope}\nkeywords: [\"{scope}-work\"]\n---\n\n# {scope}\n"),
                )
                .expect("scope identity");
            }
            let memory = workspace.path("aidlc/spaces/default/memory");
            fs::create_dir_all(&memory).expect("memory");
            fs::write(
                memory.join("org.md"),
                "# Org\n\n## Way of Working\n\n規則。\n",
            )
            .expect("org.md");
            fs::create_dir_all(workspace.path("aidlc/spaces/default/intents")).expect("intents");
            workspace
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.root.path().join(relative)
        }

        fn project_dir(&self) -> &Path {
            self.root.path()
        }

        fn layout(&self) -> Layout {
            Layout::resolve(self.project_dir())
        }

        async fn invoke(&self, argv0: &str, args: &[&str]) {
            let mut owned: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
            owned.push("--project-dir".to_string());
            owned.push(self.project_dir().to_string_lossy().into_owned());
            let completion = crate::runtime::run(argv0, &owned, self.project_dir()).await;
            assert_eq!(completion.code(), 0, "駆動は通る: {completion:?}");
        }

        /// 定義を投影させる (`next` の初回準備が走る) が intent は鋳造しない。
        async fn projected() -> Workspace {
            let workspace = Workspace::create();
            workspace.invoke("aidlc-orchestrate", &["next"]).await;
            workspace
        }

        /// 定義を投影し、`classic` の intent を 1 つ鋳造する。
        async fn minted() -> Workspace {
            let workspace = Workspace::create();
            workspace
                .invoke(
                    "aidlc-utility",
                    &["intent-create", "--scope", "classic", "--label", "demo"],
                )
                .await;
            workspace
        }

        /// active-intent カーソルを消す (ストアは残るので record だけが無くなる)。
        fn forget_cursor(&self) {
            fs::remove_file(self.path("aidlc/spaces/default/intents/active-intent"))
                .expect("カーソル");
        }
    }

    fn scope_of(name: &str) -> ScopeSlugView {
        ScopeSlugView::parse(name).expect("scope slug")
    }

    /// directive が運ぶ人間可読の文字列 (取り出しはここ 1 か所に閉じる — テスト衛生)。
    fn message_of(directive: &Directive) -> String {
        match directive {
            Directive::Error { message } | Directive::Print { message } => message.clone(),
            Directive::Parked { message, .. } => message.clone(),
            Directive::Done { reason } => reason.clone().unwrap_or_default(),
            Directive::Ask(ask) => ask.question().to_string(),
            Directive::RunStage(run) => run.stage().as_str().to_string(),
            Directive::LoadSteering(load) => load.stage().as_str().to_string(),
        }
    }

    /// directive の種類の綴り (ワイヤの `kind` に対応する観測名)。
    fn kind_of(directive: &Directive) -> &'static str {
        match directive {
            Directive::Error { .. } => "error",
            Directive::Print { .. } => "print",
            Directive::Parked { .. } => "parked",
            Directive::Done { .. } => "done",
            Directive::Ask(_) => "ask",
            Directive::RunStage(_) => "run-stage",
            Directive::LoadSteering(_) => "load-steering",
        }
    }

    /// 配信済みルールの台帳 (run-stage 以外はそもそも台帳を運ばない)。
    fn rules_in_context_of(directive: &Directive) -> Vec<String> {
        match directive {
            Directive::RunStage(run) => run.rules_in_context().to_vec(),
            Directive::Error { .. }
            | Directive::Print { .. }
            | Directive::Parked { .. }
            | Directive::Done { .. }
            | Directive::Ask(_)
            | Directive::LoadSteering(_) => Vec::new(),
        }
    }

    /// 答えの行 (`decision_kind` と、その分岐が使う材料だけを埋める)。
    fn answer_view(
        decision_kind: &str,
        stage_slug: Option<&str>,
        checkbox: Option<&str>,
    ) -> NextAnswerView {
        NextAnswerView::new(
            decision_kind.to_string(),
            Some(1),
            stage_slug.map(str::to_string),
            Some(true),
            checkbox.map(str::to_string),
            "execution-1".to_string(),
            None,
        )
    }

    /// 実行の行 (scope と state 束縛だけが分岐に効く)。
    fn execution_view(scope: &str) -> ExecutionView {
        ExecutionView::new(
            "execution-1".to_string(),
            "intent-1".to_string(),
            scope.to_string(),
            "running".to_string(),
            Some("domain-design".to_string()),
            None,
            false,
            "state-binding-1".to_string(),
        )
    }

    /// 答えの行だけを持つターン (run-stage / 計画は指さない)。
    fn turn_view(answer: NextAnswerView, scope: &str) -> NextTurnView {
        NextTurnView::new(answer, execution_view(scope), None, None, None)
    }

    // -----------------------------------------------------------------------
    // 前置ガード — リードモデルを 1 度も読まない
    // -----------------------------------------------------------------------

    /// パース失敗はそのまま逐語で中継する (他のどのガードより先)。
    #[test]
    fn the_pre_guard_relays_a_parse_error_before_anything_else() {
        let input = NextTurnInput::new()
            .with_parse_error("--review requires <adversarial|advisory|none>.")
            .with_stage("a")
            .with_phase("b");

        let directive = pre_guard(&input).expect("答えが決まる");

        assert_eq!(
            message_of(&directive),
            "--review requires <adversarial|advisory|none>."
        );
    }

    /// `--review` は 7 つのモードのどれとも併用できない。
    #[test]
    fn the_pre_guard_refuses_review_combined_with_any_other_mode() {
        let with_review = || NextTurnInput::new().with_review("advisory");
        let combinations = [
            with_review().with_read_only(ReadOnlyVerb::Status),
            with_review().with_noun_token(NounToken::new(NounFamily::Workspace, Vec::new())),
            with_review().with_compose(),
            with_review().with_single(),
            with_review().with_stage("domain-design"),
            with_review().with_phase("inception"),
            with_review().with_resume(),
        ];

        for input in combinations {
            let directive = pre_guard(&input).expect("答えが決まる");
            assert_eq!(message_of(&directive), wording::REVIEW_COMBINATION);
        }
    }

    /// Kiro の roll-forward ラッチを観測した素の `next` は、理由を付けずに畳む。
    #[test]
    fn the_pre_guard_closes_a_kiro_latched_bare_next_without_a_reason() {
        let input = NextTurnInput::new().with_kiro_latch_bare_next();

        let directive = pre_guard(&input).expect("答えが決まる");

        assert_eq!(directive, Directive::Done { reason: None });
        assert_eq!(kind_of(&directive), "done");
        assert_eq!(message_of(&directive), "", "理由を付けずに畳む");
    }

    /// 読み取り専用ユーティリティは綴りを名指し、ワークフローを進めるなと言う。
    #[test]
    fn the_pre_guard_names_the_read_only_utility_by_its_cli_spelling() {
        let input = NextTurnInput::new().with_read_only(ReadOnlyVerb::Doctor);

        let directive = pre_guard(&input).expect("答えが決まる");

        assert_eq!(
            message_of(&directive),
            wording::read_only("aidlc-utility doctor")
        );
    }

    /// 名詞トークンは逐語で通し、終端ユーティリティとして名指す。
    #[test]
    fn the_pre_guard_passes_noun_tokens_through_verbatim() {
        let input = NextTurnInput::new().with_noun_token(NounToken::new(
            NounFamily::Workspace,
            vec!["intent".to_string(), "list".to_string()],
        ));

        let directive = pre_guard(&input).expect("答えが決まる");

        assert_eq!(
            message_of(&directive),
            wording::terminal_utility("aidlc-utility intent list")
        );
    }

    /// `--stage` と `--phase` の併用は前置で拒む。
    #[test]
    fn the_pre_guard_refuses_stage_and_phase_together() {
        let input = NextTurnInput::new().with_stage("a").with_phase("inception");

        let directive = pre_guard(&input).expect("答えが決まる");

        assert_eq!(message_of(&directive), wording::STAGE_AND_PHASE);
    }

    /// どのガードにも当たらない要求は前置で答えを持たない (ラダーへ進む)。
    #[test]
    fn the_pre_guard_has_no_answer_for_an_ordinary_request() {
        assert_eq!(pre_guard(&NextTurnInput::new()), None);
    }

    /// `next` は自分でも前置ガードを引く (合成ルートを経ずに呼ばれても答えは同じ)。
    #[tokio::test]
    async fn next_applies_the_pre_guard_itself() {
        let workspace = Workspace::create();
        let input = NextTurnInput::new().with_parse_error("boom");

        let directive = next(&workspace.layout(), &input);

        assert_eq!(message_of(&directive), "boom");
    }

    // -----------------------------------------------------------------------
    // 引当の口を開く — 開けなければ描く directive が答えになる
    // -----------------------------------------------------------------------

    /// 空間名として成立しない active-space は既定へ落とさず止める。
    #[test]
    fn an_invalid_active_space_stops_the_turn_before_the_store_is_opened() {
        let workspace = Workspace::create();
        fs::write(workspace.path("aidlc/active-space"), "../escape\n").expect("space カーソル");

        let directive = next(&workspace.layout(), &NextTurnInput::new());

        assert_eq!(
            message_of(&directive),
            wording::invalid_active_space("../escape")
        );
    }

    /// ストアが無ければ「引けない」であって「行が無い」ではない。
    #[test]
    fn an_unopenable_store_is_reported_with_its_path_and_classification() {
        let workspace = Workspace::create();

        let directive = next(&workspace.layout(), &NextTurnInput::new());

        let message = message_of(&directive);
        assert!(
            message.starts_with("Read model not readable at "),
            "{message}"
        );
        assert!(message.contains("aidlc/spaces/default"), "{message}");
    }

    /// `continue` も同じ口を開くので、開けなければ同じ答えになる。
    #[test]
    fn continue_reports_the_same_unopenable_store() {
        let workspace = Workspace::create();
        let token = ContinueTokenBuilder::new(
            StageSlugView::parse("domain-design").unwrap(),
            scope_of("classic"),
            PartIndex::from_raw(1).unwrap(),
            Bindings::new(
                BundleDigest::new("b"),
                DirectiveDigest::new("d"),
                RouteDigest::new("r"),
                None,
            ),
            GateField::Gated,
        )
        .build();

        let directive = resume(&workspace.layout(), Some(&token));

        assert!(
            message_of(&directive).starts_with("Read model not readable at "),
            "{directive:?}"
        );
    }

    /// トークンを開封できなかった `continue` は fail-closed の逐語になる。
    #[test]
    fn continue_without_a_verified_token_fails_closed() {
        let workspace = Workspace::create();

        let directive = resume(&workspace.layout(), None);

        assert_eq!(message_of(&directive), wording::INVALID_CONTINUATION_TOKEN);
    }

    /// 引当の失敗は分類と所在だけを材料に運ぶ (所在を名指せない失敗は空欄になる)。
    #[test]
    fn a_read_failure_carries_only_its_classification_and_place() {
        let located = ReadModelReadError::new(ErrorKind::InvalidData, Some(PathBuf::from("/w/db")));
        assert_eq!(
            message_of(&Turn::unreadable(&located)),
            wording::read_model_unreadable("/w/db", &ErrorKind::InvalidData.to_string())
        );

        let anonymous = ReadModelReadError::broken_projection();
        assert_eq!(
            message_of(&Turn::unreadable(&anonymous)),
            wording::read_model_unreadable("", &ErrorKind::InvalidData.to_string())
        );
    }

    // -----------------------------------------------------------------------
    // 答えの綴りをそのまま描く静的な写し
    // -----------------------------------------------------------------------

    /// park は位置を名乗って止まる (`parked` directive はステージを型で運ぶ)。
    #[test]
    fn a_parked_answer_names_the_stage_it_stopped_at() {
        let view = turn_view(
            answer_view("parked", Some("domain-design"), None),
            "classic",
        );

        let directive = Turn::parked(&view);

        assert_eq!(kind_of(&directive), "parked");
        assert_eq!(message_of(&directive), wording::parked("domain-design"));
        assert_eq!(
            stage_of_parked(&directive),
            Some("domain-design"),
            "ステージは文言だけでなく型でも運ぶ"
        );
    }

    /// park directive が型で運ぶステージ slug。
    fn stage_of_parked(directive: &Directive) -> Option<&str> {
        match directive {
            Directive::Parked { stage, .. } => Some(stage.as_str()),
            Directive::Error { .. }
            | Directive::Print { .. }
            | Directive::Done { .. }
            | Directive::Ask(_)
            | Directive::RunStage(_)
            | Directive::LoadSteering(_) => None,
        }
    }

    /// park 位置の綴りが slug として読めなければ未知ステージとして拒む。
    #[test]
    fn a_parked_answer_with_an_unreadable_slug_is_refused() {
        let view = turn_view(answer_view("parked", Some("Not A Slug"), None), "classic");

        let directive = Turn::parked(&view);

        assert_eq!(kind_of(&directive), "error");
        assert_eq!(message_of(&directive), wording::unknown_stage("Not A Slug"));
        assert_eq!(
            stage_of_parked(&directive),
            None,
            "拒否はステージを型で運ばない"
        );
    }

    /// jump の行が無いのは「その名前の目的地が無い」— 文言は呼出側が渡す。
    #[test]
    fn a_missing_jump_row_uses_the_absence_wording_the_caller_supplies() {
        assert_eq!(
            message_of(&Turn::jump_command(Ok(None), "absent")),
            "absent"
        );
    }

    /// jump の引当が失敗したら、不在ではなく読取失敗として答える。
    #[test]
    fn a_failed_jump_lookup_is_reported_as_a_read_failure() {
        let error = ReadModelReadError::new(ErrorKind::WouldBlock, None);

        assert_eq!(
            Turn::jump_command(Err(error.clone()), "absent"),
            Turn::unreadable(&error)
        );
    }

    /// `invalid-target` の拒否は init ジャンプの逐語になる。
    #[test]
    fn a_jump_refused_as_an_invalid_target_names_the_initialization_guard() {
        let refused = JumpView::new(
            0,
            "state-init".to_string(),
            "refused".to_string(),
            Some("invalid-target".to_string()),
        );

        assert_eq!(
            message_of(&Turn::jump_command(Ok(Some(refused)), "absent")),
            wording::INIT_JUMP
        );
    }

    /// それ以外の拒否は目的地を名指した未知ステージになる。
    #[test]
    fn any_other_jump_refusal_names_the_target_stage() {
        let refused = JumpView::new(
            2,
            "contract-design".to_string(),
            "refused".to_string(),
            Some("out-of-scope".to_string()),
        );

        assert_eq!(
            message_of(&Turn::jump_command(Ok(Some(refused)), "absent")),
            wording::unknown_stage("contract-design")
        );
    }

    /// 受理された jump は跳ばずに解決命令を名指す。
    #[test]
    fn an_accepted_jump_names_the_resolve_command() {
        let accepted = JumpView::new(
            2,
            "contract-design".to_string(),
            "forward".to_string(),
            None,
        );

        assert_eq!(
            message_of(&Turn::jump_command(Ok(Some(accepted)), "absent")),
            wording::resolve_jump("aidlc-jump resolve --stage contract-design")
        );
    }

    /// 受理されても目的地の綴りが読めなければ未知ステージとして拒む。
    #[test]
    fn an_accepted_jump_to_an_unreadable_slug_is_refused() {
        let accepted = JumpView::new(2, "Not A Slug".to_string(), "forward".to_string(), None);

        assert_eq!(
            message_of(&Turn::jump_command(Ok(Some(accepted)), "absent")),
            wording::unknown_stage("Not A Slug")
        );
    }

    // -----------------------------------------------------------------------
    // 要求の形 → 行の鍵
    // -----------------------------------------------------------------------

    /// 4 綴りは**フラグの有無だけ**で決まり、`resume` が最優先である。
    #[test]
    fn the_request_kind_is_decided_by_the_flags_alone() {
        assert_eq!(request_kind(&NextTurnInput::new()), "bare");
        assert_eq!(
            request_kind(&NextTurnInput::new().with_freeform("fix the crash")),
            "free-text"
        );
        for reentry in [
            NextTurnInput::new().with_stage("domain-design"),
            NextTurnInput::new().with_phase("inception"),
            NextTurnInput::new().with_review("advisory"),
            NextTurnInput::new().with_new_intent("new work"),
        ] {
            assert_eq!(request_kind(&reentry), "reentry");
        }
        assert_eq!(
            request_kind(&NextTurnInput::new().with_resume().with_stage("a")),
            "resume",
            "resume は他のどの形よりも先に決まる"
        );
    }

    /// 行の真偽値はワイヤの 2 値へそのまま写る。
    #[test]
    fn the_row_gate_maps_onto_the_wire_spelling() {
        assert_eq!(gate_of(true), GateField::Gated);
        assert_eq!(gate_of(false), GateField::Ungated);
    }

    // -----------------------------------------------------------------------
    // scope 解決ラダー — どの候補を先に試すかの順序
    // -----------------------------------------------------------------------

    /// state が名乗る scope が定義に無ければ、既定へ落とさず拒む。
    #[tokio::test]
    async fn a_state_scope_that_the_definition_does_not_declare_is_refused() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("投影済みのストアは開ける");

        let refused = turn
            .resolve_scope(Some("nope"), &NextTurnInput::new())
            .expect_err("拒否になる");

        assert_eq!(
            message_of(&refused),
            wording::unknown_scope("nope", &["classic".to_string(), "express".to_string()])
        );
    }

    /// 自由記述のキーワードは、明示の名指しが無いときだけ scope を決める。
    #[tokio::test]
    async fn a_keyword_in_the_free_text_resolves_the_scope() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        let resolved = turn
            .resolve_scope(None, &NextTurnInput::new().with_freeform("express-work"))
            .expect("キーワードが当たる");

        assert_eq!(resolved.as_str(), "express");
    }

    /// 語が多い本文は説明文とみなし、キーワード推論を抑止する。
    #[tokio::test]
    async fn a_long_free_text_suppresses_keyword_inference() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        assert_eq!(
            turn.infer_scope("express-work and five more words here")
                .expect("引ける"),
            None,
            "6 語は抑止の閾値を超える"
        );
    }

    /// 環境変数の既定 scope は、他のどの候補も無いときに効く。
    #[tokio::test]
    async fn the_environment_default_scope_is_the_last_candidate_before_the_built_in_default() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        let resolved = turn
            .resolve_scope(
                None,
                &NextTurnInput::new().with_env_default_scope("express"),
            )
            .expect("有効な既定");

        assert_eq!(resolved.as_str(), "express");
    }

    /// 定義に無い環境変数の既定 scope は、変数名を名乗って拒む。
    #[tokio::test]
    async fn an_environment_default_scope_outside_the_definition_is_refused_by_name() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        let refused = turn
            .resolve_scope(None, &NextTurnInput::new().with_env_default_scope("nope"))
            .expect_err("拒否になる");

        assert_eq!(
            message_of(&refused),
            wording::invalid_env_scope("nope", &["classic".to_string(), "express".to_string()])
        );
    }

    /// 候補が 1 つも無ければ既定 scope (`classic`) に落ちる。
    #[tokio::test]
    async fn an_unnamed_turn_falls_back_to_the_built_in_default_scope() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        assert_eq!(
            turn.resolve_scope(None, &NextTurnInput::new())
                .expect("既定")
                .as_str(),
            DEFAULT_SCOPE
        );
    }

    // -----------------------------------------------------------------------
    // 設定変更 — state と同じ scope は変更にならない
    // -----------------------------------------------------------------------

    /// state と同じ scope を名指しただけで修飾子も無ければ、変更の命令は出ない。
    #[tokio::test]
    async fn naming_the_scope_the_state_already_carries_is_not_a_change() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let view = turn
            .answer(turn.execution_id().as_deref(), &NextTurnInput::new())
            .expect("引ける")
            .expect("鋳造済みなので答えがある");

        assert_eq!(
            turn.configuration_change(&NextTurnInput::new().with_scope("classic"), &view),
            None
        );
    }

    // -----------------------------------------------------------------------
    // state なしの群 (誕生) — 何も鋳造されていないワークスペース
    // -----------------------------------------------------------------------

    /// 何も名指されていなければ「state が無い」と言って始め方を案内する。
    #[tokio::test]
    async fn an_unnamed_birth_turn_reports_that_there_is_no_state() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        assert_eq!(
            message_of(&turn.birth_group(&NextTurnInput::new())),
            wording::NO_STATE
        );
    }

    /// 位置引数が scope 名そのものなら、その scope で鋳造の命令を名指す。
    #[tokio::test]
    async fn a_positional_scope_name_births_that_scope() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        let directive = turn.birth_group(&NextTurnInput::new().with_freeform("express"));

        assert_eq!(kind_of(&directive), "print");
        let message = message_of(&directive);
        assert!(
            message.contains("intent-create --scope express"),
            "{message}"
        );
        assert!(
            message.contains("then re-run `next` to continue."),
            "{message}"
        );
    }

    /// カーソルの無い記録が見つかったら、鋳造ではなくどの intent を選ぶかを問う。
    #[tokio::test]
    async fn records_without_a_cursor_ask_which_intent_to_activate() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let input = NextTurnInput::new()
            .with_freeform("express")
            .with_records_without_cursor();

        let directive = turn.birth_group(&input);

        assert_eq!(kind_of(&directive), "ask");
        assert_eq!(message_of(&directive), wording::INTENT_PICK);
    }

    /// キーワードが当たれば、その scope でよいかをコスト節つきで確認する。
    #[tokio::test]
    async fn a_keyword_hit_asks_to_confirm_that_scope_with_its_cost() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        let directive = turn.birth_group(&NextTurnInput::new().with_freeform("classic-work"));

        assert_eq!(kind_of(&directive), "ask");
        let question = message_of(&directive);
        assert!(
            question.starts_with("This looks like \"classic\" work"),
            "{question}"
        );
        assert!(question.contains(" - "), "コスト節が付く: {question}");
    }

    /// どの既製 scope も当たらなければ compose を提案し、既製の綴りを例に挙げる。
    #[tokio::test]
    async fn free_text_that_matches_nothing_offers_compose_with_the_stock_examples() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        let directive = turn.birth_group(&NextTurnInput::new().with_freeform("frobnicate"));

        assert_eq!(kind_of(&directive), "ask");
        let question = message_of(&directive);
        assert!(
            question.starts_with("None of the ready-made plans is an obvious fit"),
            "{question}"
        );
    }

    /// slug として読めない scope 名での鋳造は、未知 scope として拒む。
    #[tokio::test]
    async fn birthing_an_unreadable_scope_name_is_refused() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        let directive = turn.mint_intent("Not A Slug", None, &NextTurnInput::new(), false);

        assert_eq!(
            message_of(&directive),
            wording::unknown_scope(
                "Not A Slug",
                &["classic".to_string(), "express".to_string()]
            )
        );
    }

    // -----------------------------------------------------------------------
    // state なしの jump — 定義側の入口を引いて孤立 run-stage を届ける
    // -----------------------------------------------------------------------

    /// state を持たない `--stage` は、定義の行から孤立した配信を組む。
    #[tokio::test]
    async fn a_stateless_stage_jump_delivers_an_isolated_run_stage() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let input = NextTurnInput::new().with_stage("contract-design");

        let directive = turn.jump(&input, &scope_of("classic"), None);

        assert_eq!(kind_of(&directive), "load-steering");
        assert_eq!(message_of(&directive), "contract-design");
    }

    /// initialization フェーズのステージへは、state が無くても跳べない。
    #[tokio::test]
    async fn a_stateless_jump_into_initialization_is_refused() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        assert_eq!(
            message_of(&turn.isolated_run_stage(&scope_of("classic"), "state-init")),
            wording::INIT_JUMP
        );
    }

    /// 定義に無いステージは未知として拒む。
    #[tokio::test]
    async fn a_stateless_jump_to_an_unknown_stage_is_refused() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        assert_eq!(
            message_of(&turn.isolated_run_stage(&scope_of("classic"), "nowhere")),
            wording::unknown_stage("nowhere")
        );
    }

    /// state を持たない `--phase` は、そのフェーズの入口ステージを届ける。
    #[tokio::test]
    async fn a_stateless_phase_jump_delivers_the_entry_stage_of_that_phase() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let input = NextTurnInput::new().with_phase("INCEPTION");

        let directive = turn.jump(&input, &scope_of("classic"), None);

        assert_eq!(kind_of(&directive), "load-steering");
        assert_eq!(
            message_of(&directive),
            "domain-design",
            "フェーズ名は小文字へ畳んで引く"
        );
    }

    /// in-scope のステージが 1 つも無いフェーズは、そのフェーズ名で拒む。
    #[tokio::test]
    async fn a_stateless_phase_jump_into_an_empty_phase_is_refused() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let input = NextTurnInput::new().with_phase("operation");

        assert_eq!(
            message_of(&turn.jump(&input, &scope_of("classic"), None)),
            wording::no_stage_in_phase("operation")
        );
    }

    /// jump は `--stage` / `--phase` のどちらかが前提である (防御的な拒否)。
    #[tokio::test]
    async fn a_jump_without_a_stage_or_a_phase_is_refused() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        assert_eq!(
            message_of(&turn.jump(&NextTurnInput::new(), &scope_of("classic"), None)),
            wording::STAGE_AND_PHASE
        );
    }

    /// `--single` はステージを要る。
    #[tokio::test]
    async fn single_without_a_stage_is_refused() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        assert_eq!(
            message_of(&turn.single(&NextTurnInput::new(), &scope_of("classic"))),
            wording::SINGLE_REQUIRES_STAGE
        );
    }

    /// `--single` に定義外のステージを渡せば未知として拒む。
    #[tokio::test]
    async fn single_with_an_unknown_stage_is_refused() {
        let workspace = Workspace::projected().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let input = NextTurnInput::new().with_single().with_stage("nowhere");

        assert_eq!(
            message_of(&turn.single(&input, &scope_of("classic"))),
            wording::unknown_stage("nowhere")
        );
    }

    // -----------------------------------------------------------------------
    // ハッピーパス — 答えの綴りがそのまま行き先である
    // -----------------------------------------------------------------------

    /// `run-stage` の答えが材料の行を指していなければ、未知ステージとして拒む。
    #[tokio::test]
    async fn a_run_stage_answer_without_its_row_is_refused_as_an_unknown_stage() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let view = turn_view(
            answer_view("run-stage", Some("domain-design"), None),
            "classic",
        );

        let directive = turn.happy_path(&NextTurnInput::new(), &view, &scope_of("classic"));

        assert_eq!(
            message_of(&directive),
            wording::unknown_stage("domain-design")
        );
    }

    /// 回復可能な SKIP 不整合は、走らせるなと言って回復の命令を名指す。
    #[tokio::test]
    async fn a_recoverable_skip_inconsistency_names_the_recovery_command() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let view = turn_view(
            answer_view("recover-skip-inconsistency", Some("contract-design"), None),
            "classic",
        );

        let message =
            message_of(&turn.happy_path(&NextTurnInput::new(), &view, &scope_of("classic")));

        assert!(
            message.starts_with("Stage \"contract-design\" is SKIP"),
            "{message}"
        );
        assert!(
            message.contains("report --stage contract-design --result skipped"),
            "{message}"
        );
    }

    /// 回復の答えでも、名指されたステージの綴りが読めなければ未知として拒む。
    #[tokio::test]
    async fn a_recoverable_skip_with_an_unreadable_slug_is_refused() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let view = turn_view(
            answer_view("recover-skip-inconsistency", Some("Not A Slug"), None),
            "classic",
        );

        assert_eq!(
            message_of(&turn.happy_path(&NextTurnInput::new(), &view, &scope_of("classic"))),
            wording::unknown_stage("Not A Slug")
        );
    }

    /// 回復経路の無い SKIP 不整合は、カーソルの綴りごと拒む。
    #[tokio::test]
    async fn an_unrecoverable_skip_inconsistency_quotes_the_cursor_state() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let view = turn_view(
            answer_view(
                "inconsistent-skip",
                Some("contract-design"),
                Some("in-progress"),
            ),
            "classic",
        );

        assert_eq!(
            message_of(&turn.happy_path(&NextTurnInput::new(), &view, &scope_of("classic"))),
            wording::inconsistent_skip("contract-design", "in-progress")
        );
    }

    /// 手前のラダーで消費済みの綴りがここへ来たら、投影の破綻として名乗る。
    #[tokio::test]
    async fn a_spelling_the_ladder_already_consumed_is_reported_as_an_internal_fault() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let view = turn_view(answer_view("resume-menu", None, None), "classic");

        assert_eq!(
            message_of(&turn.happy_path(&NextTurnInput::new(), &view, &scope_of("classic"))),
            "internal: a routing decision reached the happy path (resume-menu)"
        );
    }

    // -----------------------------------------------------------------------
    // 配信 — 台帳の有無で bare / 台帳つき / 連鎖の 3 形に分かれる
    // -----------------------------------------------------------------------

    /// `read_run_stage` の 1 行を実際のストアから引く。
    fn row_of(turn: &Turn<'_>, scope: &str, stage: &str) -> RunStageView {
        FindRunStageUseCase::new(turn.daos.run_stage())
            .execute(DEFINITION_ID, scope, stage)
            .expect("引ける")
            .expect("定義にある")
    }

    /// 配信計画を指さない run-stage の行 (台帳の不在を作るための最小の行)。
    fn detached_row(plan_id: &str) -> RunStageView {
        RunStageView::new(
            "row-1".to_string(),
            DEFINITION_ID.to_string(),
            "classic".to_string(),
            "domain-design".to_string(),
            "inception".to_string(),
            plan_id.to_string(),
            "orchestrator".to_string(),
            "[]".to_string(),
            "inline".to_string(),
            true,
            "[]".to_string(),
            "domain-design.md".to_string(),
            "inception/domain-design/memory.md".to_string(),
            "[]".to_string(),
            "[]".to_string(),
            "[]".to_string(),
            None,
            None,
            None,
            "[]".to_string(),
            None,
            "route".to_string(),
            "directive".to_string(),
        )
    }

    /// 記録が解決できないターンでは run-stage を組めない (材料だけを運ぶ診断になる)。
    #[tokio::test]
    async fn a_delivery_without_a_record_reports_that_it_cannot_assemble_the_run_stage() {
        let workspace = Workspace::minted().await;
        workspace.forget_cursor();
        let layout = workspace.layout();
        assert_eq!(layout.record_dir(), None, "カーソルは消してある");
        let turn = Turn::open(&layout).expect("ストア");
        let row = row_of(&turn, "classic", "domain-design");

        let directive = turn.deliver(
            &row,
            &scope_of("classic"),
            GateField::Gated,
            false,
            None,
            None,
        );

        assert_eq!(
            message_of(&directive),
            "No workspace record was resolved for run-stage assembly."
        );
    }

    /// 台帳がまだパックされていなければ素の run-stage を届ける (別トランザクションなので正常)。
    #[tokio::test]
    async fn a_stage_whose_bundle_is_not_packed_yet_is_delivered_bare() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let row = detached_row("no-such-plan");

        let directive = turn.deliver(
            &row,
            &scope_of("classic"),
            GateField::Gated,
            false,
            None,
            None,
        );

        assert_eq!(kind_of(&directive), "run-stage");
        assert_eq!(message_of(&directive), "domain-design");
        assert!(
            rules_in_context_of(&directive).is_empty(),
            "台帳が無いので配信済みパスも無い: {directive:?}"
        );
    }

    /// 空計画は、配信済みパスの台帳だけを添えた run-stage になる。
    #[tokio::test]
    async fn an_empty_plan_delivers_a_run_stage_carrying_only_its_ledger() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let row = row_of(&turn, "classic", "domain-design");
        let plan = SteeringPlanView::new(
            "plan-1".to_string(),
            "inception".to_string(),
            "bundle".to_string(),
            0,
            r#"["memory/org.md"]"#.to_string(),
        );

        let directive = turn.deliver(
            &row,
            &scope_of("classic"),
            GateField::Gated,
            false,
            None,
            Some(SteeringDeliveryView::new(plan, None)),
        );

        assert_eq!(kind_of(&directive), "run-stage");
        assert_eq!(
            rules_in_context_of(&directive),
            ["memory/org.md".to_string()]
        );
    }

    /// 台帳の列が 1 行 JSON として開けなければ、その列と値を材料に拒む。
    #[tokio::test]
    async fn an_unreadable_delivered_paths_column_is_reported_with_its_value() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let row = row_of(&turn, "classic", "domain-design");
        let plan = SteeringPlanView::new(
            "plan-1".to_string(),
            "inception".to_string(),
            "bundle".to_string(),
            0,
            "not-json".to_string(),
        );

        let directive = turn.deliver(
            &row,
            &scope_of("classic"),
            GateField::Gated,
            false,
            None,
            Some(SteeringDeliveryView::new(plan, None)),
        );

        assert_eq!(
            message_of(&directive),
            "Read model row is not readable: delivered_paths = \"not-json\"."
        );
        assert!(
            rules_in_context_of(&directive).is_empty(),
            "拒否には台帳が付かない"
        );
    }

    /// 部の本文が開けなければ、連鎖を描かずにその列を材料に拒む。
    #[tokio::test]
    async fn an_unreadable_rules_content_column_stops_the_steering_chain() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let row = row_of(&turn, "classic", "domain-design");
        let plan = SteeringPlanView::new(
            "plan-1".to_string(),
            "inception".to_string(),
            "bundle".to_string(),
            1,
            "[]".to_string(),
        );
        let part = SteeringPartView::new(
            "plan-1".to_string(),
            "inception".to_string(),
            1,
            "not-json".to_string(),
        );

        let directive = turn.deliver(
            &row,
            &scope_of("classic"),
            GateField::Gated,
            false,
            Some("state-binding-1"),
            Some(SteeringDeliveryView::new(plan, Some(part))),
        );

        assert_eq!(
            message_of(&directive),
            "Read model row is not readable: rules_content = \"not-json\"."
        );
    }

    // -----------------------------------------------------------------------
    // `continue` — token が運ぶ鍵で 3 表を引く (ずれは全部 fail-closed)
    // -----------------------------------------------------------------------

    /// 実際の配信で使われる 4 束縛を、鋳造済みのストアから読み出して組む。
    fn live_bindings(turn: &Turn<'_>, with_state: Option<&str>) -> (Bindings, u32) {
        let row = row_of(turn, "classic", "domain-design");
        let plan = FindSteeringUseCase::new(turn.daos.steering_plan(), turn.daos.steering_part())
            .execute(row.steering_plan_id())
            .expect("引ける")
            .expect("鋳造済みなので計画がある");
        let bindings = Bindings::new(
            BundleDigest::new(plan.plan().bundle_digest()),
            DirectiveDigest::new(row.directive_digest()),
            RouteDigest::new(row.route_digest()),
            with_state.map(StateBinding::new),
        );
        (bindings, plan.plan().part_count())
    }

    /// 実行の現在の state 束縛。
    fn live_state_binding(turn: &Turn<'_>) -> String {
        turn.answer(turn.execution_id().as_deref(), &NextTurnInput::new())
            .expect("引ける")
            .expect("鋳造済み")
            .execution()
            .state_binding()
            .to_string()
    }

    fn token_for(bindings: Bindings, part: u32) -> ContinueToken {
        ContinueTokenBuilder::new(
            StageSlugView::parse("domain-design").expect("slug"),
            scope_of("classic"),
            PartIndex::from_raw(part).expect("1 始まり"),
            bindings,
            GateField::Gated,
        )
        .build()
    }

    /// token が運ぶ state 束縛が現行の実行に当たらなければ、位置が動いたと答える。
    #[tokio::test]
    async fn a_token_whose_state_binding_no_longer_matches_says_the_position_moved_on() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let (bindings, _) = live_bindings(&turn, Some("no-such-binding"));

        assert_eq!(
            message_of(&turn.resume(&token_for(bindings, 1))),
            wording::STATE_MOVED_ON
        );
    }

    /// 束縛が 1 つでもずれた token は行に当たらない (原因は区別しない)。
    #[tokio::test]
    async fn a_token_with_a_drifted_binding_is_refused_as_stale() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let drifted = Bindings::new(
            BundleDigest::new("0".repeat(64)),
            DirectiveDigest::new("0".repeat(64)),
            RouteDigest::new("0".repeat(64)),
            None,
        );

        assert_eq!(
            message_of(&turn.resume(&token_for(drifted, 1))),
            wording::STALE_CONTINUATION
        );
    }

    /// 全部届いた token は連鎖を閉じ、台帳を添えた終端 run-stage になる。
    #[tokio::test]
    async fn a_token_that_has_received_every_part_closes_with_the_terminal_run_stage() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let state = live_state_binding(&turn);
        let (bindings, part_count) = live_bindings(&turn, Some(&state));

        let directive = turn.resume(&token_for(bindings, part_count));

        assert_eq!(kind_of(&directive), "run-stage");
        assert!(
            !rules_in_context_of(&directive).is_empty(),
            "終端は配信済みパスの台帳を添える: {directive:?}"
        );
    }

    /// 存在しない部を求める token は、範囲外として拒む。
    #[tokio::test]
    async fn a_token_asking_for_a_part_beyond_the_plan_is_refused() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let (bindings, part_count) = live_bindings(&turn, None);

        assert_eq!(
            message_of(&turn.resume(&token_for(bindings, part_count + 1))),
            wording::PART_NOT_EXIST
        );
    }

    /// 記録が解決できなければ、continue でも run-stage を組み直せない。
    #[tokio::test]
    async fn a_continue_without_a_record_cannot_rebuild_the_run_stage() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let bindings = {
            let turn = Turn::open(&layout).expect("ストア");
            live_bindings(&turn, None).0
        };
        workspace.forget_cursor();
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        assert_eq!(
            message_of(&turn.resume(&token_for(bindings, 1))),
            "No workspace record was resolved for run-stage assembly."
        );
    }

    // -----------------------------------------------------------------------
    // 引けない媒体 — 「行が無い」と混ぜない
    //
    // 読取失敗はどの分岐からも出うるが、CLI からは作れない (ストアは RMU が書いた正しい
    // ものしか存在しない)。ここでは投影済みのストアを**壊してから**開き、各分岐が不在では
    // なく読取失敗を答えることを固定する。SQLite の接続は開くだけでは中身を読まないので、
    // 開くのは成功し最初の引当で潰える。
    // -----------------------------------------------------------------------

    impl Workspace {
        fn store_path(&self) -> PathBuf {
            self.path("aidlc/spaces/default/intents/.aidlc-store.sqlite")
        }

        /// ストアの中身を SQLite でないバイト列に置き換える (開けるが引けない状態)。
        fn break_store(&self) {
            fs::write(self.store_path(), b"not a sqlite database at all").expect("ストア");
        }

        /// 投影済みの定義行を落とす (定義が取り込まれていない状態を作る)。
        fn drop_definition_rows(&self) {
            let connection = rusqlite::Connection::open(self.store_path()).expect("ストア");
            connection
                .execute("DELETE FROM read_definition", [])
                .expect("定義行を落とす");
        }

        /// 答えの綴りを差し替える (`park` は本ビルドで未配線なので行を直接置く)。
        fn rewrite_decision_kind(&self, request_kind: &str, decision_kind: &str) {
            let connection = rusqlite::Connection::open(self.store_path()).expect("ストア");
            let changed = connection
                .execute(
                    "UPDATE read_next_answer SET decision_kind = ?1 WHERE request_kind = ?2",
                    rusqlite::params![decision_kind, request_kind],
                )
                .expect("答えの綴りを差し替える");
            assert_eq!(changed, 1, "その要求の形の行はちょうど 1 つある");
        }
    }

    /// 壊れたストアを掴んだターン (開くのは成功する)。
    fn broken(layout: &Layout) -> Turn<'_> {
        Turn::open(layout).expect("SQLite は開くだけでは中身を読まない")
    }

    fn is_read_failure(directive: &Directive) -> bool {
        message_of(directive).starts_with("Read model not readable at ")
    }

    /// 定義を引けなければ、そのターンは何も答えられない。
    #[tokio::test]
    async fn a_turn_that_cannot_read_the_definition_reports_a_read_failure() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();

        assert!(is_read_failure(&next(&layout, &NextTurnInput::new())));
    }

    /// 定義が取り込まれていなくてもカーソルが在れば、定義データの所在を名指して直せと言う。
    #[tokio::test]
    async fn a_cursor_without_a_projected_definition_names_the_stage_graph_file() {
        let workspace = Workspace::minted().await;
        workspace.drop_definition_rows();
        let layout = workspace.layout();

        let message = message_of(&next(&layout, &NextTurnInput::new()));

        assert!(
            message.starts_with("Stage graph not readable at "),
            "{message}"
        );
        assert!(message.contains("stage-graph.json"), "{message}");
    }

    /// 定義もカーソルも無いのは fresh なワークスペースの正常な姿である。
    #[tokio::test]
    async fn neither_a_definition_nor_a_cursor_is_reported_as_no_state() {
        let workspace = Workspace::minted().await;
        workspace.drop_definition_rows();
        workspace.forget_cursor();
        let layout = workspace.layout();

        assert_eq!(
            message_of(&next(&layout, &NextTurnInput::new())),
            wording::NO_STATE
        );
    }

    /// park 中の `--resume` は、park を外す命令を名指してから再実行せよと言う。
    #[tokio::test]
    async fn a_resume_on_a_parked_workflow_names_the_unpark_command_first() {
        let workspace = Workspace::minted().await;
        workspace.rewrite_decision_kind("resume", "unpark-then-resume");
        let layout = workspace.layout();

        assert_eq!(
            message_of(&next(&layout, &NextTurnInput::new().with_resume())),
            wording::unpark_then_resume("aidlc-state unpark")
        );
    }

    /// scope 変更の照合が引けなければ、変更なしではなく読取失敗を答える。
    #[tokio::test]
    async fn an_unreadable_scope_change_lookup_is_a_read_failure() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();
        let turn = broken(&layout);
        let view = turn_view(
            answer_view("run-stage", Some("domain-design"), None),
            "classic",
        );

        let directive = turn
            .configuration_change(&NextTurnInput::new().with_scope("express"), &view)
            .expect("読取失敗も 1 つの答えである");

        assert!(is_read_failure(&directive), "{directive:?}");
    }

    /// `--single` の行が引けなければ、未知ステージではなく読取失敗を答える。
    #[tokio::test]
    async fn an_unreadable_single_lookup_is_a_read_failure() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();
        let turn = broken(&layout);
        let input = NextTurnInput::new()
            .with_single()
            .with_stage("domain-design");

        assert!(is_read_failure(&turn.single(&input, &scope_of("classic"))));
    }

    /// フェーズの入口が引けなければ、入口不在ではなく読取失敗を答える。
    #[tokio::test]
    async fn an_unreadable_phase_entry_lookup_is_a_read_failure() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();
        let turn = broken(&layout);
        let input = NextTurnInput::new().with_phase("inception");

        assert!(is_read_failure(&turn.jump(
            &input,
            &scope_of("classic"),
            None
        )));
    }

    /// 孤立 run-stage の行が引けなければ読取失敗を答える。
    #[tokio::test]
    async fn an_unreadable_isolated_run_stage_lookup_is_a_read_failure() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();
        let turn = broken(&layout);

        assert!(is_read_failure(
            &turn.isolated_run_stage(&scope_of("classic"), "domain-design")
        ));
    }

    /// 誕生の scope 照合が引けなければ読取失敗を答える。
    #[tokio::test]
    async fn an_unreadable_birth_scope_lookup_is_a_read_failure() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();
        let turn = broken(&layout);

        assert!(is_read_failure(
            &turn.birth_group(&NextTurnInput::new().with_freeform("classic"))
        ));
    }

    /// 鋳造のコスト節を引けなければ、コスト無しで進まず読取失敗を答える。
    #[tokio::test]
    async fn an_unreadable_cost_lookup_stops_the_mint_print() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();
        let turn = broken(&layout);

        assert!(is_read_failure(&turn.mint_intent(
            "classic",
            None,
            &NextTurnInput::new(),
            false
        )));
    }

    /// 稼働中の自由記述の scope 推論が引けなければ読取失敗を答える。
    #[tokio::test]
    async fn an_unreadable_inference_on_new_work_routing_is_a_read_failure() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();
        let turn = broken(&layout);
        let view = turn_view(answer_view("new-work-routing", None, None), "classic");
        let input = NextTurnInput::new().with_freeform("classic-work");

        assert!(is_read_failure(&turn.happy_path(
            &input,
            &view,
            &scope_of("classic")
        )));
    }

    /// 配信計画が引けなければ、素の run-stage へ倒さず読取失敗を答える。
    #[tokio::test]
    async fn an_unreadable_steering_lookup_is_a_read_failure() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();
        let turn = broken(&layout);
        let row = detached_row("plan-1");

        assert!(is_read_failure(&turn.deliver(
            &row,
            &scope_of("classic"),
            GateField::Gated,
            false,
            None,
            None
        )));
    }

    /// state 束縛の照合が引けなければ、位置が動いたではなく読取失敗を答える。
    #[tokio::test]
    async fn an_unreadable_state_binding_lookup_on_continue_is_a_read_failure() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();
        let turn = broken(&layout);
        let bindings = Bindings::new(
            BundleDigest::new("b"),
            DirectiveDigest::new("d"),
            RouteDigest::new("r"),
            Some(StateBinding::new("state-binding-1")),
        );

        assert!(is_read_failure(&turn.resume(&token_for(bindings, 1))));
    }

    /// 続きの引当が引けなければ、stale ではなく読取失敗を答える。
    #[tokio::test]
    async fn an_unreadable_continuation_lookup_is_a_read_failure() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();
        let turn = broken(&layout);
        let bindings = Bindings::new(
            BundleDigest::new("b"),
            DirectiveDigest::new("d"),
            RouteDigest::new("r"),
            None,
        );

        assert!(is_read_failure(&turn.resume(&token_for(bindings, 1))));
    }

    // -----------------------------------------------------------------------
    // 表を 1 つだけ落とす — どの段の引当が潰えたかで答えが変わる
    // -----------------------------------------------------------------------

    impl Workspace {
        /// 表を 1 つだけ落とす (その段の引当だけが潰える状態を作る)。
        fn drop_table(&self, table: &str) {
            let connection = rusqlite::Connection::open(self.store_path()).expect("ストア");
            connection
                .execute(&format!("DROP TABLE {table}"), [])
                .expect("表を落とす");
        }

        /// 定義に scope 識別だけを足す (グリッド列を持たない scope を作る)。
        fn add_gridless_scope(&self, name: &str) {
            fs::write(
                self.path(&format!(".claude/scopes/aidlc-{name}.md")),
                format!("---\nname: {name}\n---\n\n# {name}\n"),
            )
            .expect("scope identity");
        }
    }

    /// park している実行は、素の `next` でも位置を名乗って止まる。
    #[tokio::test]
    async fn a_parked_execution_stops_the_bare_next_at_its_stage() {
        let workspace = Workspace::minted().await;
        workspace.rewrite_decision_kind("bare", "parked");
        let layout = workspace.layout();

        let directive = next(&layout, &NextTurnInput::new());

        assert_eq!(kind_of(&directive), "parked");
        assert_eq!(message_of(&directive), wording::parked("domain-design"));
    }

    /// 答えの表を引けなければ、定義まで読めていても答えは出ない。
    #[tokio::test]
    async fn an_unreadable_answer_table_stops_the_ladder_after_the_definition() {
        let workspace = Workspace::minted().await;
        workspace.drop_table("read_next_answer");
        let layout = workspace.layout();

        assert!(is_read_failure(&next(&layout, &NextTurnInput::new())));
    }

    /// キーワードの表を引けなければ、推論なしで進まず読取失敗を答える。
    #[tokio::test]
    async fn an_unreadable_keyword_table_stops_the_inference() {
        let workspace = Workspace::minted().await;
        workspace.drop_table("read_definition_scope_keyword");
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        assert!(is_read_failure(
            &turn.birth_group(&NextTurnInput::new().with_freeform("frobnicate"))
        ));
    }

    /// scope の有効性を引けなければ、明示 `--scope` の検証もその場で潰える。
    #[tokio::test]
    async fn an_unreadable_scope_catalog_stops_the_explicit_scope_check() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();
        let turn = broken(&layout);

        let refused = turn
            .resolve_scope(None, &NextTurnInput::new().with_scope("classic"))
            .expect_err("引けない");

        assert!(is_read_failure(&refused));
    }

    /// state の scope の有効性を引けなければ、そこで潰える。
    #[tokio::test]
    async fn an_unreadable_scope_catalog_stops_the_state_scope_check() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();
        let turn = broken(&layout);

        let refused = turn
            .resolve_scope(Some("classic"), &NextTurnInput::new())
            .expect_err("引けない");

        assert!(is_read_failure(&refused));
    }

    /// 自由記述からの推論が引けなければ、既定へ落とさずそこで潰える。
    #[tokio::test]
    async fn an_unreadable_keyword_table_stops_the_scope_ladder() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();
        let turn = broken(&layout);

        let refused = turn
            .resolve_scope(None, &NextTurnInput::new().with_freeform("classic-work"))
            .expect_err("引けない");

        assert!(is_read_failure(&refused));
    }

    /// 環境変数の既定 scope の有効性を引けなければ、そこで潰える。
    #[tokio::test]
    async fn an_unreadable_scope_catalog_stops_the_environment_default_check() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();
        let turn = broken(&layout);

        let refused = turn
            .resolve_scope(
                None,
                &NextTurnInput::new().with_env_default_scope("classic"),
            )
            .expect_err("引けない");

        assert!(is_read_failure(&refused));
    }

    /// 既製 scope の綴りを引けなければ、例なしの提案へ倒さず読取失敗を答える。
    #[tokio::test]
    async fn an_unreadable_stock_lookup_is_a_read_failure() {
        let workspace = Workspace::minted().await;
        workspace.break_store();
        let layout = workspace.layout();
        let turn = broken(&layout);

        let refused = turn.stock_examples().expect_err("引けない");

        assert!(is_read_failure(&refused));
    }

    /// グリッド列を持たない scope はコスト 4 列を持たないので、コスト節が付かない。
    #[tokio::test]
    async fn a_scope_without_grid_columns_births_without_a_cost_clause() {
        let workspace = Workspace::create();
        workspace.add_gridless_scope("feature");
        workspace.invoke("aidlc-orchestrate", &["next"]).await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        let with_cost =
            message_of(&turn.mint_intent("classic", None, &NextTurnInput::new(), false));
        let without_cost =
            message_of(&turn.mint_intent("feature", None, &NextTurnInput::new(), false));

        assert!(with_cost.contains("stages,"), "{with_cost}");
        assert!(
            !without_cost.contains("stages,"),
            "コスト 4 列が揃わない scope には節が付かない: {without_cost}"
        );
    }

    // -----------------------------------------------------------------------
    // 壊れた投影 — 行は在るが列の値が公開言語の外にある
    // -----------------------------------------------------------------------

    impl Workspace {
        /// 投影された行の 1 列を書き換える。
        fn update_row(&self, statement: &str) {
            let connection = rusqlite::Connection::open(self.store_path()).expect("ストア");
            let changed = connection.execute(statement, []).expect("列を書き換える");
            assert!(changed >= 1, "対象の行がある: {statement}");
        }
    }

    /// コスト 4 列のどれか 1 つでも欠けたらコスト節は付かない (4 列そろって初めて意味を持つ)。
    #[tokio::test]
    async fn a_cost_clause_needs_all_four_columns() {
        let workspace = Workspace::minted().await;
        let columns = [
            "cost_per_unit_stages",
            "cost_gates",
            "cost_execute",
            "cost_total",
        ];

        for column in columns {
            workspace.update_row(&format!(
                "UPDATE read_definition_scope SET {column} = NULL WHERE scope = 'classic'"
            ));
            let layout = workspace.layout();
            let turn = Turn::open(&layout).expect("ストア");

            let message =
                message_of(&turn.mint_intent("classic", None, &NextTurnInput::new(), false));

            assert!(
                !message.contains("stages,"),
                "{column} が欠けた時点でコスト節は出ない: {message}"
            );
        }
    }

    /// 台帳の列が壊れていれば、終端の run-stage を描かずにその列を材料に拒む。
    #[tokio::test]
    async fn a_terminal_continue_refuses_an_unreadable_ledger() {
        let workspace = Workspace::minted().await;
        let (bindings, part_count) = {
            let layout = workspace.layout();
            let turn = Turn::open(&layout).expect("ストア");
            live_bindings(&turn, None)
        };
        workspace.update_row("UPDATE read_steering_plan SET delivered_paths = 'not-json'");
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        assert_eq!(
            message_of(&turn.resume(&token_for(bindings, part_count))),
            "Read model row is not readable: delivered_paths = \"not-json\"."
        );
    }

    /// 続きの部の本文が壊れていれば、連鎖を描かずにその列を材料に拒む。
    #[tokio::test]
    async fn a_continue_refuses_an_unreadable_next_part() {
        let workspace = Workspace::minted().await;
        let bindings = {
            let layout = workspace.layout();
            let turn = Turn::open(&layout).expect("ストア");
            live_bindings(&turn, None).0
        };
        // 部を 1 つ先へずらし、その本文を壊す (第 1 部を受け取った token の続きになる)。
        workspace.update_row(
            "UPDATE read_steering_part SET part_index = part_index + 1, rules_content = 'not-json'",
        );
        workspace.update_row("UPDATE read_steering_plan SET part_count = part_count + 1");
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        assert_eq!(
            message_of(&turn.resume(&token_for(bindings, 1))),
            "Read model row is not readable: rules_content = \"not-json\"."
        );
    }

    /// 定義が slug として読めない scope を名乗っていたら、未知 scope として拒む。
    #[tokio::test]
    async fn a_scope_row_whose_name_is_not_a_slug_is_refused() {
        let workspace = Workspace::minted().await;
        workspace.update_row(
            "INSERT INTO read_definition_scope \
             (id, definition_id, scope, depth, keywords, skeleton, review_cap, \
              freeform_default, has_grid_column, cost_total, cost_execute, cost_gates, \
              cost_per_unit_stages, as_of) \
             VALUES ('bad-scope', 'claude', 'Not A Slug', NULL, '[]', NULL, NULL, 0, 0, \
                     NULL, NULL, NULL, NULL, 0)",
        );
        workspace.forget_cursor();
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");

        let refused = turn
            .resolve_scope(None, &NextTurnInput::new().with_scope("Not A Slug"))
            .expect_err("slug として読めない");

        assert!(
            message_of(&refused).starts_with("Unknown scope \"Not A Slug\"."),
            "{refused:?}"
        );
    }

    // -----------------------------------------------------------------------
    // ハッピーパス — 健全なワークスペースで端から端まで
    // -----------------------------------------------------------------------

    /// 素の `next` は、答えが名指すステージを配信計画の第 1 部から届ける。
    #[tokio::test]
    async fn a_bare_next_delivers_the_next_stage_from_its_first_part() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();

        let directive = next(&layout, &NextTurnInput::new());

        assert_eq!(kind_of(&directive), "load-steering");
        assert_eq!(message_of(&directive), "domain-design");
    }

    /// `--new-intent` は記述を鋳造コマンドへ運び、解決済み scope へ落ちる。
    #[tokio::test]
    async fn new_intent_carries_its_description_into_the_mint_command() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let input = NextTurnInput::new().with_new_intent("  build the auth service  ");

        let directive = next(&layout, &input);

        assert_eq!(kind_of(&directive), "print");
        let message = message_of(&directive);
        assert!(
            message.contains("--scope classic --arguments='build the auth service'"),
            "明示 scope が無ければ state の scope へ落ち、記述は trim される: {message}"
        );
        assert!(
            message.contains("Then STOP, do NOT re-run `next` in this session."),
            "{message}"
        );
    }

    /// どのキーワードも当たらない自由記述は、解決済み scope をそのまま提案に載せる。
    #[tokio::test]
    async fn new_work_routing_falls_back_to_the_resolved_scope_when_no_keyword_hits() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let view = turn_view(answer_view("new-work-routing", None, None), "classic");
        let input = NextTurnInput::new().with_freeform("frobnicate");

        let directive = turn.happy_path(&input, &view, &scope_of("express"));

        assert_eq!(kind_of(&directive), "ask");
        assert_eq!(message_of(&directive), wording::NEW_WORK_ROUTING);
        assert_eq!(
            proposed_scope_of(&directive),
            Some("express"),
            "推論が当たらなければ解決済み scope へ落ちる"
        );
    }

    /// ask directive が運ぶ新規作業の scope。
    fn proposed_scope_of(directive: &Directive) -> Option<&str> {
        match directive {
            Directive::Ask(ask) => ask.proposed_scope(),
            Directive::Error { .. }
            | Directive::Print { .. }
            | Directive::Parked { .. }
            | Directive::Done { .. }
            | Directive::RunStage(_)
            | Directive::LoadSteering(_) => None,
        }
    }

    /// キーワードが当たれば、その scope を新規作業の提案に載せる。
    #[tokio::test]
    async fn new_work_routing_proposes_the_scope_the_keyword_names() {
        let workspace = Workspace::minted().await;
        let layout = workspace.layout();
        let turn = Turn::open(&layout).expect("ストア");
        let view = turn_view(answer_view("new-work-routing", None, None), "classic");
        let input = NextTurnInput::new().with_freeform("express-work");

        let directive = turn.happy_path(&input, &view, &scope_of("classic"));

        assert_eq!(proposed_scope_of(&directive), Some("express"));
        assert_eq!(
            proposed_scope_of(&Directive::Done { reason: None }),
            None,
            "ask 以外は新規作業の scope を運ばない"
        );
    }
}
