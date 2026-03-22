pub mod errors;
pub mod functions;
pub mod interpreter;
pub mod parser;
pub mod types;

pub use errors::{EvalError, FnCallError, VarAccessError};
pub use functions::{DEFAULT_VTABLE, FnArgs, FnCallback, FnResult, VTable};
pub use types::{Env, Exp, JsonMap, JsonObject, Literal, Type, VarAccess};

#[cfg(feature = "serde")]
pub mod serde {
    pub use serde_json::{Map, Value};
}
