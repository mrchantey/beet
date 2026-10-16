/*!
* https://codesandbox.io/p/sandbox/4mrnhq
* Minimal theme switcher
*
* Pico.css - https://picocss.com
* Copyright 2019-2024 - Licensed under MIT
*/

const themeSwitcher = {
	// Config
	_scheme: "auto",
	menuTarget: "details.dropdown",
	buttonsTarget: "a[data-theme-switcher]",
	buttonAttribute: "data-theme-switcher",
	rootAttribute: "data-theme",

	// Init, following the OS until a switcher pins a scheme; nothing is stored
	init() {
		this.scheme = this._scheme;
		this.initSwitchers();
	},

	// Preferred color scheme
	get preferredColorScheme() {
		return globalThis.matchMedia("(prefers-color-scheme: dark)").matches
			? "dark"
			: "light";
	},

	// Init switchers
	initSwitchers() {
		const buttons = document.querySelectorAll(this.buttonsTarget);
		buttons.forEach((button) => {
			button.addEventListener(
				"click",
				(event) => {
					event.preventDefault();
					// Set scheme
					this.scheme = button.getAttribute(this.buttonAttribute);
					// Close dropdown
					document
						.querySelector(this.menuTarget)
						?.removeAttribute("open");
				},
				false,
			);
		});
	},

	// Set scheme
	set scheme(scheme) {
		if (scheme == "auto") {
			this._scheme = this.preferredColorScheme;
		} else if (scheme == "dark" || scheme == "light") {
			this._scheme = scheme;
		}
		this.applyScheme();
	},

	// Get scheme
	get scheme() {
		return this._scheme;
	},

	// Apply scheme
	applyScheme() {
		document
			.querySelector("html")
			?.setAttribute(this.rootAttribute, this.scheme);
	},
};

// Init
document.addEventListener("DOMContentLoaded", () => {
	themeSwitcher.init();
});
