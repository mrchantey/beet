# Verifying the web target

The HTML rendered by `$BIN <path> --accept=text/html` inlines its stylesheet, so a `file://` URL renders fully styled with no server, no assets, no routing.

```bash
npx playwright install chromium   # once
```

## Check content first (cheap)

Most checks are about content and structure, not pixels: is the text right, are the elements present and in order. For those, grep the rendered HTML or take an accessibility-tree snapshot — both are far cheaper than a screenshot and don't burn image context. Reach for a screenshot only once content is confirmed and the remaining question is genuinely visual (spacing, colour, alignment, footer-at-bottom).

## Screenshot (only when visual judgment is needed)

One-shot, opens-captures-exits:

```bash
npx playwright screenshot --full-page file:///tmp/page.html out.png
npx playwright screenshot --viewport-size=1280,820 file:///tmp/page.html out.png  # footer-at-bottom check
npx playwright screenshot --color-scheme=dark file:///tmp/page.html out.png
```

`--full-page` for content; a fixed `--viewport-size` for layout that depends on viewport height. View only the few pages that matter.

## Interactive (functional) checks

For verifying scripts (eg a form's submit handler), drive a page with the `playwright` node module: `goto` the file, `fill`/`selectOption`/`click`, read back `textContent`. Needs the module resolvable on `NODE_PATH`. Usually unnecessary: the rendered HTML hooks (`name`/`id`/inline `<script>` text) can be grepped to confirm wiring.

## Notes

- Capture the live reference page and the local page and compare side by side.
- The stateful `playwright-cli` (sessions, snapshots, clicks) exists too, but needs the full chromium build, not the headless shell that `npx playwright screenshot` uses; prefer the one-shot form above.
