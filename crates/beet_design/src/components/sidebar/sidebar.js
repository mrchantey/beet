const sidebar = document.getElementById('sidebar')
const SMALL_SCREEN = 1024
const hide = () => sidebar.setAttribute('aria-hidden', 'true')
const show = () => sidebar.setAttribute('aria-hidden', 'false')

if (globalThis.innerWidth < SMALL_SCREEN) 
	hide()
globalThis.addEventListener('resize', () => {
	if (globalThis.innerWidth >= SMALL_SCREEN)
		show()
	else
		hide()
})
