use std::{
	ffi::{CStr, CString},
	os::raw::c_char,
	ptr,
};

#[must_use]
pub unsafe fn c_str_to_string(ptr: *const c_char) -> Option<String> {
	if ptr.is_null() {
		return None;
	}
	match CStr::from_ptr(ptr).to_str() {
		Ok(s) => Some(s.to_string()),
		Err(_) => None,
	}
}

#[must_use]
pub fn string_to_c_str(s: String) -> *mut c_char {
	let sanitized = s.replace('\0', " ");
	match CString::new(sanitized) {
		Ok(c_string) => c_string.into_raw(),
		Err(_) => ptr::null_mut(),
	}
}

/// # Safety
/// The passed pointer must have been created by `string_to_c_str` or `CString::into_raw`.
pub unsafe fn free_c_string(ptr: *mut c_char) {
	if !ptr.is_null() {
		drop(CString::from_raw(ptr));
	}
}
