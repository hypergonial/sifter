use std::{borrow::Cow, collections::BTreeMap, fmt::Debug};

use crate::{
    JsonValue, VarAccessError,
    errors::EvalError,
    types::{Env, FunctionItem, VarAccess},
};

use super::types::{Exp, Value};

#[derive(Debug, Clone, Copy)]
enum Cmp {
    Lt,
    Gt,
    Leq,
    Geq,
}

fn expect_type<'a, 'b, T>(
    value: &'b Value<'a>,
    extractor: impl Fn(&'b Value<'a>) -> Option<&'b T>,
    type_name: &str,
) -> Result<&'b T, EvalError> {
    extractor(value).ok_or_else(|| EvalError::TypeError {
        message: format!("Expected {}, got {}", type_name, value.type_name()),
    })
}

fn expect_bool(value: &Value<'_>) -> Result<bool, EvalError> {
    expect_type(
        value,
        move |l| match l {
            Value::Bool(b) => Some(b),
            _ => None,
        },
        "a boolean",
    )
    .map(ToOwned::to_owned)
}

#[inline]
#[expect(clippy::unnecessary_wraps)]
const fn out(value: Value<'_>) -> Result<Cow<'_, Value<'_>>, EvalError> {
    Ok(Cow::Owned(value))
}

fn eval_neg<'a: 'c, 'b: 'c, 'c, V: JsonValue + Clone + Debug>(
    exp: &'a Exp<'a>,
    env: &'b Env<'b, V>,
) -> Result<Cow<'c, Value<'c>>, EvalError> {
    let value: bool = eval(exp, env)?.as_ref().into();
    out(Value::Bool(!value))
}

fn eval_and<'a: 'c, 'b: 'c, 'c, V: JsonValue + Clone + Debug>(
    exp1: &'a Exp<'a>,
    exp2: &'a Exp<'a>,
    env: &'b Env<'b, V>,
) -> Result<Cow<'c, Value<'c>>, EvalError> {
    let value1 = eval(exp1, env)?;
    if bool::from(value1.as_ref()) {
        let value2 = eval(exp2, env)?;
        Ok(value2)
    } else {
        out(Value::Bool(false))
    }
}

fn eval_or<'a: 'c, 'b: 'c, 'c, V: JsonValue + Clone + Debug>(
    exp1: &'a Exp<'a>,
    exp2: &'a Exp<'a>,
    env: &'b Env<'b, V>,
) -> Result<Cow<'c, Value<'c>>, EvalError> {
    let value1 = eval(exp1, env)?;
    if bool::from(value1.as_ref()) {
        Ok(value1)
    } else {
        let value2 = eval(exp2, env)?;
        Ok(value2)
    }
}

fn eval_eq<'a: 'c, 'b: 'c, 'c, V: JsonValue + Clone + Debug>(
    exp1: &'a Exp,
    exp2: &'a Exp,
    env: &'b Env<'b, V>,
) -> Result<Cow<'c, Value<'c>>, EvalError> {
    let value1 = eval(exp1, env)?;
    let value2 = eval(exp2, env)?;

    Ok(Cow::Owned(Value::Bool(value1 == value2)))
}

fn eval_neq<'a: 'c, 'b: 'c, 'c, V: JsonValue + Clone + Debug>(
    exp1: &'a Exp<'a>,
    exp2: &'a Exp<'a>,
    env: &'b Env<'b, V>,
) -> Result<Cow<'c, Value<'c>>, EvalError> {
    let res = expect_bool(&*eval_eq(exp1, exp2, env)?)?;
    out(Value::Bool(!res))
}

fn eval_cmp<'a: 'c, 'b: 'c, 'c, V: JsonValue + Clone + Debug>(
    exp1: &'a Exp<'a>,
    exp2: &'a Exp<'a>,
    env: &'b Env<'b, V>,
    cmp: Cmp,
) -> Result<Cow<'c, Value<'c>>, EvalError> {
    let value1 = eval(exp1, env)?;
    let value2 = eval(exp2, env)?;

    match (value1.as_ref(), value2.as_ref()) {
        (Value::Int(i1), Value::Int(i2)) => out(Value::Bool(match cmp {
            Cmp::Lt => i1 < i2,
            Cmp::Gt => i1 > i2,
            Cmp::Leq => i1 <= i2,
            Cmp::Geq => i1 >= i2,
        })),
        (Value::Float(f1), Value::Float(f2)) => out(Value::Bool(match cmp {
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

fn eval_varaccess<'a: 'c, 'b: 'c, 'c, V: JsonValue + Clone + Debug>(
    var: &'a VarAccess,
    env: &'b Env<'b, V>,
) -> Result<Cow<'c, Value<'c>>, EvalError> {
    let obj = var
        .access_from_bindings(env)
        .map_err(EvalError::VarAccess)?;
    let value = Value::from_json_object_cow(obj)
        .map_err(|e| EvalError::VarAccess(VarAccessError::ConversionError { message: e }))?;
    Ok(Cow::Owned(value))
}

fn eval_array<'exp: 'out, 'var: 'out, 'out, V: JsonValue + Clone + Debug>(
    array: &'exp Vec<Exp<'exp>>,
    env: &'var Env<'var, V>,
) -> Result<Cow<'out, Value<'out>>, EvalError> {
    array
        .iter()
        .map(|e| eval(e, env).map(Cow::into_owned))
        .collect::<Result<Value<'_>, _>>()
        .map(Cow::Owned)
}

fn eval_object<'exp: 'out, 'var: 'out, 'out, V: JsonValue + Clone + Debug>(
    object: &'exp BTreeMap<String, Exp<'exp>>,
    env: &'var Env<'var, V>,
) -> Result<Cow<'out, Value<'out>>, EvalError> {
    object
        .iter()
        .map(|(k, e)| (k.clone(), eval(e, env).map(Cow::into_owned)))
        .map(|(k, rv)| rv.map(|v| (k, v)))
        .collect::<Result<Value<'_>, _>>()
        .map(Cow::Owned)
}

fn eval_fncall<'exp: 'out, 'var: 'out, 'out, V: JsonValue + Clone + Debug>(
    function: &'exp FunctionItem<'exp>,
    env: &'var Env<'var, V>,
) -> Result<Cow<'out, Value<'out>>, EvalError> {
    let func = env
        .vtable()
        .get(function.name())
        .ok_or_else(|| EvalError::FunctionNotFound {
            fn_name: function.name().to_string(),
        })?;

    let args: Vec<Value<'out>> = function
        .args()
        .iter()
        .map(|arg| eval(arg, env))
        .map(|res| res.map(Cow::into_owned))
        .collect::<Result<Vec<_>, _>>()?;

    func(&args)
        .map_err(EvalError::FnCallError)
        .map(|l| Cow::Owned(l.into_owned()))
}

pub(crate) fn eval<'exp: 'out, 'var: 'out, 'out, V: JsonValue + Clone + Debug>(
    exp: &'exp Exp,
    env: &'var Env<'var, V>,
) -> Result<Cow<'out, Value<'out>>, EvalError> {
    match exp {
        Exp::Literal(value) => Ok(Cow::Borrowed(value)),
        Exp::Array(array) => eval_array(array, env),
        Exp::Object(object) => eval_object(object, env),
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

    use crate::{FnCallError, VarAccessError};

    use super::*;

    static ENV: LazyLock<Env<Value>> = LazyLock::new(|| {
        Env::new()
            .bind("x", Value::Int(42))
            .bind("y", Value::String("hello".into()))
            .bind("z", Value::Bool(true))
            .bind(
                "foo",
                Value::Object(BTreeMap::from([
                    ("bar".to_string(), Value::Int(123)),
                    ("baz".to_string(), Value::String("world".into())),
                    (
                        "qux".to_string(),
                        Value::Object(BTreeMap::from([(
                            "nested".to_string(),
                            Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
                        )])),
                    ),
                ])),
            )
            .build()
    });

    static EXPS: LazyLock<[(&str, Result<Value, EvalError>); 33]> = LazyLock::new(|| {
        [
            // Basic variable access
            (
                r#"startsWith(y, "hel") && z && foo.bar > 100"#,
                Ok(Value::Bool(true)),
            ),
            // Accessing nested properties and comparing to a literal
            (
                r#"len(y) == 5 && foo.baz == "world""#,
                Ok(Value::Bool(true)),
            ),
            // Regex match with anchors
            (
                r#"matches(y, "^h.*o$") && foo.qux.nested[1] == 2"#,
                Ok(Value::Bool(true)),
            ),
            // Literal false negated
            ("!false", Ok(Value::Bool(true))),
            // Truthy int (42) negated
            ("!x", Ok(Value::Bool(false))),
            // Truthy non-empty string negated
            ("!y", Ok(Value::Bool(false))),
            // Empty string is falsy — falls through to z
            (r#""" || z"#, Ok(Value::Bool(true))),
            // Integer 0 is falsy — falls through to z
            ("0 || z", Ok(Value::Bool(true))),
            // && returns RHS value (not just bool) when LHS is truthy — JS-like semantics
            ("z && x", Ok(Value::Int(42))),
            // || returns first truthy value (LHS z, not RHS x)
            ("z || x", Ok(Value::Bool(true))),
            // Short-circuit &&: false LHS skips evaluation of non-existent RHS variable
            ("false && unknownVar", Ok(Value::Bool(false))),
            // Short-circuit ||: truthy LHS skips evaluation of non-existent RHS variable
            ("z || unknownVar", Ok(Value::Bool(true))),
            // == with mismatched types returns false, not a TypeError
            ("x == y", Ok(Value::Bool(false))),
            // != with mismatched types (int vs bool) returns true
            ("x != z", Ok(Value::Bool(true))),
            // >= at the exact boundary
            ("x >= 42", Ok(Value::Bool(true))),
            // <= at the exact boundary
            ("x <= 42", Ok(Value::Bool(true))),
            // Array indexing: nested[0]=1 < nested[2]=3
            (
                "foo.qux.nested[0] < foo.qux.nested[2]",
                Ok(Value::Bool(true)),
            ),
            // Array equality
            ("foo.qux.nested == [1, 2, 3]", Ok(Value::Bool(true))),
            // Object equality (order of keys shouldn't matter)
            (
                "foo == {\"baz\": \"world\", \"bar\": 123, \"qux\": {\"nested\": [1, 2, 3]}}",
                Ok(Value::Bool(true)),
            ),
            // endsWith function
            (r#"endsWith(y, "lo")"#, Ok(Value::Bool(true))),
            // contains — substring present
            (r#"contains(y, "ell")"#, Ok(Value::Bool(true))),
            // contains — substring absent
            (r#"contains(y, "xyz")"#, Ok(Value::Bool(false))),
            // contains — array contains value
            ("contains(foo.qux.nested, 2)", Ok(Value::Bool(true))),
            // contains - array does not contain value
            ("contains(foo.qux.nested, 42)", Ok(Value::Bool(false))),
            // contains - object contains key
            (r#"contains(foo, "bar")"#, Ok(Value::Bool(true))),
            // contains - object does not contain key
            (r#"contains(foo, "nonexistent")"#, Ok(Value::Bool(false))),
            // ! applied directly to a function call
            (r#"!matches(y, "^w")"#, Ok(Value::Bool(true))),
            // String literal on the LHS of ==
            (r#""hello" == y"#, Ok(Value::Bool(true))),
            // Both || operands are false — no short-circuit, result is false
            ("false || false", Ok(Value::Bool(false))),
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
                "len(42)",
                Err(EvalError::FnCallError(FnCallError {
                    fn_name: "len".to_string(),
                    reason: EvalError::TypeError {
                        message: "Expected a string, array, or object, got: 'int'".to_string(),
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
            let exp = (*exp_str).try_into().unwrap();
            let result = eval(&exp, &ENV);
            assert_eq!(result.map(Cow::into_owned).as_ref(), expected.as_ref());
        }
    }

    #[test]
    fn test_eval_literal() {
        let exp = Exp::literal(Value::Int(42));
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Value::Int(42));
    }

    #[test]
    fn test_eval_var() {
        let exp = Exp::varname("x").unwrap();
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Value::Int(42));

        let exp = Exp::varname("y").unwrap();
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Value::String("hello".into()));

        let exp = Exp::varname("foo.qux.nested[1]").unwrap();
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Value::Int(2));
    }

    #[test]
    fn test_eval_fncall() {
        let exp = Exp::fn_call(FunctionItem::new("len", [Exp::varname("y").unwrap()]));
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Value::Int(5));
    }

    #[test]
    fn test_eval_neg() {
        let exp = Exp::neg(Exp::varname("z").unwrap());
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Value::Bool(false));
    }

    #[test]
    fn test_eval_and_or() {
        let exp = Exp::and(Exp::varname("z").unwrap(), Exp::varname("x").unwrap());
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Value::Int(42));

        let exp = Exp::or(Exp::varname("z").unwrap(), Exp::varname("x").unwrap());
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Value::Bool(true));
    }

    #[test]
    fn test_eval_eq_neq() {
        let exp = Exp::eq(Exp::varname("x").unwrap(), Exp::literal(Value::Int(42)));
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Value::Bool(true));

        let exp = Exp::neq(Exp::varname("x").unwrap(), Exp::literal(Value::Int(42)));
        let result = eval(&exp, &ENV).unwrap();
        assert_eq!(result.into_owned(), Value::Bool(false));
    }
}
