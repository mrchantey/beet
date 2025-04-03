/// Concatenates each provided string, separated by a space.
/// Handles String, &str, and Option<String>/Option<&str> values.
#[macro_export]
macro_rules! csx {
    ($($class:expr),*) => {
        {
            let mut builder = ClassBuilder::new();
            $(
                builder.add($class);
            )*
            builder.build()
        }
    };
}



/// Builder for CSS class strings
pub struct ClassBuilder {
	classes: String,
}

impl ClassBuilder {
	pub fn new() -> Self {
		Self {
			classes: String::new(),
		}
	}

	pub fn add<'a, T: Into<ClassPart<'a>>>(&mut self, part: T) {
		if let Some(value) = part.into().value() {
			if !value.is_empty() {
				if !self.classes.is_empty() {
					self.classes.push(' ');
				}
				self.classes.push_str(&value);
			}
		}
	}

	pub fn build(self) -> String { self.classes }
}


/// Wrapper that can represent a potential class part
pub enum ClassPart<'a> {
	Str(&'a str),
	/// If passed a string we need to own it
	String(String),
	None,
}

impl<'a> ClassPart<'a> {
	/// Returns the value of the class part, or None if it is None
	pub fn value(&self) -> Option<&str> {
		match self {
			ClassPart::Str(val) => Some(val),
			ClassPart::String(val) => Some(val),
			ClassPart::None => None,
		}
	}
}
impl<'a> From<String> for ClassPart<'a> {
	fn from(val: String) -> Self { Self::String(val) }
}
impl<'a> From<Option<String>> for ClassPart<'a> {
	fn from(val: Option<String>) -> Self {
		match val {
			Some(val) => Self::String(val),
			None => Self::None,
		}
	}
}

impl<'a> From<&'a str> for ClassPart<'a> {
	fn from(val: &'a str) -> Self { Self::Str(val) }
}
impl<'a> From<&'a mut str> for ClassPart<'a> {
	fn from(val: &'a mut str) -> Self { Self::Str(val) }
}

impl<'a> From<Option<&'a str>> for ClassPart<'a> {
	fn from(val: Option<&'a str>) -> Self {
		match val {
			Some(val) => Self::Str(val),
			None => Self::None,
		}
	}
}
impl<'a> From<Option<&'a mut str>> for ClassPart<'a> {
	fn from(val: Option<&'a mut str>) -> Self {
		match val {
			Some(val) => Self::Str(val),
			None => Self::None,
		}
	}
}

impl<'a> From<&'a Option<String>> for ClassPart<'a> {
	fn from(val: &'a Option<String>) -> Self {
		match val {
			Some(val) => Self::Str(val.as_str()),
			None => Self::None,
		}
	}
}
impl<'a> From<&'a Option<&'a str>> for ClassPart<'a> {
	fn from(val: &'a Option<&'a str>) -> Self {
		match val {
			Some(val) => Self::Str(val),
			None => Self::None,
		}
	}
}
impl<'a> From<&'a mut Option<String>> for ClassPart<'a> {
	fn from(val: &'a mut Option<String>) -> Self {
		match val {
			Some(val) => Self::Str(val.as_str()),
			None => Self::None,
		}
	}
}
impl<'a> From<&'a mut Option<&'a str>> for ClassPart<'a> {
	fn from(val: &'a mut Option<&'a str>) -> Self {
		match val {
			Some(val) => Self::Str(val),
			None => Self::None,
		}
	}
}
