(() => {
	// a thrown value as one message. QuickJS's `stack` carries only the frames
	// where V8's starts with the message, so the message is prepended unless the
	// stack already opens with it. Mirrors `protocol::JS_RUNNER`, which does the
	// same job for the host-realm backends.
	const describe = (err) => {
		if (!(err instanceof Error)) return String(err);
		const stack = String(err.stack || "");
		return stack.startsWith(err.name) ? stack : String(err) + "\n" + stack;
	};

	// the embedded engine has no host realm to post to, so a call is handed
	// straight over as an encoding the pump loop on the rust side decodes.
	globalThis.__world_send = (call) =>
		globalThis.__world_send_json(JSON.stringify(call));

	// a promise's state is not observable from outside it, and the pump has to
	// know when the script is finished to stop serving. So the script's own
	// promise records how it settled, as data the host reads between pumps.
	// `JSON.stringify` drops an undefined value, which is how a script that
	// produced none reads back.
	globalThis.__world_begin = (promise) => {
		Promise.resolve(promise).then(
			(value) => {
				globalThis.__world_done = JSON.stringify({
					status: "ok",
					value,
				});
			},
			(err) => {
				globalThis.__world_done = JSON.stringify({
					status: "err",
					message: describe(err),
				});
			},
		);
	};
})();
