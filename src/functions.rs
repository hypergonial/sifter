use std::{collections::HashMap, sync::LazyLock};

use super::types::Literal;

pub enum FnCallError {
    ArgumentCount { expected: usize, got: usize },
    TypeError { message: String },
    ValueError { message: String },
    RegexError { message: String },
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
    ]);
    it
});

fn string_unary<'a>(args: FnArgs<'a>, function: impl Fn(&'a str) -> FnResult) -> FnResult {
    if args.len() != 1 {
        return Err(FnCallError::ArgumentCount {
            expected: 1,
            got: args.len(),
        });
    }

    match &args[0] {
        Some(Literal::String(s)) => function(s),
        _ => Err(FnCallError::TypeError {
            message: "Expected a string".to_string(),
        }),
    }
}

fn string_binary<'a>(
    args: FnArgs<'a>,
    function: impl Fn(&'a str, &'a str) -> FnResult,
) -> FnResult {
    if args.len() != 2 {
        return Err(FnCallError::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }

    match (&args[0], &args[1]) {
        (Some(Literal::String(s)), Some(Literal::String(other))) => function(s, other),
        _ => Err(FnCallError::TypeError {
            message: "Expected two strings".to_string(),
        }),
    }
}

fn length(args: FnArgs<'_>) -> FnResult {
    string_unary(args, |s| Ok(Some(Literal::Int(s.len() as i64))))
}

fn starts_with(args: FnArgs<'_>) -> FnResult {
    string_binary(args, |s, other| {
        Ok(Some(Literal::Bool(s.starts_with(other))))
    })
}

fn ends_with(args: FnArgs<'_>) -> FnResult {
    string_binary(args, |s, other| Ok(Some(Literal::Bool(s.ends_with(other)))))
}

fn contains(args: FnArgs<'_>) -> FnResult {
    string_binary(args, |s, other| Ok(Some(Literal::Bool(s.contains(other)))))
}

fn matches(args: FnArgs<'_>) -> FnResult {
    string_binary(args, |s, pattern| {
        let re = regex::Regex::new(pattern).map_err(|e| FnCallError::RegexError {
            message: format!("Invalid regex pattern: {e}"),
        })?;
        Ok(Some(Literal::Bool(re.is_match(s))))
    })
}
