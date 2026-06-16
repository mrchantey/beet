// In-browser end-to-end check for the thin-client reactivity runtime
// (`reactivity.js`), the proof the Rust/deno unit tests cannot give: a real
// Chromium hydrates the page and each verb mutates the client document and
// patches the DOM, with no network round-trip and no re-render flash.
//
// It drives the *real* rendered output (not a hand-written fixture): the Rust
// test `writes_playwright_fixture` renders the counter reactively with the
// runtime inlined and every default verb plus a custom `double` verb, so the
// page here is byte-for-byte what the renderer emits. Run, in order:
//
//   cargo test -p beet_ui --lib writes_playwright_fixture   # render the fixture
//   npx playwright install chromium                         # once
//   node crates/beet_ui/src/render/html/reactivity.playwright.cjs

const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");

const { chromium } = require(
	path.join(os.homedir(), ".local/lib/node_modules/playwright"),
);

// the rendered fixture written by `writes_playwright_fixture`
const fixture = path.join(
	__dirname,
	"../../../../target/playwright/counter.html",
);
if (!fs.existsSync(fixture)) {
	console.error(
		`missing fixture: ${fixture}\nrun: cargo test -p beet_ui --lib writes_playwright_fixture`,
	);
	process.exit(1);
}

let passed = 0;
let failed = 0;
function check(name, actual, expected) {
	const ok = JSON.stringify(actual) === JSON.stringify(expected);
	if (ok) {
		passed++;
		console.log(`  ok   ${name}`);
	} else {
		failed++;
		console.error(
			`  FAIL ${name}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
		);
	}
}

async function main() {
	const browser = await chromium.launch();
	const page = await browser.newPage();
	// load the rendered page; its inlined runtime boots on `DOMContentLoaded`
	await page.goto("file://" + fixture, { waitUntil: "load" });

	// count only post-load requests: every verb must mutate locally, no round-trip
	const requests = [];
	page.on("request", (request) => requests.push(request.url()));

	// hydration: the runtime trusts the blob, leaving the correct SSR text in
	// place (no flash), and the client document matches the blob.
	check("hydrates count from SSR without flash", await text(page, "#count"), "count is 0");
	check("hydrates flag without flash", await text(page, "#flag"), "flag is false");
	check(
		"client document matches the blob on load",
		await store(page),
		{ count: 0, flag: false, status: "pending" },
	);

	// every default verb, installed from the `data-bx-verbs` blob
	await page.click("#inc"); // increment{ amount: 2 }
	check("increment patches the DOM", await text(page, "#count"), "count is 2");
	await page.click("#dec"); // decrement (default amount 1)
	check("decrement patches the DOM", await text(page, "#count"), "count is 1");
	await page.click("#tog"); // toggle
	check("toggle patches the DOM", await text(page, "#flag"), "flag is true");
	await page.click("#set"); // set{ value: "done" }
	check("set patches the DOM", await text(page, "#status"), "status is done");

	// the custom app verb (its `js_verb` shipped in `data-bx-verbs`), end to end
	await page.click("#dbl"); // double: 1 -> 2
	check("custom verb runs in the browser", await text(page, "#count"), "count is 2");

	// the client document reflects every mutation
	check("client document reflects the mutations", await store(page), {
		count: 2,
		flag: true,
		status: "done",
	});
	// the whole exchange was local: no network round-trip
	check("no network round-trip for local mutations", requests, []);

	await browser.close();
	console.log(`\n${passed} passed, ${failed} failed`);
	process.exit(failed === 0 ? 0 : 1);
}

/** The visible text of a selector. */
async function text(page, selector) {
	return (await page.textContent(selector)).trim();
}

/** The live client document `d0` from `globalThis.beet`. */
async function store(page) {
	return await page.evaluate(() => globalThis.beet.store.docs.d0);
}

main().catch((error) => {
	console.error(error);
	process.exit(1);
});
