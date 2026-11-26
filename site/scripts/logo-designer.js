// The logo designer on `docs/design/logo`: a control per `BeetLogo` parameter
// and per theme key, a live preview, and favourites kept in the visitor's
// colour preferences document as entities, each a `Name` and a `Logo`.
(() => {
	const { DEFAULTS: LOGO_DEFAULTS, PALETTES, PARTS, color, svg } = BeetLogo;
	const NAME = "bevy_ecs::name::Name";
	const LOGO = "beet_site::Logo";
	// favourites the standalone designer kept, adopted once
	const LEGACY_FAVOURITES = "beet-logo-favourites";

	const range = (min, max, step) => ({ type: "range", min, max, step });
	const select = options => ({ type: "select", options });
	/** a part's tone in the current mode, its output a swatch of the colour */
	const tone = part => ({
		...range(0, 100, 1),
		param: () => `${part}Tone${params.mode === "dark" ? "Dark" : "Light"}`,
		swatch: part,
	});
	// knobs by group, each bound to the param of its name unless it says otherwise
	const GROUPS = {
		Beets: {
			gap: range(0, 0.5, 0.005),
			chamfer: range(0, 0.6, 0.005),
			identical: { type: "checkbox" },
			reach: range(0, 0.8, 0.005),
			round: range(0, 0.2, 0.005),
		},
		Stem: {
			stemAt: range(0, 1, 0.01),
			stemDepth: range(-0.3, 0.6, 0.005),
			stemAngle: range(-90, 90, 1),
			stemSpread: range(0, 120, 1),
			stemInner: range(0, 0.6, 0.005),
			stemOuter: range(0, 1.2, 0.005),
			stemTip: range(0, 1.4, 0.005),
		},
		Colours: {
			mode: select(["light", "dark"]),
			leftPalette: select(PALETTES),
			leftTone: tone("left"),
			rightPalette: select(PALETTES),
			rightTone: tone("right"),
			bottomPalette: select(PALETTES),
			bottomTone: tone("bottom"),
			stemPalette: select(PALETTES),
			stemTone: tone("stem"),
			backgroundPalette: select(PALETTES),
			backgroundTone: tone("background"),
			backgroundOpacity: range(0, 1, 0.01),
			gradient: range(-50, 50, 1),
		},
		// the theme's key colours, changing one recomputes its palette
		Theme: Object.fromEntries(BeetTheme.PALETTES.map(key => [key, { type: "color" }])),
		Style: {
			stroke: range(0, 0.1, 0.002),
			stripes: range(0, 30, 1),
			stripeFill: range(0.05, 0.95, 0.01),
			padding: range(0, 0.6, 0.01),
		},
	};
	// gap values for the comparison strip
	const GAP_STEPS = [0, 0.05, 0.1, 0.15, 0.2, 0.3, 0.4];

	// the logo's defaults, worn in the page's scheme and the visitor's theme
	const DEFAULTS = {
		...LOGO_DEFAULTS,
		mode: document.documentElement.classList.contains("dark-scheme") ? "dark" : "light",
		...BeetTheme.keys(),
	};
	const params = { ...DEFAULTS, ...fromHash() };
	// the tones the current keys seed, the server's until a key changes
	let tones = BeetTheme.server().tones;
	// the favourite last loaded or saved, `null` for none
	let current = null;
	// renders in flight, so only the latest one draws
	let rendering = 0;

	const element = id => document.getElementById(`logo-${id}`);
	const [controls, preview, gaps, changed, saveButton, favouriteList, note] = [
		"controls",
		"preview",
		"gaps",
		"changed",
		"save",
		"favourites",
		"note",
	].map(element);
	const knobs = [];

	// build a fieldset per group, a row per knob
	for (const [group, specs] of Object.entries(GROUPS)) {
		const fieldset = document.createElement("fieldset");
		fieldset.append(Object.assign(document.createElement("legend"), { textContent: group }));
		for (const [name, spec] of Object.entries(specs)) {
			const input = document.createElement(spec.type === "select" ? "select" : "input");
			if (spec.type === "select") {
				input.append(...spec.options.map(option => new Option(option, option)));
			} else {
				input.type = spec.type;
				Object.assign(input, { min: spec.min ?? "", max: spec.max ?? "", step: spec.step ?? "" });
			}
			input.id = `logo-knob-${name}`;
			const knob = { name, spec, input, output: document.createElement("output"), param: spec.param ?? (() => name) };
			input.oninput = () => {
				params[knob.param()] = { range: Number(input.value), checkbox: input.checked }[input.type] ?? input.value;
				render();
			};
			knobs.push(knob);
			fieldset.append(
				Object.assign(document.createElement("label"), { htmlFor: input.id, textContent: name }),
				input,
				knob.output,
			);
		}
		controls.append(fieldset);
	}

	/** recompute the tones for the current keys, then draw */
	async function render() {
		const ticket = ++rendering;
		const keys = Object.fromEntries(BeetTheme.PALETTES.map(key => [key, params[key]]));
		try {
			const derived = await BeetTheme.tones(keys);
			if (ticket !== rendering) return;
			tones = derived;
			note.textContent = "";
		} catch (err) {
			if (ticket !== rendering) return;
			note.textContent = `A changed key needs the colour library, which failed to load: ${err.message}`;
		}
		draw();
	}

	function draw() {
		for (const { spec, input, output, param } of knobs) {
			const value = params[param()];
			input.value = value;
			input.checked = value === true;
			output.value = input.type === "range" ? value : "";
			// a tone's swatch, titled with the colour it resolves to
			if (spec.swatch) {
				output.style.background = output.title = color(params, tones, spec.swatch, params.mode);
				output.style.borderRadius = "0.2rem";
				output.style.color = value < 50 ? "#fff" : "#000";
			}
		}
		// identical beets reach as far as they cut
		const reach = knobs.find(knob => knob.name === "reach");
		reach.input.disabled = params.identical;
		if (params.identical) reach.input.value = reach.output.value = params.chamfer;
		// a frame that holds still, so a wider gap reads as smaller beets
		preview.innerHTML = svg(params, tones, { tight: false });
		gaps.replaceChildren(
			...GAP_STEPS.map(gap => {
				const button = document.createElement("button");
				button.type = "button";
				button.append(thumbnail({ ...params, gap }, false), `gap ${gap}`);
				button.onclick = () => {
					params.gap = gap;
					render();
				};
				return button;
			}),
		);
		const diff = Object.fromEntries(Object.entries(params).filter(([key, value]) => value !== DEFAULTS[key]));
		// leave the text alone while it is being edited, so the cursor stays put
		if (document.activeElement !== changed) changed.value = JSON.stringify(diff, null, 1);
		history.replaceState(null, "", `#${new URLSearchParams(diff)}`);
		const dirty = current !== null && JSON.stringify(favourites()[current]) !== JSON.stringify(params);
		saveButton.disabled = current === null;
		saveButton.textContent = current === null ? "Save" : `Save "${current}"${dirty ? " *" : ""}`;
	}

	/** the logo as an image rather than inline, so each svg's ids stay its own */
	function thumbnail(values, tight = true) {
		const image = new Image();
		image.src = `data:image/svg+xml,${encodeURIComponent(svg(values, tones, { tight }))}`;
		return image;
	}

	/** favourites by name, in the order they were saved */
	function favourites() {
		return Object.fromEntries(
			Object.values(beetPreferences.read().entities)
				.filter(entity => LOGO in entity)
				.map(entity => [entity[NAME], entity[LOGO]]),
		);
	}

	/** store `saved` as the favourites, leaving every other entity be */
	function storeFavourites(saved) {
		const body = beetPreferences.read();
		const others = Object.values(body.entities).filter(entity => !(LOGO in entity));
		const logos = Object.entries(saved).map(([name, values]) => ({ [NAME]: name, [LOGO]: values }));
		body.entities = Object.fromEntries([...others, ...logos].map((entity, i) => [String(i), entity]));
		beetPreferences.write(body);
	}

	function renderFavourites() {
		const saved = favourites();
		favouriteList.replaceChildren(
			...Object.entries(saved).map(([name, values]) => {
				values = { ...DEFAULTS, ...known(values) };
				const item = document.createElement("li");
				item.classList.toggle("current", name === current);
				const load = document.createElement("button");
				load.type = "button";
				load.className = "btn btn-text";
				load.append(thumbnail(values), name);
				load.onclick = () => {
					// knobs added since the save take their defaults, removed ones drop
					Object.assign(params, values);
					current = name;
					renderFavourites();
					render();
				};
				const remove = document.createElement("button");
				remove.type = "button";
				remove.className = "btn btn-text";
				remove.textContent = "×";
				remove.title = `Delete "${name}"`;
				remove.onclick = () => {
					if (!confirm(`Delete "${name}"?`)) return;
					delete saved[name];
					if (current === name) current = null;
					storeFavourites(saved);
				};
				item.append(load, remove);
				return item;
			}),
		);
	}

	function save(name) {
		current = name;
		storeFavourites({ ...favourites(), [name]: { ...params } });
	}

	/** only the entries of `values` that are still knobs */
	function known(values) {
		return Object.fromEntries(Object.entries(values).filter(([key]) => key in DEFAULTS));
	}

	/** params changed from the defaults, as carried in the url hash */
	function fromHash() {
		return Object.fromEntries(
			[...new URLSearchParams(location.hash.slice(1))]
				.filter(([key]) => key in DEFAULTS)
				.map(([key, value]) => [key, { number: Number(value), boolean: value === "true" }[typeof DEFAULTS[key]] ?? value]),
		);
	}

	// an edit that parses to an object loads, anything else is flagged and ignored
	changed.oninput = () => {
		let edited;
		try {
			edited = JSON.parse(changed.value || "{}");
		} catch { /* flagged below */ }
		const valid = edited !== null && typeof edited === "object" && !Array.isArray(edited);
		changed.classList.toggle("invalid", !valid);
		if (!valid) return;
		Object.assign(params, DEFAULTS, known(edited));
		render();
	};
	// tidy the text once editing ends
	changed.onblur = () => {
		changed.classList.remove("invalid");
		draw();
	};
	element("swap").onclick = () => {
		const swapped = { primary: "secondary", secondary: "primary" };
		for (const part of PARTS) {
			const key = `${part}Palette`;
			params[key] = swapped[params[key]] ?? params[key];
		}
		render();
	};
	element("reset").onclick = () => {
		Object.assign(params, DEFAULTS);
		current = null;
		renderFavourites();
		render();
	};
	saveButton.onclick = () => current !== null && save(current);
	element("save-as").onclick = () => {
		const name = prompt("Save as", current ?? "")?.trim();
		if (!name) return;
		if (name !== current && name in favourites() && !confirm(`Replace "${name}"?`)) return;
		save(name);
	};
	element("download").onclick = () => {
		const link = document.createElement("a");
		link.href = URL.createObjectURL(new Blob([svg(params, tones)], { type: "image/svg+xml" }));
		link.download = "beet-logo.svg";
		link.click();
		URL.revokeObjectURL(link.href);
	};
	// a favourite saved here or in another tab
	document.addEventListener("beet-preferences", () => {
		renderFavourites();
		draw();
	});

	// adopt the standalone designer's favourites, then forget them
	try {
		const legacy = JSON.parse(localStorage.getItem(LEGACY_FAVOURITES) ?? "null");
		if (legacy) {
			const adopted = Object.fromEntries(
				Object.entries(legacy).map(([name, values]) => [name, known(values)]),
			);
			storeFavourites({ ...adopted, ...favourites() });
			localStorage.removeItem(LEGACY_FAVOURITES);
		}
	} catch { /* nothing to adopt */ }
	renderFavourites();
	render();
})();
