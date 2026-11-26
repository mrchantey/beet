// The site theme as a page's scripts edit it: the palettes the server derived
// (`<ThemeData/>`), overridden by the visitor's stored keys
// (`beetPreferences`, the colour preferences `ColorSchemeScript` applies).
//
// A key the visitor has not changed reads the server's tones, so only a page
// editing a key loads the colour maths: the Material colour utilities, the
// library beet's own `Theme` mirrors, from `assets/vendor`. That file is a
// site asset rather than source, fetched once from
// https://esm.sh/@material/material-color-utilities@0.3.0/es2022/material-color-utilities.bundle.mjs
//
// A classic script defining the one global `BeetTheme`.
const BeetTheme = (() => {
	/** The palettes, each named by the `Theme` field keying it. */
	const PALETTES = [
		"primary",
		"secondary",
		"tertiary",
		"neutral_light",
		"neutral_dark",
		"neutral_variant",
		"error",
	];
	const LIBRARY = "/assets/vendor/material-color-utilities.js";
	let library = null;

	/** The server's theme: its `keys`, `tones` and tone `variables`. */
	function server() {
		const element = document.getElementById("beet-theme");
		if (!element) throw new Error("the page carries no `<ThemeData/>`");
		return JSON.parse(element.textContent);
	}

	/** The stored theme resource, `{}` when the visitor stored none. */
	function stored() {
		return globalThis.beetPreferences?.read().resources[beetPreferences.THEME] ?? {};
	}

	/** Each palette's key colour, the visitor's where they stored one. */
	function keys() {
		const theme = server();
		const own = stored();
		return Object.fromEntries(
			PALETTES.map(palette => [palette, (own[palette] ?? theme.keys[palette]).toLowerCase()]),
		);
	}

	/**
	 * Every tone of each palette `keys` seeds, the server's where a key is
	 * the server's own, so the colour maths loads only for a changed key.
	 */
	async function tones(keys) {
		const theme = server();
		const changed = PALETTES.filter(palette => keys[palette] !== theme.keys[palette].toLowerCase());
		if (!changed.length) return theme.tones;
		library ??= import(LIBRARY);
		const { argbFromHex, Hct, hexFromArgb, TonalPalette } = await library;
		const computed = { ...theme.tones };
		for (const palette of changed) {
			// a key holds its own hue and chroma, as beet's `Theme` derives it
			const ramp = TonalPalette.fromHct(Hct.fromInt(argbFromHex(keys[palette])));
			computed[palette] = Array.from({ length: 101 }, (_, tone) => hexFromArgb(ramp.tone(tone)));
		}
		return computed;
	}

	/**
	 * The css overriding every tone variable the page's sheet declares with
	 * `tones`, the `tones.css` `ColorSchemeScript` applies before paint.
	 * Doubled `:root` so it outranks the sheet's own declarations.
	 */
	function css(tones) {
		const { variables } = server();
		const declarations = Object.entries(variables).flatMap(([palette, ramp]) =>
			Object.entries(ramp).map(([tone, variable]) => `${variable}:${tones[palette][tone]};`)
		);
		return `:root:root{${declarations.join("")}}`;
	}

	/**
	 * Store `keys` as the visitor's theme, keeping only those that differ from
	 * the server's, and the tones they derive.
	 */
	async function store(keys) {
		const theme = server();
		const body = beetPreferences.read();
		const resource = { ...body.resources[beetPreferences.THEME] };
		for (const palette of PALETTES) {
			if (keys[palette] === theme.keys[palette].toLowerCase()) delete resource[palette];
			else resource[palette] = keys[palette];
		}
		const own = PALETTES.some(palette => palette in resource);
		if (Object.keys(resource).length) body.resources[beetPreferences.THEME] = resource;
		else delete body.resources[beetPreferences.THEME];
		// the tones first, so the write announcing the change applies them
		const derived = own ? css(await tones(keys)) : null;
		beetPreferences.setTones(derived);
		beetPreferences.write(body);
	}

	return { PALETTES, server, keys, tones, css, store };
})();
