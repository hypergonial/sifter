import typing as t

import sosaku

JSONValue = int | float | str | bool | None | t.Sequence["JSONValue"] | t.Mapping[str, "JSONValue"]

CURSED_STUFF = """
    len(y) == 5 &&
    startsWith(y, "he") &&
    endsWith(y, "lo") &&
    contains(y, "ell") &&
    matches(y, r"^h.*o$") &&
    foo.bar > 100 &&
    foo.bar >= 123 &&
    foo.bar <= 123 &&
    foo.bar != 0 &&
    foo.baz == "world" &&
    contains(foo, "bar") &&
    !contains(foo, "missing") &&
    foo.qux.nested[0] == 1 &&
    foo.qux.nested[1] == 2 &&
    foo.qux.nested[2] == 3 &&
    contains(foo.qux.nested, 2) &&
    !contains(foo.qux.nested, 42) &&
    foo.qux.nested == [1, 2, 3] &&
    (x > 0 && x < 100 && x >= 42 && x <= 42 && x == 42 && x != 0) &&
    (z && !false && (false || true)) &&
    (("" || y) == "hello") &&
    ({"bar": 123, "baz": "world", "qux": {"nested": [1, 2, 3]}} == foo) &&
    len(foo.qux.nested) == 3 &&
    contains(["a", "b", "c"], "b") &&
    (1 < 2 && 2 < 3 && 3 <= 3 && 4 > 3 && 4 >= 4) &&
    matches("abc123", r"^[a-z]+[0-9]+$") &&
    contains({"k": 1, "m": 2}, "k")
    """


def test_exp_parse():
    exp = sosaku.Exp(CURSED_STUFF)
    assert exp


def test_exp_eval():
    test_json: dict[str, JSONValue] = {
        "x": 42,
        "y": "hello",
        "z": True,
        "foo": {"bar": 123, "baz": "world", "qux": {"nested": [1, 2, 3]}},
    }

    exp = sosaku.Exp(CURSED_STUFF)
    assert exp.eval(test_json) is True
