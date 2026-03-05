use std::{borrow::Cow, collections::HashMap, sync::Arc};

use serde_json::value;

use crate::functions::{FnCallError, VTABLE, VTable};

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
    fn new(bindings: HashMap<Box<str>, serde_json::Value>) -> Self {
        Self {
            bindings,
            vtable: VTABLE.clone(),
        }
    }
}

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
        |l| {
            if let Literal::Bool(b) = l {
                Some(*b)
            } else {
                None
            }
        },
        "a boolean",
    )
}

fn expect_string(value: Option<Cow<'_, Literal>>) -> Result<Arc<str>, EvalError> {
    expect_type(
        value,
        |l| {
            if let Literal::String(s) = l {
                Some(s.clone())
            } else {
                None
            }
        },
        "a string",
    )
}

fn expect_int(value: Option<Cow<'_, Literal>>) -> Result<i64, EvalError> {
    expect_type(
        value,
        |l| {
            if let Literal::Int(i) = l {
                Some(*i)
            } else {
                None
            }
        },
        "an integer",
    )
}

fn expect_float(value: Option<Cow<'_, Literal>>) -> Result<f64, EvalError> {
    expect_type(
        value,
        |l| {
            if let Literal::Float(f) = l {
                Some(*f)
            } else {
                None
            }
        },
        "a float",
    )
}

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
const fn out(literal: Literal) -> Result<Option<Cow<'static, Literal>>, EvalError> {
    Ok(Some(Cow::Owned(literal)))
}

pub fn eval<'a>(exp: &'a Exp, env: &Env) -> Result<Option<Cow<'a, Literal>>, EvalError> {
    match exp {
        Exp::Literal(literal) => Ok(Some(Cow::Borrowed(literal))),
        Exp::Var(var) => Ok(var
            .access_from_bindings(&env.bindings)
            .map_err(EvalError::VariableNotFound)?
            .map(Cow::Owned::<Literal>)),
        Exp::FnCall(function) => {
            let func = env
                .vtable
                .get(function.name())
                .ok_or_else(|| EvalError::FunctionNotFound(function.name().to_string()))?;

            let args: Vec<Option<Literal>> = function
                .inputs()
                .iter()
                .map(|arg| eval(arg, env))
                .map(|res| res.map(|opt| opt.map(Cow::into_owned)))
                .collect::<Result<Vec<_>, _>>()?;

            func(&args)
                .map_err(EvalError::FnCallError)
                .map(|opt| opt.map(Cow::Owned))
        }
        Exp::Not(exp) => {
            let value = expect_bool(eval(exp, env)?)?;
            out(Literal::Bool(!value))
        }
        Exp::Or(exp1, exp2) => {
            let value1 = expect_bool(eval(exp1, env)?)?;
            if value1 {
                out(Literal::Bool(true))
            } else {
                let value2 = expect_bool(eval(exp2, env)?)?;
                out(Literal::Bool(value2))
            }
        }
        Exp::And(exp1, exp2) => {
            let value1 = expect_bool(eval(exp1, env)?)?;
            if value1 {
                let value2 = expect_bool(eval(exp2, env)?)?;
                out(Literal::Bool(value2))
            } else {
                out(Literal::Bool(false))
            }
        }
        Exp::Eq(exp1, exp2) => {
            let value1 = eval(exp1, env)?;
            let value2 = eval(exp2, env)?;

            match (value1.as_deref(), value2.as_deref()) {
                (Some(Literal::Int(i1)), Some(Literal::Int(i2))) => out(Literal::Bool(i1 == i2)),
                (Some(Literal::String(s1)), Some(Literal::String(s2))) => {
                    out(Literal::Bool(s1 == s2))
                }
                (Some(Literal::Bool(b1)), Some(Literal::Bool(b2))) => out(Literal::Bool(b1 == b2)),
                (None, _) | (_, None) => out(Literal::Bool(false)),
                _ => Err(EvalError::TypeError(format!(
                    "Cannot compare values of different types: {:?} and {:?}",
                    value1.as_deref(),
                    value2.as_deref()
                ))),
            }
        }
        Exp::Neq(exp, exp1) => todo!(),
        Exp::Gt(exp, exp1) => todo!(),
        Exp::Lt(exp, exp1) => todo!(),
        Exp::Geq(exp, exp1) => todo!(),
        Exp::Leq(exp, exp1) => todo!(),
    }
}
