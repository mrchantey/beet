// The beet thin-client reactivity runtime: a tiny, dependency-free signal engine
// that hydrates a page from the wire format the reactive HTML renderer emits
// (see `render/html/reactive_html_render.rs` for the contract) and drives it with no
// WASM.
//
// It reads the annotations already in the HTML:
//   - `<script data-bx-blob>`  : the document state, keyed by `dN`.
//   - `<script data-bx-verbs>` : the JS verbs to install (every verb, including
//                                the defaults). The runtime ships zero built-in
//                                verbs; it is pure mechanism.
//   - `data-bx-doc="dN"`       : the element topping document N's subtree.
//   - `<!--bx-ref="path"-->..<!--bx-end-->` : a bound text run.
//   - `data-bx-attr-name="path"`            : a bound attribute.
//   - `bx:event="verb{ field: @doc:path, .. }"` : an event verb trigger.
//
// A verb is a pure function of `(entity, args)`, the JS twin of the Rust
// `verb(EntityWorldMut, VerbArgs)`. `entity` is an `EntityMut` over the
// event-target node; it touches the document and the node, never `document`/
// `window` (only the DOM binding layer does). The same `EntityMut`, minus its DOM
// binding, runs under deno / a worker / the unit tests against an in-memory
// store, so the tests cover real verb behavior, not a stand-in.
//
// The runtime api and the live store are exposed on `globalThis.beet` so a test
// (deno/wasm) or the browser can read them after evaluating this file.
(function () {
	"use strict";

	// ---- document store ----------------------------------------------------
	//
	// `Value = string | number | boolean | array | object | null`, the JS mirror
	// of the Rust `Value`. The store is the one authoritative copy of each value;
	// the in-place markers only ever name a path.

	/** Read a dotted path (`a.b.0.c`) from a value, `undefined` if absent. */
	function getPath(root, path) {
		if (!path) return root;
		return path.split(".").reduce(
			(current, key) => (current == null ? undefined : current[key]),
			root,
		);
	}

	/** Write `value` at a dotted path, creating intermediate objects as needed. */
	function setPath(root, path, value) {
		const keys = path.split(".");
		let current = root;
		for (let i = 0; i < keys.length - 1; i++) {
			const key = keys[i];
			if (current[key] == null || typeof current[key] !== "object") {
				current[key] = {};
			}
			current = current[key];
		}
		current[keys[keys.length - 1]] = value;
	}

	/** A reactive store of one or more documents keyed by id (`d0`, `d1`, ..). */
	function createStore(docs) {
		const subscribers = [];
		return {
			docs: docs || {},
			get(docId, path) {
				return getPath(this.docs[docId], path);
			},
			set(docId, path, value) {
				if (this.docs[docId] == null) this.docs[docId] = {};
				setPath(this.docs[docId], path, value);
				subscribers.forEach((fn) => fn(docId, path, value));
			},
			subscribe(fn) {
				subscribers.push(fn);
			},
		};
	}

	// ---- EntityMut ---------------------------------------------------------
	//
	// The JS twin of `EntityWorldMut`: a verb acts only through it. One interface,
	// two variants. The data representation (`repr`: tag, tracked value,
	// attributes, classes) stands alone; the optional `binding` reflects mutations
	// into a real DOM node. `binding === null` is the non-DOM variant.

	/** An empty node representation, the non-DOM default. */
	function emptyRepr(tag) {
		return {
			tag: tag,
			value: undefined,
			attributes: {},
			classes: new Set(),
		};
	}

	class EntityMut {
		constructor(store, docId, repr, binding) {
			this.store = store;
			this.docId = docId;
			this.repr = repr || emptyRepr(undefined);
			this.binding = binding || null;
		}
		// document-scope-aware field access (the document is resolved once, at
		// construction, by walking up to `data-bx-doc`).
		get_field(path) {
			return this.store.get(this.docId, path);
		}
		set_field(path, value) {
			this.store.set(this.docId, path, value);
		}
		// element access, mirroring the Bevy components.
		get_tag() {
			return this.repr.tag;
		}
		get_attribute(key) {
			return this.repr.attributes[key];
		}
		set_attribute(key, value) {
			this.repr.attributes[key] = value;
			if (this.binding) this.binding.setAttribute(key, value);
		}
		get_value() {
			return this.repr.value;
		}
		set_value(value) {
			this.repr.value = value;
			if (this.binding) this.binding.setValue(value);
		}
		contains_class_name(name) {
			return this.repr.classes.has(name);
		}
		set_class(name, on) {
			if (on) this.repr.classes.add(name);
			else this.repr.classes.delete(name);
			if (this.binding) this.binding.setClass(name, on);
		}
	}

	// ---- verb table --------------------------------------------------------
	//
	// The runtime ships zero built-in verbs: it is pure mechanism. Every verb,
	// including the defaults (`increment`/`decrement`/`toggle`/`set`), arrives in
	// the `data-bx-verbs` blob and is installed below. A `field` arg is a path the
	// verb mutates via `entity.set_field`; the rest are values.

	const verbs = {};

	/** Register (or override) a verb at runtime, the JS twin of `VerbRegistry`. */
	function installVerb(name, fn) {
		verbs[name] = fn;
	}

	/** Install every verb from a `name -> js-source` map (`data-bx-verbs`),
	 *  including the defaults the renderer emits. */
	function installVerbs(map) {
		for (const name of Object.keys(map || {})) {
			// the source is a `(entity, args) => { .. }` body, the JS twin the
			// renderer co-located with the Rust verb (no codegen, a hand-written
			// twin). `new Function` sees only `entity`/`args` + globals, so the
			// body must be self-contained.
			verbs[name] = new Function("entity", "args", map[name]);
		}
	}

	// ---- verb-call parsing -------------------------------------------------
	//
	// Parse `verb{ field: @doc:a.b, amount: 3 }` into `{ verb, args }`. A binding
	// argument (`@doc:`/`@prop:`) becomes its (already absolute) path string; a
	// literal is parsed as JSON.

	function parseArgValue(raw) {
		if (raw.startsWith("@doc:")) return raw.slice(5);
		if (raw.startsWith("@prop:")) return raw.slice(6);
		try {
			return JSON.parse(raw);
		} catch (_error) {
			return raw;
		}
	}

	/** Split a `{ .. }` body on top-level commas, respecting strings and nesting. */
	function splitArgs(body) {
		const parts = [];
		let depth = 0;
		let inString = false;
		let start = 0;
		for (let i = 0; i < body.length; i++) {
			const char = body[i];
			if (inString) {
				if (char === '"' && body[i - 1] !== "\\") inString = false;
			} else if (char === '"') {
				inString = true;
			} else if (char === "[" || char === "{") {
				depth++;
			} else if (char === "]" || char === "}") {
				depth--;
			} else if (char === "," && depth === 0) {
				parts.push(body.slice(start, i));
				start = i + 1;
			}
		}
		parts.push(body.slice(start));
		return parts.map((part) => part.trim()).filter((part) => part.length > 0);
	}

	function parseVerbCall(call) {
		const trimmed = call.trim();
		const brace = trimmed.indexOf("{");
		if (brace === -1) return { verb: trimmed, args: {} };
		const verb = trimmed.slice(0, brace).trim();
		const body = trimmed.slice(brace + 1, trimmed.lastIndexOf("}"));
		const args = {};
		for (const part of splitArgs(body)) {
			const colon = part.indexOf(":");
			if (colon === -1) continue;
			args[part.slice(0, colon).trim()] = parseArgValue(
				part.slice(colon + 1).trim(),
			);
		}
		return { verb, args };
	}

	// ---- browser runtime ---------------------------------------------------
	//
	// The only code that touches `document`. It builds the store from the blob,
	// installs app verbs, indexes the markers, wires `bx:` listeners, and patches
	// bound nodes whenever a path changes. Verbs themselves never reach in here.

	const COMMENT_NODE = 8;
	const TEXT_NODE = 3;

	/** The document id governing `element`, by its nearest `data-bx-doc` ancestor. */
	function docIdFor(element) {
		const host = element && element.closest("[data-bx-doc]");
		return host ? host.getAttribute("data-bx-doc") : undefined;
	}

	/** Read a `<script type="application/json">` payload, `{}` when absent. */
	function readJsonScript(selector) {
		const element = document.querySelector(selector);
		if (!element) return {};
		try {
			return JSON.parse(element.textContent);
		} catch (_error) {
			return {};
		}
	}

	/** Collect every `<!--bx-ref-->` text run and `data-bx-attr-*` binding. */
	function collectBindings() {
		const texts = [];
		const walker = document.createTreeWalker(
			document.body,
			NodeFilter.SHOW_COMMENT,
		);
		let node = walker.nextNode();
		while (node) {
			const match = /^bx-ref="(.*)"$/.exec(node.nodeValue);
			if (match) {
				texts.push({
					start: node,
					docId: docIdFor(node.parentElement),
					path: match[1],
				});
			}
			node = walker.nextNode();
		}

		const attributes = [];
		for (const element of document.querySelectorAll("*")) {
			for (const attr of Array.from(element.attributes)) {
				if (attr.name.startsWith("data-bx-attr-")) {
					attributes.push({
						element: element,
						name: attr.name.slice("data-bx-attr-".length),
						docId: docIdFor(element),
						path: attr.value,
					});
				}
			}
		}
		return { texts, attributes };
	}

	/** Patch one bound text run to the store's value (its anchor's next sibling). */
	function patchText(binding, store) {
		const text = stringify(store.get(binding.docId, binding.path));
		const node = binding.start.nextSibling;
		if (node && node.nodeType === TEXT_NODE) {
			if (node.nodeValue !== text) node.nodeValue = text;
		} else {
			binding.start.parentNode.insertBefore(
				document.createTextNode(text),
				binding.start.nextSibling,
			);
		}
	}

	/** Patch one bound attribute to the store's value. */
	function patchAttribute(binding, store) {
		binding.element.setAttribute(
			binding.name,
			stringify(store.get(binding.docId, binding.path)),
		);
	}

	function patchAll(bindings, store) {
		bindings.texts.forEach((binding) => patchText(binding, store));
		bindings.attributes.forEach((binding) => patchAttribute(binding, store));
	}

	/** Render a value as display text (`null`/`undefined` are empty). */
	function stringify(value) {
		if (value == null) return "";
		if (typeof value === "object") return JSON.stringify(value);
		return String(value);
	}

	/** A DOM binding layer for an `EntityMut`, reflecting mutations into `element`. */
	function domBinding(element) {
		return {
			setAttribute(key, value) {
				element.setAttribute(key, stringify(value));
			},
			setValue(value) {
				if ("value" in element) element.value = stringify(value);
				else element.textContent = stringify(value);
			},
			setClass(name, on) {
				element.classList.toggle(name, on);
			},
		};
	}

	/** A node representation read from a live DOM element. */
	function reprFromDom(element) {
		const attributes = {};
		for (const attr of Array.from(element.attributes)) {
			attributes[attr.name] = attr.value;
		}
		return {
			tag: element.tagName.toLowerCase(),
			value: "value" in element ? element.value : element.textContent,
			attributes: attributes,
			classes: new Set(Array.from(element.classList)),
		};
	}

	/** Build an `EntityMut` over a live DOM element (the event target). */
	function entityForElement(element, store) {
		return new EntityMut(
			store,
			docIdFor(element),
			reprFromDom(element),
			domBinding(element),
		);
	}

	/** Wire every `bx:<event>` attribute to its verb. */
	function wireEvents(store) {
		for (const element of document.querySelectorAll("*")) {
			for (const attr of Array.from(element.attributes)) {
				if (!attr.name.startsWith("bx:")) continue;
				const event = attr.name.slice(3);
				const call = attr.value;
				element.addEventListener(event, () => {
					const parsed = parseVerbCall(call);
					const verb = verbs[parsed.verb];
					if (verb) verb(entityForElement(element, store), parsed.args);
				});
			}
		}
	}

	/** Hydrate and activate the page, returning the live store (the browser
	 *  entry point). */
	function bootstrap() {
		const store = createStore(readJsonScript("script[data-bx-blob]"));
		installVerbs(readJsonScript("script[data-bx-verbs]"));
		const bindings = collectBindings();
		// trust the blob, not the SSR text: patch once on load (a no-op when the
		// SSR value already matches, so there is no flash), then on every change.
		patchAll(bindings, store);
		store.subscribe(() => patchAll(bindings, store));
		wireEvents(store);
		return store;
	}

	// ---- exports / bootstrap ----------------------------------------------
	//
	// Expose the runtime api on `globalThis.beet` unconditionally, so deno/wasm
	// (no DOM) can read `createStore`/`EntityMut`/`installVerbs`/`verbs` after
	// evaluating this file, and the browser can introspect the live store/verbs.

	const api = {
		createStore,
		getPath,
		setPath,
		EntityMut,
		emptyRepr,
		verbs,
		installVerb,
		installVerbs,
		parseVerbCall,
		// the live store, filled in by `bootstrap` in the browser.
		store: null,
	};
	globalThis.beet = api;

	// Browser: boot once the DOM is ready, then publish the live store.
	if (typeof document !== "undefined") {
		const boot = () => {
			api.store = bootstrap();
		};
		if (document.readyState === "loading") {
			document.addEventListener("DOMContentLoaded", boot);
		} else {
			boot();
		}
	}
})();
