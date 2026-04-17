use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Debug,
    pin::Pin,
    sync::{Arc, LazyLock},
};

use base64::Engine;

use crate::{EvalError, FnCallError};

use super::types::Value;

/// A sequence of arguments passed to a function, represented as a slice of [`Value`] values.
pub type FnArgs<'a> = &'a [Value<'a>];

/// The result of a function call, which can either be a successful [`Value`] value or an error if the function call fails.
pub type FnResult<'a> = Result<Value<'a>, EvalError>;

pub type SyncFnCallback = dyn for<'a> Fn(FnArgs<'a>) -> FnResult<'a> + Send + Sync;

/// Boxed async return type for callback trait objects.
pub type AsyncFnResult<'a> = Pin<Box<dyn Future<Output = FnResult<'a>> + Send + 'a>>;

pub type AsyncFnCallback = dyn for<'a> Fn(FnArgs<'a>) -> AsyncFnResult<'a> + Send + Sync;

/// Valid types for a function callback, which can be either synchronous or asynchronous.
///
/// To create a new [`FnCallback`], use the [`FnCallback::new_sync`] or [`FnCallback::new_async`] constructors.
#[derive(Clone)]
pub enum FnCallback {
    Sync(Arc<SyncFnCallback>),
    Async(Arc<AsyncFnCallback>),
}

impl FnCallback {
    /// Create a new synchronous function callback from the given closure or function item.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use std::borrow::Cow;
    /// use sosaku::{FnArgs, FnCallback, FnResult, Value};
    ///
    /// // With function items
    /// fn length<'a>(args: FnArgs<'a>) -> FnResult<'a> {
    ///     Ok(Value::Int(args.len() as i64))
    /// }
    ///
    /// let cb = FnCallback::new_sync(length);
    ///
    /// // With closures
    /// let prefix = String::from("arg count");
    /// let cb = FnCallback::new_sync(move |args| -> FnResult<'_> {
    ///   Ok(Value::String(Cow::Owned(format!("{prefix}: {}", args.len()))))
    /// });
    /// ```
    pub fn new_sync(
        callback: impl for<'a> Fn(FnArgs<'a>) -> FnResult<'a> + Send + Sync + 'static,
    ) -> Self {
        Self::Sync(Arc::new(callback))
    }

    /// Create a new asynchronous function callback from the given async closure or async function item.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use std::{borrow::Cow, sync::Arc};
    /// use sosaku::{AsyncFnResult, FnArgs, FnCallback, FnResult, Value};
    ///
    /// // With async function items
    /// async fn async_length<'a>(args: FnArgs<'a>, state: Arc<str>) -> FnResult<'a> {
    ///     tokio::task::yield_now().await; // simulate an async operation
    ///     Ok(Value::Int(args.len() as i64))
    /// }
    ///
    /// let shared_state: Arc<str> = Arc::from("cool stuff");
    /// let async_cb = FnCallback::new_async(
    ///     // The returned future must be pinned
    ///     move |args| Box::pin(async_length(args, Arc::clone(&shared_state)))
    /// );
    ///
    /// // With async closures
    /// let shared_state = Arc::new(String::from("yippee"));
    /// let async_cb = FnCallback::new_async(move |args| -> AsyncFnResult<'_> {
    ///     let state = shared_state.clone();
    ///     Box::pin(async move {
    ///         tokio::task::yield_now().await;
    ///         Ok(Value::String(Cow::Owned(format!("{state}: {}", args.len()))))
    ///     })
    /// });
    /// ```
    pub fn new_async(
        callback: impl for<'a> Fn(FnArgs<'a>) -> AsyncFnResult<'a> + Send + Sync + 'static,
    ) -> Self {
        Self::Async(Arc::new(callback))
    }

    /// Call this [`FnCallback`] in a synchronous context.
    ///
    /// ## Arguments
    ///
    /// - `name`: The name of the function being called, used for error reporting.
    /// - `args`: The arguments to pass to the function callback.
    ///
    /// ## Returns
    ///
    /// The return value of the function call.
    ///
    /// ## Errors
    ///
    /// Returns an error if the underlying function returns an error, or if this [`FnCallback`] is
    /// an async function which cannot be called in a synchronous context.
    pub(crate) fn call_sync<'a>(
        &self,
        name: &str,
        args: FnArgs<'a>,
    ) -> Result<Value<'a>, FnCallError> {
        match self {
            Self::Sync(cb) => cb(args).map_err(|e| FnCallError {
                fn_name: name.to_string(),
                reason: Box::new(e),
            }),
            Self::Async(_) => Err(FnCallError {
                fn_name: name.to_string(),
                reason: EvalError::CallSyncinAsync.into(),
            }),
        }
    }

    /// Call this [`FnCallback`] in an asynchronous context.
    ///
    /// ## Arguments
    ///
    /// - `name`: The name of the function being called, used for error reporting if the call fails.
    /// - `args`: The arguments to pass to the function callback.
    ///
    /// ## Returns
    ///
    /// The return value of the function call.
    ///
    /// ## Errors
    ///
    /// Returns an error if the underlying function returns an error.
    pub(crate) async fn call_async<'a>(
        &self,
        name: &str,
        args: FnArgs<'a>,
    ) -> Result<Value<'a>, FnCallError> {
        match self {
            Self::Sync(cb) => cb(args).map_err(|e| FnCallError {
                fn_name: name.to_string(),
                reason: Box::new(e),
            }),
            Self::Async(cb) => cb(args).await.map_err(|e| FnCallError {
                fn_name: name.to_string(),
                reason: Box::new(e),
            }),
        }
    }
}

impl Debug for FnCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sync(_) => write!(f, "SyncFnCallback"),
            Self::Async(_) => write!(f, "AsyncFnCallback"),
        }
    }
}

/// A mapping of function names to their corresponding callback implementations,
/// used for evaluating function calls.
pub type VTable = HashMap<&'static str, FnCallback>;

/// The default function table, containing built-in functions like `length`, `startsWith`, etc.
pub static DEFAULT_VTABLE: LazyLock<VTable> = LazyLock::new(|| {
    let it: VTable = HashMap::from([
        ("matches", FnCallback::new_sync(matches)),
        ("len", FnCallback::new_sync(len)),
        ("startsWith", FnCallback::new_sync(starts_with)),
        ("endsWith", FnCallback::new_sync(ends_with)),
        ("contains", FnCallback::new_sync(contains)),
        ("bool", FnCallback::new_sync(into_bool)),
        ("string", FnCallback::new_sync(into_string)),
        ("int", FnCallback::new_sync(into_int)),
        ("float", FnCallback::new_sync(into_float)),
        ("base64Encode", FnCallback::new_sync(base64_encode)),
        ("base64Decode", FnCallback::new_sync(base64_decode)),
    ]);
    it
});

fn unary<'a>(args: FnArgs<'a>, function: impl Fn(&Value<'a>) -> FnResult<'a>) -> FnResult<'a> {
    if args.len() != 1 {
        return Err(EvalError::ArgumentCount {
            expected: 1,
            got: args.len(),
        });
    }

    function(&args[0])
}

fn string_unary<'a>(args: FnArgs<'a>, function: impl Fn(&str) -> FnResult<'a>) -> FnResult<'a> {
    unary(args, |v| {
        let Value::String(s) = v else {
            return Err(EvalError::TypeError {
                message: format!("Expected a string argument, got: '{}'", v.type_name()),
            });
        };

        function(s)
    })
}

fn binary<'a>(
    args: FnArgs<'a>,
    function: impl Fn(&Value<'a>, &Value<'a>) -> FnResult<'a>,
) -> FnResult<'a> {
    if args.len() != 2 {
        return Err(EvalError::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }

    function(&args[0], &args[1])
}

fn string_binary<'a>(
    args: FnArgs<'a>,
    function: impl Fn(&str, &str) -> FnResult<'a>,
) -> FnResult<'a> {
    binary(args, |v1, v2| {
        let Value::String(s1) = v1 else {
            return Err(EvalError::TypeError {
                message: format!(
                    "Expected a string as the first argument, got: '{}'",
                    v1.type_name()
                ),
            });
        };
        let Value::String(s2) = v2 else {
            return Err(EvalError::TypeError {
                message: format!(
                    "Expected a string as the second argument, got: '{}'",
                    v2.type_name()
                ),
            });
        };

        function(s1, s2)
    })
}

fn len(args: FnArgs<'_>) -> FnResult<'_> {
    unary(args, |v| {
        let len = match v {
            Value::String(s) => s.chars().count(),
            Value::Array(arr) => arr.len(),
            Value::Object(obj) => obj.len(),
            v => {
                return Err(EvalError::TypeError {
                    message: format!(
                        "Expected a string, array, or object, got: '{}'",
                        v.type_name()
                    ),
                });
            }
        };
        Ok(Value::Int(len as i64))
    })
}

fn starts_with(args: FnArgs<'_>) -> FnResult<'_> {
    string_binary(args, |s, other| Ok(Value::Bool(s.starts_with(other))))
}

fn ends_with(args: FnArgs<'_>) -> FnResult<'_> {
    string_binary(args, |s, other| Ok(Value::Bool(s.ends_with(other))))
}

fn contains(args: FnArgs<'_>) -> FnResult<'_> {
    binary(args, |v1, v2| match (v1, v2) {
        (Value::String(s), Value::String(sub)) => Ok(Value::Bool(s.contains(sub.as_ref()))),
        (Value::Array(arr), item) => Ok(Value::Bool(arr.iter().any(|e| e == item))),
        (Value::Object(obj), Value::String(key)) => Ok(Value::Bool(obj.contains_key(key.as_ref()))),
        _ => Err(EvalError::TypeError {
            message: format!(
                "Expected (string, string), (array, value), or (object, string) arguments, got: ('{}', '{}')",
                v1.type_name(),
                v2.type_name()
            ),
        }),
    })
}

fn matches(args: FnArgs<'_>) -> FnResult<'_> {
    string_binary(args, |s, pattern| {
        let re = regex::Regex::new(pattern).map_err(|e| EvalError::RegexError {
            message: format!("Invalid regex pattern: '{e}'"),
        })?;
        Ok(Value::Bool(re.is_match(s)))
    })
}

fn base64_encode(args: FnArgs<'_>) -> FnResult<'_> {
    string_unary(args, |s| {
        Ok(Value::String(Cow::Owned(
            base64::engine::general_purpose::STANDARD.encode(s.as_bytes()),
        )))
    })
}

fn base64_decode(args: FnArgs<'_>) -> FnResult<'_> {
    string_unary(args, |s| {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(s.as_bytes())
            .map_err(|e| EvalError::ValueError {
                message: format!("Invalid base64 string: '{e}'"),
            })?;
        let decoded = String::from_utf8(bytes).map_err(|e| EvalError::ValueError {
            message: format!("Decoded bytes are not valid UTF-8: '{e}'"),
        })?;
        Ok(Value::String(Cow::Owned(decoded)))
    })
}

fn into_bool(args: FnArgs<'_>) -> FnResult<'_> {
    if args.len() != 1 {
        return Err(EvalError::ArgumentCount {
            expected: 1,
            got: args.len(),
        });
    }

    Ok(Value::Bool(bool::from(&args[0])))
}

fn into_string<'a>(args: FnArgs<'a>) -> FnResult<'a> {
    if args.len() != 1 {
        return Err(EvalError::ArgumentCount {
            expected: 1,
            got: args.len(),
        });
    }

    let string: Cow<'a, str> = args[0].to_string().into();

    Ok(Value::String(string))
}

fn numeric_convert<'a, T>(
    args: FnArgs<'a>,
    convert: impl Fn(&Value<'a>) -> Option<T>,
    wrap: impl Fn(T) -> Value<'a>,
) -> FnResult<'a> {
    if args.len() != 1 {
        return Err(EvalError::ArgumentCount {
            expected: 1,
            got: args.len(),
        });
    }

    convert(&args[0])
        .ok_or_else(|| EvalError::TypeError {
            message: format!(
                "Expected a value that can be converted to '{}', got '{:?}'",
                std::any::type_name::<T>(),
                args[0]
            ),
        })
        .map(wrap)
}

fn into_int(args: FnArgs<'_>) -> FnResult<'_> {
    numeric_convert(args, |v| i64::try_from(v).ok(), Value::Int)
}

fn into_float(args: FnArgs<'_>) -> FnResult<'_> {
    numeric_convert(args, |v| f64::try_from(v).ok(), Value::Float)
}
