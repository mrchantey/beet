// In-browser end-to-end check for the thin-client reactivity runtime
// (`reactivity.js`), the proof the Rust/Node unit tests cannot give: a real
// browser hydrates the wire format and each verb mutates the client document and
// patches the DOM with no network round-trip and no re-render flash.
//
// Deterministic and server-free: it drives the *real* `reactivity.js` against
// HTML in the exact wire format the reactive `HtmlRenderer` emits (asserted by
// the Rust tests), via `setContent` + `addScriptTag`.
//
//   npx playwright install chromium   # once
//   node crates/beet_router/src/reactivity/reactivity.playwright.cjs

const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");

const { chromium } = require(
	path.join(os.homedir(), ".local/lib/node_modules/playwright"),
);

const runtime = fs.readFileSync(
	path.join(__dirname, "reactivity.js"),
	"utf8",
);

// The counter wire format: a marked document, bound text runs, and one `bx:`
// trigger per default verb, exactly as the reactive renderer emits it.
const COUNTER_HTML = `
<div data-bx-doc="d0">
	<p id="count">count is <!--bx-ref="counter.count"-->0<!--bx-end--></p>
	<p id="flag">flag is <!--bx-ref="flag"-->false<!--bx-end--></p>
	<p id="status">status is <!--bx-ref="status"--><!--bx-end--></p>
	<button id="inc" bx:click="increment{ field: @doc:counter.count, amount: 2 }">+</button>
	<button id="dec" bx:click="decrement{ field: @doc:counter.count }">-</button>
	<button id="tog" bx:click="toggle{ field: @doc:flag }">toggle</button>
	<button id="set" bx:click="set{ field: @doc:status, value: &quot;done&quot; }">set</button>
</div>
<script type="application/json" data-bx-blob>{"d0":{"counter":{"count":0},"flag":false,"status":""}}</script>
`;

// An app-supplied verb (the `js_verb` seam): the renderer emits the JS source in
// a `data-bx-verbs` blob, the runtime installs it beside the defaults.
const APP_VERB_HTML = `
<div data-bx-doc="d0">
	<p id="msg"><!--bx-ref="msg"-->quiet<!--bx-end--></p>
	<button id="shout" bx:click="shout{ field: @doc:msg }">shout</button>
</div>
<script type="application/json" data-bx-blob>{"d0":{"msg":"quiet"}}</script>
<script type="application/json" data-bx-verbs>{"shout":"entity.set_field(args.field, 'HEY');"}</script>
`;

let passed = 0;
let failed = 0;
function check(name, actual, expected) {
	const ok = JSON.stringify(actual) === JSON.stringify(expected);
	if (ok) {
		passed++;
		console.log(`  ok   ${name}`);
	} else {
		failed++;
		console.error(`  FAIL ${name}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
	}
}

/** Load HTML in the exact wire format, then attach the real runtime. */
async function hydrate(page, html, requests) {
	await page.setContent(html, { waitUntil: "load" });
	// count only post-hydration requests: the runtime must do no round-trip.
	requests.length = 0;
	await page.addScriptTag({ content: runtime });
}

async function main() {
	const browser = await chromium.launch();
	const page = await browser.newPage();
	const requests = [];
	page.on("request", (request) => requests.push(request.url()));

	// ---- the counter: hydration + each default verb ----------------------
	await hydrate(page, COUNTER_HTML, requests);

	// hydration: the SSR text is trusted, not overwritten (no flash).
	check(
		"hydrates count from SSR without flash",
		await page.textContent("#count"),
		"count is 0",
	);
	check(
		"client document matches the blob on load",
		await page.evaluate(() => window.beet.store.get("d0", "counter.count")),
		0,
	);

	// increment by its amount, then default decrement.
	await page.click("#inc");
	check("increment patches the DOM", await page.textContent("#count"), "count is 2");
	check(
		"increment mutates the client document",
		await page.evaluate(() => window.beet.store.get("d0", "counter.count")),
		2,
	);
	await page.click("#dec");
	check("decrement patches the DOM", await page.textContent("#count"), "count is 1");

	// toggle a boolean.
	await page.click("#tog");
	check("toggle patches the DOM", await page.textContent("#flag"), "flag is true");
	await page.click("#tog");
	check("toggle flips back", await page.textContent("#flag"), "flag is false");

	// set a string literal.
	await page.click("#set");
	check("set patches the DOM", await page.textContent("#status"), "status is done");
	check(
		"set mutates the client document",
		await page.evaluate(() => window.beet.store.get("d0", "status")),
		"done",
	);

	// the whole exchange was local: no network round-trip.
	check("no network round-trip for local mutations", requests, []);

	// ---- the app-supplied JS verb, end to end ----------------------------
	await hydrate(page, APP_VERB_HTML, requests);
	check("app verb hydrates", await page.textContent("#msg"), "quiet");
	await page.click("#shout");
	check("app verb runs in the browser", await page.textContent("#msg"), "HEY");

	await browser.close();

	console.log(`\n${passed} passed, ${failed} failed`);
	process.exit(failed === 0 ? 0 : 1);
}

main().catch((error) => {
	console.error(error);
	process.exit(1);
});
