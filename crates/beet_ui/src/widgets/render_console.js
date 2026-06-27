// Mirror console.log/info/warn/error into the on-page #beet-console panel, so a
// headless wasm program's logs (beet's info!/warn!/error! route to console.* on
// wasm) render on the page. Each call still reaches the real console.
(function () {
	const el = document.getElementById("beet-console");
	if (!el) return;
	// console method -> css level class
	const levels = {
		log: "log",
		info: "info",
		warn: "warn",
		error: "error",
		debug: "log",
	};
	const format = (args) =>
		Array.from(args)
			.map((arg) => {
				if (typeof arg === "string") return arg;
				if (arg instanceof Error) return arg.stack || arg.message;
				try {
					return JSON.stringify(arg);
				} catch (_) {
					return String(arg);
				}
			})
			.join(" ");
	Object.keys(levels).forEach((method) => {
		const original =
			typeof console[method] === "function"
				? console[method].bind(console)
				: function () {};
		console[method] = function () {
			original.apply(null, arguments);
			const line = document.createElement("div");
			line.className = "beet-console-line beet-console-" + levels[method];
			line.textContent = format(arguments);
			el.appendChild(line);
			el.scrollTop = el.scrollHeight;
		};
	});
})();
