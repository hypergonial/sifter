use std::{borrow::Cow, collections::HashMap, sync::LazyLock};

use crate::{EvalError, FnCallError};

use super::types::Literal;

pub type FnArgs<'a> = &'a [Literal<'a>];
pub type FnResult<'a> = Result<Literal<'a>, FnCallError>;
pub type FnCallback = for<'a> fn(FnArgs<'a>) -> FnResult<'a>;

pub type VTable = HashMap<&'static str, FnCallback>;

pub static DEFAULT_VTABLE: LazyLock<VTable> = LazyLock::new(|| {
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
    function: impl Fn(&'a str) -> FnResult<'a>,
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

    match &args[0] {
        Literal::String(s) => function(s),
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
    function: impl Fn(&'a str, &'a str) -> FnResult<'a>,
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

    match (&args[0], &args[1]) {
        (Literal::String(s), Literal::String(other)) => function(s, other),
        _ => Err(FnCallError {
            fn_name: fn_name.to_string(),
            reason: EvalError::TypeError {
                message: "Expected two strings".to_string(),
            }
            .into(),
        }),
    }
}

fn length(args: FnArgs<'_>) -> FnResult<'_> {
    string_unary("length", args, |s| Ok(Literal::Int(s.len() as i64)))
}

fn starts_with(args: FnArgs<'_>) -> FnResult<'_> {
    string_binary("startsWith", args, |s, other| {
        Ok(Literal::Bool(s.starts_with(other)))
    })
}

fn ends_with(args: FnArgs<'_>) -> FnResult<'_> {
    string_binary("endsWith", args, |s, other| {
        Ok(Literal::Bool(s.ends_with(other)))
    })
}

fn contains(args: FnArgs<'_>) -> FnResult<'_> {
    string_binary("contains", args, |s, other| {
        Ok(Literal::Bool(s.contains(other)))
    })
}

fn matches(args: FnArgs<'_>) -> FnResult<'_> {
    string_binary("matches", args, |s, pattern| {
        let re = regex::Regex::new(pattern).map_err(|e| FnCallError {
            fn_name: "matches".to_string(),
            reason: EvalError::RegexError {
                message: format!("Invalid regex pattern: {e}"),
            }
            .into(),
        })?;
        Ok(Literal::Bool(re.is_match(s)))
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

    Ok(Literal::Bool(bool::from(&args[0])))
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

    let string: Cow<'a, str> = (&args[0]).into();

    Ok(Literal::String(string))
}

fn numeric_convert<'a, T>(
    fn_name: &'static str,
    args: FnArgs<'a>,
    convert: impl Fn(&Literal<'a>) -> Option<T>,
    wrap: impl Fn(T) -> Literal<'a>,
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
    numeric_convert("int", args, |v| i64::try_from(v).ok(), Literal::Int)
}

fn into_float(args: FnArgs<'_>) -> FnResult<'_> {
    numeric_convert("float", args, |v| f64::try_from(v).ok(), Literal::Float)
}
