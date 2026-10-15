#!/usr/bin/env python3
"""Diff two rendered `main.tf.json` dumps line by line.

    tf_diff.py <before> <after>

Each dump is flattened to one `path = value` line per leaf, and a multi-line
string (an instance's `user_data`, a container definition) to one line per
line of the string, so a diff reads as the changed script lines rather than
one minified line. Secret-shaped values (`*KEY*=`, `*SECRET*=`, `*TOKEN*=`,
`*PASSWORD*=`) print as a short digest: a change still shows, the value never
does. Exits 1 when the dumps differ.
"""
import difflib
import hashlib
import json
import re
import sys

SECRET_NAME = r"[A-Za-z0-9_]*(KEY|SECRET|TOKEN|PASSWORD)[A-Za-z0-9_]*"
# a literal value only: a `${..}` terraform reference names no secret
SECRET = re.compile(rf"^(\s*{SECRET_NAME}=)((?!\$)\S{{16,}})")


def digest(value):
    return f"<redacted {hashlib.sha256(value.encode()).hexdigest()[:12]}>"


def redact(line):
    """Replace a long secret-shaped value with a digest of it."""
    return SECRET.sub(lambda found: found[1] + digest(found[3]), line)


def flatten(node, path, out):
    """Append a `path = value` line per leaf, a `path | line` per string line."""
    if isinstance(node, dict):
        # an env pair, ie `{"name": "BEET_SSH_HOST_KEY", "value": ".."}`
        if re.fullmatch(SECRET_NAME, str(node.get("name", ""))) and isinstance(node.get("value"), str):
            node = {**node, "value": digest(node["value"])}
        for key in sorted(node):
            flatten(node[key], f"{path}.{key}", out)
    elif isinstance(node, list):
        for i, item in enumerate(node):
            flatten(item, f"{path}[{i}]", out)
    elif isinstance(node, str) and node.lstrip().startswith(("[", "{")):
        # an embedded json document, ie an ecs `container_definitions`
        try:
            flatten(json.loads(node), f"{path}~", out)
        except ValueError:
            out.append(f"{path} = {json.dumps(node)}")
    elif isinstance(node, str) and "\n" in node:
        out.extend(f"{path} | {redact(line)}" for line in node.split("\n"))
    else:
        out.append(f"{path} = {redact(json.dumps(node))}")


def load(file):
    out = []
    flatten(json.load(open(file)), "", out)
    return out


diff = list(difflib.unified_diff(load(sys.argv[1]), load(sys.argv[2]), "before", "after", n=1, lineterm=""))
print("\n".join(diff))
sys.exit(1 if diff else 0)
