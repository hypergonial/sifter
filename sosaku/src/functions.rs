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
        ("replace", FnCallback::new_sync(replace)),
        ("format", FnCallback::new_sync(format)),
        ("join", FnCallback::new_sync(join)),
        ("split", FnCallback::new_sync(split)),
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

fn trinary<'a>(
    args: FnArgs<'a>,
    function: impl Fn(&Value<'a>, &Value<'a>, &Value<'a>) -> FnResult<'a>,
) -> FnResult<'a> {
    if args.len() != 3 {
        return Err(EvalError::ArgumentCount {
            expected: 3,
            got: args.len(),
        });
    }

    function(&args[0], &args[1], &args[2])
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

fn string_trinary<'a>(
    args: FnArgs<'a>,
    function: impl Fn(&str, &str, &str) -> FnResult<'a>,
) -> FnResult<'a> {
    trinary(args, |v1, v2, v3| {
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
        let Value::String(s3) = v3 else {
            return Err(EvalError::TypeError {
                message: format!(
                    "Expected a string as the third argument, got: '{}'",
                    v3.type_name()
                ),
            });
        };

        function(s1, s2, s3)
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

fn replace(args: FnArgs<'_>) -> FnResult<'_> {
    string_trinary(args, |s, from, to| {
        Ok(Value::String(Cow::Owned(s.replace(from, to))))
    })
}

fn join(args: FnArgs<'_>) -> FnResult<'_> {
    if args.len() != 2 {
        return Err(EvalError::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }

    let Value::Array(ref arr) = args[0] else {
        return Err(EvalError::TypeError {
            message: format!("Expected an array argument, got: '{}'", args[0].type_name()),
        });
    };

    let Value::String(sep) = &args[1] else {
        return Err(EvalError::TypeError {
            message: format!("Expected a string argument, got: '{}'", args[1].type_name()),
        });
    };

    Ok(Value::String(Cow::Owned(
        arr.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(sep.as_ref()),
    )))
}

fn split(args: FnArgs<'_>) -> FnResult<'_> {
    string_binary(args, |s, sep| {
        Ok(Value::Array(
            s.split(sep)
                .map(|part| Value::String(Cow::Owned(part.to_string())))
                .collect(),
        ))
    })
}

static FORMAT_PLACEHOLDER_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\{(\d+)\}").expect("Failed to compile format placeholder regex")
});

/// Example: format("{0} is {1}", "Rust", "awesome") -> "Rust is awesome"
///
/// Escape curly braces with double braces: "{{" or "}}" -> "{", "}"
fn format(args: FnArgs<'_>) -> FnResult<'_> {
    if args.is_empty() {
        return Err(EvalError::ArgumentCount {
            expected: 1,
            got: 0,
        });
    }

    let Value::String(format_str) = &args[0] else {
        return Err(EvalError::TypeError {
            message: format!(
                "Expected a string as the first argument, got: '{}'",
                args[0].type_name()
            ),
        });
    };

    let fmt_args = &args[1..]
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    // rough estimate to avoid too many reallocations
    let mut new =
        String::with_capacity(format_str.len() + fmt_args.iter().map(String::len).sum::<usize>());
    let mut last_match = 0;

    for m in FORMAT_PLACEHOLDER_REGEX.find_iter(format_str) {
        let is_escaped = m.start() > 0
            && &format_str[m.start() - 1..m.start()] == "{"
            && m.end() < format_str.len()
            && &format_str[m.end()..=m.end()] == "}";

        if is_escaped {
            new.push_str(&format_str[last_match..m.start()]);
            new.push_str(&m.as_str()[1..m.as_str().len() - 1]);
            last_match = m.end();
            continue;
        }

        let index_str = &format_str[m.start() + 1..m.end() - 1];

        let index: usize = index_str
            .parse::<usize>()
            .map_err(|e| EvalError::ValueError {
                message: format!("Invalid format placeholder index '{index_str}': {e}"),
            })?;
        if index >= fmt_args.len() {
            return Err(EvalError::ArgumentCount {
                expected: index + 1,
                got: args.len(),
            });
        }
        let replacement = &fmt_args[index];

        new.push_str(&format_str[last_match..m.start()]);
        new.push_str(replacement);
        last_match = m.end();
    }
    new.push_str(&format_str[last_match..]);

    Ok(Value::String(Cow::Owned(new)))
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_format() {
        let args = [
            Value::String("Hello, {0}! You have {1} new messages.".into()),
            Value::String("Alice".into()),
            Value::Int(5),
        ];

        let result = format(&args).unwrap();

        assert_eq!(
            result,
            Value::String("Hello, Alice! You have 5 new messages.".into())
        );
    }

    #[test]
    fn test_format_escaped_braces() {
        let args = [
            Value::String("This is a literal brace: {{0}}. Placeholder: {0}".into()),
            Value::String("test".into()),
        ];

        let result = format(&args).unwrap();

        assert_eq!(
            result,
            Value::String("This is a literal brace: {0}. Placeholder: test".into())
        );
    }

    #[test]
    fn test_format_placeholder_oob() {
        let args = [Value::String("Invalid placeholder: {123}".into())];
        let result = format(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_matches() {
        let args = [
            Value::String("hello world".into()),
            Value::String(r"^hello\s\w+$".into()),
        ];

        let result = matches(&args).unwrap();

        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_matches_invalid_regex() {
        let args = [
            Value::String("test".into()),
            Value::String(r"invalid(regex".into()),
        ];

        let result = matches(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_base64_encode_decode() {
        let original = "Hello, world!";
        let encode_args = [Value::String(original.into())];
        let encoded = base64_encode(&encode_args).unwrap();
        let decode_args = [encoded];
        let decoded = base64_decode(&decode_args).unwrap();
        assert_eq!(decoded, Value::String(original.into()));
    }

    #[test]
    fn test_len() {
        let args = [Value::String("hello".into())];
        let result = len(&args).unwrap();
        assert_eq!(result, Value::Int(5));

        let args = [Value::Array(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
        ])];
        let result = len(&args).unwrap();
        assert_eq!(result, Value::Int(3));
    }

    #[test]
    fn test_contains() {
        let args = [
            Value::String("hello world".into()),
            Value::String("world".into()),
        ];
        let result = contains(&args).unwrap();
        assert_eq!(result, Value::Bool(true));

        let args = [
            Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
            Value::Int(2),
        ];
        let result = contains(&args).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_replace() {
        let args = [
            Value::String("the quick brown fox".into()),
            Value::String("quick".into()),
            Value::String("lazy".into()),
        ];
        let result = replace(&args).unwrap();
        assert_eq!(result, Value::String("the lazy brown fox".into()));
    }

    #[test]
    fn test_type_conversions() {
        let args = [Value::Int(42)];
        let result = into_string(&args).unwrap();
        assert_eq!(result, Value::String("42".into()));

        let args = [Value::String("true".into())];
        let result = into_bool(&args).unwrap();
        assert_eq!(result, Value::Bool(true));

        let args = [Value::String("3.12".into())];
        let result = into_float(&args).unwrap();
        assert_eq!(result, Value::Float(3.12));
    }

    #[test]
    fn test_starts_with() {
        let args = [
            Value::String("hello world".into()),
            Value::String("hello".into()),
        ];
        let result = starts_with(&args).unwrap();
        assert_eq!(result, Value::Bool(true));

        let args = [
            Value::String("hello world".into()),
            Value::String("world".into()),
        ];
        let result = starts_with(&args).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn test_ends_with() {
        let args = [
            Value::String("hello world".into()),
            Value::String("world".into()),
        ];
        let result = ends_with(&args).unwrap();
        assert_eq!(result, Value::Bool(true));

        let args = [
            Value::String("hello world".into()),
            Value::String("hello".into()),
        ];
        let result = ends_with(&args).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn test_join() {
        let args = [
            Value::Array(vec![
                Value::String("apple".into()),
                Value::String("banana".into()),
                Value::String("cherry".into()),
            ]),
            Value::String(", ".into()),
        ];
        let result = join(&args).unwrap();
        assert_eq!(result, Value::String("apple, banana, cherry".into()));
    }

    #[test]
    fn test_split() {
        let args = [
            Value::String("apple,banana,cherry".into()),
            Value::String(",".into()),
        ];
        let result = split(&args).unwrap();
        assert_eq!(
            result,
            Value::Array(vec![
                Value::String("apple".into()),
                Value::String("banana".into()),
                Value::String("cherry".into()),
            ])
        );
    }
}
