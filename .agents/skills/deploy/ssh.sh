#!/usr/bin/env bash
# Deploy skill check `d`: the live ssh TUI, single session + multi-tenancy.
#
#   ./ssh.sh <HOST> <PORT>
#
# Local:  ./ssh.sh 127.0.0.1 8339
# Dev:    ./ssh.sh app.dev.beet.org 22
# Prod:   ./ssh.sh app.beet.org 22
#
# Runs two checks over `ssh_pty.py`, an 80x24 pty driver that reconstructs the
# final frame through a VT emulator:
#
#   1. single session: navigate home -> Design -> counter and click "More" twice,
#      asserting "You have clicked 2 times."
#   2. multi-tenancy: two concurrent sessions driven to DIFFERENT counts (A once,
#      B twice), asserting each keeps its own state.
#
# Waits scale up for a non-localhost host (network latency + a cold page render).
set -uo pipefail

HOST="${1:?usage: ssh.sh <HOST> <PORT>}"
PORT="${2:?usage: ssh.sh <HOST> <PORT>}"
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PTY="$DIR/ssh_pty.py"
OUT="$(mktemp -d)"
trap 'rm -rf "$OUT"' EXIT

# remote hosts get roughly double the settle time at every step
case "$HOST" in
	127.0.0.1 | localhost | ::1) BOOT=5; NAV=3; TICK=2 ;;
	*) BOOT=10; NAV=6; TICK=4 ;;
esac

# The navigation recipe, discovered locally and identical on every stage: at
# 80x24 the site is in its narrow layout, so the sidebar hides behind the
# hamburger. Open the menu, click Design, reopen the menu (Design has since
# auto-expanded), click counter, then click "More" `$1` times.
recipe() {
	local clicks="$1" script
	script="w:$BOOT;m:3,1;w:$TICK;m:6,6;w:$NAV;m:3,1;w:$TICK;m:6,10;w:$NAV"
	for ((i = 0; i < clicks; i++)); do
		script+=";m:9,12;w:$TICK"
	done
	echo "$script"
}

# Assert `$3` appears in the frame `$2`, labelling the result with `$1`.
expect() {
	local label="$1" file="$2" needle="$3"
	if grep -qF "$needle" "$file"; then
		echo "PASS  $label: '$needle'"
	else
		echo "FAIL  $label: '$needle' not in frame:"
		sed 's/^/      | /' "$file"
		FAILED=1
		# a dead listener means the server crashed, not a layout drift
		if grep -qiE "connection refused|connection closed|connection reset" "$file"; then
			echo "      ^ the ssh listener is gone: the KNOWN multi-tenancy crash."
			echo "        restart 'beet serve' and re-run."
		fi
	fi
}

FAILED=0
echo "== ssh $HOST:$PORT =="

echo "-- single session --"
python3 "$PTY" "$HOST" "$PORT" "$(recipe 2)" >"$OUT/single.txt" 2>&1
expect "single" "$OUT/single.txt" "You have clicked 2 times."

echo "-- two concurrent sessions --"
python3 "$PTY" "$HOST" "$PORT" "$(recipe 1)" >"$OUT/a.txt" 2>&1 &
PID_A=$!
python3 "$PTY" "$HOST" "$PORT" "$(recipe 2)" >"$OUT/b.txt" 2>&1 &
PID_B=$!
wait $PID_A $PID_B
# independent counts prove per-session state, not one shared world
expect "client A" "$OUT/a.txt" "You have clicked 1 times."
expect "client B" "$OUT/b.txt" "You have clicked 2 times."

if [[ $FAILED -eq 0 ]]; then
	echo "== ssh OK =="
else
	echo "== ssh FAILED =="
fi
exit $FAILED
