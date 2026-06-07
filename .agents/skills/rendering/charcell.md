# Charcell Renderer

The terminal layout + paint engine, `crates/beet_ui/src/render/charcell`. Read the code as you go; this is the part that bites and how to verify.

## Already handled, don't reinvent in CSS

`charcell/decorate.rs` generates leading content (the terminal's `::before`): `<li>` bullets/numbers, blockquote bars, the `<hr>` rule, `<img>` alt text. So list/quote markers exist on the terminal with no CSS `list-style`; the web is the side that restores markers. `<li>` under a `<nav>` get **no** marker (navigation, not prose).

## Facts that bite

- **`tui_inset` doubles horizontal spacing** (`box_model.rs` does `min.x *= 2`), so `1rem` left padding = **2** cells, and `0.5rem` also → 2 (`round(0.5)=1`, then `*2`). Horizontal padding is therefore always even; a **1-cell** indent is unreachable through padding, it needs an explicit per-depth indent or a charcell change.
- **A block lays out its inline children inline.** A `<summary>` (block) with an inline label + caret flows them on one row; that's why the sidebar caret sits beside its label on the terminal without flex (flex is the web's screen-gated rule).
- **Adding `padding`/`display:block` to an inline element** changes its charcell box and can shift a whole subtree. Re-render the terminal after any such change, not just the web.
- **A flex column clamps each item's cross size (width) to the container** (`resolve_line_sizes` in `flex.rs`). Items are measured at the unconstrained viewport, so without the clamp a column with `align-items: center` centres against a width wider than it has and overflows (the homepage-hero-past-the-sidebar bug). The main axis stays unclamped (height scrolls).
- **The ratatui/ANSI paint drops colour alpha** (`color_to_ratatui` uses RGB only), so a *transparent* colour renders as **black**, not invisible. To reserve a border that should not show (eg equalising a filled button with an outlined one), colour it the same as the fill, not transparent.
- **Default scheme is dark**, set by the layout (`beet_site/src/layouts/layout.rs`), not the renderer: a non-html request gets `.dark-scheme` on `<body>` so a light `OnSurface` isn't invisible on a dark terminal. Transcluded `.md`/`RenderRef` content inherits this because `RuleSetQuery::parent` + `resolve_styles` follow `RenderRef` across the transclusion boundary.

## Measure against a real terminal width

Piping either target makes crossterm fall back to 80 cols and hides width behind escapes. Run the prebuilt binary under a PTY with a set winsize. Drop this in `/tmp/charcell.py`:

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

`cargo build -p beet_site --features cli` then `COLS=50 python3 /tmp/charcell.py ~/.cargo_target/debug/beet_site blog post-1`. Run the binary directly (not `cargo run`). Stripped mode prints `width repr(line)` per row, so overflow is `awk '$1>50'`. Pass `--raw` to keep escapes, then grep SGR codes (`38;2;r;g;b` fg, `48;2;r;g;b` bg). Never eyeball widths from a truncated `repr(line[:N])`, strip escapes and print the full line.

## Tests

- `cargo test -p beet_ui --lib render::charcell` (snapshots; `--snap` to regenerate, then eyeball `.beet/snapshots/...`).
- `cargo test -p beet_site` for end-to-end prose.
- Match a fix with a regression test asserting on cell state (`buffer.iter_cells()`, `cell.style.background`) or stripped output, not exact bytes.
