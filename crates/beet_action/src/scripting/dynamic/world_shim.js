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
	// parses back; an entity handle stringifies to the same token, so either
	// form may be passed anywhere an entity is taken. `undefined` becomes `null`
	// because JSON has no undefined.
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
		get_field: (entity, path) =>
			call({ op: "get_field", entity: id(entity), path }),
		set_field: (entity, path, value) =>
			call({
				op: "set_field",
				entity: id(entity),
				path,
				value: json(value),
			}),
		entity: (entity) => handle(entity),
	};

	// one entity's half of the `world` API, with the entity already supplied.
	// `world.entity(id)` is how a script holds onto one; an event script is
	// handed its own as `target`.
	const handle = (entity) => ({
		id: id(entity),
		toString: () => id(entity),
		get: (component) => globalThis.world.get(entity, component),
		insert: (component, value) =>
			globalThis.world.insert(entity, component, value),
		remove: (component) => globalThis.world.remove(entity, component),
		despawn: () => globalThis.world.despawn(entity),
		get_field: (path) => globalThis.world.get_field(entity, path),
		set_field: (path, value) =>
			globalThis.world.set_field(entity, path, value),
	});
})();
