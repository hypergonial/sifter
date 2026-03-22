# sosaku-py

Python bindings for sosaku.

## Installation

```bash
pip install .
```

Or in a `pyproject.toml`:

```toml
[project]
dependencies = [
    "sosaku-py @ git+https://github.com/hypergonial/sosaku.git@main#subdirectory=sosaku-py"
]
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
