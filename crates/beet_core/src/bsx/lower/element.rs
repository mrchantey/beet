//! Element lowering: the tag dispatch, and the seams a type opens in it.
use super::literal::*;
use super::lowering::*;
use crate::bsx::reflect::*;
use crate::prelude::*;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::reflect::ReflectResource;
use bevy::reflect::CreateTypeData;
use bevy::reflect::Reflect;
use bevy::reflect::TypeInfo;
use bevy::reflect::TypeRegistration;

/// A component authored from its whole element rather than from attributes
/// that are its fields, ie `<Rule>`, whose attributes are style declarations.
/// Register `#[reflect(FromElement)]` and the lowering hands its tag here,
/// its value the component as a save would write it. Read with no world.
pub trait FromElement: Sized {
	/// Read the component off `element`, its directives rewritten.
	fn from_element(element: &BsxElement) -> Result<Self>;
}

/// Reflect type data for a [`FromElement`] component, so the lowering reads a
/// tag known only by name.
#[derive(Clone)]
pub struct ReflectFromElement {
	from_element: fn(&BsxElement) -> Result<Box<dyn Reflect>>,
}

impl<T: FromElement + Reflect> CreateTypeData<T> for ReflectFromElement {
	fn create_type_data(_input: ()) -> Self {
		Self {
			from_element: |element| {
				T::from_element(element)
					.map(|value| Box::new(value) as Box<dyn Reflect>)
			},
		}
	}
}

/// What a name resolves to.
enum NameKind {
	/// A `#[template]` type, expanded when the body spawns.
	Template,
	/// A resource declaration.
	Resource,
	/// A component.
	Component,
}

impl NameKind {
	/// What `registration` is, refusing a type that is none.
	fn of(registration: &TypeRegistration) -> Result<Self> {
		if registration.data::<ReflectTemplate>().is_some() {
			Self::Template.xok()
		} else if registration.data::<ReflectResource>().is_some() {
			Self::Resource.xok()
		} else if registration.data::<ReflectComponent>().is_some() {
			Self::Component.xok()
		} else {
			bevybail!(
				"type `{}` is not a registered component",
				registration.type_info().type_path()
			)
		}
	}
}

impl Lowering<'_> {
	/// Lower `el` onto the entity keyed `key`, its own already added.
	pub fn element(
		&mut self,
		el: &BsxElement,
		key: EntityKey,
		parent: Option<&EntityKey>,
	) -> Result {
		// a `slot` routes the whole node into a slot of its parent
		if let Some(slot) = el.slot_routing()
			&& parent.is_some()
		{
			self.insert(&key, &slot)?;
		}
		match el.tag.as_str() {
			"slot" => self.slot(el, &key),
			// a neutral host carrying spreads, directives and children with no
			// element of its own
			"Fragment" => self.content(el, &key),
			_ if el.is_named() => self.named(el, &key),
			_ => {
				self.insert(&key, &Element::new(el.tag.clone()))?;
				if el.tag == "template" {
					self.insert_reflected(&key, &Prefab)?;
				}
				self.content(el, &key)
			}
		}
	}

	/// Add a unit component the registry need not hold a serde form for.
	fn insert_reflected<T: bevy::reflect::TypePath>(
		&mut self,
		key: &EntityKey,
		_component: &T,
	) -> Result {
		self.entity(key).insert(T::type_path(), Value::map());
		Ok(())
	}

	/// Lower a `<slot>`, its children the fallback. One that is itself routed
	/// forwards its content.
	fn slot(&mut self, el: &BsxElement, key: &EntityKey) -> Result {
		let target = match el.attribute("name") {
			Some(name) => SlotTarget::named(name),
			None => SlotTarget::new(),
		};
		self.insert(key, &target)?;
		self.spreads(el, key)?;
		self.children(&el.children, key, None)
	}

	/// Lower an element's neutral content: its attributes, spreads and
	/// children.
	fn content(&mut self, el: &BsxElement, key: &EntityKey) -> Result {
		for attr in el.plain_attributes() {
			self.html_attribute(attr, key)?;
		}
		self.spreads(el, key)?;
		self.children(&el.children, key, None)
	}

	/// Lower one HTML attribute of the entity keyed `owner` as an entity of
	/// its own, an `@comp` binding in it binding the owner.
	fn html_attribute(
		&mut self,
		attr: &BsxAttribute,
		owner: &EntityKey,
	) -> Result {
		let components = EntityBody::default()
			.with_typed(&Attribute::new(attr.key.clone()))?;
		let key = self.attribute(owner, &attr.key, components)?;
		match &attr.value {
			AttrValue::Flag | AttrValue::Spread(_) => Ok(()),
			AttrValue::Str(string) => {
				self.insert(&key, &Value::Str(string.into()))
			}
			AttrValue::Expr(expr) => self.value_expr(expr, &key, Some(owner)),
		}
	}

	/// Lower an uppercase tag: a component, a template or a resource, else a
	/// name nothing registered.
	fn named(&mut self, el: &BsxElement, key: &EntityKey) -> Result {
		let Some(registration) =
			registration_by_name(self.types, &el.tag).cloned()
		else {
			return self.unregistered(el, key);
		};
		match NameKind::of(&registration)? {
			NameKind::Template => self.template(el, &registration, key),
			NameKind::Resource => self.resource(el, &registration, key),
			NameKind::Component => self.component(el, &registration, key),
		}
	}

	/// Lower a `#[template]` tag to its invocation, an attribute beyond its
	/// props an attribute entity the root element it builds takes, its
	/// children its slot children.
	fn template(
		&mut self,
		el: &BsxElement,
		registration: &TypeRegistration,
		key: &EntityKey,
	) -> Result {
		verify_props(el, registration)?;
		let props = match registration.type_info() {
			TypeInfo::Struct(info) => info.field_names(),
			_ => &[],
		};
		let is_prop = |attr: &&BsxAttribute| props.contains(&attr.key.as_str());
		// a template is built once from its props, so nothing would follow a
		// binding into one
		if let Some(bound) = el
			.plain_attributes()
			.filter(is_prop)
			.find(|attr| attr.is_binding())
		{
			bevybail!(
				"`<{} {}=@..>`: a template prop takes a literal, a binding \
				has nothing to sync with",
				el.tag,
				bound.key
			);
		}
		let value = match &el.tag_literal {
			Some(named) => self.named_value(named, registration)?,
			None => self.attributes_value(el, registration)?,
		};
		self.entity(key)
			.insert(registration.type_info().type_path(), value);
		for attr in el.plain_attributes().filter(|attr| !is_prop(attr)) {
			self.html_attribute(attr, key)?;
		}
		self.spreads(el, key)?;
		self.children(&el.children, key, Some(&el.tag))
	}

	/// Lower a resource declaration: the fields it names, which patch the live
	/// resource as the body spawns, the rest keeping their values.
	fn resource(
		&mut self,
		el: &BsxElement,
		registration: &TypeRegistration,
		key: &EntityKey,
	) -> Result {
		if !el.children.is_empty() {
			bevybail!(
				"`<{}>` declares a resource and cannot have children",
				el.tag
			);
		}
		if let Some(attr) = el.plain_attributes().find(|attr| attr.is_binding())
		{
			bevybail!(
				"`<{} {}=@..>`: an `@` binding cannot declare a resource field, use a literal",
				el.tag,
				attr.key
			);
		}
		let value = match &el.tag_literal {
			Some(named) => self.named_value(named, registration)?,
			None => {
				Self::assert_attributes_are_fields(
					el,
					registration.type_info(),
				)?;
				self.attributes_value(el, registration)?
			}
		};
		self.entity(key)
			.insert(registration.type_info().type_path(), value);
		self.spreads(el, key)
	}

	/// Lower a component tag, its binding valued attributes syncing the field
	/// they name and its children built under it, or held by it.
	fn component(
		&mut self,
		el: &BsxElement,
		registration: &TypeRegistration,
		key: &EntityKey,
	) -> Result {
		let info = registration.type_info();
		let is_instance = info.is::<Instance>();
		let (el, bound_props) = match is_instance {
			true => Self::take_bound_props(el)?,
			false => (alloc::borrow::Cow::Borrowed(el), Vec::new()),
		};
		let el = el.as_ref();
		let mut value = match (
			registration.data::<ReflectFromElement>(),
			&el.tag_literal,
		) {
			(Some(from_element), _) => reflect_value_ext::patch_to_value(
				(from_element.from_element)(el)?.as_partial_reflect(),
				self.types,
			)?,
			(None, Some(named)) => {
				let registration = self
					.registration(&named.name)
					.cloned()
					.unwrap_or_else(|| registration.clone());
				self.named_value(named, &registration)?
			}
			(None, None) => {
				Self::assert_attributes_are_fields(el, info)?;
				self.attributes_value(el, registration)?
			}
		};
		let holds = registration.data::<ReflectHoldChildren>();
		if let Some(holds) = holds {
			self.hold_children(el, holds, &mut value)?;
		}
		self.entity(key).insert(info.type_path(), value);
		if registration.data::<ReflectFromElement>().is_none() {
			self.field_bindings(el, key)?;
		}
		for (prop, binding) in bound_props {
			self.prop_binding(key, &prop, &binding)?;
		}
		self.spreads(el, key)?;
		match (holds, is_instance) {
			(Some(_), _) => Ok(()),
			(None, true) => self.children(&el.children, key, Some(&el.tag)),
			(None, false) => self.children(&el.children, key, None),
		}
	}

	/// The binding valued props of an [`Instance`], each left null (or its
	/// `=init`) in the element, so the props document holds every key a use
	/// passes before its source lands.
	fn take_bound_props(
		el: &BsxElement,
	) -> Result<(
		alloc::borrow::Cow<'_, BsxElement>,
		Vec<(SmolStr, BindingExpr)>,
	)> {
		const PROPS: &str = "props";
		let mut owned = el.clone();
		let fields = owned
			.attributes
			.iter_mut()
			.filter(|attr| attr.key == PROPS)
			.find_map(|attr| match &mut attr.value {
				AttrValue::Expr(ValueExpr::Literal(DataLiteral::Struct(
					fields,
				))) => Some(fields),
				_ => None,
			});
		let mut bound = Vec::new();
		for (prop, literal) in fields.into_iter().flatten() {
			let DataLiteral::Binding(binding) = literal else {
				continue;
			};
			let seed = match &binding.init {
				Some(init) => literal_to_value(init)?,
				None => Value::Null,
			};
			bound.push((prop.clone(), (**binding).clone()));
			*literal = DataLiteral::Scalar(seed);
		}
		match bound.is_empty() {
			true => (alloc::borrow::Cow::Borrowed(el), bound),
			false => (alloc::borrow::Cow::Owned(owned), bound),
		}
		.xok()
	}

	/// Lower `el`'s children into the component holding them, each group a
	/// body of its own in the field its slot names.
	fn hold_children(
		&self,
		el: &BsxElement,
		holds: &ReflectHoldChildren,
		value: &mut Value,
	) -> Result {
		let mut groups = Vec::<(Option<SmolStr>, Vec<BsxNode>)>::new();
		for child in &el.children {
			let (slot, child) = match child {
				BsxNode::Element(child) => {
					let mut child = child.clone();
					let slot = child.slot_routing().and_then(|slot| slot.name);
					child.take_attribute("slot");
					(slot, BsxNode::Element(child))
				}
				child => (None, child.clone()),
			};
			match groups.iter_mut().find(|(name, _)| *name == slot) {
				Some((_, nodes)) => nodes.push(child),
				None => groups.push((slot, vec![child])),
			}
		}
		let Value::Map(fields) = value else {
			bevybail!("`<{}>` holds its children and has no fields", el.tag);
		};
		for (slot, nodes) in groups {
			if !nodes.iter().any(BsxNode::is_significant) {
				continue;
			}
			let field = holds.field(slot.as_deref()).ok_or_else(|| {
				bevyhow!(
					"`<{}>` has no slot `{}`: {}",
					el.tag,
					slot.as_deref().unwrap_or_default(),
					holds.describe()
				)
			})?;
			let body = Lowering::lower(
				nodes,
				LowerRoot::Container(default()),
				self.parsers,
				self.types,
			)?;
			fields.insert(field, body.to_value()?);
		}
		Ok(())
	}

	/// Lower the binding valued attribute of a component tag: the source's
	/// components and a [`ReflectFieldRef`] naming the component's field, so
	/// the two sync both ways. An entity carries one such binding, the first
	/// owning it.
	fn field_bindings(&mut self, el: &BsxElement, key: &EntityKey) -> Result {
		let Some((attr, binding)) =
			el.plain_attributes().find_map(|attr| match &attr.value {
				AttrValue::Expr(ValueExpr::Binding(binding)) => {
					Some((attr, binding))
				}
				_ => None,
			})
		else {
			return Ok(());
		};
		if binding.source == BindingSource::Comp {
			bevybail!(
				"`{}=@comp:..` cannot bind a component field to another component field",
				attr.key,
			);
		}
		self.binding(binding, key, None)?;
		let component = el.tag.rsplit("::").next().unwrap_or(&el.tag);
		self.insert(key, &ReflectFieldRef::new(component, attr.key.as_str()))
	}

	/// Lower a tag nothing is registered under: an entry keyed by the name as
	/// written, its attributes or literal the value, the spawn warning and
	/// holding it, with the tag's spreads and children lowered as for any
	/// other. Structure is universal, so the entity exists in every binary
	/// and only its behavior is missing.
	fn unregistered(&mut self, el: &BsxElement, key: &EntityKey) -> Result {
		let value = match &el.tag_literal {
			Some(named) => self.named_to_held(named),
			None => self.attributes_to_held(el),
		};
		self.entity(key).insert(el.tag.as_str(), value);
		self.spreads(el, key)?;
		self.children(&el.children, key, None)
	}

	/// Lower every bare spread of `el` onto the entity keyed `key`.
	fn spreads(&mut self, el: &BsxElement, key: &EntityKey) -> Result {
		for attr in &el.attributes {
			match &attr.value {
				AttrValue::Spread(SpreadExpr::Named(named)) => {
					self.spread(named, key)?
				}
				AttrValue::Spread(SpreadExpr::Tuple(items)) => {
					for item in items {
						match item {
							SpreadItem::Named(named) => {
								self.spread(named, key)?
							}
							SpreadItem::Binding(binding) => {
								self.binding(binding, key, None)?
							}
						}
					}
				}
				_ => {}
			}
		}
		Ok(())
	}

	/// Lower one named spread: a component, a template's invocation, a
	/// resource, or a name nothing registered.
	fn spread(&mut self, named: &NamedLiteral, key: &EntityKey) -> Result {
		let Some(registration) = self.registration(&named.name).cloned() else {
			let value = self.named_to_held(named);
			self.entity(key).insert(named.name.clone(), value);
			return Ok(());
		};
		// a type that is none of the three has no meaning on an entity
		NameKind::of(&registration)?;
		let value = self.named_value(named, &registration)?;
		self.entity(key)
			.insert(registration.type_info().type_path(), value);
		Ok(())
	}
}
