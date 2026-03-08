//! Media type classification for HTTP content negotiation.
//!
//! [`MediaType`] represents common IANA media types used in HTTP
//! `Content-Type` and `Accept` headers. The term "media type" is the
//! current IANA standard, replacing the older "MIME type" terminology.
//! [MDN - Media Types](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/MIME_types)

use crate::prelude::*;

/// Common media types used in HTTP exchange.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum MediaType {
	/// `application/octet-stream` — raw bytes, the default.
	#[default]
	Bytes,
	/// `text/plain`
	Text,
	/// `text/html`
	Html,
	/// `application/xml` or `text/xml`
	Xml,
	/// `application/json`
	Json,
	/// `application/x-postcard`
	Postcard,
	/// `text/markdown`
	Markdown,
	/// `text/event-stream` — Server-Sent Events.
	EventStream,
	/// `text/css`
	Css,
	/// `application/javascript`
	Javascript,
	/// `image/png`
	Png,
	/// `image/jpeg`
	Jpeg,
	/// `image/gif`
	Gif,
	/// `image/webp`
	Webp,
	/// `image/svg+xml`
	Svg,
	/// `image/x-icon`
	Ico,
	/// `image/avif`
	Avif,
	/// `image/bmp`
	Bmp,
	/// `image/tiff`
	Tiff,
	/// `application/pdf`
	Pdf,
	/// `application/zip`
	Zip,
	/// `application/gzip`
	Gzip,
	/// `application/x-tar`
	Tar,
	/// `application/wasm`
	Wasm,
	/// `font/woff`
	Woff,
	/// `font/woff2`
	Woff2,
	/// `font/ttf`
	Ttf,
	/// `font/otf`
	Otf,
	/// `audio/mpeg`
	Mp3,
	/// `audio/ogg`
	Ogg,
	/// `audio/wav`
	Wav,
	/// `audio/flac`
	Flac,
	/// `audio/aac`
	Aac,
	/// `audio/webm`
	AudioWebm,
	/// `video/mp4`
	Mp4,
	/// `video/webm`
	VideoWebm,
	/// `video/ogg`
	VideoOgg,
	/// `application/x-yaml` or `text/yaml`
	Yaml,
	/// `text/csv`
	Csv,
	/// `application/toml`
	Toml,
	/// `application/x-www-form-urlencoded`
	FormUrlEncoded,
	/// `multipart/form-data`
	FormData,
	/// `application/x-protobuf`
	Protobuf,
	/// `application/msgpack`
	MessagePack,
	/// `application/x-sh`
	Shell,
	/// `text/x-rust`
	Rust,
	/// `text/x-python`
	Python,
	/// `application/x-typescript`
	TypeScript,
	/// `text/x-c`
	C,
	/// `text/x-c++`
	Cpp,
	/// `text/x-java`
	Java,
	/// `application/sql`
	Sql,
	/// `application/graphql`
	GraphQl,
	/// An unrecognized media type.
	Other(String),
}

impl MediaType {
	// ── Canonical strings ──────────────────────────────────────────
	const JSON: &'static str = "application/json";
	const POSTCARD: &'static str = "application/x-postcard";
	const TEXT: &'static str = "text/plain";
	const HTML: &'static str = "text/html";
	const XML: &'static str = "application/xml";
	const MARKDOWN: &'static str = "text/markdown";
	const BYTES: &'static str = "application/octet-stream";
	const EVENT_STREAM: &'static str = "text/event-stream";
	const CSS: &'static str = "text/css";
	const JAVASCRIPT: &'static str = "application/javascript";
	const PNG: &'static str = "image/png";
	const JPEG: &'static str = "image/jpeg";
	const GIF: &'static str = "image/gif";
	const WEBP: &'static str = "image/webp";
	const SVG: &'static str = "image/svg+xml";
	const ICO: &'static str = "image/x-icon";
	const AVIF: &'static str = "image/avif";
	const BMP: &'static str = "image/bmp";
	const TIFF: &'static str = "image/tiff";
	const PDF: &'static str = "application/pdf";
	const ZIP: &'static str = "application/zip";
	const GZIP: &'static str = "application/gzip";
	const TAR: &'static str = "application/x-tar";
	const WASM: &'static str = "application/wasm";
	const WOFF: &'static str = "font/woff";
	const WOFF2: &'static str = "font/woff2";
	const TTF: &'static str = "font/ttf";
	const OTF: &'static str = "font/otf";
	const MP3: &'static str = "audio/mpeg";
	const OGG: &'static str = "audio/ogg";
	const WAV: &'static str = "audio/wav";
	const FLAC: &'static str = "audio/flac";
	const AAC: &'static str = "audio/aac";
	const AUDIO_WEBM: &'static str = "audio/webm";
	const MP4: &'static str = "video/mp4";
	const VIDEO_WEBM: &'static str = "video/webm";
	const VIDEO_OGG: &'static str = "video/ogg";
	const YAML: &'static str = "application/x-yaml";
	const CSV: &'static str = "text/csv";
	const TOML: &'static str = "application/toml";
	const FORM_URL_ENCODED: &'static str = "application/x-www-form-urlencoded";
	const FORM_DATA: &'static str = "multipart/form-data";
	const PROTOBUF: &'static str = "application/x-protobuf";
	const MESSAGE_PACK: &'static str = "application/msgpack";
	const SHELL: &'static str = "application/x-sh";
	const RUST: &'static str = "text/x-rust";
	const PYTHON: &'static str = "text/x-python";
	const TYPESCRIPT: &'static str = "application/x-typescript";
	const C_LANG: &'static str = "text/x-c";
	const CPP_LANG: &'static str = "text/x-c++";
	const JAVA: &'static str = "text/x-java";
	const SQL: &'static str = "application/sql";
	const GRAPHQL: &'static str = "application/graphql";

	/// Parse a media type from a content-type string.
	///
	/// Strips parameters like `; charset=utf-8` before matching.
	pub fn from_content_type(content_type: &str) -> Self {
		let raw = content_type
			.split(';')
			.next()
			.unwrap_or(content_type)
			.trim();
		match raw {
			val if val.contains(Self::JSON) => MediaType::Json,
			val if val.contains(Self::POSTCARD) => MediaType::Postcard,
			val if val.contains(Self::HTML) => MediaType::Html,
			val if val.contains(Self::MARKDOWN) => MediaType::Markdown,
			val if val.contains(Self::EVENT_STREAM) => MediaType::EventStream,
			val if val.contains("text/xml") || val.contains(Self::XML) => {
				MediaType::Xml
			}
			val if val.contains(Self::SVG) => MediaType::Svg,
			val if val.contains(Self::TEXT) => MediaType::Text,
			val if val.contains(Self::BYTES) => MediaType::Bytes,
			val if val.contains(Self::CSS) => MediaType::Css,
			val if val.contains(Self::JAVASCRIPT) => MediaType::Javascript,
			val if val.contains(Self::PNG) => MediaType::Png,
			val if val.contains(Self::JPEG) => MediaType::Jpeg,
			val if val.contains(Self::GIF) => MediaType::Gif,
			val if val.contains(Self::WEBP) => MediaType::Webp,
			val if val.contains(Self::ICO) => MediaType::Ico,
			val if val.contains(Self::AVIF) => MediaType::Avif,
			val if val.contains(Self::BMP) => MediaType::Bmp,
			val if val.contains(Self::TIFF) => MediaType::Tiff,
			val if val.contains(Self::PDF) => MediaType::Pdf,
			val if val.contains(Self::ZIP) => MediaType::Zip,
			val if val.contains(Self::GZIP) => MediaType::Gzip,
			val if val.contains(Self::TAR) => MediaType::Tar,
			val if val.contains(Self::WASM) => MediaType::Wasm,
			val if val.contains(Self::WOFF2) => MediaType::Woff2,
			val if val.contains(Self::WOFF) => MediaType::Woff,
			val if val.contains(Self::TTF) => MediaType::Ttf,
			val if val.contains(Self::OTF) => MediaType::Otf,
			val if val.contains(Self::MP3) => MediaType::Mp3,
			val if val.contains(Self::FLAC) => MediaType::Flac,
			val if val.contains(Self::AAC) => MediaType::Aac,
			val if val.contains(Self::AUDIO_WEBM) => MediaType::AudioWebm,
			val if val.contains(Self::OGG) => MediaType::Ogg,
			val if val.contains(Self::WAV) => MediaType::Wav,
			val if val.contains(Self::MP4) => MediaType::Mp4,
			val if val.contains(Self::VIDEO_WEBM) => MediaType::VideoWebm,
			val if val.contains(Self::VIDEO_OGG) => MediaType::VideoOgg,
			val if val.contains(Self::YAML) || val.contains("text/yaml") => {
				MediaType::Yaml
			}
			val if val.contains(Self::CSV) => MediaType::Csv,
			val if val.contains(Self::TOML) => MediaType::Toml,
			val if val.contains(Self::FORM_URL_ENCODED) => {
				MediaType::FormUrlEncoded
			}
			val if val.contains(Self::FORM_DATA) => MediaType::FormData,
			val if val.contains(Self::PROTOBUF) => MediaType::Protobuf,
			val if val.contains(Self::MESSAGE_PACK) => MediaType::MessagePack,
			val if val.contains(Self::SHELL) => MediaType::Shell,
			val if val.contains(Self::RUST) => MediaType::Rust,
			val if val.contains(Self::PYTHON) => MediaType::Python,
			val if val.contains(Self::TYPESCRIPT) => MediaType::TypeScript,
			val if val.contains(Self::CPP_LANG) => MediaType::Cpp,
			val if val.contains(Self::C_LANG) => MediaType::C,
			val if val.contains(Self::JAVA) => MediaType::Java,
			val if val.contains(Self::SQL) => MediaType::Sql,
			val if val.contains(Self::GRAPHQL) => MediaType::GraphQl,
			other => MediaType::Other(other.to_string()),
		}
	}

	/// Infer the media type from a file extension.
	///
	/// The extension should be provided without a leading dot, ie `"html"`
	/// not `".html"`. Returns [`MediaType::Bytes`] for unrecognized extensions.
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// assert_eq!(MediaType::from_extension("html"), MediaType::Html);
	/// assert_eq!(MediaType::from_extension("json"), MediaType::Json);
	/// assert_eq!(MediaType::from_extension("wasm"), MediaType::Wasm);
	/// assert_eq!(MediaType::from_extension("xyz"), MediaType::Bytes);
	/// ```
	pub fn from_extension(ext: &str) -> Self {
		match ext.to_ascii_lowercase().as_str() {
			// text
			"txt" | "text" | "log" => MediaType::Text,
			"html" | "htm" => MediaType::Html,
			"css" => MediaType::Css,
			"js" | "mjs" | "cjs" => MediaType::Javascript,
			"json" | "jsonl" | "geojson" => MediaType::Json,
			"xml" | "xsl" | "xsd" => MediaType::Xml,
			"md" | "markdown" => MediaType::Markdown,
			"csv" => MediaType::Csv,
			"yaml" | "yml" => MediaType::Yaml,
			"toml" => MediaType::Toml,
			"sql" => MediaType::Sql,
			"graphql" | "gql" => MediaType::GraphQl,
			// images
			"png" => MediaType::Png,
			"jpg" | "jpeg" | "jfif" => MediaType::Jpeg,
			"gif" => MediaType::Gif,
			"webp" => MediaType::Webp,
			"svg" => MediaType::Svg,
			"ico" => MediaType::Ico,
			"avif" => MediaType::Avif,
			"bmp" => MediaType::Bmp,
			"tif" | "tiff" => MediaType::Tiff,
			// fonts
			"woff" => MediaType::Woff,
			"woff2" => MediaType::Woff2,
			"ttf" => MediaType::Ttf,
			"otf" => MediaType::Otf,
			// audio
			"mp3" => MediaType::Mp3,
			"ogg" | "oga" | "opus" => MediaType::Ogg,
			"wav" => MediaType::Wav,
			"flac" => MediaType::Flac,
			"aac" | "m4a" => MediaType::Aac,
			// video
			"mp4" | "m4v" => MediaType::Mp4,
			"webm" => MediaType::VideoWebm,
			"ogv" => MediaType::VideoOgg,
			// archives
			"zip" => MediaType::Zip,
			"gz" | "gzip" => MediaType::Gzip,
			"tar" => MediaType::Tar,
			// documents
			"pdf" => MediaType::Pdf,
			// binary / app
			"wasm" => MediaType::Wasm,
			"postcard" => MediaType::Postcard,
			"protobuf" | "proto" => MediaType::Protobuf,
			"msgpack" => MediaType::MessagePack,
			// source
			"sh" | "bash" | "zsh" | "fish" => MediaType::Shell,
			"rs" => MediaType::Rust,
			"py" => MediaType::Python,
			"ts" | "tsx" | "mts" | "cts" => MediaType::TypeScript,
			"c" | "h" => MediaType::C,
			"cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => MediaType::Cpp,
			"java" => MediaType::Java,
			// jsx is still javascript
			"jsx" => MediaType::Javascript,
			_ => MediaType::Bytes,
		}
	}

	/// Infer the media type from a file path by extracting its extension.
	///
	/// Returns [`MediaType::Bytes`] if the path has no extension or
	/// the extension is unrecognized.
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// assert_eq!(MediaType::from_path("index.html"), MediaType::Html);
	/// assert_eq!(MediaType::from_path("/assets/style.css"), MediaType::Css);
	/// assert_eq!(MediaType::from_path("no_extension"), MediaType::Bytes);
	/// ```
	pub fn from_path(path: impl AsRef<std::path::Path>) -> Self {
		path.as_ref()
			.extension()
			.and_then(|ext| ext.to_str())
			.map(Self::from_extension)
			.unwrap_or(MediaType::Bytes)
	}

	/// The canonical media type string for this type.
	pub fn as_str(&self) -> &str {
		match self {
			MediaType::Bytes => Self::BYTES,
			MediaType::Text => Self::TEXT,
			MediaType::Html => Self::HTML,
			MediaType::Xml => Self::XML,
			MediaType::Json => Self::JSON,
			MediaType::Postcard => Self::POSTCARD,
			MediaType::Markdown => Self::MARKDOWN,
			MediaType::EventStream => Self::EVENT_STREAM,
			MediaType::Css => Self::CSS,
			MediaType::Javascript => Self::JAVASCRIPT,
			MediaType::Png => Self::PNG,
			MediaType::Jpeg => Self::JPEG,
			MediaType::Gif => Self::GIF,
			MediaType::Webp => Self::WEBP,
			MediaType::Svg => Self::SVG,
			MediaType::Ico => Self::ICO,
			MediaType::Avif => Self::AVIF,
			MediaType::Bmp => Self::BMP,
			MediaType::Tiff => Self::TIFF,
			MediaType::Pdf => Self::PDF,
			MediaType::Zip => Self::ZIP,
			MediaType::Gzip => Self::GZIP,
			MediaType::Tar => Self::TAR,
			MediaType::Wasm => Self::WASM,
			MediaType::Woff => Self::WOFF,
			MediaType::Woff2 => Self::WOFF2,
			MediaType::Ttf => Self::TTF,
			MediaType::Otf => Self::OTF,
			MediaType::Mp3 => Self::MP3,
			MediaType::Ogg => Self::OGG,
			MediaType::Wav => Self::WAV,
			MediaType::Flac => Self::FLAC,
			MediaType::Aac => Self::AAC,
			MediaType::AudioWebm => Self::AUDIO_WEBM,
			MediaType::Mp4 => Self::MP4,
			MediaType::VideoWebm => Self::VIDEO_WEBM,
			MediaType::VideoOgg => Self::VIDEO_OGG,
			MediaType::Yaml => Self::YAML,
			MediaType::Csv => Self::CSV,
			MediaType::Toml => Self::TOML,
			MediaType::FormUrlEncoded => Self::FORM_URL_ENCODED,
			MediaType::FormData => Self::FORM_DATA,
			MediaType::Protobuf => Self::PROTOBUF,
			MediaType::MessagePack => Self::MESSAGE_PACK,
			MediaType::Shell => Self::SHELL,
			MediaType::Rust => Self::RUST,
			MediaType::Python => Self::PYTHON,
			MediaType::TypeScript => Self::TYPESCRIPT,
			MediaType::C => Self::C_LANG,
			MediaType::Cpp => Self::CPP_LANG,
			MediaType::Java => Self::JAVA,
			MediaType::Sql => Self::SQL,
			MediaType::GraphQl => Self::GRAPHQL,
			MediaType::Other(val) => val.as_str(),
		}
	}

	/// Whether this is a serializable format (JSON or Postcard).
	pub fn is_serializable(&self) -> bool {
		matches!(self, MediaType::Json | MediaType::Postcard)
	}

	/// Whether this is a text-based format that can be displayed as a string.
	pub fn is_text(&self) -> bool {
		matches!(
			self,
			MediaType::Text
				| MediaType::Html
				| MediaType::Xml
				| MediaType::Json
				| MediaType::Markdown
				| MediaType::EventStream
				| MediaType::Css
				| MediaType::Javascript
				| MediaType::Svg
				| MediaType::Yaml
				| MediaType::Csv
				| MediaType::Toml
				| MediaType::Sql
				| MediaType::GraphQl
				| MediaType::Shell
				| MediaType::Rust
				| MediaType::Python
				| MediaType::TypeScript
				| MediaType::C
				| MediaType::Cpp
				| MediaType::Java
		)
	}
}

impl From<&str> for MediaType {
	fn from(value: &str) -> Self { MediaType::from_content_type(value) }
}

impl From<String> for MediaType {
	fn from(value: String) -> Self { MediaType::from_content_type(&value) }
}

impl Into<Vec<MediaType>> for MediaType {
	fn into(self) -> Vec<MediaType> { vec![self] }
}

impl core::fmt::Display for MediaType {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		write!(formatter, "{}", self.as_str())
	}
}

/// Deprecated alias for [`MediaType`].
#[allow(missing_docs)]
pub type MimeType = MediaType;

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn from_content_type_json() {
		MediaType::from_content_type("application/json")
			.xpect_eq(MediaType::Json);
	}

	#[test]
	fn from_content_type_json_with_charset() {
		MediaType::from_content_type("application/json; charset=utf-8")
			.xpect_eq(MediaType::Json);
	}

	#[test]
	fn from_content_type_postcard() {
		MediaType::from_content_type("application/x-postcard")
			.xpect_eq(MediaType::Postcard);
	}

	#[test]
	fn from_content_type_html() {
		MediaType::from_content_type("text/html").xpect_eq(MediaType::Html);
	}

	#[test]
	fn from_content_type_text() {
		MediaType::from_content_type("text/plain").xpect_eq(MediaType::Text);
	}

	#[test]
	fn from_content_type_xml() {
		MediaType::from_content_type("application/xml")
			.xpect_eq(MediaType::Xml);
		MediaType::from_content_type("text/xml").xpect_eq(MediaType::Xml);
	}

	#[test]
	fn from_content_type_markdown() {
		MediaType::from_content_type("text/markdown")
			.xpect_eq(MediaType::Markdown);
	}

	#[test]
	fn from_content_type_bytes() {
		MediaType::from_content_type("application/octet-stream")
			.xpect_eq(MediaType::Bytes);
	}

	#[test]
	fn from_content_type_event_stream() {
		MediaType::from_content_type("text/event-stream")
			.xpect_eq(MediaType::EventStream);
	}

	#[test]
	fn from_content_type_unknown() {
		MediaType::from_content_type("application/x-custom")
			.xpect_eq(MediaType::Other("application/x-custom".to_string()));
	}

	#[test]
	fn from_content_type_image_types() {
		MediaType::from_content_type("image/jpeg").xpect_eq(MediaType::Jpeg);
		MediaType::from_content_type("image/gif").xpect_eq(MediaType::Gif);
		MediaType::from_content_type("image/webp").xpect_eq(MediaType::Webp);
		MediaType::from_content_type("image/svg+xml").xpect_eq(MediaType::Svg);
		MediaType::from_content_type("image/x-icon").xpect_eq(MediaType::Ico);
		MediaType::from_content_type("image/avif").xpect_eq(MediaType::Avif);
	}

	#[test]
	fn from_content_type_font_types() {
		MediaType::from_content_type("font/woff2").xpect_eq(MediaType::Woff2);
		MediaType::from_content_type("font/woff").xpect_eq(MediaType::Woff);
		MediaType::from_content_type("font/ttf").xpect_eq(MediaType::Ttf);
	}

	#[test]
	fn from_content_type_audio_video() {
		MediaType::from_content_type("audio/mpeg").xpect_eq(MediaType::Mp3);
		MediaType::from_content_type("video/mp4").xpect_eq(MediaType::Mp4);
		MediaType::from_content_type("video/webm")
			.xpect_eq(MediaType::VideoWebm);
	}

	#[test]
	fn from_content_type_yaml() {
		MediaType::from_content_type("application/x-yaml")
			.xpect_eq(MediaType::Yaml);
		MediaType::from_content_type("text/yaml").xpect_eq(MediaType::Yaml);
	}

	#[test]
	fn from_extension_common() {
		MediaType::from_extension("html").xpect_eq(MediaType::Html);
		MediaType::from_extension("htm").xpect_eq(MediaType::Html);
		MediaType::from_extension("css").xpect_eq(MediaType::Css);
		MediaType::from_extension("js").xpect_eq(MediaType::Javascript);
		MediaType::from_extension("json").xpect_eq(MediaType::Json);
		MediaType::from_extension("png").xpect_eq(MediaType::Png);
		MediaType::from_extension("jpg").xpect_eq(MediaType::Jpeg);
		MediaType::from_extension("jpeg").xpect_eq(MediaType::Jpeg);
		MediaType::from_extension("gif").xpect_eq(MediaType::Gif);
		MediaType::from_extension("svg").xpect_eq(MediaType::Svg);
		MediaType::from_extension("pdf").xpect_eq(MediaType::Pdf);
		MediaType::from_extension("wasm").xpect_eq(MediaType::Wasm);
		MediaType::from_extension("txt").xpect_eq(MediaType::Text);
		MediaType::from_extension("md").xpect_eq(MediaType::Markdown);
	}

	#[test]
	fn from_extension_case_insensitive() {
		MediaType::from_extension("HTML").xpect_eq(MediaType::Html);
		MediaType::from_extension("Json").xpect_eq(MediaType::Json);
		MediaType::from_extension("PNG").xpect_eq(MediaType::Png);
	}

	#[test]
	fn from_extension_fonts() {
		MediaType::from_extension("woff").xpect_eq(MediaType::Woff);
		MediaType::from_extension("woff2").xpect_eq(MediaType::Woff2);
		MediaType::from_extension("ttf").xpect_eq(MediaType::Ttf);
		MediaType::from_extension("otf").xpect_eq(MediaType::Otf);
	}

	#[test]
	fn from_extension_audio_video() {
		MediaType::from_extension("mp3").xpect_eq(MediaType::Mp3);
		MediaType::from_extension("ogg").xpect_eq(MediaType::Ogg);
		MediaType::from_extension("wav").xpect_eq(MediaType::Wav);
		MediaType::from_extension("mp4").xpect_eq(MediaType::Mp4);
		MediaType::from_extension("webm").xpect_eq(MediaType::VideoWebm);
	}

	#[test]
	fn from_extension_archives() {
		MediaType::from_extension("zip").xpect_eq(MediaType::Zip);
		MediaType::from_extension("gz").xpect_eq(MediaType::Gzip);
		MediaType::from_extension("tar").xpect_eq(MediaType::Tar);
	}

	#[test]
	fn from_extension_source() {
		MediaType::from_extension("rs").xpect_eq(MediaType::Rust);
		MediaType::from_extension("py").xpect_eq(MediaType::Python);
		MediaType::from_extension("ts").xpect_eq(MediaType::TypeScript);
		MediaType::from_extension("c").xpect_eq(MediaType::C);
		MediaType::from_extension("cpp").xpect_eq(MediaType::Cpp);
		MediaType::from_extension("java").xpect_eq(MediaType::Java);
		MediaType::from_extension("sh").xpect_eq(MediaType::Shell);
	}

	#[test]
	fn from_extension_data() {
		MediaType::from_extension("yaml").xpect_eq(MediaType::Yaml);
		MediaType::from_extension("yml").xpect_eq(MediaType::Yaml);
		MediaType::from_extension("csv").xpect_eq(MediaType::Csv);
		MediaType::from_extension("toml").xpect_eq(MediaType::Toml);
		MediaType::from_extension("sql").xpect_eq(MediaType::Sql);
	}

	#[test]
	fn from_extension_unknown() {
		MediaType::from_extension("xyz").xpect_eq(MediaType::Bytes);
		MediaType::from_extension("").xpect_eq(MediaType::Bytes);
	}

	#[test]
	fn from_path_works() {
		MediaType::from_path("index.html").xpect_eq(MediaType::Html);
		MediaType::from_path("/assets/style.css").xpect_eq(MediaType::Css);
		MediaType::from_path("app.wasm").xpect_eq(MediaType::Wasm);
		MediaType::from_path("no_extension").xpect_eq(MediaType::Bytes);
		MediaType::from_path("archive.tar").xpect_eq(MediaType::Tar);
	}

	#[test]
	fn as_str_roundtrip() {
		let types = vec![
			MediaType::Bytes,
			MediaType::Text,
			MediaType::Html,
			MediaType::Xml,
			MediaType::Json,
			MediaType::Postcard,
			MediaType::Markdown,
			MediaType::EventStream,
			MediaType::Css,
			MediaType::Javascript,
			MediaType::Png,
			MediaType::Jpeg,
			MediaType::Gif,
			MediaType::Webp,
			MediaType::Svg,
			MediaType::Pdf,
			MediaType::Wasm,
			MediaType::Woff2,
			MediaType::Mp3,
			MediaType::Mp4,
			MediaType::Yaml,
			MediaType::Csv,
			MediaType::Toml,
		];
		for media_type in types {
			MediaType::from_content_type(media_type.as_str())
				.xpect_eq(media_type);
		}
	}

	#[test]
	fn default_is_bytes() { MediaType::default().xpect_eq(MediaType::Bytes); }

	#[test]
	fn display() {
		format!("{}", MediaType::Json).xpect_eq("application/json");
		format!("{}", MediaType::EventStream).xpect_eq("text/event-stream");
		format!("{}", MediaType::Wasm).xpect_eq("application/wasm");
	}

	#[test]
	fn is_serializable() {
		MediaType::Json.is_serializable().xpect_true();
		MediaType::Postcard.is_serializable().xpect_true();
		MediaType::Html.is_serializable().xpect_false();
		MediaType::Text.is_serializable().xpect_false();
		MediaType::EventStream.is_serializable().xpect_false();
	}

	#[test]
	fn is_text() {
		MediaType::Html.is_text().xpect_true();
		MediaType::Json.is_text().xpect_true();
		MediaType::Css.is_text().xpect_true();
		MediaType::Rust.is_text().xpect_true();
		MediaType::Png.is_text().xpect_false();
		MediaType::Bytes.is_text().xpect_false();
		MediaType::Wasm.is_text().xpect_false();
	}
}
