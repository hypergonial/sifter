use std::{borrow::Cow, sync::LazyLock};

use regex::{Captures, Regex};

static JSON_ESCAPE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[\\"'\n\r\t]"#).expect("Failed to compile JSON escape regex"));

pub(crate) fn escape_str_for_json(s: &str) -> Cow<'_, str> {
    JSON_ESCAPE_REGEX.replace_all(s, |caps: &Captures| {
        match caps.get(0).expect("Should always exist").as_str() {
            "\\" => "\\\\",
            "\"" => "\\\"",
            "\n" => "\\n",
            "\r" => "\\r",
            "\t" => "\\t",
            _ => panic!("Unexpected match"),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_str_for_json() {
        let input = "Hello \"world\"!\nThis is a test.\tBackslash: \\";
        let expected = "Hello \\\"world\\\"!\\nThis is a test.\\tBackslash: \\\\";
        assert_eq!(escape_str_for_json(input), expected);
    }
}
