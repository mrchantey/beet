// Node.js unit tests for the thin-client reactivity runtime (`reactivity.js`).
//
// Drives each default verb against the *non-DOM* `EntityMut`, the same data
// representation the browser variant wraps, so these genuinely cover verb
// behavior rather than a stand-in. No DOM, no dependencies.
//
//   node crates/beet_ui/src/widgets/reactivity.test.cjs

const assert = require("node:assert");
const path = require("node:path");
const { createStore, EntityMut, verbs, parseVerbCall, installVerbs } =
	require(path.join(__dirname, "reactivity.js"));

let passed = 0;
let failed = 0;
function test(name, fn) {
	try {
		fn();
		passed++;
		console.log(`  ok   ${name}`);
	} catch (error) {
		failed++;
		console.error(`  FAIL ${name}\n       ${error.message}`);
	}
}

/** A non-DOM entity over a fresh single-document store. */
function entityOver(initial) {
	const store = createStore({ d0: initial });
	return { store, entity: new EntityMut(store, "d0", undefined, null) };
}

// ---- verb-call parsing -----------------------------------------------------

test("parseVerbCall strips @doc bindings and parses literals", () => {
	const parsed = parseVerbCall(
		"increment{ field: @doc:counter.count, amount: 3 }",
	);
	assert.strictEqual(parsed.verb, "increment");
	assert.strictEqual(parsed.args.field, "counter.count");
	assert.strictEqual(parsed.args.amount, 3);
});

test("parseVerbCall handles a string literal", () => {
	const parsed = parseVerbCall('set{ field: @doc:status, value: "done" }');
	assert.strictEqual(parsed.args.field, "status");
	assert.strictEqual(parsed.args.value, "done");
});

test("parseVerbCall handles a no-arg verb", () => {
	assert.deepStrictEqual(parseVerbCall("refresh"), {
		verb: "refresh",
		args: {},
	});
});

// ---- default verbs ---------------------------------------------------------

test("increment adds amount", () => {
	const { store, entity } = entityOver({ count: 0 });
	verbs.increment(entity, parseVerbCall("increment{ field: @doc:count, amount: 3 }").args);
	assert.strictEqual(store.get("d0", "count"), 3);
});

test("increment defaults amount to 1", () => {
	const { store, entity } = entityOver({ count: 5 });
	verbs.increment(entity, parseVerbCall("increment{ field: @doc:count }").args);
	assert.strictEqual(store.get("d0", "count"), 6);
});

test("increment treats a missing field as 0", () => {
	const { store, entity } = entityOver({});
	verbs.increment(entity, { field: "count" });
	assert.strictEqual(store.get("d0", "count"), 1);
});

test("decrement subtracts amount", () => {
	const { store, entity } = entityOver({ count: 10 });
	verbs.decrement(entity, parseVerbCall("decrement{ field: @doc:count, amount: 4 }").args);
	assert.strictEqual(store.get("d0", "count"), 6);
});

test("toggle flips a boolean both ways", () => {
	const { store, entity } = entityOver({ flag: false });
	verbs.toggle(entity, { field: "flag" });
	assert.strictEqual(store.get("d0", "flag"), true);
	verbs.toggle(entity, { field: "flag" });
	assert.strictEqual(store.get("d0", "flag"), false);
});

test("toggle treats a missing field as false", () => {
	const { store, entity } = entityOver({});
	verbs.toggle(entity, { field: "flag" });
	assert.strictEqual(store.get("d0", "flag"), true);
});

test("set writes the value verbatim", () => {
	const { store, entity } = entityOver({ status: "pending" });
	verbs.set(entity, parseVerbCall('set{ field: @doc:status, value: "done" }').args);
	assert.strictEqual(store.get("d0", "status"), "done");
});

// ---- scoped (nested) paths -------------------------------------------------

test("verbs honour an absolute nested path (the scoped counter)", () => {
	const { store, entity } = entityOver({ counter: { count: 0 } });
	verbs.increment(entity, parseVerbCall("increment{ field: @doc:counter.count }").args);
	assert.strictEqual(store.get("d0", "counter.count"), 1);
});

// ---- recorded mutations ----------------------------------------------------

test("a mutation fans out to store subscribers", () => {
	const store = createStore({ d0: { count: 0 } });
	const mutations = [];
	store.subscribe((docId, path, value) => mutations.push([docId, path, value]));
	const entity = new EntityMut(store, "d0", undefined, null);
	verbs.increment(entity, { field: "count", amount: 2 });
	assert.deepStrictEqual(mutations, [["d0", "count", 2]]);
});

// ---- app-supplied JS verb (the `js_verb` seam) -----------------------------

test("an app verb installed from source runs like a default", () => {
	const { store, entity } = entityOver({ msg: "" });
	installVerbs({ shout: "entity.set_field(args.field, 'HEY');" });
	verbs.shout(entity, parseVerbCall("shout{ field: @doc:msg }").args);
	assert.strictEqual(store.get("d0", "msg"), "HEY");
});

console.log(`\n${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
