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

	globalThis.setColorScheme = function (scheme) {
		globalThis.localStorage?.setItem(KEY, scheme);
		apply(scheme);
	};

	// follow the OS until an explicit choice is stored
	media.addEventListener("change", () => {
		if (!stored()) apply(media.matches ? "dark" : "light");
	});

	apply(stored() ?? (media.matches ? "dark" : "light"));
})();
