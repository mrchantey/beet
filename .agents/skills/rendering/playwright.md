# Verifying the web target

Two ways to a styled page: run the server (`cargo run -p beet_site`, :8337) and hit the live URL, or render `--accept=text/html` to a file (the stylesheet inlines, so `file://` needs no server). The live server is best for interactive checks; a file is best for a frozen snapshot. `npx playwright install chromium` once.

## Content first (cheap)

Most questions are content/structure, not pixels: is the text right, are elements present and ordered. Grep the rendered HTML (including the inlined `<style>` block) or take an accessibility-tree snapshot, both far cheaper than a screenshot. Reach for a screenshot only when the question is genuinely visual (spacing, colour, alignment, footer-at-bottom).

## Screenshot (one-shot)

```bash
npx playwright screenshot --full-page http://localhost:8337/docs out.png
npx playwright screenshot --viewport-size=1280,820 file:///tmp/page.html out.png   # footer-at-bottom
npx playwright screenshot --color-scheme=dark file:///tmp/page.html out.png
```

Clip to the element under test to save context: a small node script that resolves `playwright` from `~/.local/lib/node_modules`, `goto`s, and `(await page.$('#sidebar')).screenshot(...)`.

## Probe + interact (functional)

The same node script is how you find root causes and verify behavior without eyeballing: `page.evaluate` to dump `getComputedStyle`/`getBoundingClientRect` over the elements in question, or `click`/`fill`/`selectOption` then read back state (eg click a `<details>` summary, assert its caret's computed `transform` flipped). Capture the live reference and the local page and judge them side by side.
