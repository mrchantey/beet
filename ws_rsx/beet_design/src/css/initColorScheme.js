// a script to run before initial render to avoid FOUC.
// this will be included by the Head component

const lightClass = 'scheme-light'
const darkClass = 'scheme-dark'

const schemes = [
	lightClass,
	darkClass
]
init()
// document.addEventListener('astro:after-swap', () => 
// 	init())


function init () {
	const mql = globalThis
		.matchMedia('(prefers-color-scheme: dark)')
	const setSchemeFromMql = () => {
		if (mql.matches) 
			setScheme(darkClass)
		else 
			setScheme(lightClass)	
	}
	
	// event listeners should probably be in the application
	mql.addEventListener('change', () => {
		setSchemeFromMql()
	})

	setSchemeFromMql()
}

function setScheme(scheme) {

	schemes.forEach((t) => {
		document.documentElement.classList.remove(t)
	})

	document.documentElement.classList.add(scheme)
	setScrollBar(scheme)
}


function setScrollBar(scheme) {
	const scollbarScheme = scheme === lightClass
		? 'light'
		: 'dark'

	document.documentElement
		.setAttribute('style', 'display: none')

	document.documentElement
		.setAttribute('data-color-scheme', scollbarScheme)

	// Trigger reflow
	document.body.clientWidth

	// Show the document
	document.documentElement
		.setAttribute('style', 'display: \'\'')
}

