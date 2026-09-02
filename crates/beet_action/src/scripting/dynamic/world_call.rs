//! The request/response wire a bridged script speaks, and the host-side
//! executor that answers it.
//!
//! One call in, one reply out, correlated by [`WorldCall::id`]: the script
//! awaits a promise, the host performs the operation against the live
//! [`World`], and the reply settles that promise. Every operation is served the
//! moment it is asked for, so a script reads its own writes.
use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// One `world` call, as it left the sandbox.
///
/// ```json
/// { "id": 3, "op": "insert", "entity": "42v1", "component": "Name", "value": "ada" }
/// ```
///
/// The shapes are deliberately loose: a component is named by a string, a
/// component's value is an arbitrary [`Value`]. Which values a particular
/// component accepts is that component's business, not the wire's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldCall {
	/// Correlates this call with its [`WorldReply`]. Assigned by the shim,
	/// monotonic within one evaluation.
	pub id: u64,
	/// What the script asked for.
	#[serde(flatten)]
	pub op: WorldOp,
}

/// The operations a script can ask of the world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum WorldOp {
	/// `await world.get(entity, component)`, replying with the component's
	/// [`Value`] form, or with no value when the entity does not carry it.
	Get {
		/// The entity, in [`entity_id`](super::entity_id) form.
		entity: String,
		/// The component identifier.
		component: String,
	},
	/// `await world.entities(component)`, replying with the ids of every entity
	/// carrying it.
	Entities {
		/// The component identifier.
		component: String,
	},
	/// `await world.schema(component)`, replying with a structural description.
	Schema {
		/// The component identifier.
		component: String,
	},
	/// `await world.spawn(components)`, replying with the new entity's id.
	Spawn {
		/// The components to spawn it with, keyed by identifier.
		components: Map,
	},
	/// `await world.insert(entity, component, value)`, replying with no value.
	Insert {
		/// The entity, in [`entity_id`](super::entity_id) form.
		entity: String,
		/// The component identifier.
		component: String,
		/// The component's value.
		value: Value,
	},
	/// `await world.remove(entity, component)`, replying with no value.
	Remove {
		/// The entity, in [`entity_id`](super::entity_id) form.
		entity: String,
		/// The component identifier.
		component: String,
	},
	/// `await world.despawn(entity)`, replying with no value.
	Despawn {
		/// The entity, in [`entity_id`](super::entity_id) form.
		entity: String,
	},
}

/// The host's answer to one [`WorldCall`].
///
/// ```json
/// { "status": "ok", "id": 3 }
/// { "status": "err", "id": 3, "message": "script may not write `Name`" }
/// ```
///
/// An [`Ok`](Self::Ok) with no value settles the promise with `undefined`,
/// which is how a void operation and an absent component both read in the
/// script. An [`Err`](Self::Err) *rejects* the promise, so a refused write is an
/// error the script can catch at the call site rather than a silent no-op.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum WorldReply {
	/// The operation succeeded.
	Ok {
		/// The call this answers.
		id: u64,
		/// What it produced, absent for a void operation.
		#[serde(default, skip_serializing_if = "Option::is_none")]
		value: Option<Value>,
	},
	/// The operation failed, with the host's flattened message.
	Err {
		/// The call this answers.
		id: u64,
		/// Why it failed, as the script's `Error` message.
		message: String,
	},
}

impl WorldReply {
	/// The reply for a call that produced `value`, or failed.
	///
	/// A failure carries the error's first line only. A [`BevyError`] renders
	/// its host-side location and backtrace after the message, and none of that
	/// means anything inside the sandbox: the script is about to see this as an
	/// `Error` it may print or answer with. Every message the bridge raises is
	/// one line by construction.
	fn new(id: u64, result: Result<Option<Value>>) -> Self {
		match result {
			Ok(value) => Self::Ok { id, value },
			Err(err) => Self::Err {
				id,
				message: err
					.to_string()
					.lines()
					.next()
					.unwrap_or_default()
					.to_string(),
			},
		}
	}
}

impl WorldCall {
	/// Perform this call through `bridge`.
	///
	/// Infallible by construction: a failure is a [`WorldReply::Err`] the script
	/// can catch, never a failure of the run. A script asking for something it
	/// may not have is the sandbox working, not the host breaking.
	pub async fn execute(self, bridge: &WorldBridge) -> WorldReply {
		WorldReply::new(self.id, self.op.execute(bridge).await)
	}
}

impl WorldOp {
	/// Perform this operation, producing its reply value.
	///
	/// Async because an operation is not one indivisible step: it takes
	/// exclusive world access for as long as it needs and gives it back, so
	/// work that is legitimately asynchronous (a schema asking something beyond
	/// the world before it will accept a value) runs with nothing held. Each
	/// exclusive section is a [`WorldRead`] or [`WorldWrite`] call, which stay
	/// synchronous `&mut World` operations.
	async fn execute(self, bridge: &WorldBridge) -> Result<Option<Value>> {
		let world = bridge.world().clone();
		let exposure = bridge.exposure().clone();
		match self {
			Self::Get { entity, component } => {
				let entity = entity_id::decode(&entity)?;
				world
					.with(move |world| {
						WorldRead::get(world, entity, &component, &exposure)
					})
					.await
			}
			Self::Entities { component } => world
				.with(move |world| {
					WorldRead::entities(world, &component, &exposure)
				})
				.await
				.map(Some),
			Self::Schema { component } => world
				.with(move |world| {
					WorldRead::schema(world, &component, &exposure)
				})
				.await
				.map(Some),
			Self::Spawn { components } => {
				// every component is checked before any of them is spawned, so a
				// rejected value never leaves a half-built entity behind
				let mut validated = Map::default();
				for (ident, mut value) in components {
					Self::check(&world, &exposure, &ident, &mut value).await?;
					validated.insert(ident, value);
				}
				world
					.with(move |world| {
						WorldWrite::spawn(world, validated, &exposure)
					})
					.await?
					.xmap(entity_id::encode)
					.xmap(Value::str)
					.xmap(Some)
					.xok()
			}
			Self::Insert {
				entity,
				component,
				mut value,
			} => {
				let entity = entity_id::decode(&entity)?;
				Self::check(&world, &exposure, &component, &mut value).await?;
				world
					.with(move |world| {
						WorldWrite::insert(
							world, entity, &component, value, &exposure,
						)
					})
					.await
					.map(|_| None)
			}
			Self::Remove { entity, component } => {
				let entity = entity_id::decode(&entity)?;
				world
					.with(move |world| {
						WorldWrite::remove(world, entity, &component, &exposure)
					})
					.await
					.map(|_| None)
			}
			Self::Despawn { entity } => {
				let entity = entity_id::decode(&entity)?;
				world
					.with(move |world| WorldWrite::despawn(world, entity))
					.await
					.map(|_| None)
			}
		}
	}

	/// Check `value` against whatever `ident` declared, coercing it where the
	/// schema says to.
	///
	/// Two short exclusive sections with the validation between them, which is
	/// the shape every mutation takes: read what the world says this component
	/// accepts, let go, decide, and only then apply. The world may change in
	/// that window, and a value validated against a schema that has since been
	/// redeclared would land under the new one. That is not a regression, since
	/// a script's own calls stay ordered (it awaits each one before making the
	/// next), but it is the widest window the bridge has, and it is here rather
	/// than anywhere else.
	async fn check(
		world: &AsyncWorld,
		exposure: &ScriptExposure,
		ident: &str,
		value: &mut Value,
	) -> Result {
		let Some(schema) = world
			.with({
				let (ident, exposure) = (ident.to_string(), exposure.clone());
				move |world| {
					WorldWrite::declared_schema(world, &ident, &exposure)
				}
			})
			.await?
		else {
			return Ok(());
		};
		WorldWrite::validate(&schema, ident, value).await
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::scripting::dynamic::test_support::*;
	use beet_core::prelude::*;

	/// The headline capability: a write lands before the read that follows it,
	/// because both are served against the same live world.
	#[beet_core::test]
	async fn a_read_sees_the_write_before_it() {
		let mut world = test_world();
		let entity = world.spawn_empty().id();
		serve(
			&mut world,
			&format!(
				r#"{{"id":0,"op":"insert","entity":"{entity}","component":"Name","value":"ada"}}"#
			),
		)
		.await;
		serve(
			&mut world,
			&format!(
				r#"{{"id":1,"op":"get","entity":"{entity}","component":"Name"}}"#
			),
		)
		.await
		.xpect_eq(WorldReply::Ok {
			id: 1,
			value: Some(Value::from("ada")),
		});
	}

	#[beet_core::test]
	async fn a_spawn_replies_with_a_usable_id() {
		let mut world = test_world();
		let WorldReply::Ok {
			value: Some(Value::Str(id)),
			..
		} = serve(
			&mut world,
			r#"{"id":0,"op":"spawn","components":{"Name":"ada"}}"#,
		)
		.await
		else {
			panic!("spawn did not reply with an id");
		};
		only_name(&mut world).xpect_eq(Name::new("ada"));
		serve(
			&mut world,
			&format!(
				r#"{{"id":1,"op":"get","entity":"{id}","component":"Name"}}"#
			),
		)
		.await
		.xpect_eq(WorldReply::Ok {
			id: 1,
			value: Some(Value::from("ada")),
		});
	}

	/// A component the entity does not carry replies with no value, which the
	/// shim settles as `undefined`.
	#[beet_core::test]
	async fn an_absent_component_replies_with_no_value() {
		let mut world = test_world();
		let entity = world.spawn_empty().id();
		serve(
			&mut world,
			&format!(
				r#"{{"id":0,"op":"get","entity":"{entity}","component":"Name"}}"#
			),
		)
		.await
		.xpect_eq(WorldReply::Ok { id: 0, value: None });
	}

	#[beet_core::test]
	async fn a_void_operation_replies_with_no_value() {
		let mut world = test_world();
		let entity = world.spawn(Name::new("ada")).id();
		serve(
			&mut world,
			&format!(r#"{{"id":4,"op":"despawn","entity":"{entity}"}}"#),
		)
		.await
		.xpect_eq(WorldReply::Ok { id: 4, value: None });
		entity_count(&world).xpect_eq(0);
	}

	/// A refusal is an error reply carrying the call's id, so the shim rejects
	/// exactly the promise that asked.
	#[beet_core::test]
	async fn a_failure_replies_with_the_calls_id() {
		let mut world = test_world();
		let WorldReply::Err { id, message } = serve(
			&mut world,
			r#"{"id":9,"op":"get","entity":"nope","component":"Name"}"#,
		)
		.await
		else {
			panic!("a malformed entity should not have succeeded");
		};
		id.xpect_eq(9);
		message.xpect_contains("is not an entity id");
	}

	/// The wire tags are written by hand in the shared JS shim, so they are
	/// pinned here the way `ScriptEvent`'s are.
	#[beet_core::test]
	fn wire_tags_are_stable() {
		serde_json::to_string(&WorldCall {
			id: 1,
			op: WorldOp::Remove {
				entity: "42v1".to_string(),
				component: "Name".to_string(),
			},
		})
		.unwrap()
		.xpect_eq(
			r#"{"id":1,"op":"remove","entity":"42v1","component":"Name"}"#,
		);
		serde_json::to_string(&WorldReply::Ok { id: 1, value: None })
			.unwrap()
			.xpect_eq(r#"{"status":"ok","id":1}"#);
		serde_json::to_string(&WorldReply::Err {
			id: 1,
			message: "boom".to_string(),
		})
		.unwrap()
		.xpect_eq(r#"{"status":"err","id":1,"message":"boom"}"#);
	}

	/// The wire is [`Value`], not JSON, so the encodings a script writes must
	/// still decode into the variants the world expects.
	#[beet_core::test]
	fn a_value_wire_decodes_a_json_line() {
		let WorldOp::Insert { value, .. } = serde_json::from_str::<WorldCall>(
			r#"{"id":0,"op":"insert","entity":"0v1","component":"a.B","value":{"n":-1,"list":[1,2]}}"#,
		)
		.unwrap()
		.op
		else {
			panic!("not an insert");
		};
		let mut expected = Map::default();
		expected.insert("n", Value::Int(-1));
		expected
			.insert("list", Value::List(vec![Value::Uint(1), Value::Uint(2)]));
		value.xpect_eq(Value::Map(expected));
	}
}
