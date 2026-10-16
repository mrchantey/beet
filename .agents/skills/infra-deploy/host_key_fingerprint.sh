#!/bin/sh
# The fingerprint of the site's ssh host key, never printing the key:
#   beet secrets/exec --only=BEET_SSH_HOST_KEY -- sh host_key_fingerprint.sh
# The record is base64 of the PEM file `ssh-keygen` writes, decoded into a
# private temp dir that is removed on exit.
set -eu
umask 077
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
printf '%s' "$BEET_SSH_HOST_KEY" | base64 -d > "$dir/key"
ssh-keygen -lf "$dir/key"
