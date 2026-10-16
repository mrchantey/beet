#!/bin/bash
# Render all eight infra configs, optionally diffing against a baseline dump dir.
# Protocol: render a baseline from the CLEAN tree before touching the render
# (`render_all.sh before`), make the change, then `render_all.sh after .agents/tmp/render/before`.
# A source_code_hash-only diff is a rebuilt binary (a code-only deploy), not a config change.
# A DIFFERS prints through `tf_diff.py`, which splits `user_data` into its script lines.
# Exits non-zero on any DIFFERS, any failed render, or a stack rendered on one side only.
# Usage: render_all.sh <out-dir-name> [<baseline-dir>]   (dumps land in .agents/tmp/render/<out-dir-name>)
set -uo pipefail
skill_dir=$(cd "$(dirname "$0")" && pwd)
cd "$skill_dir/../../.."
export BEET_DEPLOY_ID=00000000-0000-7000-8000-000000000000
export BEET_DEPLOY_TIMESTAMP=1756600000s
out=.agents/tmp/render/${1:?out dir name required}
baseline=${2:-}
mkdir -p "$out"

beet=(cargo run -q -p beet-cli --features infra,extra --)
# Resolving a stack's S3 state backend asks STS for the account, so a render
# needs the deployer pair. The site entry loads `secrets.toml` itself; the
# examples load no secrets document, so they get the pair from `secrets/exec`,
# which strips every `BEET_*` knob from its child: `env` restores the pinned
# deploy id and timestamp, else each render mints its own and every diff moves.
with_aws=("${beet[@]}" secrets/exec --only=AWS_ACCESS_KEY_ID,AWS_SECRET_ACCESS_KEY --
	env BEET_DEPLOY_ID="$BEET_DEPLOY_ID" BEET_DEPLOY_TIMESTAMP="$BEET_DEPLOY_TIMESTAMP"
	"${beet[@]}")

status=0
names=()
run() { # run <dump-name> <target-dir> <command...>
	local name=$1 dir=$2; shift 2
	names+=("$name")
	rm -f "$out/$name"
	if "$@" >"$out/$name.log" 2>&1; then
		# a local backend's state path is absolute: name the checkout, so a
		# baseline rendered in a worktree compares against this one
		sed "s#$PWD/#<workspace>/#g" "target/infra/$dir/main.tf.json" >"$out/$name"
	else
		echo "RENDER FAILED: $name (see $out/$name.log)"
		status=1
	fi
}

# the stage stack's verbs moved under `site/` with the package deploy; a
# baseline rendered from a checkout before that still serves them at the root
site_validate=validate
grep -q '<Route path="site">' site/main.bsx && site_validate=site/validate
run beet-site.dev.main.tf.json beet-site "${beet[@]}" --entry=site $site_validate
run beet-site.prod.main.tf.json beet-site "${beet[@]}" --entry=site $site_validate --stage=prod
run mail-example.main.tf.json mail-example "${with_aws[@]}" --entry=examples/infra/mail.bsx validate
run lambda.main.tf.json lambda "${with_aws[@]}" --entry=examples/infra/lambda.bsx validate
run fargate.main.tf.json fargate "${with_aws[@]}" --entry=examples/infra/fargate.bsx validate
run lightsail.main.tf.json lightsail "${with_aws[@]}" --entry=examples/infra/lightsail.bsx validate
run bucket-example.main.tf.json bucket-example "${with_aws[@]}" --entry=examples/infra/bucket.bsx validate
run ssh-site.main.tf.json ssh-site "${with_aws[@]}" --entry=examples/infra/ssh_site.bsx validate

[ -n "$baseline" ] || { echo "dumped to $out (no baseline given, nothing diffed)"; exit $status; }

for name in "${names[@]}"; do
	before="$baseline/$name" after="$out/$name"
	if [ ! -f "$before" ] || [ ! -f "$after" ]; then
		echo "UNCOMPARED: $name (before: $([ -f "$before" ] && echo rendered || echo missing), after: $([ -f "$after" ] && echo rendered || echo missing))"
		status=1
	elif cmp -s "$before" "$after"; then
		echo "IDENTICAL: $name"
	elif diff <(grep -v source_code_hash "$before") <(grep -v source_code_hash "$after") >/dev/null; then
		echo "HASH-ONLY: $name (rebuilt binary, not a config change)"
	else
		echo "DIFFERS: $name"
		# one line per changed leaf or script line, secrets as digests
		python3 "$skill_dir/tf_diff.py" "$before" "$after" | head -80
		status=1
	fi
done
exit $status
