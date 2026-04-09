use thiserror::Error;

pub use nom::error::Error as NomError;
pub type ParseError = NomError<String>;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Error calling function '{fn_name}': {reason}")]
pub struct FnCallError {
    pub fn_name: String,
    #[source]
    pub reason: Box<EvalError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VarAccessError {
    #[error("Variable access is empty")]
    EmptyAccess,
    #[error("Variable not found: {variable}")]
    VariableNotFound { variable: String },
    #[error("Object '{object}' does not contain key '{key}'")]
    ObjectKeyError { object: String, key: String },
    #[error("Type error: {message}")]
    TypeError { message: String },
    #[error("Index out of bounds: {message}")]
    IndexOutOfBounds { message: String },
    #[error("Conversion error: {message}")]
    ConversionError { message: String },
}

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
    #[error("Cannot call an async function in a sync context")]
    CallSyncinAsync,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    #[error(transparent)]
    Eval(#[from] EvalError),
    #[error("Parse Error: {0}")]
    Parse(#[from] ParseError),
}
