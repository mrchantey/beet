#!/usr/bin/env bash
# Deploy skill check `d`: the live ssh TUI, single session + multi-tenancy.
#
#   ./ssh.sh <HOST> <PORT>
#
# Local:  ./ssh.sh 127.0.0.1 8339
# Dev:    ./ssh.sh app.dev.beet.org 22
# Prod:   ./ssh.sh app.beet.org 22
#
# Runs five checks, four over `ssh_pty.py`, an 80x24 pty driver that reconstructs
# the final frame through a VT emulator:
#
#   1. single session: navigate home -> ECS Router -> Folk Technology through the
#      sidebar, then Alt+Left back, asserting the frame is ECS Router's.
#   2. multi-tenancy: two concurrent sessions driven to DIFFERENT pages through
#      their history (A back once, B back then forward), asserting each keeps its
#      own navigator.
#   3. open at a path: two concurrent sessions open the unlisted counter by its
#      url (`ssh -tt <host> /docs/design/counter`) and click More to DIFFERENT
#      counts (A once, B twice), proving the path opening, the `bx:click` scripts
#      over a live session, and per-session document state. A renamed post's
#      old url (`/blog/post-6`) must land on the post its `<Redirect/>` names,
#      and an unknown url show the site's not-found page, as the web answers
#      both.
#   4. no pty: `ssh -T <host>` and `ssh <host> /blog` are told on stderr to add
#      `-t` and exit 1, with nothing on stdout, rather than hanging until the
#      pty timeout (and the hint survives the close, which a server-side
#      disconnect used to drop).
#   5. images: a graphics-capable session loads a page carrying a raster and the
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

# The navigation recipe, identical on every stage: at 80x24 the site is in its
# narrow layout, so the sidebar hides behind the hamburger. Open the menu, click
# "ECS Router", reopen it, click "Folk Technology", then press each history key
# given (`aleft` back, `aright` forward). The docs are unlisted, so the sidebar
# never leads to them: the counter is reached by its url instead (check 3).
#
# Everything below the menu button is clicked BY LABEL (`t:`), not by cell: the
# sidebar lists every blog post, so a single new post shifts every row under it
# and a pinned `m:` silently clicks its neighbour. The hamburger keeps its cell,
# being the one control whose position no content can move.
recipe() {
	local script key
	script="w:$BOOT;m:3,1;w:$TICK;t:ECS Router;w:$NAV;m:3,1;w:$TICK;t:Folk Technology;w:$NAV"
	for key in "$@"; do
		script+=";k:$key;w:$NAV"
	done
	echo "$script"
}

# A body word on each post's first screen, single words so no wrap can split
# them (the titles render fullwidth, so an ASCII title never matches).
ECS="bevyfication"
FOLK="Wollongong"

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

# Assert the frame `$2` shows `$3` and not `$4`, labelling the result with `$1`.
expect_page() {
	local label="$1" file="$2" other="$4"
	expect "$label" "$file" "$3"
	if grep -qF "$other" "$file"; then
		echo "FAIL  $label: '$other' also in frame, so it is not one page"
		FAILED=1
	fi
}

FAILED=0
echo "== ssh $HOST:$PORT =="

echo "-- single session --"
python3 "$PTY" "$HOST" "$PORT" "$(recipe aleft)" >"$OUT/single.txt" 2>&1
expect_page "single (back to ECS Router)" "$OUT/single.txt" "$ECS" "$FOLK"

echo "-- two concurrent sessions --"
python3 "$PTY" "$HOST" "$PORT" "$(recipe aleft)" >"$OUT/a.txt" 2>&1 &
PID_A=$!
python3 "$PTY" "$HOST" "$PORT" "$(recipe aleft aright)" >"$OUT/b.txt" 2>&1 &
PID_B=$!
wait $PID_A $PID_B
# different pages through one shared prefix prove per-session navigators, not
# one shared world
expect_page "client A (back)" "$OUT/a.txt" "$ECS" "$FOLK"
expect_page "client B (back, forward)" "$OUT/b.txt" "$FOLK" "$ECS"

echo "-- open at a path (the unlisted counter) --"
# the counter page closes the sidebar, so More sits at the left of the body;
# clicked by label like every other control
counter() {
	local script="w:$BOOT" i
	for ((i = 0; i < $1; i++)); do
		script+=";t:More;w:$TICK"
	done
	PTY_PATH=/docs/design/counter python3 "$PTY" "$HOST" "$PORT" "$script"
}
counter 1 >"$OUT/count-a.txt" 2>&1 &
PID_A=$!
counter 2 >"$OUT/count-b.txt" 2>&1 &
PID_B=$!
wait $PID_A $PID_B
expect "client A (More once)" "$OUT/count-a.txt" "You have clicked 1 times"
expect "client B (More twice)" "$OUT/count-b.txt" "You have clicked 2 times"
PTY_PATH=/blog/post-6 python3 "$PTY" "$HOST" "$PORT" "w:$BOOT" >"$OUT/renamed.txt" 2>&1 &
PID_A=$!
PTY_PATH=/no-such-page python3 "$PTY" "$HOST" "$PORT" "w:$BOOT" >"$OUT/unknown.txt" 2>&1 &
PID_B=$!
wait $PID_A $PID_B
expect "renamed url (follows its redirect)" "$OUT/renamed.txt" "$FOLK"
expect "unknown url (not found)" "$OUT/unknown.txt" "Route /no-such-page not found."

echo "-- no pty --"
# no terminal to draw into: the hint is a diagnostic, so stderr, never the
# stdout a pipe would capture as the answer
no_pty() {
	local label="$1" code
	shift
	ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
		-o ConnectTimeout=15 -o LogLevel=ERROR -p "$PORT" "$@" \
		</dev/null >"$OUT/nopty.out" 2>"$OUT/nopty.err"
	code=$?
	if [[ $code -eq 1 && ! -s "$OUT/nopty.out" ]] && grep -qF "ssh -t" "$OUT/nopty.err"; then
		echo "PASS  no pty ($label): exit 1, stderr '$(cat "$OUT/nopty.err")'"
	else
		echo "FAIL  no pty ($label): expected exit 1, an 'ssh -t' hint on stderr and empty stdout, got exit $code:"
		sed 's/^/      stdout | /' "$OUT/nopty.out"
		sed 's/^/      stderr | /' "$OUT/nopty.err"
		FAILED=1
	fi
}
no_pty "shell" -T "root@$HOST"
no_pty "path" "root@$HOST" /blog

echo "-- images (kitty graphics) --"
# The sidebar overlay lists every post; "Folk Technology" is post-6, the one
# carrying a raster, reached by label since every post added above it moves its
# row. The image is well below the fold, so scroll it into view: a raster is
# transmitted only once its box is on screen, and 31 downs lands it straddling
# the bottom of the port. The ~2.7MB transmit takes as long as the link does
# (instant locally, ~75s over a ~40KB/s path), so drain until the cropped
# placement arrives rather than for a fixed time.
DOWNS="$(printf 'k:down;%.0s' {1..31})"
CROPPED='\x1b_Ga=p[^\x1b]*,w='
PTY_TERM=xterm-kitty PTY_RAW="$OUT/img.raw" \
	python3 "$PTY" "$HOST" "$PORT" \
	"w:$BOOT;m:3,1;w:$TICK;t:Folk Technology;w:$((NAV * 2));${DOWNS}u:300:$CROPPED;w:$TICK" \
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
