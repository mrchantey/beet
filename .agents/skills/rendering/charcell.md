# Charcell Renderer

The terminal layout + paint engine, `crates/beet_ui/src/render/charcell`. Read the code as you go; this is the part that bites and how to verify.

## Already handled, don't reinvent in CSS

`charcell/decorate.rs` generates leading content (the terminal's `::before`): `<li>` bullets/numbers, blockquote bars, the `<hr>` rule, `<img>` alt text (`[image]: …`), the `<iframe>` link (`[iframe]: …`, the whole prefix is part of the OSC-8 link), and the generic `<details>`/`<summary>` disclosure caret (`▸` closed / `▾` open). So list/quote markers exist on the terminal with no CSS `list-style`; the web is the side that restores markers. `<li>` under a `<nav>` get **no** marker (navigation, not prose).

`decorate.rs` also runs target-only structural fixups the cascade can't express, since `DecorateSet` runs after `ResolveStylesSet` and may mutate the resolved `LayoutStyle`/`BoxStyle`: `apply_disclosure` collapses a closed generic `<details>` (sets non-`<summary>` children to `Display::None`; the sidebar's `SIDEBAR_GROUP` is exempt and always expands), and `apply_table_vertical_borders` gives each non-first cell of a `.table-vertical-borders` table a left border mirroring its bottom rule (the web does this with a `td + td` sibling rule in `reset.css`). Register a new such system in `DecorateSet` (`charcell/plugin.rs`).

## What the engine now honours (don't reach for CSS-only workarounds)

- **Explicit `width`/`height`** size a box on the terminal too. Absolute (`Px`/`Rem`, 1rem≈1cell) and viewport lengths resolve in `box_model.rs` `from_box_style` (the cell viewport, so `50vw`=half the columns, *not* `into_rem`'s pixel scaling) and feed `measure_node`/`resolve_height`. **Percent** can't resolve in the bottom-up measure pass (the containing block isn't known yet), so it stays content-sized there and is resolved top-down in the layout pass via `explicit_box_size`, against the parent's content rect — wired into `block_layout_rects` (block children), `flex_layout_rects` (item base size), so a `width:50%` block is half its container and a `width:100%` table still fills its column. CSS `width` is content-box, so `explicit_box_size` adds the child's own overhead back to the footprint.
- **Tables** (`charcell/table.rs`) lay out as a column-aligned grid off `Display::Table` + `Display::TableCell` alone — a *row* is any node with `table-cell` children (covers `<tr>` and a markdown `<thead>` holding cells directly), so `<tr>`/`<thead>`/`<tbody>` need no display and are skipped via a `managed` set in `layout_nodes`. Columns take their widest cell, scaled down to fit.
- **Last child drops its bottom margin** in block flow (`measure_block`/`resolve_block_height`/`block_layout_rects` via `node_bottom_margin`), the cross-target `:last-child { margin-bottom: 0 }` — fixes the blockquote/card double gutter and the nested-list gap. The last child keeps its full rect (no clamp) so its empty trailing-margin row spills into the container's padding rather than clipping content.
- **Tabs in `<pre>` expand to 4-col stops** (`inline.rs` `split_pre_lines`, matching web `tab-size: 4`); a raw `\t` left in a cell makes the real terminal re-expand and overflow the code box. A single trailing empty line is dropped there too (fenced code ends with `\n`).
- **Block-leaf text decoration** (eg the iframe link's underline) only covers the glyphs, not the row-filling pad: `paint_text` paints the padded line undecorated, then overlays the glyphs decorated.
- **Anonymous block boxes**: a list item mixing leading inline content with a nested list gets its inline run wrapped in a `<div>` at parse time (`tree_builder.rs` `wrap_inline_runs`) so it flows on one line instead of each child breaking to its own.

## Facts that bite

- **`tui_inset` doubles horizontal spacing** (`box_model.rs` does `min.x *= 2`), so `1rem` left padding = **2** cells, and `0.5rem` also → 2 (`round(0.5)=1`, then `*2`). Horizontal padding is therefore always even; a **1-cell** indent is unreachable through padding, it needs an explicit per-depth indent or a charcell change.
- **A block lays out its inline children inline.** A `<summary>` (block) with an inline label + caret flows them on one row; that's why the sidebar caret sits beside its label on the terminal without flex (flex is the web's screen-gated rule).
- **Adding `padding`/`display:block` to an inline element** changes its charcell box and can shift a whole subtree. Re-render the terminal after any such change, not just the web.
- **A flex column clamps each item's cross size (width) to the container** (`resolve_line_sizes` in `flex.rs`). Items are measured at the unconstrained viewport, so without the clamp a column with `align-items: center` centres against a width wider than it has and overflows (the homepage-hero-past-the-sidebar bug). The main axis stays unclamped (height scrolls).
- **The ratatui/ANSI paint drops colour alpha** (`color_to_ratatui` uses RGB only), so a *transparent* colour renders as **black**, not invisible. To reserve a border that should not show (eg equalising a filled button with an outlined one), colour it the same as the fill, not transparent.
- **Default scheme is dark**, set by the layout (`rsx_site/src/layout.rs`), not the renderer: a non-html request gets `.dark-scheme` on `<body>` so a light `OnSurface` isn't invisible on a dark terminal. Transcluded `.md`/`Portal` content inherits this because `RuleSetQuery::parent` + `resolve_styles` follow `Portal` across the transclusion boundary.
- **An empty `Value` reserves no row** (`text.rs` `measure_text`): a blank `<input>`/`<textarea>` hugs its padding/border, matching a control with no value. This matters because the live/interactive path (`CharcellTuiPlugin`/serve) installs `FormPlugin`, which seeds every form control an empty `Value::str("")` for editability, while the static one-shot render path (`CharcellPlugin`, eg `rsx_site --features cli`, `AnsiTermRenderer`) does **not** — so a form-control box can measure a row taller on the serve than in a CLI render. The exception is anything carrying a [`Marker`] (a `<select>`'s label): its empty value is submission state but the marker paints, so it keeps its row.

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

`cargo build -p rsx_site --features cli` then `COLS=50 python3 /tmp/charcell.py ~/.cargo_target/debug/rsx_site counter`. Run the binary directly (not `cargo run`). Stripped mode prints `width repr(line)` per row, so overflow is `awk '$1>50'`. Pass `--raw` to keep escapes, then grep SGR codes (`38;2;r;g;b` fg, `48;2;r;g;b` bg). Never eyeball widths from a truncated `repr(line[:N])`, strip escapes and print the full line.

## Tests

- `cargo test -p beet_ui --lib render::charcell` (snapshots; `--snap` to regenerate, then eyeball `.beet/snapshots/...`).
- `cargo test -p rsx_site` for end-to-end prose.
- Match a fix with a regression test asserting on cell state (`buffer.iter_cells()`, `cell.style.background`) or stripped output, not exact bytes.
