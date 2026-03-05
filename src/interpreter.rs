use std::{borrow::Cow, collections::HashMap, sync::Arc};

use crate::{
    functions::{FnCallError, VTABLE, VTable},
    types::{FunctionItem, VarAccess},
};

use super::types::{Exp, Literal};

pub enum EvalError {
    VariableNotFound(String),
    FunctionNotFound(String),
    FnCallError(FnCallError),
    TypeError(String),
    ValueError(String),
}

pub struct Env {
    bindings: HashMap<Box<str>, serde_json::Value>,
    vtable: VTable,
}

impl Env {
    pub fn new(bindings: HashMap<Box<str>, serde_json::Value>) -> Self {
        Self {
            bindings,
            vtable: VTABLE.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Cmp {
    Lt,
    Gt,
    Leq,
    Geq,
}

#[expect(clippy::needless_pass_by_value)]
fn expect_type<T>(
    value: Option<Cow<'_, Literal>>,
    extractor: impl Fn(&Literal) -> Option<T>,
    type_name: &str,
) -> Result<T, EvalError> {
    let value = value.as_deref();

    value.map_or_else(
        || {
            Err(EvalError::TypeError(format!(
                "Expected {type_name}, got None"
            )))
        },
        |literal| {
            extractor(literal).ok_or_else(|| {
                EvalError::TypeError(format!("Expected {type_name}, got {literal:?}"))
            })
        },
    )
}

fn expect_bool(value: Option<Cow<'_, Literal>>) -> Result<bool, EvalError> {
    expect_type(
        value,
        |l| match l {
            Literal::Bool(b) => Some(*b),
            _ => None,
        },
        "a boolean",
    )
}

#[expect(dead_code)]
fn expect_string(value: Option<Cow<'_, Literal>>) -> Result<Arc<str>, EvalError> {
    expect_type(
        value,
        |l| match l {
            Literal::String(s) => Some(s.clone()),
            _ => None,
        },
        "a string",
    )
}

#[expect(dead_code)]
fn expect_int(value: Option<Cow<'_, Literal>>) -> Result<i64, EvalError> {
    expect_type(
        value,
        |l| match l {
            Literal::Int(i) => Some(*i),
            _ => None,
        },
        "an integer",
    )
}

#[expect(dead_code)]
fn expect_float(value: Option<Cow<'_, Literal>>) -> Result<f64, EvalError> {
    expect_type(
        value,
        |l| match l {
            Literal::Float(f) => Some(*f),
            _ => None,
        },
        "a float",
    )
}

#[expect(dead_code, clippy::needless_pass_by_value)]
fn expect_null(value: Option<Cow<'_, Literal>>) -> Result<(), EvalError> {
    if value.is_none() {
        Ok(())
    } else {
        Err(EvalError::TypeError(format!(
            "Expected null, got {:?}",
            value.as_deref()
        )))
    }
}

#[inline]
#[expect(clippy::unnecessary_wraps)]
const fn out(literal: Literal) -> Result<Option<Cow<'static, Literal>>, EvalError> {
    Ok(Some(Cow::Owned(literal)))
}

fn eval_not<'a>(exp: &'a Exp, env: &Env) -> Result<Option<Cow<'a, Literal>>, EvalError> {
    let value = eval(exp, env)?.as_deref().is_some_and(bool::from);
    out(Literal::Bool(!value))
}

fn eval_and<'a>(
    exp1: &'a Exp,
    exp2: &'a Exp,
    env: &Env,
) -> Result<Option<Cow<'a, Literal>>, EvalError> {
    let value1 = eval(exp1, env)?;
    if value1.as_deref().is_some_and(bool::from) {
        let value2 = eval(exp2, env)?;
        Ok(value2)
    } else {
        out(Literal::Bool(false))
    }
}

fn eval_or<'a>(
    exp1: &'a Exp,
    exp2: &'a Exp,
    env: &Env,
) -> Result<Option<Cow<'a, Literal>>, EvalError> {
    let value1 = eval(exp1, env)?;
    if value1.as_deref().is_some_and(bool::from) {
        Ok(value1)
    } else {
        let value2 = eval(exp2, env)?;
        Ok(value2)
    }
}

fn eval_eq<'a>(
    exp1: &'a Exp,
    exp2: &'a Exp,
    env: &Env,
) -> Result<Option<Cow<'a, Literal>>, EvalError> {
    let value1 = eval(exp1, env)?;
    let value2 = eval(exp2, env)?;

    match (value1.as_deref(), value2.as_deref()) {
        (Some(Literal::Int(i1)), Some(Literal::Int(i2))) => out(Literal::Bool(i1 == i2)),
        (Some(Literal::String(s1)), Some(Literal::String(s2))) => out(Literal::Bool(s1 == s2)),
        (Some(Literal::Bool(b1)), Some(Literal::Bool(b2))) => out(Literal::Bool(b1 == b2)),
        (None, _) | (_, None) => out(Literal::Bool(false)),
        _ => Err(EvalError::TypeError(format!(
            "Cannot compare values of different types: {:?} and {:?}",
            value1.as_deref(),
            value2.as_deref()
        ))),
    }
}

fn eval_neq<'a>(
    exp1: &'a Exp,
    exp2: &'a Exp,
    env: &Env,
) -> Result<Option<Cow<'a, Literal>>, EvalError> {
    let res = eval_eq(exp1, exp2, env)?;
    let eq_value = expect_bool(res)?;
    out(Literal::Bool(!eq_value))
}

fn eval_cmp<'a>(
    exp1: &'a Exp,
    exp2: &'a Exp,
    env: &Env,
    cmp: Cmp,
) -> Result<Option<Cow<'a, Literal>>, EvalError> {
    let value1 = eval(exp1, env)?;
    let value2 = eval(exp2, env)?;

    match (value1.as_deref(), value2.as_deref()) {
        (Some(Literal::Int(i1)), Some(Literal::Int(i2))) => out(Literal::Bool(match cmp {
            Cmp::Lt => i1 < i2,
            Cmp::Gt => i1 > i2,
            Cmp::Leq => i1 <= i2,
            Cmp::Geq => i1 >= i2,
        })),
        (Some(Literal::Float(f1)), Some(Literal::Float(f2))) => out(Literal::Bool(match cmp {
            Cmp::Lt => f1 < f2,
            Cmp::Gt => f1 > f2,
            Cmp::Leq => f1 <= f2,
            Cmp::Geq => f1 >= f2,
        })),
        _ => Err(EvalError::TypeError(format!(
            "Cannot compare values of different types: {:?} and {:?}",
            value1.as_deref(),
            value2.as_deref()
        ))),
    }
}

fn eval_varaccess<'a>(
    var: &'a VarAccess,
    env: &Env,
) -> Result<Option<Cow<'a, Literal>>, EvalError> {
    var.access_from_bindings(&env.bindings)
        .map_err(EvalError::VariableNotFound)
        .map(|opt| opt.map(Cow::Owned::<Literal>))
}

fn eval_fncall<'a>(
    function: &'a FunctionItem,
    env: &Env,
) -> Result<Option<Cow<'a, Literal>>, EvalError> {
    let func = env
        .vtable
        .get(function.name())
        .ok_or_else(|| EvalError::FunctionNotFound(function.name().to_string()))?;

    let args: Vec<Option<Literal>> = function
        .args()
        .iter()
        .map(|arg| eval(arg, env))
        .map(|res| res.map(|opt| opt.map(Cow::into_owned)))
        .collect::<Result<Vec<_>, _>>()?;

    func(&args)
        .map_err(EvalError::FnCallError)
        .map(|opt| opt.map(Cow::Owned))
}

pub(super) fn eval<'a>(exp: &'a Exp, env: &Env) -> Result<Option<Cow<'a, Literal>>, EvalError> {
    match exp {
        Exp::Literal(literal) => Ok(Some(Cow::Borrowed(literal))),
        Exp::Var(var) => eval_varaccess(var, env),
        Exp::FnCall(function) => eval_fncall(function, env),
        Exp::Not(exp) => eval_not(exp, env),
        Exp::Or(exp1, exp2) => eval_or(exp1, exp2, env),
        Exp::And(exp1, exp2) => eval_and(exp1, exp2, env),
        Exp::Eq(exp1, exp2) => eval_eq(exp1, exp2, env),
        Exp::Neq(exp, exp1) => eval_neq(exp, exp1, env),
        Exp::Gt(exp, exp1) => eval_cmp(exp, exp1, env, Cmp::Gt),
        Exp::Lt(exp, exp1) => eval_cmp(exp, exp1, env, Cmp::Lt),
        Exp::Geq(exp, exp1) => eval_cmp(exp, exp1, env, Cmp::Geq),
        Exp::Leq(exp, exp1) => eval_cmp(exp, exp1, env, Cmp::Leq),
    }
}
