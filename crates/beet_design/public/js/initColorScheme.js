// a script to run before initial render to avoid FOUC.
// the scheme from 


init()
// document.addEventListener('astro:after-swap', () => 
// 	init())


function init () {
	const mql = globalThis
		.matchMedia('(prefers-color-scheme: dark)')
	const setSchemeFromMql = () => {
		if (mql.matches) 
			setScheme('dark')
		else 
			setScheme('light')	
	}
	
	// event listeners should probably be in the application
	mql.addEventListener('change', () => {
		setSchemeFromMql()
	})

	setSchemeFromMql()
}


/* eslint-disable no-undef */
const materialSchemes = [
	'light',
	'light-medium-contrast',
	'light-high-contrast',
	'dark',
	'dark-medium-contrast',
	'dark-high-contrast'
]

function materialThemeToSimple(scheme) {
	if (scheme === 'light' || scheme === 'light-medium-contrast' || scheme === 'light-high-contrast') {
		return 'light'
	} else {
		return 'dark'
	}
}

function setScheme(scheme) {

	materialSchemes.forEach((t) => {
		document.documentElement.classList.remove(t)
	})

	document.documentElement.classList.add(scheme)
	setScrollBar(materialThemeToSimple(scheme))
}


function setScrollBar(color) {

	document.documentElement
		.setAttribute('style', 'display: none')

	document.documentElement
		.setAttribute('data-color-scheme', color)

	// Trigger reflow
	document.body.clientWidth

	// Show the document
	document.documentElement
		.setAttribute('style', 'display: \'\'')
}

