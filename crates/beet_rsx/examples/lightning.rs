use lightningcss::printer::PrinterOptions;
use lightningcss::selector::Component;
use lightningcss::stylesheet::ParserOptions;
use lightningcss::stylesheet::StyleSheet;
use parcel_selectors::attr::AttrSelectorOperator;
use parcel_selectors::attr::ParsedCaseSensitivity;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ScopedStyles {
	component_styles: HashMap<String, String>,
	component_ids: HashMap<String, String>,
}

impl ScopedStyles {
	pub fn new() -> Self {
		ScopedStyles {
			component_styles: HashMap::new(),
			component_ids: HashMap::new(),
		}
	}

	// Generate a unique component ID
	fn generate_component_id(&self, component_name: &str) -> String {
		// let mut rng = rand::thread_rng();
		// let random_suffix: u32 = rng.gen();
		component_name.to_string()
	}

	// Add styles for a component
	pub fn add_component_styles(
		&mut self,
		component_name: &str,
		css: &str,
	) -> Result<String, String> {
		let component_id = self.generate_component_id(component_name);

		// Parse the CSS
		let mut stylesheet =
			StyleSheet::parse(css, ParserOptions::default())
				.map_err(|e| format!("Failed to parse CSS: {:?}", e))?;

		// Transform selectors to add data attribute
		stylesheet.rules.0.iter_mut().for_each(|rule| {
			if let lightningcss::rules::CssRule::Style(style_rule) = rule {
				style_rule.selectors.0.iter_mut().for_each(|selector| {
					// Add data-beet-cid attribute selector to each selector
					selector.append(Component::AttributeInNoNamespace {
						local_name: "data-beet-cid".into(),
						operator: AttrSelectorOperator::Equal,
						value: component_id.clone().into(),
						case_sensitivity: ParsedCaseSensitivity::CaseSensitive,
						never_matches: false,
					});
				});
			}
		});

		// Print the transformed CSS
		let printed = stylesheet
			.to_css(PrinterOptions::default())
			.map_err(|e| format!("Failed to print CSS: {:?}", e))?;

		self.component_styles
			.insert(component_name.to_string(), printed.code);
		self.component_ids
			.insert(component_name.to_string(), component_id.clone());

		Ok(component_id)
	}

	// Get the component ID
	pub fn get_component_id(&self, component_name: &str) -> Option<&String> {
		self.component_ids.get(component_name)
	}

	// Get the transformed styles for a component
	pub fn get_component_styles(
		&self,
		component_name: &str,
	) -> Option<&String> {
		self.component_styles.get(component_name)
	}

	// Get all styles combined
	pub fn get_all_styles(&self) -> String {
		self.component_styles
			.values()
			.cloned()
			.collect::<Vec<_>>()
			.join("\n\n")
	}
}

// Helper function to add component ID to HTML
pub fn apply_scope_to_html(html: &str, component_id: &str) -> String {
	// This is a simple implementation - you might want to use a proper HTML parser
	let root_element_end = html.find('>').unwrap_or(html.len());
	let (start, end) = html.split_at(root_element_end);

	format!("{} data-beet-cid=\"{}\">{}", start, component_id, end)
}

fn main() {
	let mut styles = ScopedStyles::new();

	// Test adding component styles
	let css = ".button { color: blue; }";
	let component_id = styles.add_component_styles("Button", css).unwrap();

	// Verify data attribute selector is added
	let scoped_styles = styles.get_component_styles("Button").unwrap();
	assert!(
		scoped_styles.contains(&format!("data-beet-cid=\"{}\"", component_id))
	);

	// Test HTML scoping
	let html = "<div>Hello</div>";
	let scoped_html = apply_scope_to_html(html, &component_id);
	assert!(
		scoped_html.contains(&format!("data-beet-cid=\"{}\"", component_id))
	);
	println!("{}", scoped_html);
	println!("{}", scoped_styles);
}
