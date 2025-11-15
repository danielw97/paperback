#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(non_camel_case_types)]

// Module declarations
pub mod ffi;
pub mod update;
pub mod utils;

// Re-export FFI functions for C interop
use std::os::raw::c_char;

pub use update::{
	paperback_check_for_updates, paperback_free_update_result, paperback_update_result, paperback_update_status,
};

/// # Safety
/// This function must only be called with a pointer returned by the Rust side that was allocated using `ffi::alloc_c_string` (or an equivalent function meant to pair with this deallocator). Passing any other pointer will result in undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn paperback_free_string(s: *mut c_char) {
	unsafe {
		ffi::free_c_string(s);
	}
}

pub use utils::{
	encoding::paperback_convert_to_utf8,
	text::{paperback_collapse_whitespace, paperback_remove_soft_hyphens, paperback_trim_string, paperback_url_decode},
	zip::{paperback_find_zip_entry, paperback_read_zip_entry},
};
