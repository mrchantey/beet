// Seeds the document color scheme before first paint to avoid a flash, then
// follows the OS preference unless the page pins a scheme. Nothing is stored,
// so a pin lasts as long as the page.
// `setColorScheme("light" | "dark")` pins a scheme and applies it.
// `LIGHT_SCHEME` / `DARK_SCHEME` are injected by the `ColorSchemeScript` widget.
(function () {
	const root = document.documentElement;
	const media = globalThis.matchMedia("(prefers-color-scheme: dark)");

	function apply(scheme) {
		root.classList.remove(LIGHT_SCHEME, DARK_SCHEME);
		root.classList.add(scheme === "dark" ? DARK_SCHEME : LIGHT_SCHEME);
	}

	// the browser chrome wears the pinned scheme too: each `<Head>` tint names
	// its scheme, and while the page follows the OS its media query decides
	function tint(scheme) {
		const tints = document.querySelectorAll(
			'meta[name="theme-color"][data-scheme]',
		);
		for (const meta of tints) {
			meta.media = meta.dataset.scheme === scheme ? "all" : "not all";
		}
	}

	function pin(scheme) {
		pinned = scheme;
		apply(scheme);
		tint(scheme);
	}

	// an explicit `?color-scheme=light|dark` query param pins the scheme,
	// mirroring the CLI `--color-scheme` flag on the terminal target.
	function queryOverride() {
		const value = new URLSearchParams(globalThis.location?.search).get(
			"color-scheme",
		);
		return value === "light" || value === "dark" ? value : null;
	}

	let pinned = null;
	globalThis.setColorScheme = pin;

	// follow the OS until the page pins a scheme
	media.addEventListener("change", () => {
		if (!pinned) apply(media.matches ? "dark" : "light");
	});

	const override = queryOverride();
	if (override) pin(override);
	else apply(media.matches ? "dark" : "light");
})();
