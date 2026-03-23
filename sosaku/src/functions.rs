use std::{borrow::Cow, collections::HashMap, sync::LazyLock};

use crate::{EvalError, FnCallError};

use super::types::Value;

/// A sequence of arguments passed to a function, represented as a slice of [`Value`] values.
pub type FnArgs<'a> = &'a [Value<'a>];

/// The result of a function call, which can either be a successful [`Value`] value or an error if the function call fails.
pub type FnResult<'a> = Result<Value<'a>, FnCallError>;

/// The type of a function callback, takes a slice of arguments and returns a result or an error.
pub type FnCallback = for<'a> fn(FnArgs<'a>) -> FnResult<'a>;

/// A mapping of function names to their corresponding callback implementations,
/// used for evaluating function calls.
pub type VTable = HashMap<&'static str, FnCallback>;

/// The default function table, containing built-in functions like `length`, `startsWith`, etc.
pub static DEFAULT_VTABLE: LazyLock<VTable> = LazyLock::new(|| {
    let it: VTable = HashMap::from([
        ("matches", matches as FnCallback),
        ("len", len),
        ("startsWith", starts_with),
        ("endsWith", ends_with),
        ("contains", contains),
        ("bool", into_bool),
        ("string", into_string),
        ("int", into_int),
        ("float", into_float),
    ]);
    it
});

fn unary<'a>(
    fn_name: &'static str,
    args: FnArgs<'a>,
    function: impl Fn(&Value<'a>) -> FnResult<'a>,
) -> FnResult<'a> {
    if args.len() != 1 {
        return Err(FnCallError {
            fn_name: fn_name.to_string(),
            reason: EvalError::ArgumentCount {
                expected: 1,
                got: args.len(),
            }
            .into(),
        });
    }

    function(&args[0])
}

fn binary<'a>(
    fn_name: &'static str,
    args: FnArgs<'a>,
    function: impl Fn(&Value<'a>, &Value<'a>) -> FnResult<'a>,
) -> FnResult<'a> {
    if args.len() != 2 {
        return Err(FnCallError {
            fn_name: fn_name.to_string(),
            reason: EvalError::ArgumentCount {
                expected: 2,
                got: args.len(),
            }
            .into(),
        });
    }

    function(&args[0], &args[1])
}

fn string_binary<'a>(
    fn_name: &'static str,
    args: FnArgs<'a>,
    function: impl Fn(&str, &str) -> FnResult<'a>,
) -> FnResult<'a> {
    binary(fn_name, args, |v1, v2| {
        let Value::String(s1) = v1 else {
            return Err(FnCallError {
                fn_name: fn_name.to_string(),
                reason: EvalError::TypeError {
                    message: format!(
                        "Expected a string as the first argument, got: '{}'",
                        v1.type_name()
                    ),
                }
                .into(),
            });
        };
        let Value::String(s2) = v2 else {
            return Err(FnCallError {
                fn_name: fn_name.to_string(),
                reason: EvalError::TypeError {
                    message: format!(
                        "Expected a string as the second argument, got: '{}'",
                        v2.type_name()
                    ),
                }
                .into(),
            });
        };

        function(s1, s2)
    })
}

fn len(args: FnArgs<'_>) -> FnResult<'_> {
    unary("len", args, |v| {
        let len = match v {
            Value::String(s) => s.chars().count(),
            Value::Array(arr) => arr.len(),
            Value::Object(obj) => obj.len(),
            v => {
                return Err(FnCallError {
                    fn_name: "len".to_string(),
                    reason: EvalError::TypeError {
                        message: format!(
                            "Expected a string, array, or object, got: '{}'",
                            v.type_name()
                        ),
                    }
                    .into(),
                });
            }
        };
        Ok(Value::Int(len as i64))
    })
}

fn starts_with(args: FnArgs<'_>) -> FnResult<'_> {
    string_binary("startsWith", args, |s, other| {
        Ok(Value::Bool(s.starts_with(other)))
    })
}

fn ends_with(args: FnArgs<'_>) -> FnResult<'_> {
    string_binary("endsWith", args, |s, other| {
        Ok(Value::Bool(s.ends_with(other)))
    })
}

fn contains(args: FnArgs<'_>) -> FnResult<'_> {
    binary("contains", args, |v1, v2| {
        match (v1, v2) {
        (Value::String(s), Value::String(sub)) => Ok(Value::Bool(s.contains(sub.as_ref()))),
        (Value::Array(arr), item) => Ok(Value::Bool(arr.iter().any(|e| e == item))),
        (Value::Object(obj), Value::String(key)) => Ok(Value::Bool(obj.contains_key(key.as_ref()))),
        _ => Err(FnCallError {
            fn_name: "contains".to_string(),
            reason: EvalError::TypeError {
                message: format!("Expected (string, string), (array, value), or (object, string) arguments, got: ('{}', '{}')", v1.type_name(), v2.type_name()),
            }
            .into(),
        }),
    }
    })
}

fn matches(args: FnArgs<'_>) -> FnResult<'_> {
    string_binary("matches", args, |s, pattern| {
        let re = regex::Regex::new(pattern).map_err(|e| FnCallError {
            fn_name: "matches".to_string(),
            reason: EvalError::RegexError {
                message: format!("Invalid regex pattern: '{e}'"),
            }
            .into(),
        })?;
        Ok(Value::Bool(re.is_match(s)))
    })
}

fn into_bool(args: FnArgs<'_>) -> FnResult<'_> {
    if args.len() != 1 {
        return Err(FnCallError {
            fn_name: "bool".to_string(),
            reason: EvalError::ArgumentCount {
                expected: 1,
                got: args.len(),
            }
            .into(),
        });
    }

    Ok(Value::Bool(bool::from(&args[0])))
}

fn into_string<'a>(args: FnArgs<'a>) -> FnResult<'a> {
    if args.len() != 1 {
        return Err(FnCallError {
            fn_name: "string".to_string(),
            reason: EvalError::ArgumentCount {
                expected: 1,
                got: args.len(),
            }
            .into(),
        });
    }

    let string: Cow<'a, str> = args[0].to_string().into();

    Ok(Value::String(string))
}

fn numeric_convert<'a, T>(
    fn_name: &'static str,
    args: FnArgs<'a>,
    convert: impl Fn(&Value<'a>) -> Option<T>,
    wrap: impl Fn(T) -> Value<'a>,
) -> FnResult<'a> {
    if args.len() != 1 {
        return Err(FnCallError {
            fn_name: fn_name.to_string(),
            reason: EvalError::ArgumentCount {
                expected: 1,
                got: args.len(),
            }
            .into(),
        });
    }

    convert(&args[0])
        .ok_or_else(|| FnCallError {
            fn_name: fn_name.to_string(),
            reason: EvalError::TypeError {
                message: format!(
                    "Expected a value that can be converted to {fn_name}, got {:?}",
                    args[0]
                ),
            }
            .into(),
        })
        .map(wrap)
}

fn into_int(args: FnArgs<'_>) -> FnResult<'_> {
    numeric_convert("int", args, |v| i64::try_from(v).ok(), Value::Int)
}

fn into_float(args: FnArgs<'_>) -> FnResult<'_> {
    numeric_convert("float", args, |v| f64::try_from(v).ok(), Value::Float)
}
