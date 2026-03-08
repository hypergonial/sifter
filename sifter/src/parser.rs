use std::borrow::Cow;

use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{escaped, is_not, tag, take_while},
    character::complete::{char, digit1, multispace0, one_of},
    combinator::{map_res, opt, recognize},
    error::ParseError,
    multi::{separated_list0, separated_list1},
    sequence::{delimited, terminated},
};

use super::types::{Exp, FunctionItem, Literal, VarAccess, VarName};

static KEYWORDS: [&str; 3] = ["true", "false", "null"];

struct BinaryOperator<'a> {
    op: &'static str,
    func: fn(Exp<'a>, Exp<'a>) -> Exp<'a>,
}

impl<'a> BinaryOperator<'a> {
    const fn new(op: &'static str, func: fn(Exp<'a>, Exp<'a>) -> Exp<'a>) -> Self {
        Self { op, func }
    }
}

/// Remove trailing whitespaces from the inner parser
fn ws<'a, F, O, E>(inner: F) -> impl Parser<&'a str, Output = O, Error = E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E> + 'a,
    E: ParseError<&'a str>,
{
    terminated(inner, multispace0)
}

/// Parse an integer
fn integer(input: &str) -> IResult<&str, i64> {
    map_res(recognize(opt(one_of("+-")).and(digit1)), |o: &str| {
        o.parse::<i64>()
    })
    .parse(input)
}

/// Parse a floating point number
fn double(input: &str) -> IResult<&str, f64> {
    map_res(
        recognize(opt(one_of("+-")).and(digit1).and(tag(".")).and(digit1)),
        |o: &str| o.parse::<f64>(),
    )
    .parse(input)
}

/// Parse a quoted string, handling both single and double quotes, as well as escaped characters
fn string<'a>(input: &'a str) -> IResult<&'a str, Cow<'a, str>> {
    let (input, value) = alt((
        delimited(
            char('\''),
            opt(escaped(is_not("\\'"), '\\', char('\''))),
            char('\''),
        ),
        delimited(
            char('"'),
            opt(escaped(is_not("\\\""), '\\', char('"'))),
            char('"'),
        ),
    ))
    .parse(input)?;

    let value: Cow<'a, str> = match value {
        None => Cow::from(""),
        Some(s) if s.contains("\\'") | s.contains("\\\"") => {
            Cow::from(s.replace("\\'", "'").replace("\\\"", "\""))
        }
        Some(s) => Cow::from(s), // no escapes — zero copy from input into Arc
    };

    Ok((input, value))
}

/// Parse a non-keyword identifier
fn parse_non_keyword(input: &str) -> IResult<&str, &str> {
    map_res(
        take_while(|c: char| c.is_ascii_alphanumeric()),
        |v: &str| {
            if v.is_empty() {
                Err("Parsed empty string")
            } else if KEYWORDS.contains(&v) {
                Err("Parsed a keyword")
            } else {
                Ok(v)
            }
        },
    )
    .parse(input)
}

// Parse a variable name: A variable name is a series of non-keywords separated by dots, with an optional indexer at the end (e.g. "foo.bar[0]")
pub(super) fn parse_variable_name(input: &str) -> IResult<&str, VarAccess> {
    let (input, names) = separated_list1(
        char('.'),
        parse_non_keyword.and(opt(delimited(ws(char('[')), ws(digit1), ws(char(']'))))),
    )
    .parse(input)?;

    // If any of the names start with a digit, it's an error (e.g. "foo.0bar")
    for (name, _) in &names {
        if name
            .chars()
            .next()
            .expect("Variable name is empty, should have been caught by parse_non_keyword")
            .is_ascii_digit()
        {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Digit,
            )));
        }
    }

    let varaccess = VarAccess::new(
        names
            .into_iter()
            .map(|(name, index)| {
                VarName::new(
                    name,
                    index.map(|i| i.parse::<usize>().expect("Failed to parse index")),
                )
            })
            .collect(),
    );

    Ok((input, varaccess))
}

// Function that tries all ops & returns the remaining input & the op that worked (if any)
fn try_ops<'a, 'b, 'c>(
    ops: &'b [BinaryOperator<'c>],
    input: &'a str,
) -> IResult<&'a str, &'b BinaryOperator<'c>> {
    for op in ops {
        let parsed: IResult<&str, &str> = ws(tag(op.op)).parse(input);
        if let Ok((remainder, _)) = parsed {
            return Ok((remainder, op));
        }
    }
    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Tag,
    )))
}

/// Parse a left-associative binary operator
///
/// ## Parameters
///
/// - `parser`: The parser for the individual operands
/// - `ops`: The operators to apply
/// - `input`: The input string
///
/// ## Returns
///
/// The parsed expression
fn parse_left_assoc<'a, 'b, E: ParseError<&'a str>>(
    mut parser: impl Parser<&'a str, Output = Exp<'a>, Error = E>,
    ops: &'b [BinaryOperator<'a>],
    input: &'a str,
) -> IResult<&'a str, Exp<'a>, E> {
    let (mut input, mut current) = parser.parse(input)?;

    loop {
        // Break out of the loop if no operator was matched
        let Ok((i, op)) = try_ops(ops, input) else {
            return Ok((input, current));
        };
        // RHS should always exist
        let (i, rhs) = parser.parse(i)?;
        current = (op.func)(current, rhs);
        input = i;
    }
}

/// Parse a right-associative binary operator
///
/// ## Parameters
///
/// - `parser`: The parser for the individual operands
/// - `ops`: The operators to apply
/// - `input`: The input string
///
/// ## Returns
///
/// The parsed expression
#[expect(dead_code)]
fn parse_right_assoc<'a, 'b, E: ParseError<&'a str>>(
    mut parser: impl Parser<&'a str, Output = Exp<'a>, Error = E>,
    ops: &'b [BinaryOperator<'a>],
    input: &'a str,
) -> IResult<&'a str, Exp<'a>, E> {
    let (mut input, mut current) = parser.parse(input)?;

    let mut stack = Vec::new();

    loop {
        // Break out of the loop if no operator was matched
        let Ok((i, op)) = try_ops(ops, input) else {
            break;
        };
        // RHS should always exist
        let (i, rhs) = parser.parse(i)?;
        stack.push((op, current));
        current = rhs;
        input = i;
    }

    while let Some((op, lhs)) = stack.pop() {
        current = (op.func)(lhs, current);
    }

    Ok((input, current))
}

/// Parse a non-associative binary operator
///
/// ## Parameters
///
/// - `parser`: The parser for the individual operands
/// - `ops`: The operators to apply
/// - `input`: The input string
///
/// ## Returns
///
/// The parsed expression
///
/// ## Errors
///
/// - If the number of operands is not 1 or 2
/// - If the parser fails
fn parse_non_assoc<'a, 'b, E: ParseError<&'a str> + 'a>(
    parser: impl Parser<&'a str, Output = Exp<'a>, Error = E>,
    op: &'b BinaryOperator<'a>,
    input: &'a str,
) -> IResult<&'a str, Exp<'a>, E> {
    let (input, mut exprs) = separated_list1(ws(tag(op.op)), parser).parse(input)?;

    let proc = match exprs.len() {
        1 => Ok(exprs.pop().expect("Impossible")),
        2 => Ok((op.func)(exprs.remove(0), exprs.remove(0))),
        _ => Err(nom::Err::Error(E::from_error_kind(
            input,
            nom::error::ErrorKind::Count,
        )))?,
    }?;

    Ok((input, proc))
}

/// Parse a matcher function from the input string and return a `ParserFunction` struct
///
/// ## Parameters
/// - `input`: The input string to parse, e.g. "startsWith('hello')"
///
/// ## Returns
/// - `Ok(ParserFunction)`: If the parsing is successful, returns a `ParserFunction` struct containing the function name and value
///
/// ## Errors
///
/// - If the input string does not match the expected pattern (e.g. missing parentheses, missing quotes, etc.), a parsing error will be returned.
fn parse_fn(input: &str) -> IResult<&str, FunctionItem<'_>> {
    let (input, (name, _, _, _, value, _, _)) = (
        map_res(take_while(|c: char| c.is_alphabetic()), |v: &str| {
            if v.is_empty() {
                Err("Empty function name")
            } else {
                Ok(v)
            }
        }), // Function name
        take_while(|c: char| c.is_whitespace()),
        char('('),
        take_while(|c: char| c.is_whitespace()),
        // The parameter list, comma-separated, with an optional trailing comma at the end
        separated_list0(ws(char(',')), parse_exp)
            .and(opt(ws(char(','))))
            .map(|(list, _)| list),
        take_while(|c: char| c.is_whitespace()),
        char(')'),
    )
        .parse(input)?;

    Ok((input, FunctionItem::new(name, value)))
}

/// Parse a literal value (integer, float, boolean, or string)
fn parse_literal(input: &str) -> IResult<&str, Literal<'_>> {
    alt((
        ws(double).map(Literal::Float),
        ws(integer).map(Literal::Int),
        alt((ws(tag("true")), ws(tag("false")))).map(|v: &str| Literal::Bool(v == "true")),
        ws(tag("null")).map(|_| Literal::Null),
        ws(string).map(Literal::String),
    ))
    .parse(input)
}

/// Parse an atomic expression
fn parse_atom(input: &str) -> IResult<&str, Exp<'_>> {
    alt((
        ws(parse_literal).map(Exp::literal),
        // Function call
        ws(parse_fn).map(Exp::fn_call),
        // Variable names
        ws(parse_variable_name).map(|v: VarAccess| Exp::Var(v)),
        // Parenthesized expressions
        delimited(ws(char('(')), parse_exp, ws(char(')'))),
    ))
    .parse(input)
}

/// Parse a negation (prefix unary !) operator
fn parse_neg(input: &str) -> IResult<&str, Exp<'_>> {
    // Try reading a negation operator
    let Ok((input, _)): IResult<&str, &str> = tag("!")(input) else {
        return parse_atom(input);
    };
    // If successful, wrap the resulting expression in a negation
    let (input, exp) = parse_atom(input)?;
    Ok((input, Exp::neg(exp)))
}

fn parse_comp(input: &str) -> IResult<&str, Exp<'_>> {
    parse_left_assoc(
        parse_neg,
        &[
            BinaryOperator::new(">=", Exp::geq),
            BinaryOperator::new("<=", Exp::leq),
            BinaryOperator::new(">", Exp::gt),
            BinaryOperator::new("<", Exp::lt),
        ],
        input,
    )
}

fn parse_neq(input: &str) -> IResult<&str, Exp<'_>> {
    parse_non_assoc(parse_comp, &BinaryOperator::new("!=", Exp::neq), input)
}

fn parse_eq(input: &str) -> IResult<&str, Exp<'_>> {
    parse_non_assoc(parse_neq, &BinaryOperator::new("==", Exp::eq), input)
}

fn parse_and(input: &str) -> IResult<&str, Exp<'_>> {
    parse_left_assoc(parse_eq, &[BinaryOperator::new("&&", Exp::and)], input)
}

fn parse_or(input: &str) -> IResult<&str, Exp<'_>> {
    parse_left_assoc(parse_and, &[BinaryOperator::new("||", Exp::or)], input)
}

pub(super) fn parse_exp(input: &str) -> IResult<&str, Exp<'_>> {
    parse_or(input)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_atom() {
        assert_eq!(parse_atom("123"), Ok(("", Exp::literal(Literal::Int(123)))));
        assert_eq!(
            parse_atom("-123  "),
            Ok(("", Exp::literal(Literal::Int(-123))))
        );
        assert_eq!(
            parse_atom("123.456  "),
            Ok(("", Exp::literal(Literal::Float(123.456))))
        );
        assert_eq!(
            parse_atom("-123.456  "),
            Ok(("", Exp::literal(Literal::Float(-123.456))))
        );
        assert_eq!(
            parse_atom("true  "),
            Ok(("", Exp::literal(Literal::Bool(true))))
        );
        assert_eq!(
            parse_atom("false  "),
            Ok(("", Exp::literal(Literal::Bool(false))))
        );
        assert_eq!(parse_atom("abc"), Ok(("", Exp::varname("abc").unwrap())));

        assert_eq!(
            parse_atom("'hello\\' world'    "),
            Ok(("", Exp::literal(Literal::String("hello' world".into()))))
        );

        assert_eq!(parse_atom("null "), Ok(("", Exp::literal(Literal::Null))));
    }

    #[test]
    fn test_var_access() {
        assert_eq!(
            parse_variable_name("foo.bar[ 0 ].baz"),
            Ok((
                "",
                VarAccess::new(vec![
                    VarName::new("foo", None),
                    VarName::new("bar", Some(0)),
                    VarName::new("baz", None),
                ])
            ))
        );
        // Variable names with digits should work as long as they don't start with a digit
        assert_eq!(
            parse_variable_name("foo1.bar2[3].baz4"),
            Ok((
                "",
                VarAccess::new(vec![
                    VarName::new("foo1", None),
                    VarName::new("bar2", Some(3)),
                    VarName::new("baz4", None),
                ])
            ))
        );
        // Variable names that start with a digit should fail
        assert!(parse_variable_name("foo.0bar").is_err());
    }

    #[test]
    fn test_neg() {
        assert_eq!(
            parse_neg("!123"),
            Ok(("", Exp::neg(Exp::literal(Literal::Int(123)))))
        );
        assert_eq!(
            parse_neg("!true"),
            Ok(("", Exp::neg(Exp::literal(Literal::Bool(true)))))
        );
        assert_eq!(
            parse_neg("!false"),
            Ok(("", Exp::neg(Exp::literal(Literal::Bool(false)))))
        );
        assert_eq!(
            parse_neg("!abc"),
            Ok(("", Exp::neg(Exp::varname("abc").unwrap())))
        );
    }

    #[test]
    fn test_comp() {
        assert_eq!(
            parse_comp("123 > 456"),
            Ok((
                "",
                Exp::gt(
                    Exp::literal(Literal::Int(123)),
                    Exp::literal(Literal::Int(456))
                )
            ))
        );
        assert_eq!(
            parse_comp("123 < 456"),
            Ok((
                "",
                Exp::lt(
                    Exp::literal(Literal::Int(123)),
                    Exp::literal(Literal::Int(456))
                )
            ))
        );
        assert_eq!(
            parse_comp("123 >= 456"),
            Ok((
                "",
                Exp::geq(
                    Exp::literal(Literal::Int(123)),
                    Exp::literal(Literal::Int(456))
                )
            ))
        );
        assert_eq!(
            parse_comp("123 <= 456"),
            Ok((
                "",
                Exp::leq(
                    Exp::literal(Literal::Int(123)),
                    Exp::literal(Literal::Int(456))
                )
            ))
        );
    }

    #[test]
    fn test_eq() {
        assert_eq!(
            parse_exp("1 == 2"),
            Ok((
                "",
                Exp::eq(Exp::literal(Literal::Int(1)), Exp::literal(Literal::Int(2)))
            ))
        );
        assert!(parse_exp("1 == 2 == 3").is_err());
    }

    #[test]
    fn test_neq() {
        assert_eq!(
            parse_exp("1 != 2"),
            Ok((
                "",
                Exp::neq(Exp::literal(Literal::Int(1)), Exp::literal(Literal::Int(2)))
            ))
        );
        assert!(parse_exp("1 != 2 != 3").is_err());
    }

    #[test]
    fn test_and() {
        assert_eq!(
            parse_exp("true && false"),
            Ok((
                "",
                Exp::and(
                    Exp::literal(Literal::Bool(true)),
                    Exp::literal(Literal::Bool(false))
                )
            ))
        );
        assert_eq!(
            parse_exp("true && false && true"),
            Ok((
                "",
                Exp::and(
                    Exp::and(
                        Exp::literal(Literal::Bool(true)),
                        Exp::literal(Literal::Bool(false))
                    ),
                    Exp::literal(Literal::Bool(true))
                )
            ))
        );
    }

    #[test]
    fn test_and_or_precedence() {
        assert_eq!(
            parse_exp("true || false && false"),
            Ok((
                "",
                Exp::or(
                    Exp::literal(Literal::Bool(true)),
                    Exp::and(
                        Exp::literal(Literal::Bool(false)),
                        Exp::literal(Literal::Bool(false))
                    )
                )
            ))
        );
    }

    #[test]
    fn test_or() {
        assert_eq!(
            parse_exp("true || false"),
            Ok((
                "",
                Exp::or(
                    Exp::literal(Literal::Bool(true)),
                    Exp::literal(Literal::Bool(false))
                )
            ))
        );
        assert_eq!(
            parse_exp("true || false || true"),
            Ok((
                "",
                Exp::or(
                    Exp::or(
                        Exp::literal(Literal::Bool(true)),
                        Exp::literal(Literal::Bool(false))
                    ),
                    Exp::literal(Literal::Bool(true))
                )
            ))
        );
    }

    #[test]
    fn test_parens() {
        assert_eq!(
            parse_exp("(true || false) && true"),
            Ok((
                "",
                Exp::and(
                    Exp::or(
                        Exp::literal(Literal::Bool(true)),
                        Exp::literal(Literal::Bool(false))
                    ),
                    Exp::literal(Literal::Bool(true))
                )
            ))
        );
    }

    #[test]
    fn test_variable_names() {
        // Works
        assert_eq!(parse_exp("foo"), Ok(("", Exp::varname("foo").unwrap())));

        // Works
        assert_eq!(
            parse_exp("foo||bar"),
            Ok((
                "",
                Exp::or(Exp::varname("foo").unwrap(), Exp::varname("bar").unwrap())
            ))
        );

        assert_eq!(
            parse_exp("foo    && bar"),
            Ok((
                "",
                Exp::and(Exp::varname("foo").unwrap(), Exp::varname("bar").unwrap())
            ))
        );
    }

    #[test]
    fn test_crazy_nested() {
        assert_eq!(
            parse_exp("!(1 > 2) && (3 <= 4 || 5 != 6)"),
            Ok((
                "",
                Exp::and(
                    Exp::neg(Exp::gt(
                        Exp::literal(Literal::Int(1)),
                        Exp::literal(Literal::Int(2))
                    )),
                    Exp::or(
                        Exp::leq(Exp::literal(Literal::Int(3)), Exp::literal(Literal::Int(4))),
                        Exp::neq(Exp::literal(Literal::Int(5)), Exp::literal(Literal::Int(6)))
                    )
                )
            ))
        );
    }

    #[test]
    fn test_crazy_nested_fn() {
        assert_eq!(
            parse_exp("!startsWith('hello') && (3 <= 4 || 5 != 6)"),
            Ok((
                "",
                Exp::and(
                    Exp::neg(Exp::fn_call(FunctionItem::new(
                        "startsWith",
                        vec![Exp::literal(Literal::String("hello".into()))]
                    ))),
                    Exp::or(
                        Exp::leq(Exp::literal(Literal::Int(3)), Exp::literal(Literal::Int(4))),
                        Exp::neq(Exp::literal(Literal::Int(5)), Exp::literal(Literal::Int(6)))
                    )
                )
            ))
        );
    }

    #[test]
    fn test_string_literal() {
        assert_eq!(
            parse_literal("'hello world'"),
            Ok(("", Literal::String("hello world".into())))
        );
        assert_eq!(
            parse_literal("\"hello world\""),
            Ok(("", Literal::String("hello world".into())))
        );
        assert_eq!(
            parse_literal("'hello \\'world\\''"),
            Ok(("", Literal::String("hello 'world'".into())))
        );
        assert_eq!(
            parse_literal("\"hello \\\"world\\\"\""),
            Ok(("", Literal::String("hello \"world\"".into())))
        );
    }

    #[test]
    fn test_fn_parser() {
        let (_, parser_function) =
            parse_fn("startsWith('hello')").expect("Failed to parse matcher function");

        assert_eq!(parser_function.name(), "startsWith");
        assert_eq!(
            parser_function.args(),
            vec![Exp::literal(Literal::String("hello".into()))]
        );
    }

    #[test]
    fn test_fn_parser_noargs() {
        let (_, parser_function) = parse_fn("isEmpty()").expect("Failed to parse matcher function");

        assert_eq!(parser_function.name(), "isEmpty");
        assert_eq!(parser_function.args(), vec![]);
    }

    #[test]
    fn test_fn_parser_doublequote() {
        let (_, parser_function) =
            parse_fn("startsWith(\"hello\")").expect("Failed to parse matcher function");

        assert_eq!(parser_function.name(), "startsWith");
        assert_eq!(
            parser_function.args(),
            vec![Exp::literal(Literal::String("hello".into()))]
        );
    }

    #[test]
    fn test_fn_parser_multiple_args() {
        let (_, parser_function) =
            parse_fn("between(1, 10)").expect("Failed to parse matcher function");

        assert_eq!(parser_function.name(), "between");
        assert_eq!(
            parser_function.args(),
            vec![
                Exp::literal(Literal::Int(1)),
                Exp::literal(Literal::Int(10))
            ]
        );
    }

    #[test]
    fn test_fn_parser_trailing_comma() {
        let (_, parser_function) =
            parse_fn("between(1, 10,)").expect("Failed to parse matcher function");

        assert_eq!(parser_function.name(), "between");
        assert_eq!(
            parser_function.args(),
            vec![
                Exp::literal(Literal::Int(1)),
                Exp::literal(Literal::Int(10))
            ]
        );
    }

    #[test]
    fn test_fn_parser_nested_args() {
        let (_, parser_function) =
            parse_fn("between(length('hello'), 10)").expect("Failed to parse matcher function");

        assert_eq!(parser_function.name(), "between");
        assert_eq!(
            parser_function.args(),
            vec![
                Exp::fn_call(FunctionItem::new(
                    "length",
                    vec![Exp::literal(Literal::String("hello".into()))]
                )),
                Exp::literal(Literal::Int(10))
            ]
        );
    }

    #[test]
    fn test_whitespace_between_fn_and_parentheses() {
        let (_, parser_function) =
            parse_fn("startsWith   (  'hello'  )").expect("Failed to parse matcher function");

        assert_eq!(parser_function.name(), "startsWith");
        assert_eq!(
            parser_function.args(),
            vec![Exp::literal(Literal::String("hello".into()))]
        );
    }

    #[test]
    fn test_missing_parentheses() {
        let result = parse_fn("startsWith'hello')");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_quotes() {
        let result = parse_fn("startsWith('hello)");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_value() {
        let (_, parser_function) =
            parse_fn("startsWith('')").expect("Failed to parse matcher function");

        assert_eq!(parser_function.name(), "startsWith");
        assert_eq!(
            parser_function.args(),
            vec![Exp::literal(Literal::String("".into()))]
        );
    }
}
