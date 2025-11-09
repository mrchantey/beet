// a script to run before initial render to avoid FOUC.
// this should be included by the Head component

const lightClass = "scheme-light";
const darkClass = "scheme-dark";

const schemes = [lightClass, darkClass];
init();

function init() {
	const mediaQueryList = globalThis.matchMedia("(prefers-color-scheme: dark)");
	const setSchemeFromMediaQueryList = () => {
		if (mediaQueryList.matches) setScheme(darkClass);
		else setScheme(lightClass);
	};

	// event listeners should probably be in the application
	mediaQueryList.addEventListener("change", () => {
		setSchemeFromMediaQueryList();
	});

	setSchemeFromMediaQueryList();
}

function setScheme(scheme) {
	schemes.forEach((t) => {
		document.documentElement.classList.remove(t);
	});

	document.documentElement.classList.add(scheme);
	setScrollBar(scheme);
}

function setScrollBar(scheme) {
	const scollbarScheme = scheme === lightClass ? "light" : "dark";

	// edit: i dont think this is required and is causing fouc
	// document.documentElement
	// 	.setAttribute('style', 'display: none')

	document.documentElement.setAttribute("data-color-scheme", scollbarScheme);

	// Trigger reflow by reading a property
	document.body.clientWidth;

	// Show the document
	// edit: i dont think this is required and is causing fouc
	// document.documentElement
	// 	.setAttribute('style', 'display: \'\'')
}
