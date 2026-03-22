# sosaku-cli

A command-line interface for sosaku.

## Installation

```bash
cargo install --path .
```

## Example Usage

```bash
# as a one-liner
sosaku "test.var == 5 && startsWith(test.var2, 'hello')" --var test='{"var": 5, "var2": "hello world"}'

# or with a file
echo '{"var": 5, "var2": "hello world"}' > bindings.json
sosaku "test.var == 5 && startsWith(test.var2, 'hello')" --var test=bindings.json

# or piped from stdin
echo '{"var": 5, "var2": "hello world"}' | sosaku "input.var == 5 && startsWith(input.var2, 'hello')"
```

See `sosaku --help` for more options.
