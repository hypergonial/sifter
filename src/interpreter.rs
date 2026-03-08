use std::{borrow::Cow, collections::HashMap, sync::Arc};

use thiserror::Error;

use crate::{
    functions::{FnCallError, VTABLE, VTable},
    types::{FunctionItem, VarAccess, VarAccessError},
};

use super::types::{Exp, Literal};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EvalError {
    #[error(transparent)]
    VarAccess(#[from] VarAccessError),
    #[error(transparent)]
    FnCallError(#[from] FnCallError),
    #[error("Undefined function: {fn_name}")]
    FunctionNotFound { fn_name: String },
    #[error("Type Error: {message}")]
    TypeError { message: String },
    #[error("Value Error: {message}")]
    ValueError { message: String },
    #[error("Regex Error: {message}")]
    RegexError { message: String },
    #[error("Argument Error: Expected {expected} arguments, but got {got}")]
    ArgumentCount { expected: usize, got: usize },
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

    pub fn new_with_vtable(bindings: HashMap<Box<str>, serde_json::Value>, vtable: VTable) -> Self {
        Self { bindings, vtable }
    }

    #[inline]
    pub const fn bindings(&self) -> &HashMap<Box<str>, serde_json::Value> {
        &self.bindings
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
    value: Cow<'_, Literal>,
    extractor: impl Fn(&Literal) -> Option<T>,
    type_name: &str,
) -> Result<T, EvalError> {
    let value = value.as_ref();

    extractor(value).ok_or_else(|| EvalError::TypeError {
        message: format!("Expected {}, got {}", type_name, value.type_name()),
    })
}

fn expect_bool(value: Cow<'_, Literal>) -> Result<bool, EvalError> {
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
fn expect_string(value: Cow<'_, Literal>) -> Result<Arc<str>, EvalError> {
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
fn expect_int(value: Cow<'_, Literal>) -> Result<i64, EvalError> {
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
fn expect_float(value: Cow<'_, Literal>) -> Result<f64, EvalError> {
    expect_type(
        value,
        |l| match l {
            Literal::Float(f) => Some(*f),
            _ => None,
        },
        "a float",
    )
}

#[expect(dead_code)]
fn expect_null(value: Cow<'_, Literal>) -> Result<(), EvalError> {
    expect_type(
        value,
        |l| match l {
            Literal::Null => Some(()),
            _ => None,
        },
        "null",
    )
}

#[inline]
#[expect(clippy::unnecessary_wraps)]
const fn out(literal: Literal) -> Result<Cow<'static, Literal>, EvalError> {
    Ok(Cow::Owned(literal))
}

fn eval_neg<'a>(exp: &'a Exp, env: &Env) -> Result<Cow<'a, Literal>, EvalError> {
    let value: bool = eval(exp, env)?.as_ref().into();
    out(Literal::Bool(!value))
}

fn eval_and<'a>(exp1: &'a Exp, exp2: &'a Exp, env: &Env) -> Result<Cow<'a, Literal>, EvalError> {
    let value1 = eval(exp1, env)?;
    if bool::from(value1.as_ref()) {
        let value2 = eval(exp2, env)?;
        Ok(value2)
    } else {
        out(Literal::Bool(false))
    }
}

fn eval_or<'a>(exp1: &'a Exp, exp2: &'a Exp, env: &Env) -> Result<Cow<'a, Literal>, EvalError> {
    let value1 = eval(exp1, env)?;
    if bool::from(value1.as_ref()) {
        Ok(value1)
    } else {
        let value2 = eval(exp2, env)?;
        Ok(value2)
    }
}

fn eval_eq<'a>(exp1: &'a Exp, exp2: &'a Exp, env: &Env) -> Result<Cow<'a, Literal>, EvalError> {
    let value1 = eval(exp1, env)?;
    let value2 = eval(exp2, env)?;

    match (value1.as_ref(), value2.as_ref()) {
        (Literal::Int(i1), Literal::Int(i2)) => out(Literal::Bool(i1 == i2)),
        (Literal::String(s1), Literal::String(s2)) => out(Literal::Bool(s1 == s2)),
        (Literal::Bool(b1), Literal::Bool(b2)) => out(Literal::Bool(b1 == b2)),
        (Literal::Null, Literal::Null) => out(Literal::Bool(true)),
        _ => out(Literal::Bool(false)),
    }
}

fn eval_neq<'a>(exp1: &'a Exp, exp2: &'a Exp, env: &Env) -> Result<Cow<'a, Literal>, EvalError> {
    let res = eval_eq(exp1, exp2, env)?;
    let eq_value = expect_bool(res)?;
    out(Literal::Bool(!eq_value))
}

fn eval_cmp<'a>(
    exp1: &'a Exp,
    exp2: &'a Exp,
    env: &Env,
    cmp: Cmp,
) -> Result<Cow<'a, Literal>, EvalError> {
    let value1 = eval(exp1, env)?;
    let value2 = eval(exp2, env)?;

    match (value1.as_ref(), value2.as_ref()) {
        (Literal::Int(i1), Literal::Int(i2)) => out(Literal::Bool(match cmp {
            Cmp::Lt => i1 < i2,
            Cmp::Gt => i1 > i2,
            Cmp::Leq => i1 <= i2,
            Cmp::Geq => i1 >= i2,
        })),
        (Literal::Float(f1), Literal::Float(f2)) => out(Literal::Bool(match cmp {
            Cmp::Lt => f1 < f2,
            Cmp::Gt => f1 > f2,
            Cmp::Leq => f1 <= f2,
            Cmp::Geq => f1 >= f2,
        })),
        _ => Err(EvalError::TypeError {
            message: format!(
                "Cannot compare values of different types: {} and {}",
                value1.as_ref().type_name(),
                value2.as_ref().type_name()
            ),
        }),
    }
}

fn eval_varaccess<'a>(var: &'a VarAccess, env: &Env) -> Result<Cow<'a, Literal>, EvalError> {
    var.access_from_bindings(&env.bindings)
        .map_err(EvalError::VarAccess)
        .map(|opt| opt.map_or(Cow::Owned(Literal::Null), Cow::Owned::<Literal>))
}

fn eval_fncall<'a>(function: &'a FunctionItem, env: &Env) -> Result<Cow<'a, Literal>, EvalError> {
    let func = env
        .vtable
        .get(function.name())
        .ok_or_else(|| EvalError::FunctionNotFound {
            fn_name: function.name().to_string(),
        })?;

    let args: Vec<Literal> = function
        .args()
        .iter()
        .map(|arg| eval(arg, env))
        .map(|res| res.map(Cow::into_owned))
        .collect::<Result<Vec<_>, _>>()?;

    func(&args).map_err(EvalError::FnCallError).map(Cow::Owned)
}

pub(super) fn eval<'a>(exp: &'a Exp, env: &Env) -> Result<Cow<'a, Literal>, EvalError> {
    match exp {
        Exp::Literal(literal) => Ok(Cow::Borrowed(literal)),
        Exp::Var(var) => eval_varaccess(var, env),
        Exp::FnCall(function) => eval_fncall(function, env),
        Exp::Neg(exp) => eval_neg(exp, env),
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::LazyLock;

    use super::*;

    static ENV: LazyLock<Env> = LazyLock::new(|| {
        let bindings = HashMap::from([
            ("x".into(), serde_json::json!(42)),
            ("y".into(), serde_json::json!("hello")),
            ("z".into(), serde_json::json!(true)),
            (
                "foo".into(),
                serde_json::json!({
                    "bar": 123,
                    "baz": "world",
                    "qux": {
                        "nested": [
                            1,
                            2,
                            3
                        ]
                    }
                }),
            ),
        ]);
        Env::new(bindings)
    });

    static EXPS: LazyLock<[(&str, Result<Literal, EvalError>); 27]> = LazyLock::new(|| {
        [
            // Basic variable access
            (
                r#"startsWith(y, "hel") && z && foo.bar > 100"#,
                Ok(Literal::Bool(true)),
            ),
            // Accessing nested properties and comparing to a literal
            (
                r#"length(y) == 5 && foo.baz == "world""#,
                Ok(Literal::Bool(true)),
            ),
            // Regex match with anchors
            (
                r#"matches(y, "^h.*o$") && foo.qux.nested[1] == 2"#,
                Ok(Literal::Bool(true)),
            ),
            // Literal false negated
            ("!false", Ok(Literal::Bool(true))),
            // Truthy int (42) negated
            ("!x", Ok(Literal::Bool(false))),
            // Truthy non-empty string negated
            ("!y", Ok(Literal::Bool(false))),
            // Empty string is falsy — falls through to z
            (r#""" || z"#, Ok(Literal::Bool(true))),
            // Integer 0 is falsy — falls through to z
            ("0 || z", Ok(Literal::Bool(true))),
            // && returns RHS value (not just bool) when LHS is truthy — JS-like semantics
            ("z && x", Ok(Literal::Int(42))),
            // || returns first truthy value (LHS z, not RHS x)
            ("z || x", Ok(Literal::Bool(true))),
            // Short-circuit &&: false LHS skips evaluation of non-existent RHS variable
            ("false && unknownVar", Ok(Literal::Bool(false))),
            // Short-circuit ||: truthy LHS skips evaluation of non-existent RHS variable
            ("z || unknownVar", Ok(Literal::Bool(true))),
            // == with mismatched types returns false, not a TypeError
            ("x == y", Ok(Literal::Bool(false))),
            // != with mismatched types (int vs bool) returns true
            ("x != z", Ok(Literal::Bool(true))),
            // >= at the exact boundary
            ("x >= 42", Ok(Literal::Bool(true))),
            // <= at the exact boundary
            ("x <= 42", Ok(Literal::Bool(true))),
            // Array indexing: nested[0]=1 < nested[2]=3
            (
                "foo.qux.nested[0] < foo.qux.nested[2]",
                Ok(Literal::Bool(true)),
            ),
            // endsWith function
            (r#"endsWith(y, "lo")"#, Ok(Literal::Bool(true))),
            // contains — substring present
            (r#"contains(y, "ell")"#, Ok(Literal::Bool(true))),
            // contains — substring absent
            (r#"contains(y, "xyz")"#, Ok(Literal::Bool(false))),
            // ! applied directly to a function call
            (r#"!matches(y, "^w")"#, Ok(Literal::Bool(true))),
            // String literal on the LHS of ==
            (r#""hello" == y"#, Ok(Literal::Bool(true))),
            // Both || operands are false — no short-circuit, result is false
            ("false || false", Ok(Literal::Bool(false))),
            (
                "x > y",
                Err(EvalError::TypeError {
                    message: "Cannot compare values of different types: int and string".to_string(),
                }),
            ),
            (
                "unknownFunc()",
                Err(EvalError::FunctionNotFound {
                    fn_name: "unknownFunc".to_string(),
                }),
            ),
            (
                "unknownVar",
                Err(EvalError::VarAccess(VarAccessError::VariableNotFound {
                    variable: "unknownVar".to_string(),
                })),
            ),
            (
                "length(42)",
                Err(EvalError::FnCallError(FnCallError {
                    fn_name: "length".to_string(),
                    reason: EvalError::TypeError {
                        message: "Expected a string".to_string(),
                    }
                    .into(),
                })),
            ),
        ]
    });

    #[test]
    fn test_exps() {
        for (exp_str, expected) in &*EXPS {
            println!("Testing expression: {exp_str}");
            let exp = Exp::parse(exp_str).unwrap();
            let result = eval(&exp, &ENV);
            assert_eq!(result.map(Cow::into_owned).as_ref(), expected.as_ref());
        }
    }

    #[test]
    fn test_eval_literal() {
        let exp = Exp::literal(Literal::Int(42));
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Literal::Int(42));
    }

    #[test]
    fn test_eval_var() {
        let exp = Exp::varname("x").unwrap();
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Literal::Int(42));

        let exp = Exp::varname("y").unwrap();
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Literal::String("hello".into()));

        let exp = Exp::varname("foo.qux.nested[1]").unwrap();
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Literal::Int(2));
    }

    #[test]
    fn test_eval_fncall() {
        let exp = Exp::fn_call(FunctionItem::new("length", [Exp::varname("y").unwrap()]));
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Literal::Int(5));
    }

    #[test]
    fn test_eval_neg() {
        let exp = Exp::neg(Exp::varname("z").unwrap());
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Literal::Bool(false));
    }

    #[test]
    fn test_eval_and_or() {
        let exp = Exp::and(Exp::varname("z").unwrap(), Exp::varname("x").unwrap());
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Literal::Int(42));

        let exp = Exp::or(Exp::varname("z").unwrap(), Exp::varname("x").unwrap());
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Literal::Bool(true));
    }

    #[test]
    fn test_eval_eq_neq() {
        let exp = Exp::eq(Exp::varname("x").unwrap(), Exp::literal(Literal::Int(42)));
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Literal::Bool(true));

        let exp = Exp::neq(Exp::varname("x").unwrap(), Exp::literal(Literal::Int(42)));
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Literal::Bool(false));
    }
}
