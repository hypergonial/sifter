use std::{collections::HashMap, sync::LazyLock};

use super::types::Literal;

pub enum FnCallError {
    ArgumentCount { expected: usize, got: usize },
    TypeError { message: String },
    ValueError { message: String },
    RegexError { message: String },
}

pub type FnCallback = Box<dyn Fn(&[Literal]) -> Result<Literal, FnCallError> + Send + Sync>;

pub type VTable = HashMap<&'static str, FnCallback>;

pub(super) static VTABLE: LazyLock<VTable> = LazyLock::new(|| {
    let mut it: VTable = HashMap::new();
    it.insert("startsWith", Box::new(starts_with));
    it.insert("endsWith", Box::new(ends_with));
    it.insert("contains", Box::new(contains));
    it.insert("length", Box::new(length));
    it
});

fn string_unary(
    inputs: &[Literal],
    function: impl Fn(&str) -> Result<Literal, FnCallError>,
) -> Result<Literal, FnCallError> {
    if inputs.len() != 1 {
        return Err(FnCallError::ArgumentCount {
            expected: 1,
            got: inputs.len(),
        });
    }

    match &inputs[0] {
        Literal::String(s) => function(s),
        _ => Err(FnCallError::TypeError {
            message: "Expected a string".to_string(),
        }),
    }
}

fn string_binary(
    inputs: &[Literal],
    function: impl Fn(&str, &str) -> Result<Literal, FnCallError>,
) -> Result<Literal, FnCallError> {
    if inputs.len() != 2 {
        return Err(FnCallError::ArgumentCount {
            expected: 2,
            got: inputs.len(),
        });
    }

    match (&inputs[0], &inputs[1]) {
        (Literal::String(s), Literal::String(other)) => function(s, other),
        _ => Err(FnCallError::TypeError {
            message: "Expected two strings".to_string(),
        }),
    }
}

fn length(inputs: &[Literal]) -> Result<Literal, FnCallError> {
    string_unary(inputs, |s| Ok(Literal::Int(s.len() as i64)))
}

fn starts_with(inputs: &[Literal]) -> Result<Literal, FnCallError> {
    string_binary(inputs, |s, other| Ok(Literal::Bool(s.starts_with(other))))
}

fn ends_with(inputs: &[Literal]) -> Result<Literal, FnCallError> {
    string_binary(inputs, |s, other| Ok(Literal::Bool(s.ends_with(other))))
}

fn contains(inputs: &[Literal]) -> Result<Literal, FnCallError> {
    string_binary(inputs, |s, other| Ok(Literal::Bool(s.contains(other))))
}

fn matches(inputs: &[Literal]) -> Result<Literal, FnCallError> {
    string_binary(inputs, |s, pattern| {
        let re = regex::Regex::new(pattern).map_err(|e| FnCallError::RegexError {
            message: format!("Invalid regex pattern: {e}"),
        })?;
        Ok(Literal::Bool(re.is_match(s)))
    })
}
