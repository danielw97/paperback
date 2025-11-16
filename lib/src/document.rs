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
	pub fn to_int(&self) -> i32 {
		match self {
			MarkerType::Heading1 => 0,
			MarkerType::Heading2 => 1,
			MarkerType::Heading3 => 2,
			MarkerType::Heading4 => 3,
			MarkerType::Heading5 => 4,
			MarkerType::Heading6 => 5,
			MarkerType::PageBreak => 6,
			MarkerType::SectionBreak => 7,
			MarkerType::TocItem => 8,
			MarkerType::Link => 9,
			MarkerType::List => 10,
			MarkerType::ListItem => 11,
		}
	}

	pub fn from_int(value: i32) -> Option<Self> {
		match value {
			0 => Some(MarkerType::Heading1),
			1 => Some(MarkerType::Heading2),
			2 => Some(MarkerType::Heading3),
			3 => Some(MarkerType::Heading4),
			4 => Some(MarkerType::Heading5),
			5 => Some(MarkerType::Heading6),
			6 => Some(MarkerType::PageBreak),
			7 => Some(MarkerType::SectionBreak),
			8 => Some(MarkerType::TocItem),
			9 => Some(MarkerType::Link),
			10 => Some(MarkerType::List),
			11 => Some(MarkerType::ListItem),
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
	pub fn new(marker_type: MarkerType, position: usize) -> Self {
		Self { marker_type, position, text: String::new(), reference: String::new(), level: 0 }
	}

	pub fn with_text(mut self, text: String) -> Self {
		self.text = text;
		self
	}

	pub fn with_reference(mut self, reference: String) -> Self {
		self.reference = reference;
		self
	}

	pub fn with_level(mut self, level: i32) -> Self {
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
	pub fn new() -> Self {
		Self { content: String::new(), markers: Vec::new(), content_display_len: 0 }
	}

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

	pub fn current_position(&self) -> usize {
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
	pub fn new(name: String, reference: String, offset: usize) -> Self {
		Self { name, reference, offset, children: Vec::new() }
	}

	pub fn add_child(&mut self, child: TocItem) {
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

	pub fn with_title(mut self, title: String) -> Self {
		self.title = title;
		self
	}

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
	pub fn new(file_path: String) -> Self {
		Self { file_path, password: None }
	}

	pub fn with_password(mut self, password: String) -> Self {
		self.password = Some(password);
		self
	}
}
