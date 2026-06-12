// Connects to the beet client_io channel and reloads the page on a `reload`
// message. Reconnects with exponential backoff, reloading on a successful
// reconnect after a disconnect (the server restarted under us).
// `CLIENT_IO_PORT` is injected by the `LiveReloadScript` widget.
(function () {
	const INITIAL_RETRY_MILLIS = 500;
	const MAX_RETRY_MILLIS = 10000;
	let retryMillis = INITIAL_RETRY_MILLIS;
	let wasDisconnected = false;

	function connect() {
		const socket = new WebSocket(
			`ws://${location.hostname}:${CLIENT_IO_PORT}`,
		);
		socket.addEventListener("open", () => {
			retryMillis = INITIAL_RETRY_MILLIS;
			if (wasDisconnected) location.reload();
		});
		socket.addEventListener("message", (ev) => {
			if (ev.data === "reload") location.reload();
		});
		socket.addEventListener("close", () => {
			wasDisconnected = true;
			setTimeout(connect, retryMillis);
			retryMillis = Math.min(retryMillis * 2, MAX_RETRY_MILLIS);
		});
	}
	connect();
})();
