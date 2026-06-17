# Iterating on Beet Rendering

Pages are authored once as target-agnostic scenes and rendered to two targets: the web (HTML + CSS) and the terminal (charcell ANSI). This skill is how to change them and get it right. System reference: `rendering-system.md`. Charcell internals + true terminal width: `charcell.md`. Screenshots: `playwright.md`. No-code (markup) sites + matching a compiled reference for parity: `no-code-sites.md`. Live reference (previous, web-only site): `.agents/reference/beet_old`.

## Attitude

The live reference is the bar. "Not broken" is not the bar. Find the gap between it and the new target, then close it.

- **Make the call.** Nothing is "deferred" or "needs design". Pick the right behavior and implement it. If the framework can't express it (a missing CSS combinator, a missing property), add that capability rather than working around it in the widget.
- **Fix the cause, not the symptom.** Misaligned carets, runaway indents, a rail that resizes per page are usually *one* upstream cause in the cascade. Patching the widget hides it and it comes back. Instrument until you can see *why* a node sits where it does, then fix that one thing. (The sidebar's whole list of visible bugs reduced to two cascade bugs: a media-blind rule merge and properties wrongly marked inherited.)
- **Both targets, every change.** They share the rule set but use different layout engines; a fix on one can break the other.
- **It's trivial to look.** One command renders the terminal, one screenshots the web. No excuse for shipping something that looks wrong — watch for empty lines, runaway indents, misaligned carets.

## The loop

The CLI binary renders one route to stdout and exits; the default binary serves the web on :8337. They are different feature sets, so each switch recompiles, batch all terminal checks, then all web checks.

1. Change the page, widget, or rule.
2. Routes changed? `cargo run -p beet_site --no-default-features --features codegen`.
3. **Terminal:** `cargo run -p beet_site --features=cli -- <path> --accept=text/ansi-term` (strip escapes to read; true width via the PTY harness in `charcell.md`).
4. **Web:** `cargo run -p beet_site` serves :8337, screenshot with playwright (`playwright.md`).
5. **Tests:** `cargo test -p beet_ui --lib`, `cargo test -p beet_site` (`--snap` to update snapshots).

## Instrument to root cause

A throwaway harness beats re-rendering the whole site. Build the *real* widget in a world with the *real* rule set and dump per-node facts:

- **Charcell:** spawn the scene in a `(scene plugins, CharcellPlugin, MaterialStylePlugin)` world, attach a `FlexBuffer`, run the `PostParseTree` schedule, then query every entity's `Element`/`LayoutRect`/`LayoutStyle`/`BoxStyle` and print tag, display, `x..max`, resolved padding. One run shows whether a node is misplaced by its own box or by an inherited/leaked value.
- **Web:** in `page.evaluate`, `getComputedStyle` + `getBoundingClientRect` over the elements in question; dump width/display/padding/transform. A "134px rail that should be 256px" is one query from the answer (a flex sibling shrinking it).

Promote a harness that earns its keep into a regression test (see `widgets/sidebar.rs` `mod test`).

## Read the generated CSS, not just pixels

Charcell reads the style enum; CSS reads `AsCssValue`. They can disagree, a value valid for the terminal may serialize to CSS the browser silently drops. After a style change, grep the `<style>` block for the selector and confirm the declarations are real CSS, then screenshot.

## Sub-agents

Optional, for breadth (crawling the live site, porting many similar pages). They do not inherit `CLAUDE.md`; restate the conventions that matter in the prompt.

## When done

Fold what the pass taught you back into these files. Cut anything now wrong or redundant. Keep them terse.
