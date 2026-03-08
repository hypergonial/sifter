use serde_json::json;
use sifter::{Env, Exp};

#[allow(clippy::unwrap_used)]
fn main() {
    let env = Env::new()
        .bind("x", json!(42))
        .bind("y", json!("hello"))
        .bind("z", json!(true))
        .bind(
            "foo",
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
        )
        .build();

    let exp: Exp =
        r#"length(y) < x && matches(foo.qux.nested[3], "fo{5}ur") && int(foo.quux) == foo.bar"#
            .try_into()
            .unwrap();
    let res = exp.eval(&env).unwrap();

    println!("Result: {res:?}");
}
