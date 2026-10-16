#!/usr/bin/env python3
"""Count files holding the site's ssh host key in any encoding, never printing it.

    beet secrets/exec --only=BEET_SSH_HOST_KEY -- python3 host_key_scan.py <path>..

Reads the `BEET_SSH_HOST_KEY` record (base64 of the PEM file `ssh-keygen`
writes) and looks for it as given, as the PEM body (whole, or any one line of
it), and as the ed25519 private seed and secret in hex and base64, with json
escapes and whitespace flattened. Prints one `HIT <file>: <encodings>` line per
holding file and a closing count; exits 1 on any hit.
"""
import base64
import os
import re
import struct
import sys


def field(buf, at):
    """The length-prefixed field at `at`, and the offset after it."""
    (size,) = struct.unpack(">I", buf[at:at + 4])
    return buf[at + 4:at + 4 + size], at + 4 + size


record = os.environ["BEET_SSH_HOST_KEY"].strip()
pem = base64.b64decode(record).decode()
lines = [line for line in pem.splitlines() if line and not line.startswith("-----")]
binary = base64.b64decode("".join(lines))
# openssh-key-v1: magic, cipher, kdf, kdf options, key count, public blob, then
# the private section: two check ints, the key type, the public key, the secret
at = len(b"openssh-key-v1\0")
for _ in range(3):
    _, at = field(binary, at)
_, at = field(binary, at + 4)
private, _ = field(binary, at)
_, at = field(private, 8)
_, at = field(private, at)
secret, _ = field(private, at)
seed = secret[:32]

needles = {
    "record": record,
    "pem body": "".join(lines),
    "pem armor": "OPENSSH PRIVATE KEY",
    "seed hex": seed.hex(),
    "seed base64": base64.b64encode(seed).decode().rstrip("="),
    "seed base64url": base64.urlsafe_b64encode(seed).decode().rstrip("="),
    "secret base64": base64.b64encode(secret).decode().rstrip("="),
}
needles.update({f"pem line {i}": line for i, line in enumerate(lines) if len(line) >= 20})

files = holding = 0
for root in sys.argv[1:]:
    paths = [root] if os.path.isfile(root) else [
        os.path.join(dir, name) for dir, _, names in os.walk(root) for name in names
    ]
    for path in paths:
        try:
            text = open(path, "rb").read().decode("utf-8", "replace")
        except OSError:
            continue
        flat = re.sub(r"\\n|\s", "", text)
        found = sorted(name for name, needle in needles.items() if needle in text or needle in flat)
        files += 1
        if found:
            holding += 1
            print(f"HIT {path}: {', '.join(found)}")
print(f"scanned {files} files, {holding} holding key material")
sys.exit(1 if holding else 0)
