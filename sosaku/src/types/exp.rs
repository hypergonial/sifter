use std::{borrow::Cow, collections::BTreeMap, fmt::Debug};

use nom::Finish;

#[cfg(feature = "serde_json")]
use serde::Deserialize;

use crate::{
    Env, JsonValue, ParseError,
    errors::EvalError,
    parser::parse_exp,
    types::{func::FunctionItem, value::Value, var::VarAccess},
};

/// Represents an Abstract Syntax Tree (AST) for sosaku expressions,
/// which can be evaluated in a given environment to produce a literal value.
#[derive(Debug, Clone, PartialEq)]
pub enum Exp<'a> {
    Literal(Value<'a>),
    FnCall(FunctionItem<'a>),
    Var(VarAccess),
    Neg(Box<Self>),
    Or(Box<Self>, Box<Self>),
    And(Box<Self>, Box<Self>),
    Eq(Box<Self>, Box<Self>),
    Neq(Box<Self>, Box<Self>),
    Gt(Box<Self>, Box<Self>),
    Lt(Box<Self>, Box<Self>),
    Geq(Box<Self>, Box<Self>),
    Leq(Box<Self>, Box<Self>),
    Array(Vec<Self>),
    Object(BTreeMap<String, Self>),
}

impl<'exp> Exp<'exp> {
    /// Create a new [`Exp`] from a string representation of an expression.
    ///
    /// # Parameters
    ///
    /// - `string`: The string representation of the expression to parse.
    ///
    /// # Returns
    ///
    /// - <code>Ok([`Exp`])</code> if the expression was successfully parsed from the string.
    ///
    /// # Errors
    ///
    /// - If there was an error parsing the expression from the string,
    ///   such as a syntax error, an `Err` will be returned containing the parsing error details.
    ///
    ///   Note that semantic errors (e.g. undefined variables, type errors) are not handled by this
    ///   function and will not result in an error being returned here. Those errors will be encountered
    ///   during evaluation of the expression, and will be returned as [`EvalError`]s from the [`Exp::eval`] method.
    pub fn new(string: impl Into<&'exp str>) -> Result<Self, ParseError> {
        string.into().try_into()
    }

    /// Turn the expression into an owned version, where all borrowed data is cloned into owned data.
    ///
    /// This is useful for cases where you want to take ownership of an [`Exp`] that may contain
    /// borrowed data (e.g. from a JSON value) and ensure that it is fully owned and independent of any original data sources.
    ///
    /// Note that this will recursively clone all borrowed data in the expression, so it may be expensive for large expressions with a lot of borrowed data.
    /// However, if the expression is already fully owned, this will simply return a clone of the expression without any additional cloning of data.
    ///
    /// # Returns
    ///
    /// - An owned version of this expression, where all borrowed data has been cloned into owned data.
    pub fn into_owned(self) -> Exp<'static> {
        match self {
            Exp::Literal(lit) => Exp::Literal(lit.into_owned()),
            Exp::FnCall(func) => Exp::FnCall(FunctionItem::new(
                func.name().to_string(),
                func.args()
                    .iter()
                    .map(|e| e.clone().into_owned())
                    .collect::<Vec<_>>(),
            )),
            Exp::Var(var) => Exp::Var(var),
            Exp::Neg(e) => Exp::Neg(Box::new(e.into_owned())),
            Exp::Or(l, r) => Exp::Or(Box::new(l.into_owned()), Box::new(r.into_owned())),
            Exp::And(l, r) => Exp::And(Box::new(l.into_owned()), Box::new(r.into_owned())),
            Exp::Eq(l, r) => Exp::Eq(Box::new(l.into_owned()), Box::new(r.into_owned())),
            Exp::Neq(l, r) => Exp::Neq(Box::new(l.into_owned()), Box::new(r.into_owned())),
            Exp::Gt(l, r) => Exp::Gt(Box::new(l.into_owned()), Box::new(r.into_owned())),
            Exp::Lt(l, r) => Exp::Lt(Box::new(l.into_owned()), Box::new(r.into_owned())),
            Exp::Geq(l, r) => Exp::Geq(Box::new(l.into_owned()), Box::new(r.into_owned())),
            Exp::Leq(l, r) => Exp::Leq(Box::new(l.into_owned()), Box::new(r.into_owned())),
            Exp::Array(elems) => Exp::Array(elems.into_iter().map(Exp::into_owned).collect()),
            Exp::Object(map) => {
                Exp::Object(map.into_iter().map(|(k, v)| (k, v.into_owned())).collect())
            }
        }
    }

    /// Evaluate the expression in the given environment and return the resulting literal value.
    ///
    /// ## Parameters
    ///
    /// - `env`: The [`Env`] to evaluate the expression in, which contains variable bindings and function definitions.
    ///
    /// ## Returns
    ///
    /// - <code>Ok([Cow]<'_, [Literal]>)</code> if the expression was successfully evaluated, where the `Literal` is the resulting value of the expression.
    ///
    /// ## Errors
    ///
    /// - If there was an error during evaluation, such as a type error or undefined variable, an [`EvalError`] will be returned.
    pub fn eval<'var, 'out, V: JsonValue + Clone + Debug>(
        &'exp self,
        env: &'var Env<'var, V>,
    ) -> Result<Cow<'out, Value<'out>>, EvalError>
    where
        'exp: 'out,
        'var: 'out,
    {
        crate::interpreter::eval(self, env)
    }

    /// Create a new [`Exp`] representing a literal value.
    ///
    /// ## Parameters
    ///
    /// - `lit`: The literal value to create an expression for.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the literal value.
    #[inline]
    pub const fn literal(lit: Value<'exp>) -> Self {
        Self::Literal(lit)
    }

    /// Create a new [`Exp`] representing a variable access.
    ///
    /// ## Parameters
    ///
    /// - `accessor`: The variable access to create an expression for.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the variable access.
    #[inline]
    pub const fn var(accessor: VarAccess) -> Self {
        Self::Var(accessor)
    }

    /// Create a new [`Exp`] representing a function call.
    ///
    /// ## Parameters
    ///
    /// - `accessor`: The variable access syntax, e.g. "foo.bar[0].baz"
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the function call.
    ///
    /// ## Errors
    ///
    /// - If the variable access syntax is invalid
    #[inline]
    pub fn varname(accessor: &str) -> Result<Self, nom::error::Error<&str>> {
        VarAccess::try_from(accessor).map(Self::var)
    }

    /// Create a new [`Exp`] representing an array literal.
    ///
    /// ## Parameters
    ///
    /// - `elems`: The elements of the array, represented as a vector of [`Exp`] expressions.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the array literal.
    #[inline]
    pub const fn array(elems: Vec<Self>) -> Self {
        Self::Array(elems)
    }

    /// Create a new [`Exp`] representing an object literal.
    ///
    /// ## Parameters
    ///
    /// - `map`: The key-value pairs of the object, represented as a `BTreeMap`
    ///   where the key is a string and the value is an [`Exp`] expression.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the object literal.
    #[inline]
    pub const fn object(map: BTreeMap<String, Self>) -> Self {
        Self::Object(map)
    }

    /// Create a new [`Exp`] representing a function call.
    ///
    /// ## Parameters
    ///
    /// - `func`: The function to call, which includes the function name and its arguments.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the function call.
    #[inline]
    pub const fn fn_call(func: FunctionItem<'exp>) -> Self {
        Self::FnCall(func)
    }

    /// Create a new [`Exp`] representing a negation of another expression.
    ///
    /// ## Parameters
    ///
    /// - `exp`: The expression to negate.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the negation of the given expression.
    #[inline]
    #[expect(clippy::should_implement_trait)]
    pub fn neg(exp: Self) -> Self {
        Self::Neg(Box::new(exp))
    }

    /// Create a new [`Exp`] representing a logical OR of two expressions.
    ///
    /// ## Parameters
    ///
    /// - `lhs`: The left-hand side expression of the OR operation.
    /// - `rhs`: The right-hand side expression of the OR operation.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the logical OR of the two given expressions.
    #[inline]
    pub fn or(lhs: Self, rhs: Self) -> Self {
        Self::Or(Box::new(lhs), Box::new(rhs))
    }

    /// Create a new [`Exp`] representing a logical AND of two expressions.
    ///
    /// ## Parameters
    ///
    /// - `lhs`: The left-hand side expression of the AND operation.
    /// - `rhs`: The right-hand side expression of the AND operation.
    ///
    /// ## Returns
    /// - An [`Exp`] enum representing the logical AND of the two given expressions.
    #[inline]
    pub fn and(lhs: Self, rhs: Self) -> Self {
        Self::And(Box::new(lhs), Box::new(rhs))
    }

    /// Create a new [`Exp`] representing an equality comparison of two expressions.
    ///
    /// ## Parameters
    ///
    /// - `lhs`: The left-hand side expression of the equality comparison.
    /// - `rhs`: The right-hand side expression of the equality comparison.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the equality comparison of the two given expressions.
    #[inline]
    pub fn eq(lhs: Self, rhs: Self) -> Self {
        Self::Eq(Box::new(lhs), Box::new(rhs))
    }

    /// Create a new [`Exp`] representing an inequality comparison of two expressions.
    ///
    /// ## Parameters
    ///
    /// - `lhs`: The left-hand side expression of the inequality comparison.
    /// - `rhs`: The right-hand side expression of the inequality comparison.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the inequality comparison of the two given expressions.
    #[inline]
    pub fn neq(lhs: Self, rhs: Self) -> Self {
        Self::Neq(Box::new(lhs), Box::new(rhs))
    }

    /// Create a new [`Exp`] representing a greater-than comparison of two expressions.
    ///
    /// ## Parameters
    ///
    /// - `lhs`: The left-hand side expression of the greater-than comparison.
    /// - `rhs`: The right-hand side expression of the greater-than comparison.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the greater-than comparison of the two given expressions.
    #[inline]
    pub fn gt(lhs: Self, rhs: Self) -> Self {
        Self::Gt(Box::new(lhs), Box::new(rhs))
    }

    /// Create a new [`Exp`] representing a less-than comparison of two expressions.
    ///
    /// ## Parameters
    /// - `lhs`: The left-hand side expression of the less-than comparison.
    /// - `rhs`: The right-hand side expression of the less-than comparison.
    ///
    /// ## Returns
    /// - An [`Exp`] enum representing the less-than comparison of the two given expressions.
    #[inline]
    pub fn lt(lhs: Self, rhs: Self) -> Self {
        Self::Lt(Box::new(lhs), Box::new(rhs))
    }

    /// Create a new [`Exp`] representing a greater-than-or-equal-to comparison of two expressions.
    ///
    /// ## Parameters
    /// - `lhs`: The left-hand side expression of the greater-than-or-equal-to comparison.
    /// - `rhs`: The right-hand side expression of the greater-than-or-equal-to comparison.
    ///
    /// ## Returns
    /// - An [`Exp`] enum representing the greater-than-or-equal-to comparison of the two given expressions.
    #[inline]
    pub fn geq(lhs: Self, rhs: Self) -> Self {
        Self::Geq(Box::new(lhs), Box::new(rhs))
    }

    /// Create a new [`Exp`] representing a less-than-or-equal-to comparison of two expressions.
    ///
    /// ## Parameters
    /// - `lhs`: The left-hand side expression of the less-than-or-equal-to comparison.
    /// - `rhs`: The right-hand side expression of the less-than-or-equal-to comparison.
    ///
    /// ## Returns
    /// - An [`Exp`] enum representing the less-than-or-equal-to comparison of the two given expressions.
    #[inline]
    pub fn leq(lhs: Self, rhs: Self) -> Self {
        Self::Leq(Box::new(lhs), Box::new(rhs))
    }
}

impl<'a> TryFrom<&'a str> for Exp<'a> {
    type Error = nom::error::Error<String>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let (remainder, exp) = parse_exp(value).finish()?;

        if !remainder.trim().is_empty() {
            return Err(nom::error::Error::new(
                remainder.to_string(),
                nom::error::ErrorKind::Eof,
            ));
        }

        Ok(exp)
    }
}

#[cfg(feature = "serde_json")]
impl<'de> Deserialize<'de> for Exp<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        Self::try_from(s).map_err(serde::de::Error::custom)
    }
}
