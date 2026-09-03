//! [`SchemaCommit`]: applying a schema edit to the data it describes.
use crate::prelude::*;
use core::pin::Pin;

/// The future a [`SchemaCommit`] backfill walk returns, so the walk can recurse
/// into nested structs and list items.
type BackfillFuture<'a> = Pin<Box<dyn 'a + Send + Future<Output = Result>>>;

/// A schema edit resolved against the data it describes.
///
/// Evolution happens here, at commit time, not at read time: an edit that would
/// leave existing data invalid is **rejected** unless every affected field
/// carries a resolution. An added required field needs an
/// [`OnMissing::Default`], an [`OnMissing::Computed`], or to be optional; a
/// retyped field needs its existing values to validate, or an
/// [`OnMissing::Computed`] conversion. A *removed* field needs nothing: a
/// struct schema that forbids additional keys is the exhaustive statement of
/// what the data is, so the commit drops the values it no longer declares.
///
/// The commit is transactional: the walk runs against a copy, so the new schema
/// and every backfill apply together or not at all, and every backfilled value
/// is validated against its field's schema before assignment.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaCommit {
	/// The schema being committed.
	schema: ValueSchema,
}

impl SchemaCommit {
	/// Declare a commit of `schema`.
	pub fn new(schema: ValueSchema) -> Self { Self { schema } }

	/// The schema this commit applies.
	pub fn schema(&self) -> &ValueSchema { &self.schema }

	/// Apply the commit to `value`, backfilling every field whose policy
	/// resolves it and leaving `value` untouched if anything fails.
	pub async fn apply(&self, value: &mut Value) -> Result {
		self.apply_in(SchemaResolver::default(), value).await
	}

	/// [`apply`](Self::apply), resolving each [`ValueSchema::Reference`] against
	/// `resolver`, so a commit of a composed schema evolves the data its
	/// referenced schemas describe too.
	pub async fn apply_in(
		&self,
		resolver: SchemaResolver<'_>,
		value: &mut Value,
	) -> Result {
		let mut next = value.clone();
		Self::backfill(resolver, &self.schema, &mut next).await?;
		self.schema
			.assert_valid_in(resolver, "schema commit", &mut next)
			.await?;
		*value = next;
		OK
	}

	/// Walk `schema` over `value`, resolving every field the edit left absent or
	/// mistyped. Errors name the field, so a rejected commit says what to add.
	fn backfill<'a>(
		resolver: SchemaResolver<'a>,
		schema: &'a ValueSchema,
		value: &'a mut Value,
	) -> BackfillFuture<'a> {
		Box::pin(async move {
			match (schema, value) {
				(ValueSchema::Struct(schema), Value::Map(map)) => {
					for field in &schema.fields {
						Self::backfill_field(resolver, field, map).await?;
					}
					// a schema declaring its keys exhaustively is what the data
					// now is, so a removed field's values go with it. The commit
					// is where a schema edit resolves against its data, and an
					// orphaned key has nothing left that could resolve it: it
					// would only fail validation, with removal impossible.
					if !schema.allow_additional {
						let declared = schema
							.fields
							.iter()
							.map(|field| field.key.clone())
							.collect::<HashSet<_>>();
						map.0.retain(|key, _| declared.contains(key));
					}
				}
				(ValueSchema::List(schema), Value::List(items)) => {
					for item in items.iter_mut() {
						Self::backfill(resolver, &schema.item, item).await?;
					}
				}
				(ValueSchema::Map(schema), Value::Map(map)) => {
					for item in map.0.values_mut() {
						Self::backfill(resolver, &schema.value, item).await?;
					}
				}
				// a null satisfies an optional; anything else backfills as the inner
				(ValueSchema::Optional(inner), value) => {
					if !value.is_null() {
						Self::backfill(resolver, inner, value).await?;
					}
				}
				// a reference the resolver answers backfills as its target
				(ValueSchema::Reference(name), value) => {
					if let Some(target) = resolver.schema(name) {
						Self::backfill(resolver, target, value).await?;
					}
				}
				// scalars and enums carry no field policies
				_ => {}
			}
			OK
		})
	}

	/// Resolve one struct field against the map holding it.
	async fn backfill_field(
		resolver: SchemaResolver<'_>,
		field: &NamedFieldSchema,
		map: &mut Map,
	) -> Result {
		let Some(current) = map.0.get_mut(field.key.as_str()) else {
			// absent and optional is already valid, absent and required needs a
			// resolution to have been declared.
			if !field.required {
				return OK;
			}
			let Some(policy) = &field.on_missing else {
				bevybail!(
					"required field `{}` has no value and declares no resolution, \
					add a default, a computed script, or make it optional",
					field.key
				);
			};
			let value = policy.resolve(&field.key, None).await?;
			map.insert(
				field.key.clone(),
				Self::checked(resolver, field, value).await?,
			);
			return OK;
		};

		// resolve nested fields first, so a container evolves through its own
		// children before the value as a whole is judged.
		Self::backfill(resolver, &field.schema, current).await?;
		// a present value that now validates is left alone (validation coerces
		// in place, eg an int key into a uint one).
		if field.schema.validate_in(resolver, current).await.is_empty() {
			return OK;
		}

		// a retype: only a computed conversion can rescue an existing value,
		// since a default would silently discard what the user typed.
		let Some(policy @ OnMissing::Computed { .. }) = &field.on_missing
		else {
			bevybail!(
				"field `{}` no longer accepts its existing value and declares no \
				computed conversion",
				field.key
			);
		};
		let value = policy.resolve(&field.key, Some(current)).await?;
		*current = Self::checked(resolver, field, value).await?;
		OK
	}

	/// Validate a resolved value against its field's schema before it is
	/// assigned, so a bad policy fails the commit rather than corrupting data.
	async fn checked(
		resolver: SchemaResolver<'_>,
		field: &NamedFieldSchema,
		mut value: Value,
	) -> Result<Value> {
		field
			.schema
			.assert_valid_in(
				resolver,
				&format!("field `{}`", field.key),
				&mut value,
			)
			.await?;
		value.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// `{ label: String }`, the schema before the edit.
	fn item_schema(fields: Vec<NamedFieldSchema>) -> ValueSchema {
		ValueSchema::Struct(StructSchema {
			name: Some("TodoItem".into()),
			allow_additional: false,
			fields,
		})
	}

	fn label() -> NamedFieldSchema {
		NamedFieldSchema::new("label", ValueSchema::String(default()))
	}

	fn difficult() -> NamedFieldSchema {
		NamedFieldSchema::new(
			"is_really_difficult",
			ValueSchema::Bool(default()),
		)
	}

	#[crate::test]
	async fn added_required_field_without_resolution_is_rejected() {
		let mut value = value!({ "label": "buy milk" });
		SchemaCommit::new(item_schema(vec![label(), difficult()]))
			.apply(&mut value)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("is_really_difficult");
		// the rejected commit left the data untouched
		value.xpect_eq(value!({ "label": "buy milk" }));
	}

	#[crate::test]
	async fn added_optional_field_needs_no_resolution() {
		let mut value = value!({ "label": "buy milk" });
		SchemaCommit::new(item_schema(vec![label(), difficult().optional()]))
			.apply(&mut value)
			.await
			.unwrap();
		value.xpect_eq(value!({ "label": "buy milk" }));
	}

	#[crate::test]
	async fn default_backfills_and_validates() {
		let mut value = value!({ "label": "buy milk" });
		SchemaCommit::new(item_schema(vec![
			label(),
			difficult().with_on_missing(OnMissing::Default(value!(false))),
		]))
		.apply(&mut value)
		.await
		.unwrap();
		value.xpect_eq(
			value!({ "label": "buy milk", "is_really_difficult": false }),
		);
	}

	/// A backfill whose value does not satisfy the field's own schema fails the
	/// commit rather than writing data the schema rejects.
	#[crate::test]
	async fn wrongly_typed_default_fails_the_commit() {
		let mut value = value!({ "label": "buy milk" });
		SchemaCommit::new(item_schema(vec![
			label(),
			difficult().with_on_missing(OnMissing::Default(value!("nope"))),
		]))
		.apply(&mut value)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("is_really_difficult");
		value.xpect_eq(value!({ "label": "buy milk" }));
	}

	/// The commit descends into a list of items, so a document holding rows
	/// evolves every row under one schema edit.
	#[crate::test]
	async fn backfills_every_row() {
		let mut value = value!({ "items": [
			{ "label": "a" },
			{ "label": "b" },
		] });
		let schema = ValueSchema::Struct(StructSchema {
			name: None,
			allow_additional: false,
			fields: vec![NamedFieldSchema::new(
				"items",
				ValueSchema::List(ListSchema {
					item: Box::new(item_schema(vec![
						label(),
						difficult()
							.with_on_missing(OnMissing::Default(value!(false))),
					])),
					min_items: None,
					max_items: None,
					unique: false,
				}),
			)],
		});
		SchemaCommit::new(schema).apply(&mut value).await.unwrap();
		value.xpect_eq(value!({ "items": [
			{ "label": "a", "is_really_difficult": false },
			{ "label": "b", "is_really_difficult": false },
		] }));
	}

	/// A commit of a composed schema evolves through its references, so editing
	/// the `TodoItem` schema backfills the rows of a document that only names
	/// it.
	#[crate::test]
	async fn backfills_through_a_reference() {
		let mut registry = SchemaRegistry::default();
		registry.insert(
			"TodoItem",
			item_schema(vec![
				label(),
				difficult().with_on_missing(OnMissing::Default(value!(false))),
			]),
		);
		let resolver = SchemaResolver::default().with_schemas(&registry);
		let mut value = value!([{ "label": "a" }]);
		SchemaCommit::new(ValueSchema::List(ListSchema {
			item: Box::new(ValueSchema::Reference("TodoItem".into())),
			min_items: None,
			max_items: None,
			unique: false,
		}))
		.apply_in(resolver, &mut value)
		.await
		.unwrap();
		value
			.xpect_eq(value!([{ "label": "a", "is_really_difficult": false }]));
	}

	/// Removing a field takes its data with it: a schema forbidding additional
	/// keys says exactly what the data is, and an orphaned value has nothing
	/// that could resolve it, so it would only fail validation forever.
	#[crate::test]
	async fn a_removed_field_drops_its_values() {
		let mut value = value!({ "label": "buy milk", "done": true });
		SchemaCommit::new(item_schema(vec![label()]))
			.apply(&mut value)
			.await
			.unwrap();
		value.xpect_eq(value!({ "label": "buy milk" }));
	}

	/// A schema that permits additional keys keeps them: it never claimed to be
	/// the whole story.
	#[crate::test]
	async fn additional_keys_survive_where_permitted() {
		let ValueSchema::Struct(mut schema) = item_schema(vec![label()]) else {
			panic!("a struct schema");
		};
		schema.allow_additional = true;
		let mut value = value!({ "label": "buy milk", "note": "later" });
		SchemaCommit::new(ValueSchema::Struct(schema))
			.apply(&mut value)
			.await
			.unwrap();
		value.xpect_eq(value!({ "label": "buy milk", "note": "later" }));
	}

	/// A retype with no computed conversion is rejected: the existing value
	/// stays, and the author is told what is missing.
	#[crate::test]
	async fn retype_without_conversion_is_rejected() {
		let mut value = value!({ "label": "buy milk" });
		SchemaCommit::new(item_schema(vec![NamedFieldSchema::new(
			"label",
			ValueSchema::I64(default()),
		)]))
		.apply(&mut value)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("computed conversion");
	}

	/// The retype path reaches the js runtime seam, the one piece of the
	/// evolution mechanism still to be built.
	#[crate::test]
	#[should_panic(expected = "js runtime seam")]
	async fn retype_with_computed_reaches_the_seam() {
		let mut value = value!({ "label": "12" });
		SchemaCommit::new(item_schema(vec![
			NamedFieldSchema::new("label", ValueSchema::I64(default()))
				.with_on_missing(OnMissing::Computed {
					script: "parseInt(input)".into(),
				}),
		]))
		.apply(&mut value)
		.await
		.ok();
	}
}
