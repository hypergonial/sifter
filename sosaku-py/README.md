# sosaku

Python bindings for [sosaku](https://github.com/hypergonial/sosaku).

## Installation

```bash
pip install sosaku
```

Or in a `pyproject.toml`:

```toml
[project]
dependencies = [
    "sosaku>=0.1.0"
]
```

Or in a `requirements.txt`:

```txt
sosaku-py>=0.1.0
```

## Usage

```python
import sosaku

exp = sosaku.Exp("test.var == 5 && startsWith(test.var2, 'hello')")
bindings = {"test": {"var": 5, "var2": "hello world"}}

print(exp.eval(bindings))
```

## Development

Create a new virtual environment and install the package in editable mode:

```bash
uv venv
source .venv/bin/activate
```

Install maturin:

```bash
uv tool install maturin
```

Then, you can build the package in editable mode:

```bash
maturin develop
```
