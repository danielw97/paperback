use std::{
	fs::File,
	io::{Read, Seek},
	os::raw::c_char,
	ptr,
};

use zip::ZipArchive;

use super::text::url_decode;
use crate::ffi::{c_str_to_string, string_to_c_str};

pub fn read_zip_entry_by_index<R: Read + Seek>(archive: &mut ZipArchive<R>, index: usize) -> Result<String, String> {
	let mut entry = archive.by_index(index).map_err(|e| format!("Failed to get entry: {e}"))?;
	let mut contents = String::new();
	entry.read_to_string(&mut contents).map_err(|e| format!("Failed to read entry: {e}"))?;
	Ok(contents)
}

pub fn read_zip_entry_by_name<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Result<String, String> {
	let mut entry = archive.by_name(name).map_err(|e| format!("Failed to get entry '{name}': {e}"))?;
	let mut contents = String::new();
	entry.read_to_string(&mut contents).map_err(|e| format!("Failed to read entry '{name}': {e}"))?;
	Ok(contents)
}

pub fn find_zip_entry<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>, filename: &str) -> Option<usize> {
	for i in 0..archive.len() {
		if let Ok(entry) = archive.by_index(i) {
			if entry.name() == filename {
				return Some(i);
			}
		}
	}
	let decoded = url_decode(filename);
	if decoded != filename {
		for i in 0..archive.len() {
			if let Ok(entry) = archive.by_index(i) {
				if entry.name() == decoded {
					return Some(i);
				}
			}
		}
	}
	for i in 0..archive.len() {
		if let Ok(entry) = archive.by_index(i) {
			let entry_name = entry.name();
			let decoded_entry_name = url_decode(entry_name);
			if decoded_entry_name == filename || decoded_entry_name == decoded {
				return Some(i);
			}
		}
	}
	None
}

#[no_mangle]
pub extern fn paperback_read_zip_entry(zip_path: *const c_char, entry_name: *const c_char) -> *mut c_char {
	let zip_path_str = match unsafe { c_str_to_string(zip_path) } {
		Some(s) => s,
		None => return ptr::null_mut(),
	};
	let entry_name_str = match unsafe { c_str_to_string(entry_name) } {
		Some(s) => s,
		None => return ptr::null_mut(),
	};
	let file = match File::open(&zip_path_str) {
		Ok(f) => f,
		Err(_) => return ptr::null_mut(),
	};
	let mut archive = match ZipArchive::new(file) {
		Ok(a) => a,
		Err(_) => return ptr::null_mut(),
	};
	match read_zip_entry_by_name(&mut archive, &entry_name_str) {
		Ok(contents) => string_to_c_str(contents),
		Err(_) => ptr::null_mut(),
	}
}

#[no_mangle]
pub extern fn paperback_find_zip_entry(
	zip_path: *const c_char,
	entry_name: *const c_char,
	out_index: *mut usize,
) -> i32 {
	if out_index.is_null() {
		return -1;
	}
	let zip_path_str = match unsafe { c_str_to_string(zip_path) } {
		Some(s) => s,
		None => return -1,
	};
	let entry_name_str = match unsafe { c_str_to_string(entry_name) } {
		Some(s) => s,
		None => return -1,
	};
	let file = match File::open(&zip_path_str) {
		Ok(f) => f,
		Err(_) => return -1,
	};
	let Ok(mut archive) = ZipArchive::new(file) else { return -1 };
	find_zip_entry(&mut archive, &entry_name_str).map_or(0, |index| {
		unsafe { *out_index = index };
		1
	})
}
