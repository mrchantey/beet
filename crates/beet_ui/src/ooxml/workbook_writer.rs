//! [`WorkbookWriter`], a workbook's edited cells written back.
use super::node::OoxmlRead;
use super::ooxml_query::*;
use super::workbook::SheetCellStyle;
use super::writer::*;
use crate::prelude::*;
use beet_core::prelude::*;
use ooxmlsdk::parts::PartRef;

type Ns = OoxmlNamespace;

/// The re-projection of a workbook's edits.
pub(super) struct WorkbookWriter;

impl WorkbookWriter {
	/// Writes every edited cell, answering whether any was.
	pub fn reproject(world: &mut World, root: Entity) -> Result<bool> {
		let cells = world.with_state::<(
			Query<&Children>,
			Query<(&CellAddress, Option<&SheetCellStyle>)>,
			Query<&OoxmlNode>,
			Query<(), With<Value>>,
		), _>(|(children, cells, nodes, values)| {
			children
				.iter_descendants_depth_first(root)
				.filter_map(|entity| {
					let (address, style) = cells.get(entity).ok()?;
					let texts = children
						.iter_descendants(entity)
						.chain([entity])
						.filter(|node| values.contains(*node))
						.collect::<Vec<_>>();
					Some((
						entity,
						address.clone(),
						style.map(|style| style.0),
						nodes.contains(entity),
						texts,
					))
				})
				.collect::<Vec<_>>()
		});
		let mut any = false;
		for (cell, address, style, is_written, texts) in cells {
			let changed = match is_written {
				true => {
					texts.iter().any(|text| OoxmlRead::edited(world, *text))
				}
				// a gap is a cell once anything is written in it
				false => !texts.is_empty(),
			};
			if !changed {
				continue;
			}
			if world.with_state::<Cells, _>(|cells| cells.is_locked(cell)) {
				bevybail!(
					"{address} is locked; only an unlocked cell takes a value"
				);
			}
			let value = world
				.with_state::<DocumentText, _>(|text| text.own_text(cell))
				.trim()
				.to_string();
			Self::write(world, cell, &address, style, &value);
			any = true;
		}
		Ok(any)
	}

	/// Writes `value` as `cell`'s only content, as a person typing it would:
	/// led by `=` its formula in an `f`, with no value until the book
	/// recalculates on open, a number in a `v`, anything else an inline
	/// string, a gap becoming the `c` it stands for.
	fn write(
		world: &mut World,
		cell: Entity,
		address: &CellAddress,
		style: Option<u32>,
		value: &str,
	) {
		let element = |entity: Entity| {
			world
				.entity(entity)
				.get::<OoxmlNode>()
				.and_then(OoxmlNode::element)
				.cloned()
		};
		let named = match element(cell) {
			Some(named) => named,
			None => {
				let row = world
					.entity(cell)
					.get::<ChildOf>()
					.and_then(|parent| element(parent.parent()))
					.unwrap_or_else(|| {
						OoxmlElement::new("row", Some(Ns::SPREADSHEET.into()))
					});
				let mut named = row.sibling(Ns::SPREADSHEET, "c");
				named.set_attribute(OoxmlAttribute::new(
					"r",
					None,
					address.a1(),
				));
				if let Some(style) = style.filter(|style| *style != 0) {
					named.set_attribute(OoxmlAttribute::new(
						"s",
						None,
						style.to_string(),
					));
				}
				named
			}
		};
		world.entity_mut(cell).despawn_children();
		let mut written = named.clone();
		let formula = value.strip_prefix('=').map(str::trim);
		let is_number = Self::is_number(value);
		match formula.is_some() || is_number {
			true => {
				written.remove_attribute(None, "t");
			}
			false => written.set_attribute(OoxmlAttribute::new(
				"t",
				None,
				"inlineStr",
			)),
		}
		world.entity_mut(cell).insert(OoxmlNode::Element(written));
		if let Some(formula) = formula {
			world.entity_mut(cell).insert(CellFormula(formula.into()));
			world.spawn((
				OoxmlNode::Element(named.sibling(Ns::SPREADSHEET, "f")),
				ChildOf(cell),
				children![Value::str(formula)],
			));
			return;
		}
		world.entity_mut(cell).remove::<CellFormula>();
		match is_number {
			true => {
				world.spawn((
					OoxmlNode::Element(named.sibling(Ns::SPREADSHEET, "v")),
					ChildOf(cell),
					children![Value::str(value)],
				));
			}
			false => {
				let mut text = named.sibling(Ns::SPREADSHEET, "t");
				text.set_attribute(OoxmlAttribute::new(
					"xml:space",
					Some(Ns::XML.into()),
					"preserve",
				));
				world.spawn((
					OoxmlNode::Element(named.sibling(Ns::SPREADSHEET, "is")),
					ChildOf(cell),
					children![(OoxmlNode::Element(text), children![
						Value::str(value)
					])],
				));
			}
		}
	}

	/// Whether `text` is a plain decimal, `-?digits` with an optional
	/// fraction.
	fn is_number(text: &str) -> bool {
		let digits = text.strip_prefix('-').unwrap_or(text);
		let (whole, fraction) = digits.split_once('.').unwrap_or((digits, "0"));
		!whole.is_empty()
			&& !fraction.is_empty()
			&& whole
				.chars()
				.chain(fraction.chars())
				.all(|char| char.is_ascii_digit())
	}

	/// Marks the workbook to recalculate every formula on open.
	pub fn recalculate(file: &mut OoxmlFile) -> Result {
		let Some(workbook) = file
			.parts()
			.into_iter()
			.find(|part| matches!(part, PartRef::WorkbookPart(_)))
		else {
			return Ok(());
		};
		let path = file.path(&workbook).unwrap_or_default();
		let Some(nodes) = file.xml_at(&path)? else {
			return Ok(());
		};
		let mut world = World::new();
		let part = OoxmlNode::spawn_part(&mut world, path.clone(), &nodes)?;
		let (book, calculation, after) =
			world.with_state::<OoxmlQuery, _>(|tree| {
				let book = tree
					.children(part)
					.into_iter()
					.find(|child| tree.is(*child, Ns::SPREADSHEET, "workbook"));
				let calculation = book.and_then(|book| {
					tree.child(book, Ns::SPREADSHEET, "calcPr")
				});
				// `calcPr` follows these in the schema's sequence
				let after = book.and_then(|book| {
					tree.children(book).into_iter().rposition(|child| {
						tree.element(child).is_some_and(|element| {
							matches!(
								element.local_name(),
								"fileVersion"
									| "fileSharing" | "workbookPr"
									| "workbookProtection" | "bookViews"
									| "sheets" | "functionGroups"
									| "externalReferences" | "definedNames"
							)
						})
					})
				});
				(book, calculation, after)
			});
		let Some(book) = book else {
			return Ok(());
		};
		let full = OoxmlAttribute::new("fullCalcOnLoad", None, "1");
		match calculation {
			Some(calculation) => {
				if let Some(mut node) =
					world.entity_mut(calculation).get_mut::<OoxmlNode>()
					&& let Some(element) = node.element_mut()
				{
					element.set_attribute(full);
				}
			}
			None => {
				let named = world
					.with_state::<OoxmlQuery, _>(|query| {
						query.element(book).cloned()
					})
					.ok_or_else(|| bevyhow!("the workbook part has no root"))?;
				let mut element = named.sibling(Ns::SPREADSHEET, "calcPr");
				element.set_attribute(full);
				let calculation = world.spawn(OoxmlNode::Element(element)).id();
				world.entity_mut(book).insert_child(
					after.map_or(0, |index| index + 1),
					calculation,
				);
			}
		}
		let text =
			world.with_state::<OoxmlWriter, _>(|writer| writer.write(part));
		file.set_data(&path, text.into_bytes())
	}
}

#[cfg(test)]
pub(crate) mod test {
	use super::super::renderer::test::*;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A protected sheet: `A1` a locked label, `D3:F3` an unlocked merged
	/// field holding a prompt, `D4` an unlocked formula, and column `G`
	/// unlocked by its column style where no cell is written.
	pub(crate) fn workbook() -> MediaBytes {
		sheet("<sheetProtection sheet=\"1\"/>")
	}

	/// [`workbook`]'s sheet with `protection` after its data, its cells
	/// locked only when that protects it.
	fn sheet(protection: &str) -> MediaBytes {
		OoxmlFile::workbook(
			"<xf/><xf><protection locked=\"0\"/></xf>",
			&[(
				"Start Here",
				&format!("<cols><col min=\"7\" max=\"7\" style=\"1\"/></cols><sheetData>\
				 <row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>Business name</t></is></c></row>\
				 <row r=\"3\"><c r=\"D3\" s=\"1\" t=\"inlineStr\"><is><t>Enter it here</t></is></c>\
				 <c r=\"E3\" s=\"1\"/><c r=\"F3\" s=\"1\"/><c r=\"H3\"/></row>\
				 <row r=\"4\"><c r=\"D4\" s=\"1\"><f>1+1</f><v>2</v></c></row>\
				 </sheetData>{protection}<mergeCells count=\"1\"><mergeCell ref=\"D3:F3\"/></mergeCells>"),
			)],
		)
		.unwrap()
	}

	/// An unprotected sheet locks nothing, whatever its formats say.
	#[beet_core::test]
	fn unprotected_sheets_lock_nothing() {
		let (mut world, root) = parse(&sheet(""));
		CellText::listing(&mut world, root)
			.into_iter()
			.map(|cell| cell.address.to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"Start Here!A1".to_string(),
				"Start Here!A3".into(),
				"Start Here!B3".into(),
				"Start Here!C3".into(),
				"Start Here!D3".into(),
				"Start Here!G3".into(),
				"Start Here!H3".into(),
				"Start Here!A4".into(),
				"Start Here!B4".into(),
				"Start Here!C4".into(),
				"Start Here!D4".into(),
			]);
		for cell in ["Start Here!A1", "Start Here!H3"] {
			edit(&mut world, |edit| {
				edit.set_cell_text(
					root,
					&CellAddress::parse(cell).unwrap(),
					"x",
				)
			})
			.unwrap();
		}
	}

	/// A protected sheet's locked cells say so in HTML, and its markdown,
	/// which has no say over editing, reads as an unprotected one's.
	#[beet_core::test]
	fn renders_locked_cells() {
		fn render(
			bytes: &MediaBytes,
			mut renderer: impl DocRenderer,
		) -> String {
			let (mut world, root) = parse(bytes);
			renderer
				.render(&mut RenderContext::new(
					&mut world,
					root,
					&RequestParts::default(),
				))
				.unwrap()
				.to_string()
		}
		render(&workbook(), HtmlRenderer::new())
			.xpect_starts_with("<table><caption>Start Here</caption>")
			.xpect_contains("<td contenteditable=\"false\">Business name</td>")
			.xpect_contains("<td colspan=\"3\">Enter it here</td><td></td><td contenteditable=\"false\"></td>")
			.xpect_contains("<td>2</td>");
		render(&workbook(), MarkdownRenderer::new())
			.xpect_eq(render(&sheet(""), MarkdownRenderer::new()));
	}

	#[beet_core::test]
	fn lists_and_sets_unlocked_cells() {
		let (mut world, root) = parse(&workbook());
		CellText::listing(&mut world, root)
			.iter()
			.map(ToString::to_string)
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"| Start Here!D3 | Enter it here |".to_string(),
				"| Start Here!G3 | (empty) |".into(),
				"| Start Here!D4 | =1+1 |".into(),
			]);
		let set = |world: &mut World, cell: &str, text: &str| {
			edit(world, |edit| {
				edit.set_cell_text(
					root,
					&CellAddress::parse(cell).unwrap(),
					text,
				)
			})
		};
		// a merged range lists once, and any slot it covers writes its first
		let written = set(&mut world, "Start Here!E3", "Acme Stalls").unwrap();
		world
			.entity(written)
			.get::<CellAddress>()
			.unwrap()
			.to_string()
			.xpect_eq("Start Here!D3");
		set(&mut world, "Start Here!G3", "120.5").unwrap();
		set(&mut world, "Start Here!A1", "x")
			.unwrap_err()
			.to_string()
			.xpect_contains("is locked");
		// a formula replaces a formula, computed when the book is next opened
		set(&mut world, "Start Here!D4", "=G3*2").unwrap();
		set(&mut world, "Nowhere!A1", "x").xpect_err();

		let bytes = write(&mut world, root, MediaType::Xlsx);
		part(&bytes, "xl/worksheets/sheet1.xml")
			.xpect_contains("<c r=\"D3\" s=\"1\" t=\"inlineStr\"><is><t xml:space=\"preserve\">Acme Stalls</t></is></c>")
			// the gap's cell, created with its column's style
			.xpect_contains("<c r=\"G3\" s=\"1\"><v>120.5</v></c><c r=\"H3\"/>")
			.xpect_contains("<c r=\"D4\" s=\"1\"><f>G3*2</f></c>");
		part(&bytes, "xl/workbook.xml")
			.xpect_contains("<calcPr calcId=\"191029\" fullCalcOnLoad=\"1\"/>");
		let (mut world, root) = parse(&bytes);
		cells(&mut world, root).xpect_eq(vec![
			"Acme Stalls".to_string(),
			"120.5".into(),
			"=G3*2".into(),
		]);
	}
}
