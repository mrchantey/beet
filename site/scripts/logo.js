// The beet logo, generated from a handful of parameters.
//
// Three beets cut from one arrangement of nine lines, three in each direction
// of a triangular grid: horizontal (`y`), rising (`n`, ie `/`) and falling
// (`p`, ie `\`). Every beet edge lies on one of these lines, so neighbouring
// beets share their geometry by construction, and at `gap = 0` they tessellate
// perfectly into one large triangle, whose centre is the centre of the image.
// Each beet is a triangle with one sharp corner and its other two cut to the
// same depth, so it mirrors about the axis through that corner for any
// parameters. A stem grows from the right beet's outer edge.
//
// Colours are Material tones of the theme's palettes: every coloured part
// picks a palette and a tone per mode, read from the tones the site's
// `<Theme>` derives, so the logo holds no colour of its own.
//
// Lengths are in grid units, the distance between neighbouring horizontal lines.
//
// A classic script, no `import` or `export`, defining the one global
// `BeetLogo`: the logo designer page loads it with `<script src>`, and the
// entry's `<ScriptDerivation>` runs it to write the files `files` answers into
// `assets/logo`. The hand drawn original is `assets/logo/logo-hand-drawn.jpg`.
const BeetLogo = (() => {
	/** Every knob the logo has. */
	const DEFAULTS = {
		// the channel between neighbouring beets, 0 tessellates perfectly
		gap: 0.15,
		// the depth of the left and right beets' cuts, the right beet reaching this
		// far past the grid so its neighbours' lines cut its corners
		chamfer: 0.26,
		// ties `reach` to `chamfer`, so the three beets are identical
		identical: true,
		// how far the left and bottom beets reach left past the grid, which is
		// also the depth of the bottom beet's cuts, one of them filled by the left
		// beet's tip. The hand drawn original reaches further than it cuts.
		reach: 0.39,
		// corner rounding radius
		round: 0,
		// where the stem grows from, 0 to 1 down the right beet's outer edge
		stemAt: 0.55,
		// how far inside the beet's edge the stem's two sides converge
		stemDepth: 0,
		// stem rotation from square to the edge in degrees, positive turning upward
		stemAngle: 30,
		// angle between the stem's two sides, in degrees
		stemSpread: 50,
		// distance from the convergence point to the stem's base
		stemInner: 0.12,
		// distance from the convergence point to the stem's shoulders
		stemOuter: 0.475,
		// distance from the convergence point to the stem's tip
		stemTip: 0.555,
		// outline width, 0 for solid fills
		stroke: 0,
		// horizontal stripes across the whole logo, 0 for none
		stripes: 0,
		// the filled fraction of each stripe
		stripeFill: 0.6,
		// `light` or `dark`, choosing which of each part's two tones it wears, or
		// `auto`, wearing the light ones and the dark ones where the viewer
		// prefers a dark scheme
		mode: "light",
		// each coloured part's palette and its tone in each mode: the beets three
		// tones of primary, light to dark from right to left to bottom, the stem
		// secondary and the background the site's `Background` role
		leftPalette: "primary",
		leftToneLight: 43,
		leftToneDark: 56,
		rightPalette: "primary",
		rightToneLight: 65,
		rightToneDark: 78,
		bottomPalette: "primary",
		bottomToneLight: 24,
		bottomToneDark: 27,
		stemPalette: "secondary",
		stemToneLight: 52,
		stemToneDark: 62,
		backgroundPalette: "neutral",
		backgroundToneLight: 94,
		backgroundToneDark: 8,
		// background opacity, 0 for transparent
		backgroundOpacity: 1,
		// tones each beet shifts by from its top to its bottom, 0 for flat
		gradient: 0,
		// space around the logo
		padding: 0.08,
	};

	/**
	 * The palettes a part can take its colour from. `neutral` is split per mode
	 * as the site's is, the light scheme's cream and the dark scheme's ink.
	 */
	const PALETTES = ["primary", "secondary", "tertiary", "neutral", "neutral_variant", "error"];

	/** The coloured parts, each with a palette and a tone per mode in [`DEFAULTS`]. */
	const PARTS = ["left", "right", "bottom", "stem", "background"];

	/**
	 * The hex colour `part` wears in `mode`, shifted `shift` tones, from `tones`,
	 * every tone of each palette (`tones.primary[40]`) as a `<Theme>` derives
	 * them.
	 */
	function color(params, tones, part, mode, shift = 0) {
		const p = { ...DEFAULTS, ...params };
		const dark = mode === "dark";
		const name = p[`${part}Palette`];
		const palette = name === "neutral" ? (dark ? "neutral_dark" : "neutral_light") : name;
		const tone = p[`${part}Tone${dark ? "Dark" : "Light"}`] + shift;
		return tones[palette][Math.round(Math.min(100, Math.max(0, tone)))];
	}

	/**
	 * The line arrangement: each line is an axis from [`AXES`] and the value that
	 * axis holds along it. Moving a line moves every beet edge lying on it.
	 */
	function lines(params) {
		const { chamfer } = params;
		const reach = params.identical ? chamfer : params.reach;
		return {
			// horizontal
			top: ["y", -1],
			middle: ["y", 0],
			bottom: ["y", 1],
			// rising
			riseLeft: ["n", -1 - reach + chamfer],
			riseMiddle: ["n", 0],
			riseRight: ["n", 1],
			// falling
			fallLeft: ["p", -reach],
			fallMiddle: ["p", 1 - chamfer],
			fallRight: ["p", 1 + chamfer],
		};
	}

	/**
	 * Each beet is the region on one side of each of its bounding lines:
	 * `[line, side]` where side `1` keeps the larger values and `-1` the smaller.
	 */
	const BEETS = [
		{
			id: "left",
			bounds: [
				["top", 1],
				["fallLeft", 1],
				["riseMiddle", -1],
				// cut corners
				["riseLeft", 1],
				["fallMiddle", -1],
			],
		},
		{
			id: "right",
			bounds: [
				["middle", -1],
				["riseMiddle", 1],
				["fallRight", -1],
				// cut corners
				["top", 1],
				["riseRight", -1],
			],
		},
		{
			id: "bottom",
			bounds: [
				["middle", 1],
				["fallLeft", 1],
				["riseRight", -1],
				// the corner the left beet's tip fills
				["riseMiddle", 1],
				// cut corner
				["bottom", -1],
			],
		},
	];

	/**
	 * Each axis as the unit normal it measures along, so offsetting a line by `d`
	 * moves it a perpendicular distance `d`. `n - p = y` at every point.
	 */
	const AXES = {
		y: [0, 1],
		p: [Math.sqrt(3) / 2, -1 / 2],
		n: [Math.sqrt(3) / 2, 1 / 2],
	};

	/** Polygons for every beet and the stem. */
	function geometry(params = {}) {
		const p = { ...DEFAULTS, ...params };
		const named = lines(p);
		const beets = BEETS.map(beet => ({
			id: beet.id,
			points: beet.bounds.reduce(
				(polygon, [name, side]) => {
					const [axis, at] = named[name];
					// insetting every edge by half the gap opens the channels
					return clip(polygon, AXES[axis], at + (side * p.gap) / 2, side);
				},
				// a polygon comfortably larger than any beet
				[[-9, -9], [9, -9], [9, 9], [-9, 9]],
			),
		}));
		return { beets, stem: stem(p, named) };
	}

	/**
	 * The centre of the large triangle the beets tessellate into at `gap = 0`,
	 * bounded by the top, left falling and right rising lines: the point a third
	 * of its height from each side.
	 */
	function centre(params = {}) {
		const named = lines({ ...DEFAULTS, ...params });
		const [top, left, right] = [named.top[1], named.fallLeft[1], named.riseRight[1]];
		const third = (right - left - top) / 3;
		return solve(AXES.p, left + third, AXES.n, right - third);
	}

	/**
	 * The logo as an svg document, coloured from `tones` (see [`color`]). In
	 * `auto` mode every paint is its light colour, restyled to its dark one by an
	 * embedded `prefers-color-scheme` rule, so one file follows the viewer, ie a
	 * favicon.
	 */
	function svg(params = {}, tones, { tight = true } = {}) {
		const p = { ...DEFAULTS, ...params };
		const { beets, stem } = geometry(p);
		// a loose frame holds still while the gap changes, a tight one crops to the art
		const framed = tight ? [...beets, stem] : [...geometry({ ...p, gap: 0 }).beets, stem];
		const [minX, minY, size] = frame(centre(p), framed.flatMap(shape => shape.points), p.padding + p.stroke);

		// each paint a slot with its colour per mode, its attribute the light one
		// where the mode is auto
		const auto = p.mode === "auto";
		const darkRules = [];
		const paint = (slot, property, part, shift = 0) => {
			const light = color(p, tones, part, auto ? "light" : p.mode, shift);
			if (!auto) return `${property}="${light}"`;
			darkRules.push(`.beet-${slot}{${property}:${color(p, tones, part, "dark", shift)}}`);
			return `${property}="${light}" class="beet-${slot}"`;
		};
		const fill = (slot, part) =>
			p.stroke > 0
				? `fill="none" ${paint(slot, "stroke", part)} stroke-width="${round(p.stroke)}" stroke-linejoin="round"`
				: paint(slot, "fill", part);
		const clipped = p.stripes > 0 ? ` clip-path="url(#beet-stripes)"` : "";
		// a beet with a gradient fills from its own, top to bottom
		const gradients = p.gradient === 0
			? ""
			: beets
					.map(
						beet => `
		<linearGradient id="beet-${beet.id}-gradient" x1="0" y1="0" x2="0" y2="1">
			<stop offset="0" ${paint(`${beet.id}-from`, "stop-color", beet.id)}/>
			<stop offset="1" ${paint(`${beet.id}-to`, "stop-color", beet.id, p.gradient)}/>
		</linearGradient>`,
					)
					.join("");
		const beetPaths = beets
			.map(beet => {
				const painted = p.gradient === 0
					? fill(beet.id, beet.id)
					: p.stroke > 0
					? `fill="none" stroke="url(#beet-${beet.id}-gradient)" stroke-width="${round(p.stroke)}" stroke-linejoin="round"`
					: `fill="url(#beet-${beet.id}-gradient)"`;
				return `\t\t<path id="beet-${beet.id}" d="${pathData(beet.points, p.round)}" ${painted}/>`;
			})
			.join("\n");
		const square = [["x", minX], ["y", minY], ["width", size], ["height", size]]
			.map(([key, value]) => `${key}="${round(value)}"`)
			.join(" ");
		const background = p.backgroundOpacity > 0
			? `
	<rect id="background" ${square} ${paint("background", "fill", "background")} fill-opacity="${p.backgroundOpacity}"/>`
			: "";
		const stemPath = `<path id="stem" d="${pathData(stem.points, p.round)}" ${fill("stem", "stem")}${clipped}/>`;
		const style = darkRules.length
			? `
	<style>@media (prefers-color-scheme: dark){${darkRules.join("")}}</style>`
			: "";

		return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${[minX, minY, size, size].map(round).join(" ")}">${style}
	<defs>${gradients}${stripeMask(p, minX, minY, size, size)}
	</defs>${background}
	<g id="beets"${clipped}>
${beetPaths}
	</g>
	${stemPath}
</svg>
`;
	}

	/**
	 * The files the site serves, each transparent and cropped to the art:
	 * `logo.svg` following the viewer's scheme (the favicon), and
	 * `logo-light.svg` and `logo-dark.svg` for a page to show by its own scheme.
	 * The entry `<ScriptDerivation>` calls, handed the theme's `tones`.
	 */
	function files({ tones }, params = {}) {
		const published = { ...params, backgroundOpacity: 0 };
		return {
			"logo.svg": svg({ ...published, mode: "auto" }, tones),
			"logo-light.svg": svg({ ...published, mode: "light" }, tones),
			"logo-dark.svg": svg({ ...published, mode: "dark" }, tones),
		};
	}

	// ---- the stem ----

	/**
	 * A wedge between two rays from a point on (or inside) the right beet's outer
	 * edge, cut square at `stemInner` and pointed from `stemOuter` to `stemTip`.
	 * The point follows the edge as the gap moves it.
	 */
	function stem(p, named) {
		// the outer edge runs between the top line and the rising right line
		const edge = AXES.p;
		const edgeAt = named.fallRight[1];
		const start = solve(edge, edgeAt, AXES.y, named.top[1]);
		const end = solve(edge, edgeAt, AXES.n, named.riseRight[1]);
		const origin = add(lerp(start, end, p.stemAt), mul(edge, -(p.gap / 2 + p.stemDepth)));
		// svg's y points down, so turning upward is turning counterclockwise
		const heading = rotate(edge, -p.stemAngle);
		const ray = (degrees, radius) => add(origin, mul(rotate(heading, degrees), radius));
		const half = p.stemSpread / 2;
		return {
			id: "stem",
			points: [
				ray(-half, p.stemInner),
				ray(-half, p.stemOuter),
				ray(0, p.stemTip),
				ray(half, p.stemOuter),
				ray(half, p.stemInner),
			],
		};
	}

	// ---- svg helpers ----

	function stripeMask(p, x, y, width, height) {
		if (p.stripes <= 0) return "";
		const band = height / p.stripes;
		const rects = Array.from(
			{ length: p.stripes },
			(_, i) =>
				`\t\t\t<rect x="${round(x)}" y="${round(y + i * band)}" width="${round(width)}" height="${round(band * p.stripeFill)}"/>`,
		).join("\n");
		return `
		<clipPath id="beet-stripes">
${rects}
		</clipPath>`;
	}

	/** A closed path, each corner rounded by up to `radius` where it is nonzero. */
	function pathData(points, radius) {
		const corner = (point, i) => {
			const prev = points[(i + points.length - 1) % points.length];
			const next = points[(i + 1) % points.length];
			// never round past half an edge
			const cut = Math.min(radius, dist(point, prev) / 2, dist(point, next) / 2);
			return cut > 0
				? `${fmt(toward(point, prev, cut))} Q${fmt(point)} ${fmt(toward(point, next, cut))}`
				: fmt(point);
		};
		return `M${points.map(corner).join(" L")} Z`;
	}

	/**
	 * `[x, y, size]` of the smallest square centred on `centre` holding every
	 * point, padded.
	 */
	function frame(centre, points, pad) {
		const half = Math.max(...points.flatMap(point => sub(point, centre).map(Math.abs))) + pad;
		return [centre[0] - half, centre[1] - half, half * 2];
	}

	// ---- geometry helpers ----

	/** Keeps the part of a convex polygon where `side * (normal . point - at) >= 0`. */
	function clip(polygon, normal, at, side) {
		const inside = point => side * (dot(normal, point) - at) >= 0;
		return polygon.flatMap((point, i) => {
			const next = polygon[(i + 1) % polygon.length];
			const kept = inside(point) ? [point] : [];
			if (inside(point) === inside(next)) return kept;
			// the edge crosses the line, keep the crossing too
			const t = (at - dot(normal, point)) / (dot(normal, next) - dot(normal, point));
			return [...kept, lerp(point, next, t)];
		});
	}

	/** The point on both lines `a . point = atA` and `b . point = atB`. */
	function solve(a, atA, b, atB) {
		const det = cross(a, b);
		return [(atA * b[1] - atB * a[1]) / det, (a[0] * atB - b[0] * atA) / det];
	}

	const add = (a, b) => [a[0] + b[0], a[1] + b[1]];
	const sub = (a, b) => [a[0] - b[0], a[1] - b[1]];
	const mul = (a, scalar) => [a[0] * scalar, a[1] * scalar];
	const dot = (a, b) => a[0] * b[0] + a[1] * b[1];
	const cross = (a, b) => a[0] * b[1] - a[1] * b[0];
	const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);
	const lerp = (a, b, t) => add(a, mul(sub(b, a), t));
	/** `distance` along the segment from `from` to `to` */
	const toward = (from, to, distance) => lerp(from, to, distance / dist(from, to));
	function rotate(vector, degrees) {
		const angle = (degrees * Math.PI) / 180;
		const [cos, sin] = [Math.cos(angle), Math.sin(angle)];
		return [vector[0] * cos - vector[1] * sin, vector[0] * sin + vector[1] * cos];
	}
	const round = value => Math.round(value * 1000) / 1000;
	const fmt = point => `${round(point[0])},${round(point[1])}`;

	return { DEFAULTS, PALETTES, PARTS, color, geometry, centre, svg, files };
})();
