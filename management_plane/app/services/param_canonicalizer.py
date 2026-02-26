import json


def canonicalize_params(p) -> str:
    if p is None or p == "":
        return ""

    if isinstance(p, dict):
        return _canonicalize_dict(p)

    if isinstance(p, str):
        try:
            parsed = json.loads(p)
            if isinstance(parsed, dict):
                return _canonicalize_dict(parsed)
        except (json.JSONDecodeError, ValueError):
            pass
        return p

    return ""


def _flatten(d, prefix=""):
    items = {}
    for k, v in d.items():
        key = f"{prefix}.{k}" if prefix else k
        if isinstance(v, dict):
            items.update(_flatten(v, key))
        else:
            items[key] = v
    return items


def _canonicalize_dict(d: dict) -> str:
    flat = _flatten(d)
    tokens = []
    for k in sorted(flat.keys()):
        v = flat[k]
        if isinstance(v, list):
            value_str = ",".join(sorted(str(x) for x in v))
        else:
            value_str = str(v)
        tokens.append(f"{k}={value_str}")
    return " ".join(tokens)
