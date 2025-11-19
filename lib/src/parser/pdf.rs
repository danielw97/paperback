use std::{
	ffi::{CStr, CString, c_void},
	ptr,
};

use anyhow::{Result, anyhow, bail};

use crate::{
	document::{Document, DocumentBuffer, Marker, MarkerType, ParserContext, ParserFlags, TocItem},
	parser::{PASSWORD_REQUIRED_ERROR_PREFIX, Parser, utils::extract_title_from_path},
	utils::text::{collapse_whitespace, trim_string},
};

pub struct PdfParser;

impl Parser for PdfParser {
	fn name(&self) -> &'static str {
		"PDF Documents"
	}

	fn extensions(&self) -> &[&str] {
		&["pdf"]
	}

	fn supported_flags(&self) -> ParserFlags {
		ParserFlags::SUPPORTS_PAGES | ParserFlags::SUPPORTS_TOC
	}

	fn parse(&self, context: &ParserContext) -> Result<Document> {
		let _library = PdfiumLibrary::new();
		let document = PdfDocument::load(&context.file_path, context.password.as_deref())?;
		let mut buffer = DocumentBuffer::new();
		let mut page_offsets = Vec::new();
		let page_count = document.page_count()?;
		for page_index in 0..page_count {
			let marker_position = buffer.current_position();
			buffer.add_marker(
				Marker::new(MarkerType::PageBreak, marker_position).with_text(format!("Page {}", page_index + 1)),
			);
			page_offsets.push(marker_position);
			let page = match document.load_page(page_index) {
				Some(page) => page,
				None => continue,
			};
			if let Some(text_page) = page.load_text_page() {
				let raw_text = text_page.extract_text();
				let lines = process_text_lines(&raw_text);
				for line in lines {
					buffer.append(&line);
					buffer.append("\n");
				}
			}
		}
		let title =
			document.extract_metadata(b"Title\0").unwrap_or_else(|| extract_title_from_path(&context.file_path));
		let author = document.extract_metadata(b"Author\0").unwrap_or_default();
		let toc_items = document.extract_toc(&page_offsets);
		let mut doc = Document::new();
		doc.set_buffer(buffer);
		doc.title = title;
		doc.author = author;
		doc.toc_items = toc_items;
		Ok(doc)
	}
}

struct PdfiumLibrary;

impl PdfiumLibrary {
	fn new() -> Self {
		unsafe {
			ffi::FPDF_InitLibrary();
		}
		Self
	}
}

impl Drop for PdfiumLibrary {
	fn drop(&mut self) {
		unsafe {
			ffi::FPDF_DestroyLibrary();
		}
	}
}

struct PdfDocument {
	handle: ffi::FPDF_DOCUMENT,
}

impl PdfDocument {
	fn load(path: &str, password: Option<&str>) -> Result<Self> {
		let path_cstr = CString::new(path).map_err(|_| anyhow!("PDF path contains embedded NUL bytes"))?;
		let password_cstr = match password {
			Some(pwd) if !pwd.is_empty() => {
				Some(CString::new(pwd).map_err(|_| anyhow!("PDF password contains embedded NUL bytes"))?)
			}
			_ => None,
		};
		let handle = unsafe {
			ffi::FPDF_LoadDocument(path_cstr.as_ptr(), password_cstr.as_ref().map_or(ptr::null(), |pwd| pwd.as_ptr()))
		};
		if handle.is_null() {
			return Err(map_pdfium_error("Failed to open PDF document"));
		}
		Ok(Self { handle })
	}

	fn page_count(&self) -> Result<i32> {
		let count = unsafe { ffi::FPDF_GetPageCount(self.handle) };
		if count < 0 {
			bail!("Failed to read page count");
		}
		Ok(count)
	}

	fn load_page(&self, index: i32) -> Option<PdfPage> {
		let handle = unsafe { ffi::FPDF_LoadPage(self.handle, index) };
		if handle.is_null() { None } else { Some(PdfPage { handle }) }
	}

	fn extract_metadata(&self, tag: &[u8]) -> Option<String> {
		let tag_cstr = CStr::from_bytes_with_nul(tag).ok()?;
		let length = unsafe { ffi::FPDF_GetMetaText(self.handle, tag_cstr.as_ptr(), ptr::null_mut(), 0) };
		if length <= 2 {
			return None;
		}
		let mut buffer = vec![0u16; length as usize / 2];
		let written = unsafe {
			ffi::FPDF_GetMetaText(self.handle, tag_cstr.as_ptr(), buffer.as_mut_ptr() as *mut c_void, length)
		};
		if written <= 2 {
			return None;
		}
		sanitize_utf16_buffer(&buffer, written)
	}

	fn extract_toc(&self, page_offsets: &[usize]) -> Vec<TocItem> {
		let first = unsafe { ffi::FPDFBookmark_GetFirstChild(self.handle, ptr::null_mut()) };
		if first.is_null() {
			return Vec::new();
		}
		extract_outline_items(self.handle, first, page_offsets)
	}
}

impl Drop for PdfDocument {
	fn drop(&mut self) {
		if !self.handle.is_null() {
			unsafe {
				ffi::FPDF_CloseDocument(self.handle);
			}
		}
	}
}

struct PdfPage {
	handle: ffi::FPDF_PAGE,
}

impl PdfPage {
	fn load_text_page(&self) -> Option<PdfTextPage> {
		let handle = unsafe { ffi::FPDFText_LoadPage(self.handle) };
		if handle.is_null() { None } else { Some(PdfTextPage { handle }) }
	}
}

impl Drop for PdfPage {
	fn drop(&mut self) {
		if !self.handle.is_null() {
			unsafe {
				ffi::FPDF_ClosePage(self.handle);
			}
		}
	}
}

struct PdfTextPage {
	handle: ffi::FPDF_TEXTPAGE,
}

impl PdfTextPage {
	fn extract_text(&self) -> String {
		let char_count = unsafe { ffi::FPDFText_CountChars(self.handle) };
		if char_count <= 0 {
			return String::new();
		}
		let mut buffer = vec![0u16; (char_count + 1) as usize];
		let written = unsafe { ffi::FPDFText_GetText(self.handle, 0, char_count, buffer.as_mut_ptr()) };
		if written <= 1 {
			return String::new();
		}
		let actual_len = (written as usize).saturating_sub(1);
		buffer.truncate(actual_len);
		String::from_utf16_lossy(&buffer)
	}
}

impl Drop for PdfTextPage {
	fn drop(&mut self) {
		if !self.handle.is_null() {
			unsafe {
				ffi::FPDFText_ClosePage(self.handle);
			}
		}
	}
}

fn process_text_lines(raw_text: &str) -> Vec<String> {
	raw_text
		.lines()
		.filter_map(|line| {
			let collapsed = collapse_whitespace(line);
			let trimmed = trim_string(&collapsed);
			if trimmed.is_empty() { None } else { Some(trimmed) }
		})
		.collect()
}

fn extract_outline_items(
	document: ffi::FPDF_DOCUMENT,
	mut bookmark: ffi::FPDF_BOOKMARK,
	page_offsets: &[usize],
) -> Vec<TocItem> {
	let mut items = Vec::new();
	while !bookmark.is_null() {
		let name = read_bookmark_title(bookmark).unwrap_or_default();
		let offset = unsafe {
			let dest = ffi::FPDFBookmark_GetDest(document, bookmark);
			if dest.is_null() {
				usize::MAX
			} else {
				let page_index = ffi::FPDFDest_GetDestPageIndex(document, dest);
				if page_index < 0 {
					usize::MAX
				} else {
					page_offsets.get(page_index as usize).copied().unwrap_or(usize::MAX)
				}
			}
		};
		let mut toc_item = TocItem::new(name, String::new(), offset);
		let child = unsafe { ffi::FPDFBookmark_GetFirstChild(document, bookmark) };
		if !child.is_null() {
			toc_item.children = extract_outline_items(document, child, page_offsets);
		}
		items.push(toc_item);
		bookmark = unsafe { ffi::FPDFBookmark_GetNextSibling(document, bookmark) };
	}
	items
}

fn read_bookmark_title(bookmark: ffi::FPDF_BOOKMARK) -> Option<String> {
	let length = unsafe { ffi::FPDFBookmark_GetTitle(bookmark, ptr::null_mut(), 0) };
	if length <= 2 {
		return None;
	}
	let mut buffer = vec![0u16; length as usize / 2];
	let written = unsafe { ffi::FPDFBookmark_GetTitle(bookmark, buffer.as_mut_ptr() as *mut c_void, length) };
	if written <= 2 {
		return None;
	}
	sanitize_utf16_buffer(&buffer, written)
}

fn sanitize_utf16_buffer(buffer: &[u16], written_bytes: u32) -> Option<String> {
	let total_units = (written_bytes as usize / 2).saturating_sub(1);
	if total_units == 0 {
		return None;
	}
	buffer.get(..total_units).map(|slice| String::from_utf16_lossy(slice))
}

fn map_pdfium_error(default_message: &str) -> anyhow::Error {
	let last_error = unsafe { ffi::FPDF_GetLastError() };
	match last_error {
		ffi::FPDF_ERR_PASSWORD => anyhow!("{PASSWORD_REQUIRED_ERROR_PREFIX}Password required or incorrect"),
		code if code != 0 => anyhow!("{default_message} (PDFium error code {code})"),
		_ => anyhow!("{default_message}"),
	}
}

mod ffi {
	#![allow(non_camel_case_types)]

	use std::ffi::c_void;

	pub type FPDF_DOCUMENT = *mut c_void;
	pub type FPDF_PAGE = *mut c_void;
	pub type FPDF_TEXTPAGE = *mut c_void;
	pub type FPDF_BOOKMARK = *mut c_void;
	pub type FPDF_DEST = *mut c_void;

	pub const FPDF_ERR_PASSWORD: u32 = 4;

	#[link(name = "pdfium")]
	unsafe extern "C" {
		pub fn FPDF_InitLibrary();
		pub fn FPDF_DestroyLibrary();
		pub fn FPDF_LoadDocument(file_path: *const i8, password: *const i8) -> FPDF_DOCUMENT;
		pub fn FPDF_CloseDocument(document: FPDF_DOCUMENT);
		pub fn FPDF_GetLastError() -> u32;
		pub fn FPDF_GetPageCount(document: FPDF_DOCUMENT) -> i32;
		pub fn FPDF_LoadPage(document: FPDF_DOCUMENT, page_index: i32) -> FPDF_PAGE;
		pub fn FPDF_ClosePage(page: FPDF_PAGE);
		pub fn FPDFText_LoadPage(page: FPDF_PAGE) -> FPDF_TEXTPAGE;
		pub fn FPDFText_ClosePage(text_page: FPDF_TEXTPAGE);
		pub fn FPDFText_CountChars(text_page: FPDF_TEXTPAGE) -> i32;
		pub fn FPDFText_GetText(text_page: FPDF_TEXTPAGE, start_index: i32, count: i32, result: *mut u16) -> i32;
		pub fn FPDF_GetMetaText(document: FPDF_DOCUMENT, tag: *const i8, buffer: *mut c_void, buflen: u32) -> u32;
		pub fn FPDFBookmark_GetFirstChild(document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_BOOKMARK;
		pub fn FPDFBookmark_GetNextSibling(document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_BOOKMARK;
		pub fn FPDFBookmark_GetTitle(bookmark: FPDF_BOOKMARK, buffer: *mut c_void, buflen: u32) -> u32;
		pub fn FPDFBookmark_GetDest(document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_DEST;
		pub fn FPDFDest_GetDestPageIndex(document: FPDF_DOCUMENT, dest: FPDF_DEST) -> i32;
	}
}
