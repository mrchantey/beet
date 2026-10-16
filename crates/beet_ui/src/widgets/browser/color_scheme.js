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

	// an explicit `?color-scheme=light|dark` query param pins the scheme,
	// mirroring the CLI `--color-scheme` flag on the terminal target.
	function queryOverride() {
		const value = new URLSearchParams(globalThis.location?.search).get(
			"color-scheme",
		);
		return value === "light" || value === "dark" ? value : null;
	}

	let pinned = queryOverride();

	globalThis.setColorScheme = function (scheme) {
		pinned = scheme;
		apply(scheme);
	};

	// follow the OS until the page pins a scheme
	media.addEventListener("change", () => {
		if (!pinned) apply(media.matches ? "dark" : "light");
	});

	apply(pinned ?? (media.matches ? "dark" : "light"));
})();
