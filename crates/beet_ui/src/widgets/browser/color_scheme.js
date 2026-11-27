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
// through auto, light and dark, showing whichever of its
// `[data-color-scheme-icon]` children names the pin. A favicon link with a
// `data-dark-href` twin follows the system's scheme alone, as the browser
// chrome it sits in does.
//
// `globalThis.beetPreferences` is the document's api for a page's scripts:
// `read()`/`write(body)` the scene body, `setScheme("light" | "dark" | null)`,
// `tones()`/`setTones(css | null)`, and `THEME`, the theme resource's key.
// Every change, a write in another tab included, dispatches
// `beet-preferences` on `document`, and a change of the applied scheme, the
// OS's included, `beet-color-scheme` with the `scheme` as its detail.
// `LIGHT_SCHEME`, `DARK_SCHEME` and `THEME`, the theme's type path, are
// injected by the `ColorSchemeScript` widget.
(function () {
	const root = document.documentElement;
	const media = globalThis.matchMedia("(prefers-color-scheme: dark)");
	const STORE = "store:preferences:";
	const SCENE = STORE + "scene.json";
	const TONES = STORE + "tones.css";
	const CYCLE = { auto: "light", light: "dark", dark: "auto" };

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

	// a favicon sits in the browser's chrome, which follows the system, so it
	// follows the system too, never a pinned scheme: an icon link with a
	// `data-dark-href` twin wears it while the system is dark, a new url the
	// browser fetches and draws afresh rather than its held raster
	function followSystemIcons() {
		for (const link of document.querySelectorAll("link[rel~=icon][data-dark-href]")) {
			link.dataset.lightHref ??= link.getAttribute("href");
			const href = media.matches ? link.dataset.darkHref : link.dataset.lightHref;
			if (link.getAttribute("href") !== href) link.setAttribute("href", href);
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

	// each toggle shows the icon of the stored pin, one of the three it holds
	function labelToggles() {
		const stored = storedScheme() ?? "auto";
		for (const toggle of document.querySelectorAll("[data-color-scheme-toggle]")) {
			toggle.dataset.colorSchemeToggle = stored;
			toggle.title = `Colour scheme: ${stored}`;
			for (const icon of toggle.querySelectorAll("[data-color-scheme-icon]")) {
				icon.hidden = icon.dataset.colorSchemeIcon !== stored;
			}
		}
	}

	// the scheme last applied, `null` before the first paint
	let applied = null;

	// restyle with transitions off for a frame, so a scheme change lands at
	// once everywhere rather than fading wherever a rule eases its colours
	function snap(restyle) {
		const style = document.createElement("style");
		style.textContent = "*,*::before,*::after{transition:none!important}";
		document.head.append(style);
		restyle();
		// a forced style flush, so the change resolves with transitions off
		getComputedStyle(root).color;
		requestAnimationFrame(() => style.remove());
	}

	function apply() {
		const pinned = queryScheme() ?? storedScheme();
		const scheme = pinned ?? (media.matches ? "dark" : "light");
		const restyle = () => {
			root.classList.remove(LIGHT_SCHEME, DARK_SCHEME);
			root.classList.add(scheme === "dark" ? DARK_SCHEME : LIGHT_SCHEME);
			applyTones();
		};
		if (applied === null) restyle();
		else snap(restyle);
		tint(pinned);
		labelToggles();
		if (applied !== null && applied !== scheme) {
			document.dispatchEvent(
				new CustomEvent("beet-color-scheme", { detail: { scheme } }),
			);
		}
		applied = scheme;
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

	media.addEventListener("change", () => {
		apply();
		followSystemIcons();
	});
	// another tab wrote the document
	globalThis.addEventListener("storage", (event) => {
		if (event.key === null || event.key.startsWith(STORE)) changed();
	});
	document.addEventListener("click", (event) => {
		if (!event.target.closest?.("[data-color-scheme-toggle]")) return;
		const next = CYCLE[storedScheme() ?? "auto"];
		setScheme(next === "auto" ? null : next);
	});
	document.addEventListener("DOMContentLoaded", () => {
		labelToggles();
		followSystemIcons();
	});
	apply();
})();
