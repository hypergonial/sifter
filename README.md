# sosaku

DSL for filtering JSON, for the individual components, see the following:

- [sosaku](./sosaku): The core Rust crate for evaluating expressions against a set of JSON values.
- [sosaku-cli](./sosaku-cli): A command-line interface for sosaku.
- [sosaku-py](./sosaku-py): Python bindings for sosaku.

## Short Example

```json
{"var": 5, "var2": "hello world", "nested": {"var3": [1,2,3]}}
```

```txt
input.var == 5 && startsWith(input.var2, 'hello') && input.nested.var3[1] == 2
```

Returns: `true`
