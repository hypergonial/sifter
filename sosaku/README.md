# sosaku

A crate for evaluating expressions against a set of bindings. It supports a variety of operators and functions, and can be used in a variety of contexts.

## Installation

Add the crate to your project:

```bash
cargo add sosaku
```

If you need `serde`/`serde_json` support, enable the `serde` feature:

```bash
cargo add sosaku --features serde
```

## Example Usage

> [!NOTE]
> This example assumes you have the `serde_json` crate in your dependencies and the `serde` feature enabled for `sosaku`.

```rust
use serde_json::json;
use sosaku::{Exp, Env};

fn main() {
    let exp = Exp::new("test.var == 5 && startsWith(test.var2, 'hello')").unwrap();

    let value = json!({"var": 5, "var2": "hello world"});
    let env = Env::new().bind_ref("test", &value).build();

    let result = exp.eval(&env).unwrap();
    println!("{result}"); // true
}
```
