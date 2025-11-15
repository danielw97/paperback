#[cxx::bridge]
pub mod ffi {
	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
	pub enum UpdateStatus {
		Available,
		UpToDate,
		HttpError,
		NetworkError,
		InvalidResponse,
		NoDownload,
		InvalidInput,
		InternalError,
	}

	pub struct UpdateResult {
		pub status: UpdateStatus,
		pub http_status: i32,
		pub latest_version: String,
		pub download_url: String,
		pub release_notes: String,
		pub error_message: String,
	}

	pub struct ParserInfo {
		pub name: String,
		pub extensions: Vec<String>,
		pub flags: u32,
	}

	pub struct FfiMarker {
		pub marker_type: i32,
		pub position: usize,
		pub text: String,
		pub reference: String,
		pub level: i32,
	}

	pub struct FfiTocItem {
		pub name: String,
		pub reference: String,
		pub offset: usize,
	}

	pub struct FfiDocumentStats {
		pub word_count: usize,
		pub line_count: usize,
		pub char_count: usize,
	}

	pub struct FfiDocument {
		pub title: String,
		pub author: String,
		pub content: String,
		pub markers: Vec<FfiMarker>,
		pub toc_items: Vec<FfiTocItem>,
		pub stats: FfiDocumentStats,
	}

	extern "Rust" {
		fn check_for_updates(current_version: &str, is_installer: bool) -> Result<UpdateResult>;
		fn remove_soft_hyphens(input: &str) -> Result<String>;
		fn url_decode(encoded: &str) -> Result<String>;
		fn collapse_whitespace(input: &str) -> Result<String>;
		fn trim_string(input: &str) -> Result<String>;
		fn convert_to_utf8(input: &[u8]) -> Result<String>;
		fn read_zip_entry(zip_path: &str, entry_name: &str) -> Result<String>;
		fn find_zip_entry(zip_path: &str, entry_name: &str) -> Result<usize>;
		fn get_available_parsers() -> Result<Vec<ParserInfo>>;
		fn parse_document(file_path: &str, password: &str) -> Result<FfiDocument>;
		fn get_parser_for_extension(extension: &str) -> Result<String>;
	}
}

use std::fs::File;

use self::ffi::UpdateStatus;
use crate::{
	document::{ParserContext, TocItem},
	parser, update as update_module,
	utils::{encoding, text, zip as zip_module},
};

fn check_for_updates(current_version: &str, is_installer: bool) -> Result<ffi::UpdateResult, String> {
	match update_module::check_for_updates(current_version, is_installer) {
		Ok(outcome) => match outcome {
			update_module::UpdateCheckOutcome::UpdateAvailable(result) => Ok(ffi::UpdateResult {
				status: UpdateStatus::Available,
				http_status: result.http_status,
				latest_version: result.latest_version,
				download_url: result.download_url,
				release_notes: result.release_notes,
				error_message: String::new(),
			}),
			update_module::UpdateCheckOutcome::UpToDate(latest_version) => Ok(ffi::UpdateResult {
				status: UpdateStatus::UpToDate,
				http_status: 0,
				latest_version,
				download_url: String::new(),
				release_notes: String::new(),
				error_message: String::new(),
			}),
		},
		Err(err) => {
			let (status, http_status) = match &err {
				update_module::UpdateError::InvalidVersion(_) => (UpdateStatus::InvalidInput, 0),
				update_module::UpdateError::HttpError(code) => (UpdateStatus::HttpError, *code),
				update_module::UpdateError::NetworkError(_) => (UpdateStatus::NetworkError, 0),
				update_module::UpdateError::InvalidResponse(_) => (UpdateStatus::InvalidResponse, 0),
				update_module::UpdateError::NoDownload(_) => (UpdateStatus::NoDownload, 0),
			};
			Ok(ffi::UpdateResult {
				status,
				http_status,
				latest_version: String::new(),
				download_url: String::new(),
				release_notes: String::new(),
				error_message: err.to_string(),
			})
		}
	}
}

fn remove_soft_hyphens(input: &str) -> Result<String, String> {
	Ok(text::remove_soft_hyphens(input))
}

fn url_decode(encoded: &str) -> Result<String, String> {
	Ok(text::url_decode(encoded))
}

fn collapse_whitespace(input: &str) -> Result<String, String> {
	Ok(text::collapse_whitespace(input))
}

fn trim_string(input: &str) -> Result<String, String> {
	Ok(text::trim_string(input))
}

fn convert_to_utf8(input: &[u8]) -> Result<String, String> {
	Ok(encoding::convert_to_utf8(input))
}

fn read_zip_entry(zip_path: &str, entry_name: &str) -> Result<String, String> {
	let file = File::open(zip_path).map_err(|e| format!("Failed to open ZIP file: {e}"))?;
	let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Failed to read ZIP archive: {e}"))?;
	zip_module::read_zip_entry_by_name(&mut archive, entry_name)
}

fn find_zip_entry(zip_path: &str, entry_name: &str) -> Result<usize, String> {
	let file = File::open(zip_path).map_err(|e| format!("Failed to open ZIP file: {e}"))?;
	let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Failed to read ZIP archive: {e}"))?;
	zip_module::find_zip_entry(&mut archive, entry_name)
		.ok_or_else(|| format!("Entry '{entry_name}' not found in ZIP archive"))
}

fn get_available_parsers() -> Result<Vec<ffi::ParserInfo>, String> {
	let parsers = parser::get_all_parsers();
	Ok(parsers
		.into_iter()
		.map(|p| ffi::ParserInfo { name: p.name, extensions: p.extensions, flags: p.flags.bits() })
		.collect())
}

fn parse_document(file_path: &str, password: &str) -> Result<ffi::FfiDocument, String> {
	let mut context = ParserContext::new(file_path.to_string());
	if !password.is_empty() {
		context = context.with_password(password.to_string());
	}
	let doc = parser::parse_document(&context)?;
	// Convert TOC items to flat list (cxx doesn't support recursive types easily)
	let toc_items = flatten_toc_items(&doc.toc_items);
	Ok(ffi::FfiDocument {
		title: doc.title,
		author: doc.author,
		content: doc.buffer.content,
		markers: doc
			.buffer
			.markers
			.into_iter()
			.map(|m| ffi::FfiMarker {
				marker_type: m.marker_type.to_int(),
				position: m.position,
				text: m.text,
				reference: m.reference,
				level: m.level,
			})
			.collect(),
		toc_items,
		stats: ffi::FfiDocumentStats {
			word_count: doc.stats.word_count,
			line_count: doc.stats.line_count,
			char_count: doc.stats.char_count,
		},
	})
}

fn get_parser_for_extension(extension: &str) -> Result<String, String> {
	parser::get_parser_name_for_extension(extension)
		.ok_or_else(|| format!("No parser found for extension: .{}", extension))
}

fn flatten_toc_items(items: &[TocItem]) -> Vec<ffi::FfiTocItem> {
	let mut result = Vec::new();
	fn flatten_recursive(items: &[TocItem], result: &mut Vec<ffi::FfiTocItem>) {
		for item in items {
			result.push(ffi::FfiTocItem {
				name: item.name.clone(),
				reference: item.reference.clone(),
				offset: item.offset,
			});
			flatten_recursive(&item.children, result);
		}
	}
	flatten_recursive(items, &mut result);
	result
}
