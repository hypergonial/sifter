pub mod errors;
pub mod functions;
pub mod interpreter;
pub mod parser;
pub mod types;

pub use serde_json::Value as JsonValue;

pub use errors::{EvalError, FnCallError, VarAccessError};
pub use functions::{DEFAULT_VTABLE, FnArgs, FnCallback, FnResult, VTable};
pub use types::{Env, Exp, Literal, Type, VarAccess};
