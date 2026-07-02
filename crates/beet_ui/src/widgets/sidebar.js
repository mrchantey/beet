// Responsive sidebar behaviour. The served nav has no `aria-hidden` attribute,
// so the `sidebar-hidden` CSS rule already hides the rail below `BREAKPOINT`
// before this script runs (no flash). This script only keeps the attribute in
// sync for assistive tech and wires the menu-button toggle: `aria-hidden` on
// `#sidebar` is what the rule reacts to (`:not([aria-hidden="false"])`).
// `BREAKPOINT` is injected by the `Sidebar` widget so it tracks
// `SIDEBAR_BREAKPOINT_PX`.
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
