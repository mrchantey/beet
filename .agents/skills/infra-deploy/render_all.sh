#!/bin/bash
# Render all eight infra configs, optionally diffing against a baseline dump dir.
# Protocol: render a baseline from the CLEAN tree before touching the render
# (`render_all.sh before`), make the change, then `render_all.sh after .agents/tmp/render/before`.
# A source_code_hash-only diff is a rebuilt binary (a code-only deploy), not a config change.
# Usage: render_all.sh <out-dir-name> [<baseline-dir>]   (dumps land in .agents/tmp/render/<out-dir-name>)
set -uo pipefail
cd "$(dirname "$0")/../../.."
export BEET_DEPLOY_ID=00000000-0000-7000-8000-000000000000
export BEET_DEPLOY_TIMESTAMP=1756600000s
out=.agents/tmp/render/${1:?out dir name required}
baseline=${2:-}
mkdir -p "$out"

run() { # run <dump-name> <target-dir> <args...>
	local name=$1 dir=$2; shift 2
	if AWS_PROFILE= cargo run -q -p beet-cli --features infra,extra -- "$@" >"$out/$name.log" 2>&1; then
		cp "target/infra/$dir/main.tf.json" "$out/$name"
	else
		echo "RENDER FAILED: $name (see $out/$name.log)"
	fi
}

run beet-site.dev.main.tf.json beet-site --main=site validate
run beet-site.prod.main.tf.json beet-site --main=site validate --stage=prod
run mail-example.main.tf.json mail-example --main=examples/infra/mail.bsx validate
run lambda.main.tf.json lambda --main=examples/infra/lambda.bsx validate
run fargate.main.tf.json fargate --main=examples/infra/fargate.bsx validate
run lightsail.main.tf.json lightsail --main=examples/infra/lightsail.bsx validate
run bucket-example.main.tf.json bucket-example --main=examples/infra/bucket.bsx validate
run ssh-site.main.tf.json ssh-site --main=examples/infra/ssh_site.bsx validate

[ -n "$baseline" ] || { echo "dumped to $out (no baseline given, nothing diffed)"; exit 0; }

status=0
for file in "$baseline"/*.json; do
	name=$(basename "$file")
	if cmp -s "$file" "$out/$name"; then
		echo "IDENTICAL: $name"
	elif diff <(grep -v source_code_hash "$file") <(grep -v source_code_hash "$out/$name") >/dev/null; then
		echo "HASH-ONLY: $name (rebuilt binary, not a config change)"
	else
		echo "DIFFERS: $name"
		diff "$file" "$out/$name" | head -40
		status=1
	fi
done
exit $status
