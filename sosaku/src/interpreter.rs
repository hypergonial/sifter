use std::{borrow::Cow, collections::BTreeMap, fmt::Debug};

use crate::{
    JsonValue, VarAccessError,
    errors::EvalError,
    types::{Env, VarAccess},
};

use super::types::{Exp, Value};

#[inline]
#[expect(clippy::unnecessary_wraps)]
const fn out(value: Value<'_>) -> Result<Cow<'_, Value<'_>>, EvalError> {
    Ok(Cow::Owned(value))
}

fn eval_varaccess<'a: 'c, 'b: 'c, 'c, V: JsonValue + Clone + Debug>(
    var: &'a VarAccess,
    env: &'b Env<'b, '_, V>,
) -> Result<Cow<'c, Value<'c>>, EvalError> {
    let obj = var
        .access_from_bindings(env)
        .map_err(EvalError::VarAccess)?;
    let value = Value::from_json_object_cow(obj)
        .map_err(|e| EvalError::VarAccess(VarAccessError::ConversionError { message: e }))?;
    Ok(Cow::Owned(value))
}

#[derive(Debug, Clone, Copy)]
enum Cmp {
    Lt,
    Gt,
    Leq,
    Geq,
}

// Iterative impl

#[derive(Debug)]
enum Frame<'exp> {
    // Value
    ObjectKey(&'exp String),
    ToEval(&'exp Exp<'exp>),
    // Operators
    Neg,
    Or {
        rhs: &'exp Exp<'exp>,
    },
    And {
        rhs: &'exp Exp<'exp>,
    },
    Eq,
    Neq,
    Cmp(Cmp),
    FnCall {
        function_name: &'exp str,
        args_len: usize,
    },
    Array {
        total_len: usize,
    },
    Object {
        total_len: usize,
    },
}

trait FrameExt<'exp> {
    fn push_frames(&mut self, exp: &'exp Exp<'exp>);
}

impl<'exp> FrameExt<'exp> for Vec<Frame<'exp>> {
    /// Push the necessary frames onto the stack to evaluate the given expression
    /// in the next eval iteration(s).
    ///
    /// ## Example
    ///
    /// Expression: `Exp::Array(1,2,3)`
    ///
    /// Frames pushed (top of stack last):
    /// - `Frame::Array { total_len: 3 }`
    /// - `Frame::ToEval(Exp::Literal(3))`
    /// - `Frame::ToEval(Exp::Literal(2))`
    /// - `Frame::ToEval(Exp::Literal(1))`
    ///
    /// The eval loop will then pop the values,
    /// push them onto the value stack, and when it sees the [`Frame::Array`] frame,
    /// it will pop the 3 values from the value stack and construct the array value.
    fn push_frames(&mut self, exp: &'exp Exp<'exp>) {
        match exp {
            Exp::Literal(_) | Exp::Var(_) => self.push(Frame::ToEval(exp)),
            Exp::Neg(inner) => {
                self.push(Frame::Neg);
                self.push(Frame::ToEval(inner));
            }
            Exp::And(lhs, rhs) => {
                self.push(Frame::And { rhs });
                self.push(Frame::ToEval(lhs));
            }
            Exp::Or(lhs, rhs) => {
                self.push(Frame::Or { rhs });
                self.push(Frame::ToEval(lhs));
            }
            Exp::Eq(lhs, rhs) => {
                self.push(Frame::Eq);
                self.push(Frame::ToEval(rhs));
                self.push(Frame::ToEval(lhs));
            }
            Exp::Neq(lhs, rhs) => {
                self.push(Frame::Neq);
                self.push(Frame::ToEval(rhs));
                self.push(Frame::ToEval(lhs));
            }
            Exp::Gt(lhs, rhs) => {
                self.push(Frame::Cmp(Cmp::Gt));
                self.push(Frame::ToEval(rhs));
                self.push(Frame::ToEval(lhs));
            }
            Exp::Lt(lhs, rhs) => {
                self.push(Frame::Cmp(Cmp::Lt));
                self.push(Frame::ToEval(rhs));
                self.push(Frame::ToEval(lhs));
            }
            Exp::Geq(lhs, rhs) => {
                self.push(Frame::Cmp(Cmp::Geq));
                self.push(Frame::ToEval(rhs));
                self.push(Frame::ToEval(lhs));
            }
            Exp::Leq(lhs, rhs) => {
                self.push(Frame::Cmp(Cmp::Leq));
                self.push(Frame::ToEval(rhs));
                self.push(Frame::ToEval(lhs));
            }
            Exp::FnCall(func) => {
                self.push(Frame::FnCall {
                    function_name: func.name(),
                    args_len: func.args().len(),
                });
                for arg in func.args().iter().rev() {
                    self.push(Frame::ToEval(arg));
                }
            }
            Exp::Array(items) => {
                self.push(Frame::Array {
                    total_len: items.len(),
                });
                for item in items {
                    self.push(Frame::ToEval(item));
                }
            }
            Exp::Object(map) => {
                self.push(Frame::Object {
                    total_len: map.len(),
                });
                for (key, value) in map {
                    self.push(Frame::ToEval(value));
                    self.push(Frame::ObjectKey(key));
                }
            }
        }
    }
}

fn eval_neg<'exp: 'out, 'out>(values: &mut Vec<Cow<'out, Value<'out>>>) {
    let value: bool = values.pop().expect("Value stack underflow").as_ref().into();
    values.push(Cow::Owned(Value::Bool(!value)));
}

fn eval_and<'exp: 'out, 'out>(
    rhs: &'exp Exp<'exp>,
    stack: &mut Vec<Frame<'exp>>,
    values: &mut Vec<Cow<'out, Value<'out>>>,
) {
    let lhs = values.pop().expect("Value stack underflow");
    if bool::from(&*lhs) {
        stack.push(Frame::ToEval(rhs));
    } else {
        values.push(Cow::Owned(Value::Bool(false)));
    }
}

fn eval_or<'exp: 'out, 'out>(
    rhs: &'exp Exp<'exp>,
    stack: &mut Vec<Frame<'exp>>,
    values: &mut Vec<Cow<'out, Value<'out>>>,
) {
    let lhs = values.pop().expect("Value stack underflow");
    if bool::from(&*lhs) {
        values.push(lhs);
    } else {
        stack.push(Frame::ToEval(rhs));
    }
}

fn eval_eq<'exp: 'out, 'out>(values: &mut Vec<Cow<'out, Value<'out>>>) {
    let rhs = values.pop().expect("Value stack underflow");
    let lhs = values.pop().expect("Value stack underflow");
    values.push(Cow::Owned(Value::Bool(lhs == rhs)));
}

fn eval_neq<'exp: 'out, 'out>(values: &mut Vec<Cow<'out, Value<'out>>>) {
    let rhs = values.pop().expect("Value stack underflow");
    let lhs = values.pop().expect("Value stack underflow");
    values.push(Cow::Owned(Value::Bool(lhs != rhs)));
}

fn eval_cmp<'exp: 'out, 'out>(
    cmp: Cmp,
    values: &mut Vec<Cow<'out, Value<'out>>>,
) -> Result<(), EvalError> {
    let rhs = values.pop().expect("Value stack underflow");
    let lhs = values.pop().expect("Value stack underflow");

    let res = match (lhs.as_ref(), rhs.as_ref()) {
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
                lhs.as_ref().type_name(),
                rhs.as_ref().type_name()
            ),
        }),
    }?;

    values.push(res);
    Ok(())
}

fn eval_array<'exp: 'out, 'out>(len: usize, values: &mut Vec<Cow<'out, Value<'out>>>) {
    let mut items = Vec::with_capacity(len);

    for _ in 0..len {
        items.push(values.pop().expect("Value stack underflow").into_owned());
    }

    values.push(Cow::Owned(Value::Array(items)));
}

fn eval_object<'exp: 'out, 'out>(
    len: usize,
    keys: &mut Vec<&'exp String>,
    values: &mut Vec<Cow<'out, Value<'out>>>,
) {
    let mut map = BTreeMap::new();

    for _ in 0..len {
        let key = keys.pop().expect("Object key stack underflow").clone();
        let value = values.pop().expect("Value stack underflow").into_owned();
        map.insert(key, value);
    }

    values.push(Cow::Owned(Value::Object(map)));
}

fn eval_fncall<'exp: 'out, 'var: 'out, 'out>(
    function_name: &'exp str,
    args_len: usize,
    env: &Env<'var, '_, impl JsonValue + Clone + Debug>,
    values: &mut Vec<Cow<'out, Value<'out>>>,
) -> Result<(), EvalError> {
    let func = env
        .vtable()
        .get(function_name)
        .ok_or_else(|| EvalError::FunctionNotFound {
            fn_name: function_name.to_string(),
        })?;

    let args: Vec<Value<'out>> = values
        .drain(values.len() - args_len..)
        .map(Cow::into_owned)
        .collect();

    let ret = func
        .call_sync(function_name, &args)
        .map_err(EvalError::FnCallError)
        .map(|l| Cow::Owned::<Value<'out>>(l.into_owned()))?;

    values.push(ret);

    Ok(())
}

async fn eval_fncall_async<'exp: 'out, 'var: 'out, 'out>(
    function_name: &'exp str,
    args_len: usize,
    env: &Env<'var, '_, impl JsonValue + Clone + Debug>,
    values: &mut Vec<Cow<'out, Value<'out>>>,
) -> Result<(), EvalError> {
    let func = env
        .vtable()
        .get(function_name)
        .ok_or_else(|| EvalError::FunctionNotFound {
            fn_name: function_name.to_string(),
        })?;

    let args: Vec<Value<'out>> = values
        .drain(values.len() - args_len..)
        .map(Cow::into_owned)
        .collect();

    let ret = func
        .call_async(function_name, &args)
        .await
        .map_err(EvalError::FnCallError)
        .map(|l| Cow::Owned::<Value<'out>>(l.into_owned()))?;

    values.push(ret);

    Ok(())
}

pub(crate) fn eval<'exp: 'out, 'var: 'out, 'out, V: JsonValue + Clone + Debug>(
    exp: &'exp Exp<'exp>,
    env: &'var Env<'var, '_, V>,
) -> Result<Cow<'out, Value<'out>>, EvalError> {
    let mut stack = vec![Frame::ToEval(exp)];
    let mut values: Vec<Cow<'out, Value<'out>>> = Vec::new();
    let mut obj_keys: Vec<&'exp String> = Vec::new();

    while let Some(frame) = stack.pop() {
        match frame {
            Frame::ToEval(exp) => match exp {
                Exp::Literal(lit) => values.push(Cow::Borrowed(lit)),
                Exp::Var(var) => values.push(eval_varaccess(var, env)?),
                _ => stack.push_frames(exp),
            },
            Frame::ObjectKey(k) => obj_keys.push(k),
            Frame::Neg => eval_neg(&mut values),
            Frame::And { rhs } => eval_and(rhs, &mut stack, &mut values),
            Frame::Or { rhs } => eval_or(rhs, &mut stack, &mut values),
            Frame::Eq => eval_eq(&mut values),
            Frame::Neq => eval_neq(&mut values),
            Frame::Cmp(cmp) => eval_cmp(cmp, &mut values)?,
            Frame::FnCall {
                function_name,
                args_len,
            } => eval_fncall(function_name, args_len, env, &mut values)?,

            Frame::Array { total_len } => eval_array(total_len, &mut values),
            Frame::Object { total_len } => eval_object(total_len, &mut obj_keys, &mut values),
        }
    }

    values.pop().ok_or_else(|| EvalError::ValueError {
        message: "No value on stack after evaluation".to_string(),
    })
}

pub(crate) async fn eval_async<'exp: 'out, 'var: 'out, 'out, V: JsonValue + Clone + Debug>(
    exp: &'exp Exp<'exp>,
    env: &'var Env<'var, '_, V>,
) -> Result<Cow<'out, Value<'out>>, EvalError> {
    let mut stack = vec![Frame::ToEval(exp)];
    let mut values: Vec<Cow<'out, Value<'out>>> = Vec::new();
    let mut obj_keys: Vec<&'exp String> = Vec::new();

    while let Some(frame) = stack.pop() {
        match frame {
            Frame::ToEval(exp) => match exp {
                Exp::Literal(lit) => values.push(Cow::Borrowed(lit)),
                Exp::Var(var) => values.push(eval_varaccess(var, env)?),
                _ => stack.push_frames(exp),
            },
            Frame::ObjectKey(k) => obj_keys.push(k),
            Frame::Neg => eval_neg(&mut values),
            Frame::And { rhs } => eval_and(rhs, &mut stack, &mut values),
            Frame::Or { rhs } => eval_or(rhs, &mut stack, &mut values),
            Frame::Eq => eval_eq(&mut values),
            Frame::Neq => eval_neq(&mut values),
            Frame::Cmp(cmp) => eval_cmp(cmp, &mut values)?,
            Frame::FnCall {
                function_name,
                args_len,
            } => eval_fncall_async(function_name, args_len, env, &mut values).await?,

            Frame::Array { total_len } => eval_array(total_len, &mut values),
            Frame::Object { total_len } => eval_object(total_len, &mut obj_keys, &mut values),
        }
    }

    values.pop().ok_or_else(|| EvalError::ValueError {
        message: "No value on stack after evaluation".to_string(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::{Arc, LazyLock};

    use crate::{
        DEFAULT_VTABLE, FnArgs, FnCallError, FnCallback, FnResult, VarAccessError,
        types::FunctionItem,
    };

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

    async fn async_func(_args: FnArgs<'_>, state: Arc<str>) -> FnResult<'_> {
        tokio::task::yield_now().await;
        Ok(Value::String(Cow::Owned(state.to_string())))
    }

    static ASYNC_CALLBACK: LazyLock<FnCallback> = LazyLock::new(|| {
        let state: Arc<str> = Arc::from("async result");
        FnCallback::new_async(move |args| Box::pin(async_func(args, Arc::clone(&state))))
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

    #[tokio::test]
    async fn test_exps_async() {
        for (exp_str, expected) in &*EXPS {
            println!("Testing expression (async): {exp_str}");
            let exp = (*exp_str).try_into().unwrap();
            let result = eval_async(&exp, &ENV).await;
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

    #[tokio::test]
    async fn test_eval_fncall_async() {
        let callback = ASYNC_CALLBACK.clone();
        let mut vtable = DEFAULT_VTABLE.clone();
        vtable.insert("asyncFunc", callback);

        let env = Env::<Value>::new()
            .use_vtable(vtable)
            .bind("x", Value::Int(42))
            .build();

        let exp = Exp::fn_call(FunctionItem::new("asyncFunc", []));
        let result = eval_async(&exp, &env).await.unwrap();
        assert_eq!(result.into_owned(), Value::String("async result".into()));
    }

    #[test]
    fn test_eval_fncall_async_in_sync() {
        let callback = ASYNC_CALLBACK.clone();
        let mut vtable = DEFAULT_VTABLE.clone();
        vtable.insert("asyncFunc", callback);

        let env = Env::<Value>::new()
            .use_vtable(vtable)
            .bind("x", Value::Int(42))
            .build();

        let exp = Exp::fn_call(FunctionItem::new("asyncFunc", []));
        let result = eval(&exp, &env);
        assert_eq!(
            result.unwrap_err(),
            EvalError::FnCallError(FnCallError {
                fn_name: "asyncFunc".to_string(),
                reason: EvalError::CallSyncinAsync.into(),
            })
        );
    }
}
