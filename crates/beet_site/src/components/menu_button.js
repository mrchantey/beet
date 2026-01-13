hideMenuButton();

function hideMenuButton() {
	const menuButton = document.getElementById("menu-button");
	const sidebar = document.getElementById("sidebar");
	if (!sidebar) {
		menuButton?.setAttribute("hidden", "");
	}
}
