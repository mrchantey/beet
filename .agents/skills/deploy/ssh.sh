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
#   3. images: a graphics-capable session loads a page carrying a raster and the
#      raw pty stream must contain a kitty transmit AND a cropped placement. Only
#      the term name matters, not the window: the TUI transmits solely to a
#      terminal advertising support, so at the default `xterm-256color` a
#      completely broken image path still renders a clean frame and checks 1-2
#      pass. The window stays the default 80x24, where the 1280x960 photo both
#      bounds to its scroll port and is scrolled past it, so this check covers
#      sizing and cropping together.
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
			echo "        restart the served entry and re-run."
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

echo "-- images (kitty graphics) --"
# The sidebar overlay lists every post, and "Folk Technology" (post-6, the one
# carrying a raster) sits at row 15 in the overlay, whose width is fixed so the
# cell is the same at every window size. The image is well below the fold, so
# scroll it into view: a raster is transmitted only once its box is on screen,
# and 32 downs lands it straddling the bottom of the port.
DOWNS="$(printf 'k:down;%.0s' {1..32})"
PTY_TERM=xterm-kitty PTY_RAW="$OUT/img.raw" \
	python3 "$PTY" "$HOST" "$PORT" \
	"w:$BOOT;m:3,1;w:$TICK;m:6,15;w:$((NAV * 2));${DOWNS}w:$((NAV * 2))" \
	>"$OUT/img.txt" 2>&1
# `a=t` is the transmit; the emulator drops APC, so grep the raw stream
if grep -qa $'\x1b_Ga=t' "$OUT/img.raw"; then
	echo "PASS  images: kitty transmit present ($(stat -c%s "$OUT/img.raw") raw bytes)"
else
	echo "FAIL  images: no kitty transmit in the raw pty stream"
	echo "      the <img> fetch/decode path is broken, or the page never loaded:"
	sed 's/^/      | /' "$OUT/img.txt"
	FAILED=1
fi
# `a=p` is the placement, and the `w=`/`h=` source-rect keys ride it only when the
# raster is CROPPED. A raster taller than its scroll port used to be dropped
# whole rather than clipped, a silent blank hole with no image and no alt marker,
# so a placement that never crops means that regression is back (or the raster
# no longer straddles the port, which is the same check failing usefully).
if grep -qa $'\x1b_Ga=p[^\x1b]*,w=' "$OUT/img.raw"; then
	echo "PASS  images: cropped placement $(grep -ao $'\x1b_Ga=p[^\x1b]*' "$OUT/img.raw" | tail -1 | tr -d '\033')"
else
	echo "FAIL  images: no cropped placement; a partly scrolled raster was dropped"
	grep -ao $'\x1b_Ga=p[^\x1b]*' "$OUT/img.raw" | tail -3 | sed 's/^/      | /'
	FAILED=1
fi

if [[ $FAILED -eq 0 ]]; then
	echo "== ssh OK =="
else
	echo "== ssh FAILED =="
fi
exit $FAILED
