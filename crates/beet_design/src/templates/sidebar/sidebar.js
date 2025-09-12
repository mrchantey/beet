// SIDEBAR RESIZE
const SCREEN_BREAKPOINT = 1024;

// TODO mobile sidebar button
// hideOnSmallScreen()
handleCurrentPage();

function hideOnSmallScreen() {
	const sidebar = document.getElementById("sidebar");
	const hide = () => sidebar.setAttribute("aria-hidden", "true");
	const show = () => sidebar.setAttribute("aria-hidden", "false");

	if (globalThis.innerWidth < SCREEN_BREAKPOINT) hide();
	globalThis.addEventListener("resize", () => {
		if (globalThis.innerWidth >= SCREEN_BREAKPOINT) show();
		else hide();
	});
}

/** if the link's url matches the location href, give it the current page attribute */
function handleCurrentPage() {
	// const links = document.querySelectorAll<HTMLAnchorElement>('a.bm-c-sidebar__link')
	// const sublists = document.querySelectorAll<HTMLAnchorElement>('details.bm-c-sidebar__sublist')
	const links = document.querySelectorAll("a.bm-c-sidebar__link");
	const sublists = document.querySelectorAll("details.bm-c-sidebar__sublist");

	// console.log('location', globalThis.location.href)

	// handle show selected
	links.forEach((link) => {
		// console.log('links', link.href)
		if (link.href === globalThis.location.href)
			link.setAttribute("aria-current", "page");
		else link.removeAttribute("aria-current");
	});

	// handle open sublists
	sublists.forEach((sublist) => {
		if (
			sublist.hasAttribute("data-always-expand") ||
			sublist.querySelector('a[aria-current="page"]')
		) {
			sublist.setAttribute("open", "true");
		} else {
			sublist.removeAttribute("open");
		}
	});

	// handle sidebar toggle
	links.forEach((link) => {
		link.addEventListener("click", () => {
			if (globalThis.innerWidth < SCREEN_BREAKPOINT) {
				document.getElementById("sidebar").setAttribute("aria-hidden", "true");
			}
		});
	});
}
