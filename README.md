# sosaku

DSL for filtering JSON, for the individual components, see the following:

- [sosaku](./sosaku): The core Rust crate for evaluating expressions against a set of JSON values.
- [sosaku-cli](./sosaku-cli): A command-line interface for sosaku.
- [sosaku-py](./sosaku-py): Python bindings for sosaku.

## Short Example

```json
test = {"var": 5, "var2": "hello world", "nested": {"var3": [1,2,3]}}
```

```txt
test.var == 5 && startsWith(test.var2, 'hello') && test.nested.var3[1] == 2
```

Returns: `true`
