# Iterate on the Charcell Renderer

The iteration cycle for changes to `crates/beet_ui/src/render/charcell` (terminal layout + paint). The code reads cleanly enough to learn as you go; this is just how to verify changes quickly.

## Two render targets

Always check both, they exercise different buffers:

- **Layout example** (fixed `Buffer`, ANSI): `cargo run -p beet_ui --example layout --features=terminal`. The `terminal` feature is required so it measures the real terminal via crossterm, otherwise it falls back to 80 cols. The example prints each demo plus a final pass/fail line asserting no rendered line exceeds the measured width.
- **beet_site CLI** (auto-growing `FlexBuffer`, stdout): `cargo run -p beet_site --features cli -- blog post-1`. The real-world prose path (markdown, sidebar, header/footer, syntax highlighting).

## Measure against a real terminal width

Piping either target into a tool makes crossterm fall back to 80 cols, and the visible width is hidden behind ANSI/OSC escapes. To pin a width and read true output, run the prebuilt binary under a PTY with a set winsize. Drop this in `/tmp/charcell.py`:

```python
import os,sys,select,fcntl,termios,struct,re,unicodedata
cols=int(os.environ.get("COLS","80")); rows=300
mfd,sfd=os.openpty()
fcntl.ioctl(sfd,termios.TIOCSWINSZ,struct.pack("HHHH",rows,cols,0,0))
if os.fork()==0:
    os.setsid(); os.dup2(sfd,0); os.dup2(sfd,1); os.dup2(sfd,2); os.close(mfd)
    os.environ["TERM"]="xterm"; os.execvp(sys.argv[1],sys.argv[1:])
os.close(sfd); out=b""
while True:
    if not select.select([mfd],[],[],3)[0]: break
    try: d=os.read(mfd,65536)
    except OSError: break
    if not d: break
    out+=d
raw="--raw" in sys.argv
txt=out.decode("utf-8","replace")
if not raw:
    txt=re.sub(r"\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)","",txt)  # OSC-8
    txt=re.sub(r"\x1b\[[0-9;?]*[ -/]*[@-~]","",txt)          # CSI/SGR
    def w(s): return sum(2 if unicodedata.east_asian_width(c) in "WF" else (0 if unicodedata.combining(c) else 1) for c in s)
    for line in txt.split("\n"): print(f"{w(line.rstrip(chr(13))):3} {line.rstrip(chr(13))!r}")
else:
    sys.stdout.buffer.write(out)
```

- Build first, then run the binary path directly (not `cargo run`, which adds its own output): `cargo build -p beet_site --features cli` then `COLS=50 python3 /tmp/charcell.py /home/$USER/.cargo_target/debug/beet_site blog post-1`. The binary lives under the workspace target dir.
- Stripped mode prints `width repr(line)` per row, so visible widths are obvious and overflow is easy to spot with awk (`$1>50`).
- Pass `--raw` as a trailing arg to dump bytes with escapes intact, then `grep`/`cat -v` for the SGR codes when debugging a specific cell's foreground (`38;2;r;g;b`) or background (`48;2;r;g;b`).

Gotcha: never eyeball widths from a truncated `repr(line[:N])`, the slice hides real content and invents phantom truncation. Strip escapes and print the full line.

## Tests

- Unit + snapshot: `cargo test -p beet_ui --lib render::charcell`. Layout output is snapshot tested; regenerate intended changes with `cargo test -p beet_ui --lib render::charcell -- --snap`, then eyeball the diff under `.beet/snapshots/...`.
- End to end prose: `cargo test -p beet_site`.
- Match a fix with a regression test in the relevant module's `mod tests`, asserting on cell state (`buffer.iter_cells()`, `cell.style.background`) or stripped output, not exact bytes.

## Cycle

1. Reproduce the issue in both targets at a few widths (eg 30/50/80) via the PTY, confirming what's actually wrong (clipped text, wrong width, wrong colour) rather than trusting the first glance.
2. Make the change, rebuild the affected crate.
3. Re-run the PTY checks at the same widths, then the test suites above, refreshing snapshots only for intended changes.
