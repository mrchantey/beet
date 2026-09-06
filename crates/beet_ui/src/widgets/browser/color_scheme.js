// Seeds the document color scheme before first paint to avoid a flash, then
// follows the OS preference until the user makes an explicit choice.
// `setColorScheme("light" | "dark")` persists an explicit choice and applies it.
// `LIGHT_SCHEME` / `DARK_SCHEME` are injected by the `ColorSchemeScript` widget.
(function () {
	const KEY = "color-scheme";
	const root = document.documentElement;
	const media = globalThis.matchMedia("(prefers-color-scheme: dark)");

	function apply(scheme) {
		root.classList.remove(LIGHT_SCHEME, DARK_SCHEME);
		root.classList.add(scheme === "dark" ? DARK_SCHEME : LIGHT_SCHEME);
	}

	function stored() {
		return globalThis.localStorage?.getItem(KEY);
	}

	// an explicit `?color-scheme=light|dark` query param wins and is persisted,
	// mirroring the CLI `--color-scheme` flag on the terminal target.
	function queryOverride() {
		const value = new URLSearchParams(globalThis.location?.search).get(KEY);
		return value === "light" || value === "dark" ? value : null;
	}

	globalThis.setColorScheme = function (scheme) {
		globalThis.localStorage?.setItem(KEY, scheme);
		apply(scheme);
	};

	// follow the OS until an explicit choice is stored
	media.addEventListener("change", () => {
		if (!stored()) apply(media.matches ? "dark" : "light");
	});

	const override = queryOverride();
	if (override) globalThis.setColorScheme(override);
	else apply(stored() ?? (media.matches ? "dark" : "light"));
})();
