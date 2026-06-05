# Iterating on Beet Rendering

The build/verify loop and how to judge quality. System reference: `rendering-system.md`. Charcell internals + true terminal width: `charcell.md`. Screenshots: `playwright.md`.

Reference material:

- `.agents/reference/beet_old` — the previous site's source (an older, web-only API). Read it when porting a page or deciding how something should look.
- The live site is the quality bar: a clean Material rendering of the same content. "Not broken" is not the bar; "as good as the live reference" is.

## The loop

`BIN=~/.cargo_target/debug/beet_site`.

1. Make the change (page, widget, or rule).
2. If routes changed: `cargo run -p beet_site --no-default-features --features codegen`.
3. Build: `cargo build -p beet_site --no-default-features --features cli`.
4. Charcell: `$BIN <path segments> </dev/null` (eg `$BIN blog post-1 </dev/null`).
5. Web: `$BIN <path segments> --accept=text/html </dev/null > page.html` — a full document with the stylesheet inlined.
6. Tests: `cargo test -p beet_ui --lib`, `cargo test -p beet_site` (update snapshots with `--snap`).

Always check both targets. They use different buffers; a change right on one can be wrong on the other.

Two binary gotchas:

- The CLI server reads stdin, so always redirect `</dev/null` or it hangs.
- `cargo build`/`cargo test` (default features) overwrite the binary with the web target, which binds an HTTP server instead of rendering. Rebuild `--no-default-features --features cli` before charcell/HTML renders.

## Read the generated CSS, not just the pixels

A rule can compile and still serialize to invalid CSS that browsers silently drop (eg a value whose charcell spelling is not a CSS keyword, like `flex-direction: vertical`). After a styling change, grep the `<style>` block in the HTML output for the selector and confirm the declarations are valid CSS, then screenshot. Pixels alone hide this class of bug.

## Screenshots

The HTML from step 5 inlines its CSS, so `file://` renders fully styled with no server. Once: `npx playwright install chromium`. Then `npx playwright screenshot --full-page file:///abs/page.html out.png` (or `--viewport-size=1280,820` to check footer-at-bottom). See `playwright.md`. Capture the live reference and the local page and judge them side by side. View only the few pages that matter; screenshots are heavy in context.

## Sub-agents

Optional, for breadth (crawling the live site, porting many similar pages). They do not inherit `CLAUDE.md`; restate the conventions that matter (no em dashes, no mid-sentence markdown breaks, testing/import rules) in the prompt.

## When done

Update these skill files with what the pass taught you: fold in new facts, fix anything that misled you, and cut whatever is now redundant. Keep them terse.
