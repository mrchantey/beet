# Verifying the web target

Two ways to a styled page: run the server (`cargo run -p rsx_site`, :8337) and hit the live URL, or render `--accept=text/html` to a file (the stylesheet inlines, so `file://` needs no server). The live server is best for interactive checks; a file is best for a frozen snapshot. All browser driving goes through the in-house webdriver (`beet_net::webdriver`): `chromedriver` and a chromium/chrome on PATH are the only dependencies.

## Content first (cheap)

Most questions are content/structure, not pixels: is the text right, are elements present and ordered. Grep the rendered HTML (including the inlined `<style>` block) or take an accessibility-tree snapshot, both far cheaper than a screenshot. Reach for a screenshot only when the question is genuinely visual (spacing, colour, alignment, footer-at-bottom).

## Screenshot (one-shot)

```bash
beet screenshot http://localhost:8337/docs --output=out.png
beet screenshot 'file:///tmp/page.html' --width=1280 --height=820 --output=out.png   # footer-at-bottom
beet screenshot 'http://localhost:8337/docs?color-scheme=dark' --output=dark.png
beet screenshot http://localhost:8337/docs --selector='#sidebar' --output=sidebar.png  # clip to the element under test
beet screenshot http://localhost:8337/docs --full-page --output=full.png
```

Color schemes ride the url (`?color-scheme=light|dark`), applied server-side. `--selector` auto-waits for the element and crops to it, saving context when reading the png.

## Probe + interact (functional)

Root causes and behavior checks without eyeballing: write a scratch `#[ignore = "smoketest"]` test (or extend an existing browser smoketest) driving `Page`/`Element` from `beet_net::prelude::webdriver`. `page.evaluate_value("getComputedStyle(...)")` dumps computed style or `getBoundingClientRect` as plain JSON; `page.click("summary")` (re-queries on staleness) then read back state with the auto-waiting matchers (`page.find("#caret").await.xpect_attr(..)`). The reactivity proof at `crates/beet_ui/src/render/html/reactive_html_render.rs` (`reactivity_in_browser`) is the reference for the full pattern: collectors for console/network, trusted clicks, `evaluate_value` for state. Capture the live reference and the local page and judge them side by side.
