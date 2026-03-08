use std::collections::HashMap;

use serde_json::json;
use sifter::{Env, Exp};

#[allow(clippy::unwrap_used)]
fn main() {
    let bindings = HashMap::from([
        ("x".into(), json!(42)),
        ("y".into(), json!("hello")),
        ("z".into(), json!(true)),
        (
            "foo".into(),
            json!({
                "bar": 123,
                "baz": "world",
                "qux": {
                    "nested": [
                        1,
                        2,
                        3,
                        "fooooour"
                    ]
                },
                "quux": "123"
            }),
        ),
    ]);

    let exp = Exp::parse(
        r#"length(y) < x && matches(foo.qux.nested[3], "fo{5}ur") && int(foo.quux) == foo.bar"#,
    )
    .unwrap();
    let res = exp.eval(&Env::new(bindings)).unwrap();

    println!("Result: {res:?}");
}
