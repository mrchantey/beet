// Applies the visitor's colour preferences before first paint, and keeps
// every tab applying them as they change.
//
// The preferences are a scene body in localStorage, laid out as the
// `LocalStorageStore` named `preferences` lays out its files (the key
// `store:preferences:<path>`, the value base64), so a wasm runtime reads the
// same document as an ordinary store file:
// - `scene.json`: the scene body. Its `beet_ui::Theme` resource's `scheme`
//   pins the scheme, `Light` or `Dark`; absent, the page follows the OS.
// - `tones.css`: palette tones a palette editor derived from the theme keys it
//   stored, as css variable overrides, injected as written so applying them
//   needs no colour maths.
//
// A `?color-scheme=light|dark` query param pins the scheme for the page,
// mirroring the CLI `--color-scheme` flag on the terminal target, over any
// stored pin. A `[data-color-scheme-toggle]` button cycles the stored pin
// through auto, light and dark.
//
// `globalThis.beetPreferences` is the document's api for a page's scripts:
// `read()`/`write(body)` the scene body, `setScheme("light" | "dark" | null)`,
// `tones()`/`setTones(css | null)`, and `THEME`, the theme resource's key.
// Every change, a write in another tab included, dispatches
// `beet-preferences` on `document`.
// `LIGHT_SCHEME`, `DARK_SCHEME` and `THEME`, the theme's type path, are
// injected by the `ColorSchemeScript` widget.
(function () {
	const root = document.documentElement;
	const media = globalThis.matchMedia("(prefers-color-scheme: dark)");
	const STORE = "store:preferences:";
	const SCENE = STORE + "scene.json";
	const TONES = STORE + "tones.css";
	const CYCLE = { auto: "light", light: "dark", dark: "auto" };
	// line icons in the text colour, the auto one matching the toggle's markup
	const icon = (body) =>
		`<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;
	const ICONS = {
		auto: icon(
			'<circle cx="12" cy="12" r="9"/><path d="M12 3a9 9 0 0 0 0 18z" fill="currentColor"/>',
		),
		light: icon(
			'<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>',
		),
		dark: icon('<path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z"/>'),
	};

	// a store file as text, `null` when absent or unreadable (storage denied)
	function readFile(key) {
		try {
			const raw = localStorage.getItem(key);
			if (raw === null) return null;
			const bytes = Uint8Array.from(atob(raw), (char) => char.charCodeAt(0));
			return new TextDecoder().decode(bytes);
		} catch {
			return null;
		}
	}

	// write a store file, `null` removing it
	function writeFile(key, text) {
		try {
			if (text === null) return localStorage.removeItem(key);
			let binary = "";
			for (const byte of new TextEncoder().encode(text)) {
				binary += String.fromCharCode(byte);
			}
			localStorage.setItem(key, btoa(binary));
		} catch (err) {
			console.warn("preferences not stored:", err);
		}
	}

	function read() {
		try {
			const body = JSON.parse(readFile(SCENE));
			if (body && typeof body === "object") {
				body.resources ??= {};
				body.entities ??= {};
				return body;
			}
		} catch { /* an unreadable document reads as empty */ }
		return { $type: "org.beet.scene#body", resources: {}, entities: {} };
	}

	function write(body) {
		writeFile(SCENE, JSON.stringify(body, null, "\t"));
		changed();
	}

	// the page's own pin, `?color-scheme=light|dark`
	function queryScheme() {
		const value = new URLSearchParams(globalThis.location?.search).get(
			"color-scheme",
		);
		return value === "light" || value === "dark" ? value : null;
	}

	// the stored pin, `null` to follow the OS
	function storedScheme() {
		const scheme = read().resources[THEME]?.scheme;
		return scheme === "Light" ? "light" : scheme === "Dark" ? "dark" : null;
	}

	function setScheme(scheme) {
		const body = read();
		const theme = { ...body.resources[THEME] };
		if (scheme === "light" || scheme === "dark") {
			theme.scheme = scheme === "dark" ? "Dark" : "Light";
		} else delete theme.scheme;
		if (Object.keys(theme).length) body.resources[THEME] = theme;
		else delete body.resources[THEME];
		write(body);
	}

	function setTones(css) {
		writeFile(TONES, css || null);
		changed();
	}

	// the browser chrome wears a pinned scheme too: each `<Head>` tint names
	// its scheme, and while the page follows the OS its media query decides
	function tint(pinned) {
		const tints = document.querySelectorAll(
			'meta[name="theme-color"][data-scheme]',
		);
		for (const meta of tints) {
			meta.dataset.media ??= meta.media;
			meta.media = !pinned
				? meta.dataset.media
				: meta.dataset.scheme === pinned
				? "all"
				: "not all";
		}
	}

	function applyTones() {
		const css = readFile(TONES);
		let style = document.getElementById("beet-preferences-tones");
		if (!css) return style?.remove();
		if (!style) {
			style = document.createElement("style");
			style.id = "beet-preferences-tones";
			document.head.append(style);
		}
		style.textContent = css;
	}

	function labelToggles() {
		const stored = storedScheme() ?? "auto";
		for (const toggle of document.querySelectorAll("[data-color-scheme-toggle]")) {
			if (toggle.dataset.colorSchemeToggle !== stored) {
				toggle.innerHTML = ICONS[stored];
				toggle.dataset.colorSchemeToggle = stored;
			}
			toggle.title = `Colour scheme: ${stored}`;
		}
	}

	function apply() {
		const pinned = queryScheme() ?? storedScheme();
		const scheme = pinned ?? (media.matches ? "dark" : "light");
		root.classList.remove(LIGHT_SCHEME, DARK_SCHEME);
		root.classList.add(scheme === "dark" ? DARK_SCHEME : LIGHT_SCHEME);
		tint(pinned);
		applyTones();
		labelToggles();
	}

	function changed() {
		apply();
		document.dispatchEvent(new CustomEvent("beet-preferences"));
	}

	globalThis.beetPreferences = {
		THEME,
		read,
		write,
		setScheme,
		tones: () => readFile(TONES),
		setTones,
	};
	globalThis.setColorScheme = setScheme;

	media.addEventListener("change", apply);
	// another tab wrote the document
	globalThis.addEventListener("storage", (event) => {
		if (event.key === null || event.key.startsWith(STORE)) changed();
	});
	document.addEventListener("click", (event) => {
		if (!event.target.closest?.("[data-color-scheme-toggle]")) return;
		const next = CYCLE[storedScheme() ?? "auto"];
		setScheme(next === "auto" ? null : next);
	});
	document.addEventListener("DOMContentLoaded", labelToggles);
	apply();
})();
