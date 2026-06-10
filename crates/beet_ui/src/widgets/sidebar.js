// Responsive sidebar behaviour. The rail starts hidden below `BREAKPOINT` and
// visible above it, follows the viewport across resizes, and is toggled by the
// header menu button. Visibility is expressed as `aria-hidden` on `#sidebar`,
// which the `sidebar-hidden` style rule reacts to. `BREAKPOINT` is injected by
// the `Sidebar` widget so it tracks `SIDEBAR_BREAKPOINT_PX`.
(function () {
	function init() {
		const sidebar = document.getElementById("sidebar");
		const menuButton = document.getElementById("menu-button");
		if (!sidebar) return;

		const hide = () => sidebar.setAttribute("aria-hidden", "true");
		const show = () => sidebar.setAttribute("aria-hidden", "false");
		const isHidden = () => sidebar.getAttribute("aria-hidden") === "true";
		const narrow = () => globalThis.innerWidth < BREAKPOINT;

		// seed from the current viewport, then track resizes across the breakpoint
		narrow() ? hide() : show();
		globalThis.addEventListener("resize", () => (narrow() ? hide() : show()));

		// the menu button toggles the rail on narrow screens
		menuButton?.addEventListener("click", () => isHidden() ? show() : hide());

		// Disabled, causes flash of rerender just to navigate..
		// following a link on a narrow screen collapses the rail back
		// sidebar.querySelectorAll("a").forEach((link) =>
		// 	link.addEventListener("click", () => narrow() && hide()),
		// );
	}

	if (document.readyState === "loading") {
		document.addEventListener("DOMContentLoaded", init);
	} else init();
})();
