# Iterating on Beet Rendering

How to do rendering work well: the build/verify loop, how to use sub-agents, and how to judge visual quality. This is the process companion to the system reference.

## Further reading

- `rendering-system.md` (adjacent): how the rendering system, classes, tokens, widgets, and routing work. Read this first.
- `charcell.md` (adjacent): the terminal (charcell) renderer internals, plus how to measure true terminal width under a PTY.
- `playwright.md` (adjacent): the Playwright CLI for browser automation, accessibility snapshots, and screenshots.

Spend a little time up front: either read `rendering-system.md`, or for a large task spawn one initial sub-agent to map the current rendering approach and write a short cheat sheet the other agents can read. Re-deriving the system in every agent is the expensive path.

## The iteration loop

Rendering work is iterative: try something, look at the result on both targets, adjust. A single cycle:

1. Make the change (a page, widget, or rule).
2. If routes changed, regenerate codegen: `cargo run -p beet_site --no-default-features --features codegen`.
3. Build and fix compile errors: `cargo build -p beet_site --features cli` (and `--features web` for the web target).
4. Render the terminal target: `cargo run -p beet_site --features cli -- <path segments>`.
5. Render the web target: run the server (`cargo run -p beet_site --features web`, serves `127.0.0.1:8337`) and view the page.
6. Run tests where relevant: `cargo test -p beet_site`, `cargo test -p beet_ui --lib render::charcell`.

Always check both targets. They exercise different buffers and a change that looks right on one can be wrong on the other.

## Prefer tokens over CSS

The whole point of this workflow is target-agnostic rendering, and CSS is not target-agnostic: anything written in CSS will not translate to the terminal renderer or other future targets. Whenever you reach for CSS, prefer a semantic class backed by a registered rule instead. Raw CSS via a `<style>` element is an available escape hatch, but use it sparingly and only when a token or class genuinely cannot express the result.

## Using sub-agents

Sub-agents are excellent for breadth: crawling a live site, diffing structures, searching the codebase, porting many similar pages. Use them to keep the orchestrating agent's context clean.

Sub-agents do not inherit the parent's `AGENTS.md` / `CLAUDE.md` instructions. Conventions you rely on (for example: no mid-sentence line breaks in markdown, no em dashes, testing and import conventions) must be restated in the sub-agent's prompt, or the sub-agent will not follow them. Pass down the specific rules that matter for the work you are handing off.

## Visual verification

The hardest part to get right is judging how the output actually looks, and it is the part most easily lost when handing work between agents.

Set the quality bar against the live site. The local site may make different stylistic choices, but the live site serves as a clean example of a high-quality Material rendering of the same content; strive for that same level of quality. The bar is not "is it broken / unstyled" (a low bar that is easy to pass), it is "is this as good as the live reference".

Screenshots fill context very quickly, so they should live inside sub-agents, not the orchestrator. But capturing is not the valuable part: the sub-agent must also judge the result against the bar above and iterate on it, rather than capturing a screenshot, summarizing it as text, and handing the summary back. A text summary loses aesthetic judgment, and "not broken" will get relayed up as "looks fine" when it does not.

Concretely:

- The orchestrating agent should avoid pulling screenshots into its own context. If visual verification or visual iteration is needed, spawn a visual-verification sub-agent that does the looking, the judging, and the iterating, and give it the authority to keep going until the page meets the bar.
- Sub-agents read their own screenshots. They can pass back paths to screenshots if they genuinely need to discuss the state of things, but prefer giving the sub-agent enough context and authority to act on what it sees rather than routing pixels (or judgments about pixels) back up the chain.
- Compare against the live reference directly: capture live-vs-local pairs and judge them side by side, not in isolation.

Use accessibility-tree snapshots first to confirm structure and content cheaply, and reserve screenshots for the actual visual judgment. See the adjacent `playwright.md` for both.
</content>
