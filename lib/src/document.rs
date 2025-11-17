use std::collections::HashMap;

use bitflags::bitflags;

use crate::utils::text::display_len;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerType {
	Heading1,
	Heading2,
	Heading3,
	Heading4,
	Heading5,
	Heading6,
	PageBreak,
	SectionBreak,
	TocItem,
	Link,
	List,
	ListItem,
}

impl MarkerType {
	#[must_use]
	pub const fn to_int(&self) -> i32 {
		match self {
			Self::Heading1 => 0,
			Self::Heading2 => 1,
			Self::Heading3 => 2,
			Self::Heading4 => 3,
			Self::Heading5 => 4,
			Self::Heading6 => 5,
			Self::PageBreak => 6,
			Self::SectionBreak => 7,
			Self::TocItem => 8,
			Self::Link => 9,
			Self::List => 10,
			Self::ListItem => 11,
		}
	}

	#[must_use]
	pub const fn from_int(value: i32) -> Option<Self> {
		match value {
			0 => Some(Self::Heading1),
			1 => Some(Self::Heading2),
			2 => Some(Self::Heading3),
			3 => Some(Self::Heading4),
			4 => Some(Self::Heading5),
			5 => Some(Self::Heading6),
			6 => Some(Self::PageBreak),
			7 => Some(Self::SectionBreak),
			8 => Some(Self::TocItem),
			9 => Some(Self::Link),
			10 => Some(Self::List),
			11 => Some(Self::ListItem),
			_ => None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct Marker {
	pub marker_type: MarkerType,
	pub position: usize,
	pub text: String,
	pub reference: String,
	pub level: i32,
}

impl Marker {
	#[must_use]
	pub const fn new(marker_type: MarkerType, position: usize) -> Self {
		Self { marker_type, position, text: String::new(), reference: String::new(), level: 0 }
	}

	#[must_use]
	pub fn with_text(mut self, text: String) -> Self {
		self.text = text;
		self
	}

	#[must_use]
	pub fn with_reference(mut self, reference: String) -> Self {
		self.reference = reference;
		self
	}

	#[must_use]
	pub const fn with_level(mut self, level: i32) -> Self {
		self.level = level;
		self
	}
}

#[derive(Debug, Clone)]
pub struct DocumentBuffer {
	pub content: String,
	pub markers: Vec<Marker>,
	content_display_len: usize,
}

impl DocumentBuffer {
	#[must_use]
	pub const fn new() -> Self {
		Self { content: String::new(), markers: Vec::new(), content_display_len: 0 }
	}

	#[must_use]
	pub fn with_content(content: String) -> Self {
		let len = display_len(&content);
		Self { content, markers: Vec::new(), content_display_len: len }
	}

	pub fn add_marker(&mut self, marker: Marker) {
		self.markers.push(marker);
	}

	pub fn append(&mut self, text: &str) {
		self.content.push_str(text);
		self.content_display_len += display_len(text);
	}

	#[must_use]
	pub const fn current_position(&self) -> usize {
		self.content_display_len
	}
}

impl Default for DocumentBuffer {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug, Clone)]
pub struct TocItem {
	pub name: String,
	pub reference: String,
	pub offset: usize,
	pub children: Vec<TocItem>,
}

impl TocItem {
	#[must_use]
	pub const fn new(name: String, reference: String, offset: usize) -> Self {
		Self { name, reference, offset, children: Vec::new() }
	}

	pub fn add_child(&mut self, child: Self) {
		self.children.push(child);
	}
}

#[derive(Debug, Clone, Default)]
pub struct DocumentStats {
	pub word_count: usize,
	pub line_count: usize,
	pub char_count: usize,
}

impl DocumentStats {
	#[must_use]
	pub fn from_text(text: &str) -> Self {
		let char_count = text.chars().count();
		let line_count = text.lines().count();
		let word_count = text.split_whitespace().count();
		Self { word_count, line_count, char_count }
	}
}

#[derive(Debug, Clone)]
pub struct Document {
	pub title: String,
	pub author: String,
	pub buffer: DocumentBuffer,
	pub toc_items: Vec<TocItem>,
	pub id_positions: HashMap<String, usize>,
	pub spine_items: Vec<String>,
	pub manifest_items: HashMap<String, String>,
	pub stats: DocumentStats,
}

impl Document {
	#[must_use]
	pub fn new() -> Self {
		Self {
			title: String::new(),
			author: String::new(),
			buffer: DocumentBuffer::new(),
			toc_items: Vec::new(),
			id_positions: HashMap::new(),
			spine_items: Vec::new(),
			manifest_items: HashMap::new(),
			stats: DocumentStats::default(),
		}
	}

	#[must_use]
	pub fn with_title(mut self, title: String) -> Self {
		self.title = title;
		self
	}

	#[must_use]
	pub fn with_author(mut self, author: String) -> Self {
		self.author = author;
		self
	}

	pub fn set_buffer(&mut self, buffer: DocumentBuffer) {
		self.buffer = buffer;
	}

	pub fn compute_stats(&mut self) {
		self.stats = DocumentStats::from_text(&self.buffer.content);
	}
}

impl Default for Document {
	fn default() -> Self {
		Self::new()
	}
}

bitflags! {
	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
	pub struct ParserFlags: u32 {
		const NONE = 0;
		const SUPPORTS_SECTIONS = 1 << 0;
		const SUPPORTS_TOC = 1 << 1;
		const SUPPORTS_PAGES = 1 << 2;
		const SUPPORTS_LISTS = 1 << 3;
	}
}

#[derive(Debug, Clone)]
pub struct ParserContext {
	pub file_path: String,
	pub password: Option<String>,
}

impl ParserContext {
	#[must_use]
	pub const fn new(file_path: String) -> Self {
		Self { file_path, password: None }
	}

	#[must_use]
	pub fn with_password(mut self, password: String) -> Self {
		self.password = Some(password);
		self
	}
}
