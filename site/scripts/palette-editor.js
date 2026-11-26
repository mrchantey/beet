// The palette editor on `docs/design/color_schemes`: a colour input per theme
// key, each change stored as the visitor's theme (`BeetTheme.store`), which
// recolours every page in this browser.
(() => {
	const editor = document.getElementById("palette-editor");
	const note = document.getElementById("palette-note");
	const inputs = Object.fromEntries(
		BeetTheme.PALETTES.map(palette => {
			const input = Object.assign(document.createElement("input"), { type: "color", id: `palette-${palette}` });
			const label = Object.assign(document.createElement("label"), { htmlFor: input.id, textContent: palette });
			label.style.cssText = "display: flex; align-items: center; gap: 0.4rem";
			label.prepend(input);
			editor.append(label);
			return [palette, input];
		}),
	);
	// one store at a time, the latest keys winning, so a dragged picker keeps up
	let pending = null;
	let storing = false;

	async function store(keys) {
		pending = keys;
		if (storing) return;
		storing = true;
		while (pending) {
			const next = pending;
			pending = null;
			try {
				await BeetTheme.store(next);
				note.textContent = "";
			} catch (err) {
				note.textContent = `A changed key needs the colour library, which failed to load: ${err.message}`;
			}
		}
		storing = false;
	}

	function load() {
		const keys = BeetTheme.keys();
		for (const [palette, input] of Object.entries(inputs)) input.value = keys[palette];
	}

	editor.oninput = () =>
		store(Object.fromEntries(Object.entries(inputs).map(([palette, input]) => [palette, input.value])));
	document.getElementById("palette-reset").onclick = () => {
		const { keys } = BeetTheme.server();
		store(Object.fromEntries(BeetTheme.PALETTES.map(palette => [palette, keys[palette].toLowerCase()])));
	};
	// a change from another tab
	document.addEventListener("beet-preferences", load);
	load();
})();
