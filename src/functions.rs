use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use thiserror::Error;

use crate::interpreter::EvalError;

use super::types::Literal;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Error calling function '{fn_name}': {reason}")]
pub struct FnCallError {
    pub fn_name: String,
    #[source]
    pub reason: Box<EvalError>,
}

pub type FnArgs<'a> = &'a [Option<Literal>];
pub type FnResult = Result<Option<Literal>, FnCallError>;
pub type FnCallback = fn(FnArgs<'_>) -> FnResult;

pub type VTable = HashMap<&'static str, FnCallback>;

pub static VTABLE: LazyLock<VTable> = LazyLock::new(|| {
    let it: VTable = HashMap::from([
        ("matches", matches as FnCallback),
        ("length", length),
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

fn string_unary<'a>(
    fn_name: &'static str,
    args: FnArgs<'a>,
    function: impl Fn(&'a str) -> FnResult,
) -> FnResult {
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

    match &args[0] {
        Some(Literal::String(s)) => function(s),
        _ => Err(FnCallError {
            fn_name: fn_name.to_string(),
            reason: EvalError::TypeError {
                message: "Expected a string".to_string(),
            }
            .into(),
        }),
    }
}

fn string_binary<'a>(
    fn_name: &'static str,
    args: FnArgs<'a>,
    function: impl Fn(&'a str, &'a str) -> FnResult,
) -> FnResult {
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

    match (&args[0], &args[1]) {
        (Some(Literal::String(s)), Some(Literal::String(other))) => function(s, other),
        _ => Err(FnCallError {
            fn_name: fn_name.to_string(),
            reason: EvalError::TypeError {
                message: "Expected two strings".to_string(),
            }
            .into(),
        }),
    }
}

fn length(args: FnArgs<'_>) -> FnResult {
    string_unary("length", args, |s| Ok(Some(Literal::Int(s.len() as i64))))
}

fn starts_with(args: FnArgs<'_>) -> FnResult {
    string_binary("startsWith", args, |s, other| {
        Ok(Some(Literal::Bool(s.starts_with(other))))
    })
}

fn ends_with(args: FnArgs<'_>) -> FnResult {
    string_binary("endsWith", args, |s, other| {
        Ok(Some(Literal::Bool(s.ends_with(other))))
    })
}

fn contains(args: FnArgs<'_>) -> FnResult {
    string_binary("contains", args, |s, other| {
        Ok(Some(Literal::Bool(s.contains(other))))
    })
}

fn matches(args: FnArgs<'_>) -> FnResult {
    string_binary("matches", args, |s, pattern| {
        let re = regex::Regex::new(pattern).map_err(|e| FnCallError {
            fn_name: "matches".to_string(),
            reason: EvalError::RegexError {
                message: format!("Invalid regex pattern: {e}"),
            }
            .into(),
        })?;
        Ok(Some(Literal::Bool(re.is_match(s))))
    })
}

fn into_bool(args: FnArgs<'_>) -> FnResult {
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

    Ok(Some(Literal::Bool(
        args[0].as_ref().is_some_and(bool::from),
    )))
}

fn into_string(args: FnArgs<'_>) -> FnResult {
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

    let string: Arc<str> = args[0].as_ref().map_or_else(|| "null".into(), Into::into);

    Ok(Some(Literal::String(string)))
}

fn numeric_convert<T>(
    fn_name: &'static str,
    args: FnArgs<'_>,
    convert: impl Fn(&Literal) -> Option<T>,
    wrap: impl Fn(T) -> Literal,
) -> FnResult {
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

    match &args[0] {
        Some(value) => convert(value)
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
            .map(|v| Some(wrap(v))),
        None => Err(FnCallError {
            fn_name: fn_name.to_string(),
            reason: EvalError::TypeError {
                message: format!("Expected a value that can be converted to {fn_name}, got null"),
            }
            .into(),
        }),
    }
}

fn into_int(args: FnArgs<'_>) -> FnResult {
    numeric_convert("int", args, |v| i64::try_from(v).ok(), Literal::Int)
}

fn into_float(args: FnArgs<'_>) -> FnResult {
    numeric_convert("float", args, |v| f64::try_from(v).ok(), Literal::Float)
}
