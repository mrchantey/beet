//! A workbook read as tables: each sheet a `<table>` of its rows and cells.
use super::ooxml_query::*;
use crate::prelude::*;
use beet_core::prelude::*;
use ooxmlsdk::parts::PartRef;

type Ns = OoxmlNamespace;

/// Reads a workbook's sheets into the one tree.
pub(crate) struct WorkbookProjection {
	/// Whether each cell format, by index, unlocks its cells.
	unlocked: Vec<bool>,
	/// The shared strings, by index.
	strings: Vec<String>,
}

/// One sheet as the projection reads it.
#[derive(Default)]
struct SheetRead {
	/// The `sheetData` element, the sheet's table.
	data: Option<Entity>,
	/// Each column range's style, `(min, max, style)`.
	columns: Vec<(u32, u32, u32)>,
	rows: Vec<RowRead>,
	/// Every merged range, its first cell and its last, `(column, row)`.
	merges: Vec<((u32, u32), (u32, u32))>,
	/// Text nodes only the writer reads.
	hidden: Vec<Entity>,
	/// A value element's words, ie its shared string, by entity.
	shown: Vec<(Entity, String)>,
}

struct RowRead {
	entity: Entity,
	index: u32,
	/// The row's style when it carries one for its absent cells.
	style: Option<u32>,
	cells: Vec<CellRead>,
}

struct CellRead {
	entity: Entity,
	column: u32,
	style: u32,
	formula: Option<SmolStr>,
}

impl WorkbookProjection {
	/// Reads a workbook under `root`: every worksheet in tab order a part,
	/// read and projected, a chartsheet having no cells to read.
	pub(crate) fn build(
		world: &mut World,
		root: Entity,
		file: &OoxmlFile,
	) -> Result {
		let workbook = file
			.parts()
			.into_iter()
			.find(|part| matches!(part, PartRef::WorkbookPart(_)))
			.ok_or_else(|| bevyhow!("the workbook has no workbook part"))?;
		let related = file.related(&workbook);
		let find = |is: fn(&PartRef) -> bool| {
			related
				.iter()
				.find(|(_, part)| is(part))
				.map(|(_, part)| part)
		};
		let read = |part: Option<&PartRef>| -> Result<Option<Vec<BsxNode>>> {
			part.map(|part| file.data(part))
				.transpose()?
				.flatten()
				.map(OoxmlFile::read_xml)
				.transpose()
		};
		let projection = Self {
			unlocked: read(find(|part| {
				matches!(part, PartRef::WorkbookStylesPart(_))
			}))?
			.map(|nodes| Self::unlocked_styles(&nodes))
			.unwrap_or_default(),
			strings: read(find(|part| {
				matches!(part, PartRef::SharedStringTablePart(_))
			}))?
			.map(|nodes| Self::shared_strings(&nodes))
			.unwrap_or_default(),
		};
		let workbook_xml = read(Some(&workbook))?.unwrap_or_default();
		let sheets = BsxNode::document_element(&workbook_xml)
			.and_then(|root| root.child("sheets"))
			.map(|sheets| {
				sheets
					.children_named("sheet")
					.filter_map(|sheet| {
						Some((
							SmolStr::new(sheet.attribute("name")?),
							SmolStr::new(sheet.prefixed_attribute("id")?),
						))
					})
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		for (name, id) in sheets {
			let Some((_, part)) = related.iter().find(|(related_id, part)| {
				*related_id == id && matches!(part, PartRef::WorksheetPart(_))
			}) else {
				continue;
			};
			let entity = OoxmlParser::spawn_part(world, root, file, part)?;
			projection.project(world, entity, name);
		}
		Ok(())
	}

	/// Whether each cell format of `cellXfs` unlocks its cells: a cell is
	/// locked unless its format says otherwise.
	fn unlocked_styles(styles: &[BsxNode]) -> Vec<bool> {
		BsxNode::document_element(styles)
			.and_then(|root| root.child("cellXfs"))
			.map(|formats| {
				formats
					.children_named("xf")
					.map(|format| {
						format
							.child("protection")
							.and_then(|protection| {
								protection.attribute("locked")
							})
							.is_some_and(|locked| {
								matches!(locked, "0" | "false")
							})
					})
					.collect()
			})
			.unwrap_or_default()
	}

	/// The shared strings, each its text or its runs' text joined.
	fn shared_strings(table: &[BsxNode]) -> Vec<String> {
		BsxNode::document_element(table)
			.map(|root| {
				root.children_named("si")
					.map(|item| match item.child("t") {
						Some(text) => text.text(),
						None => item
							.children_named("r")
							.filter_map(|run| run.child("t"))
							.map(|text| text.text())
							.collect(),
					})
					.collect()
			})
			.unwrap_or_default()
	}

	/// Projects one sheet: its data a `<table>` captioned with its tab name,
	/// its rows `<tr>`s, its cells `<td>`s with their addresses, locks and
	/// formulas, a merged range's first cell spanning the rest, and every
	/// gap before a row's last cell filled by a projection-only `<td>`.
	fn project(&self, world: &mut World, part: Entity, name: SmolStr) {
		let sheet =
			world.with_state::<OoxmlQuery, _>(|tree| self.read(&tree, part));
		let Some(data) = sheet.data else {
			return;
		};
		Projected::apply(
			world,
			sheet
				.hidden
				.iter()
				.map(|entity| Projected::Hide(*entity))
				.collect(),
		);
		for (entity, text) in sheet.shown.iter().cloned() {
			world.entity_mut(entity).insert(Value::Str(text.into()));
		}
		Tagged::new("table").insert(world, data);
		let caption = world
			.spawn((Element::new("caption"), children![Value::Str(
				name.clone()
			)]))
			.id();
		world.entity_mut(data).insert_child(0, caption);
		// each merged cell's range's first, and each first's span
		let mut masters = HashMap::<(u32, u32), (u32, u32)>::default();
		let mut spans = HashMap::<(u32, u32), (u32, u32)>::default();
		for (first, last) in &sheet.merges {
			spans.insert(*first, (last.0 - first.0 + 1, last.1 - first.1 + 1));
			for row in first.1..=last.1 {
				for column in first.0..=last.0 {
					if (column, row) != *first {
						masters.insert((column, row), *first);
					}
				}
			}
		}
		let mut cells = HashMap::<(u32, u32), Entity>::default();
		let mut covered = Vec::<(Entity, (u32, u32))>::new();
		for row in &sheet.rows {
			Tagged::new("tr").insert(world, row.entity);
			let present = row
				.cells
				.iter()
				.map(|cell| (cell.column, cell))
				.collect::<HashMap<_, _>>();
			let last =
				row.cells.iter().map(|cell| cell.column).max().unwrap_or(0);
			for column in 1..=last {
				let entity = match present.get(&column) {
					Some(cell) => {
						if let Some(formula) = &cell.formula {
							world
								.entity_mut(cell.entity)
								.insert(CellFormula(formula.clone()));
						}
						self.lock(world, cell.entity, cell.style);
						cell.entity
					}
					None => {
						let style = self.inherited_style(&sheet, row, column);
						let gap = world.spawn(SheetCellStyle(style)).id();
						let index =
							Self::position(world, row.entity, &present, column);
						world.entity_mut(row.entity).insert_child(index, gap);
						self.lock(world, gap, style);
						gap
					}
				};
				world.entity_mut(entity).insert(SheetCellAddress {
					sheet: name.clone(),
					column,
					row: row.index,
				});
				cells.insert((column, row.index), entity);
				match (
					masters.get(&(column, row.index)),
					spans.get(&(column, row.index)),
				) {
					(Some(master), _) => covered.push((entity, *master)),
					(None, Some((columns, rows))) => {
						let mut cell = Tagged::new("td");
						if *columns > 1 {
							cell = cell.with("colspan", columns.to_string());
						}
						if *rows > 1 {
							cell = cell.with("rowspan", rows.to_string());
						}
						cell.insert(world, entity);
					}
					(None, None) => Tagged::new("td").insert(world, entity),
				}
			}
		}
		// a covered cell shows its range's first, and has no `<td>` of its own
		for (entity, master) in covered {
			if let Some(master) = cells.get(&master) {
				world.entity_mut(entity).insert(CoveredBy(*master));
			}
		}
	}

	/// The index in `row`'s children a gap at `column` is inserted at:
	/// before the first present cell past it.
	fn position(
		world: &World,
		row: Entity,
		present: &HashMap<u32, &CellRead>,
		column: u32,
	) -> usize {
		let after = present
			.iter()
			.filter(|(present, _)| **present > column)
			.min_by_key(|(present, _)| **present)
			.map(|(_, cell)| cell.entity);
		let children = world
			.entity(row)
			.get::<Children>()
			.map(|children| children.to_vec())
			.unwrap_or_default();
		after
			.and_then(|after| children.iter().position(|child| *child == after))
			.unwrap_or(children.len())
	}

	/// Locks `cell` unless its format unlocks it.
	fn lock(&self, world: &mut World, cell: Entity, style: u32) {
		if !self.unlocked.get(style as usize).copied().unwrap_or(false) {
			world.entity_mut(cell).insert(CellLocked);
		}
	}

	/// The style a cell absent from the sheet is drawn with: its row's when
	/// the row carries one, else its column's.
	fn inherited_style(
		&self,
		sheet: &SheetRead,
		row: &RowRead,
		column: u32,
	) -> u32 {
		row.style.unwrap_or_else(|| {
			sheet
				.columns
				.iter()
				.find(|(min, max, _)| *min <= column && column <= *max)
				.map(|(.., style)| *style)
				.unwrap_or(0)
		})
	}

	fn read(&self, tree: &OoxmlQuery, part: Entity) -> SheetRead {
		let mut sheet = SheetRead::default();
		let number = |entity: Entity, key: &str| {
			tree.attribute(entity, None, key)
				.and_then(|value| value.parse::<u32>().ok())
		};
		for entity in tree.descendants(part) {
			let Some(element) = tree.element(entity) else {
				// text is a value's or an inline string's, anything else
				// only the writer reads
				let read = tree
					.parent(entity)
					.and_then(|parent| tree.element(parent))
					.is_some_and(|parent| {
						matches!(parent.local_name(), "t" | "v")
					});
				if tree.is_text(entity) && !read {
					sheet.hidden.push(entity);
				}
				continue;
			};
			if element.namespace.as_deref() != Some(Ns::SPREADSHEET) {
				continue;
			}
			match element.local_name() {
				"sheetData" => sheet.data = Some(entity),
				"col" => {
					if let (Some(min), Some(max)) =
						(number(entity, "min"), number(entity, "max"))
					{
						sheet.columns.push((
							min,
							max,
							number(entity, "style").unwrap_or(0),
						));
					}
				}
				"mergeCell" => {
					let range = tree
						.attribute(entity, None, "ref")
						.and_then(|range| range.split_once(':'))
						.and_then(|(first, last)| {
							Some((
								SheetCellAddress::parse_a1(first)?,
								SheetCellAddress::parse_a1(last)?,
							))
						});
					if let Some(range) = range {
						sheet.merges.push(range);
					}
				}
				"row" => {
					let custom = matches!(
						tree.attribute(entity, None, "customFormat"),
						Some("1" | "true")
					);
					sheet.rows.push(RowRead {
						entity,
						index: number(entity, "r")
							.unwrap_or(sheet.rows.len() as u32 + 1),
						style: custom.then(|| number(entity, "s").unwrap_or(0)),
						cells: Vec::new(),
					});
				}
				"c" => {
					let Some(row) = sheet.rows.last_mut() else {
						continue;
					};
					let column = tree
						.attribute(entity, None, "r")
						.and_then(SheetCellAddress::parse_a1)
						.map(|(column, _)| column)
						.unwrap_or_else(|| {
							row.cells.last().map_or(1, |cell| cell.column + 1)
						});
					let formula = tree
						.child(entity, Ns::SPREADSHEET, "f")
						.map(|formula| SmolStr::new(tree.text(formula)));
					let value = tree.child(entity, Ns::SPREADSHEET, "v");
					let kind = tree.attribute(entity, None, "t");
					match (kind, value) {
						(Some("s"), Some(value)) => {
							let text = tree.text(value);
							sheet.hidden.extend(Self::texts(tree, value));
							let shown = text
								.trim()
								.parse::<usize>()
								.ok()
								.and_then(|index| self.strings.get(index))
								.cloned()
								.unwrap_or_default();
							sheet.shown.push((value, shown));
						}
						(Some("b"), Some(value)) => {
							let shown = match tree.text(value).trim() {
								"1" => "true",
								_ => "false",
							};
							sheet.hidden.extend(Self::texts(tree, value));
							sheet.shown.push((value, shown.into()));
						}
						_ => {}
					}
					row.cells.push(CellRead {
						entity,
						column,
						style: number(entity, "s").unwrap_or(0),
						formula,
					});
				}
				_ => {}
			}
		}
		sheet
	}

	/// The text nodes under `entity`.
	fn texts(tree: &OoxmlQuery, entity: Entity) -> Vec<Entity> {
		tree.descendants(entity)
			.into_iter()
			.filter(|descendant| tree.is_text(*descendant))
			.collect()
	}
}
