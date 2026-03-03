use super::Parser;
use crate::ast::{
    Provenance, RawBranch, RawCompStep, RawConstruct, RawFailureHandler, RawJoinPolicy, RawStep,
    RawStepTarget,
};
use crate::error::ElabError;
use crate::lexer::Token;
use std::collections::BTreeMap;

impl<'a> Parser<'a> {
    pub(super) fn parse_flow(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance();
        let id = self.take_word()?;
        self.expect_lbrace()?;
        let mut snapshot = String::new();
        let mut entry = String::new();
        let mut entry_line = line;
        let mut steps = BTreeMap::new();
        while self.peek() != &Token::RBrace {
            let field_line = self.cur_line();
            let key = self.take_word()?;
            self.expect_colon()?;
            match key.as_str() {
                "snapshot" => {
                    snapshot = self.take_word()?;
                }
                "entry" => {
                    entry_line = field_line;
                    entry = self.take_word()?;
                }
                "steps" => {
                    steps = self.parse_steps()?;
                }
                _ => return Err(self.err(format!("unknown Flow field '{}'", key))),
            }
        }
        self.expect_rbrace()?;
        Ok(RawConstruct::Flow {
            id,
            snapshot,
            entry,
            entry_line,
            steps,
            prov: Provenance {
                file: self.filename.clone(),
                line,
            },
        })
    }

    pub(super) fn parse_steps(&mut self) -> Result<BTreeMap<String, RawStep>, ElabError> {
        let mut steps = BTreeMap::new();
        self.expect_lbrace()?;
        while self.peek() != &Token::RBrace {
            let step_line = self.cur_line();
            let step_id = self.take_word()?;
            self.expect_colon()?;
            let step_kind = self.take_word()?;
            let step = self.parse_step_body(&step_kind, step_line)?;
            steps.insert(step_id, step);
        }
        self.expect_rbrace()?;
        Ok(steps)
    }

    fn parse_step_body(&mut self, kind: &str, step_line: u32) -> Result<RawStep, ElabError> {
        self.expect_lbrace()?;
        let step = match kind {
            "OperationStep" => {
                let mut op = String::new();
                let mut persona = String::new();
                let mut outcomes = BTreeMap::new();
                let mut on_failure = None;
                while self.peek() != &Token::RBrace {
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "op" => {
                            op = self.take_word()?;
                        }
                        "persona" => {
                            persona = self.take_word()?;
                        }
                        "outcomes" => {
                            outcomes = self.parse_outcomes()?;
                        }
                        "on_failure" => {
                            on_failure = Some(self.parse_failure_handler()?);
                        }
                        _ => return Err(self.err(format!("unknown OperationStep field '{}'", key))),
                    }
                }
                RawStep::OperationStep {
                    op,
                    persona,
                    outcomes,
                    on_failure,
                    line: step_line,
                }
            }
            "BranchStep" => {
                let mut condition = None;
                let mut persona = String::new();
                let mut if_true = None;
                let mut if_false = None;
                while self.peek() != &Token::RBrace {
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "condition" => {
                            condition = Some(self.parse_expr()?);
                        }
                        "persona" => {
                            persona = self.take_word()?;
                        }
                        "if_true" => {
                            if_true = Some(self.parse_step_target()?);
                        }
                        "if_false" => {
                            if_false = Some(self.parse_step_target()?);
                        }
                        _ => return Err(self.err(format!("unknown BranchStep field '{}'", key))),
                    }
                }
                RawStep::BranchStep {
                    condition: condition.ok_or_else(|| self.err("BranchStep missing condition"))?,
                    persona,
                    if_true: if_true.ok_or_else(|| self.err("BranchStep missing if_true"))?,
                    if_false: if_false.ok_or_else(|| self.err("BranchStep missing if_false"))?,
                    line: step_line,
                }
            }
            "HandoffStep" => {
                let mut from_persona = String::new();
                let mut to_persona = String::new();
                let mut next = String::new();
                while self.peek() != &Token::RBrace {
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "from_persona" => {
                            from_persona = self.take_word()?;
                        }
                        "to_persona" => {
                            to_persona = self.take_word()?;
                        }
                        "next" => {
                            next = self.take_word()?;
                        }
                        _ => return Err(self.err(format!("unknown HandoffStep field '{}'", key))),
                    }
                }
                RawStep::HandoffStep {
                    from_persona,
                    to_persona,
                    next,
                    line: step_line,
                }
            }
            "SubFlowStep" => {
                let mut flow = String::new();
                let mut flow_line = step_line;
                let mut persona = String::new();
                let mut on_success = None;
                let mut on_failure = None;
                while self.peek() != &Token::RBrace {
                    let field_line = self.cur_line();
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "flow" => {
                            flow_line = field_line;
                            flow = self.take_word()?;
                        }
                        "persona" => {
                            persona = self.take_word()?;
                        }
                        "on_success" => {
                            on_success = Some(self.parse_step_target()?);
                        }
                        "on_failure" => {
                            on_failure = Some(self.parse_failure_handler()?);
                        }
                        _ => return Err(self.err(format!("unknown SubFlowStep field '{}'", key))),
                    }
                }
                RawStep::SubFlowStep {
                    flow,
                    flow_line,
                    persona,
                    on_success: on_success
                        .ok_or_else(|| self.err("SubFlowStep missing on_success"))?,
                    on_failure: on_failure
                        .ok_or_else(|| self.err("SubFlowStep missing on_failure"))?,
                    line: step_line,
                }
            }
            "ParallelStep" => {
                let mut branches = Vec::new();
                let mut branches_line = step_line;
                let mut join = None;
                while self.peek() != &Token::RBrace {
                    let field_line = self.cur_line();
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "branches" => {
                            branches_line = field_line;
                            branches = self.parse_branches()?;
                        }
                        "join" => {
                            join = Some(self.parse_join_policy()?);
                        }
                        _ => return Err(self.err(format!("unknown ParallelStep field '{}'", key))),
                    }
                }
                RawStep::ParallelStep {
                    branches,
                    branches_line,
                    join: join.ok_or_else(|| self.err("ParallelStep missing join"))?,
                    line: step_line,
                }
            }
            _ => return Err(self.err(format!("unknown step kind '{}'", kind))),
        };
        self.expect_rbrace()?;
        Ok(step)
    }

    fn parse_outcomes(&mut self) -> Result<BTreeMap<String, RawStepTarget>, ElabError> {
        let mut outcomes = BTreeMap::new();
        self.expect_lbrace()?;
        while self.peek() != &Token::RBrace {
            let label = self.take_word()?;
            self.expect_colon()?;
            let target = self.parse_step_target()?;
            outcomes.insert(label, target);
        }
        self.expect_rbrace()?;
        Ok(outcomes)
    }

    fn parse_step_target(&mut self) -> Result<RawStepTarget, ElabError> {
        if self.is_word("Terminal") {
            self.advance();
            self.advance_lparen()?;
            let outcome = self.take_word()?;
            self.expect_rparen()?;
            return Ok(RawStepTarget::Terminal { outcome });
        }
        let line = self.cur_line();
        let name = self.take_word()?;
        Ok(RawStepTarget::StepRef(name, line))
    }

    fn parse_failure_handler(&mut self) -> Result<RawFailureHandler, ElabError> {
        let kind = self.take_word()?;
        match kind.as_str() {
            "Terminate" => {
                self.advance_lparen()?;
                if self.is_word("outcome") {
                    self.advance();
                    self.expect_colon()?;
                }
                let outcome = self.take_word()?;
                self.expect_rparen()?;
                Ok(RawFailureHandler::Terminate { outcome })
            }
            "Compensate" => {
                self.advance_lparen()?;
                let mut comp_steps = Vec::new();
                let mut then_outcome = String::new();
                while self.peek() != &Token::RParen {
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "steps" => {
                            comp_steps = self.parse_comp_steps()?;
                        }
                        "then" => {
                            self.expect_word("Terminal")?;
                            self.advance_lparen()?;
                            then_outcome = self.take_word()?;
                            self.expect_rparen()?;
                        }
                        _ => return Err(self.err(format!("unknown Compensate field '{}'", key))),
                    }
                }
                self.expect_rparen()?;
                Ok(RawFailureHandler::Compensate {
                    steps: comp_steps,
                    then: then_outcome,
                })
            }
            "Escalate" => {
                self.advance_lparen()?;
                let mut to_persona = String::new();
                let mut next = String::new();
                while self.peek() != &Token::RParen {
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "to" | "to_persona" => {
                            to_persona = self.take_word()?;
                        }
                        "next" => {
                            next = self.take_word()?;
                        }
                        _ => return Err(self.err(format!("unknown Escalate field '{}'", key))),
                    }
                }
                self.expect_rparen()?;
                Ok(RawFailureHandler::Escalate { to_persona, next })
            }
            _ => Err(self.err(format!("unknown failure handler kind '{}'", kind))),
        }
    }

    fn parse_branches(&mut self) -> Result<Vec<RawBranch>, ElabError> {
        self.expect_lbracket()?;
        let mut branches = Vec::new();
        while self.peek() != &Token::RBracket {
            branches.push(self.parse_branch()?);
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect_rbracket()?;
        Ok(branches)
    }

    fn parse_branch(&mut self) -> Result<RawBranch, ElabError> {
        self.expect_word("Branch")?;
        self.expect_lbrace()?;
        let mut id = String::new();
        let mut entry = String::new();
        let mut steps = BTreeMap::new();
        while self.peek() != &Token::RBrace {
            let key = self.take_word()?;
            self.expect_colon()?;
            match key.as_str() {
                "id" => {
                    id = self.take_word()?;
                }
                "entry" => {
                    entry = self.take_word()?;
                }
                "steps" => {
                    steps = self.parse_steps()?;
                }
                _ => return Err(self.err(format!("unknown Branch field '{}'", key))),
            }
        }
        self.expect_rbrace()?;
        Ok(RawBranch { id, entry, steps })
    }

    fn parse_join_policy(&mut self) -> Result<RawJoinPolicy, ElabError> {
        self.expect_word("JoinPolicy")?;
        self.expect_lbrace()?;
        let mut on_all_success = None;
        let mut on_any_failure = None;
        let mut on_all_complete = None;
        while self.peek() != &Token::RBrace {
            let key = self.take_word()?;
            self.expect_colon()?;
            match key.as_str() {
                "on_all_success" => {
                    on_all_success = Some(self.parse_step_target()?);
                }
                "on_any_failure" => {
                    on_any_failure = Some(self.parse_failure_handler()?);
                }
                "on_all_complete" => {
                    if self.is_word("null") {
                        self.advance();
                    } else {
                        on_all_complete = Some(self.parse_step_target()?);
                    }
                }
                _ => return Err(self.err(format!("unknown JoinPolicy field '{}'", key))),
            }
        }
        self.expect_rbrace()?;
        Ok(RawJoinPolicy {
            on_all_success,
            on_any_failure,
            on_all_complete,
        })
    }

    fn parse_comp_steps(&mut self) -> Result<Vec<RawCompStep>, ElabError> {
        let mut steps = Vec::new();
        self.expect_lbracket()?;
        while self.peek() != &Token::RBracket {
            self.expect_lbrace()?;
            let mut op = String::new();
            let mut persona = String::new();
            let mut on_failure = String::new();
            while self.peek() != &Token::RBrace {
                let key = self.take_word()?;
                self.expect_colon()?;
                match key.as_str() {
                    "op" => {
                        op = self.take_word()?;
                    }
                    "persona" => {
                        persona = self.take_word()?;
                    }
                    "on_failure" => {
                        self.expect_word("Terminal")?;
                        self.advance_lparen()?;
                        on_failure = self.take_word()?;
                        self.expect_rparen()?;
                    }
                    _ => return Err(self.err(format!("unknown comp step field '{}'", key))),
                }
            }
            self.expect_rbrace()?;
            steps.push(RawCompStep {
                op,
                persona,
                on_failure,
            });
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect_rbracket()?;
        Ok(steps)
    }
}
