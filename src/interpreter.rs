use std::{collections::HashMap, sync::Arc};

use crate::functions::{FnCallError, VTable};

use super::functions::VTABLE;
use super::types::{Exp, Literal};

pub struct Env {
    bindings: HashMap<&'static str, Literal>,
    vtable: &'static VTable,
}

pub enum EvalError {
    VariableNotFound(String),
    FunctionNotFound(String),
    FnCallError(FnCallError),
    TypeError(String),
    ValueError(String),
}

impl Env {
    fn new(bindings: HashMap<&'static str, Literal>) -> Self {
        Self {
            bindings,
            vtable: &VTABLE,
        }
    }
}

/* pub fn eval(exp: impl Into<Exp>, env: Option<Env>) -> Result<Literal, EvalError> {
    let exp = Arc::new(exp.into());
    let env = Arc::new(env.unwrap_or_else(|| Env {
        bindings: HashMap::new(),
        vtable: &VTABLE,
    }));

    eval_loop(exp, env)
}

fn eval_loop(exp: Arc<Exp>, env: Arc<Env>) -> Result<Literal, EvalError> {
    match &*exp {
        Exp::Literal(literal) => todo!(),
        Exp::Var(var_access) => todo!(),
        Exp::FnCall(function) => todo!(),
        Exp::Not(exp) => todo!(),
        Exp::Or(exp, exp1) => todo!(),
        Exp::And(exp, exp1) => todo!(),
        Exp::Eq(exp, exp1) => todo!(),
        Exp::Neq(exp, exp1) => todo!(),
        Exp::Gt(exp, exp1) => todo!(),
        Exp::Lt(exp, exp1) => todo!(),
        Exp::Geq(exp, exp1) => todo!(),
        Exp::Leq(exp, exp1) => todo!(),
    }
}
 */
