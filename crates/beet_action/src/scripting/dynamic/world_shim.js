(() => {
	// every call in flight, by id, so a reply settles exactly the promise that
	// asked for it. Nothing here is reachable from the script: the map and the
	// counter live in this closure.
	const pending = new Map();
	let next = 0;

	const call = (op) =>
		new Promise((resolve, reject) => {
			const id = next++;
			pending.set(id, { resolve, reject });
			globalThis.__world_send({ id, ...op });
		});

	// the host's half: one reply per call, settling or rejecting it. An `err`
	// rejects with a real `Error`, so a refused write is catchable at the call
	// site rather than a silent no-op.
	globalThis.__world_reply = (reply) => {
		const settle = pending.get(reply.id);
		if (settle === undefined) return;
		pending.delete(reply.id);
		if (reply.status === "ok") settle.resolve(reply.value);
		else settle.reject(new Error(reply.message));
	};

	// an entity is an opaque string token, so `String(entity)` is what the host
	// parses back; `undefined` becomes `null` because JSON has no undefined.
	const id = (entity) => String(entity);
	const json = (value) => (value === undefined ? null : value);

	globalThis.world = {
		get: (entity, component) =>
			call({ op: "get", entity: id(entity), component }),
		entities: (component) => call({ op: "entities", component }),
		schema: (component) => call({ op: "schema", component }),
		spawn: (components) =>
			call({ op: "spawn", components: components || {} }),
		insert: (entity, component, value) =>
			call({
				op: "insert",
				entity: id(entity),
				component,
				value: json(value),
			}),
		remove: (entity, component) =>
			call({ op: "remove", entity: id(entity), component }),
		despawn: (entity) => call({ op: "despawn", entity: id(entity) }),
	};
})();
