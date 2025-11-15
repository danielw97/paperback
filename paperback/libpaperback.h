#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum paperback_update_status {
	PAPERBACK_UPDATE_STATUS_AVAILABLE = 0,
	PAPERBACK_UPDATE_STATUS_UP_TO_DATE = 1,
	PAPERBACK_UPDATE_STATUS_HTTP_ERROR = 2,
	PAPERBACK_UPDATE_STATUS_NETWORK_ERROR = 3,
	PAPERBACK_UPDATE_STATUS_INVALID_RESPONSE = 4,
	PAPERBACK_UPDATE_STATUS_NO_DOWNLOAD = 5,
	PAPERBACK_UPDATE_STATUS_INVALID_INPUT = 6,
	PAPERBACK_UPDATE_STATUS_INTERNAL_ERROR = 7
} paperback_update_status;

typedef struct paperback_update_result {
	paperback_update_status status;
	int32_t http_status;
	const char* latest_version;
	const char* download_url;
	const char* release_notes;
	const char* error_message;
} paperback_update_result;

paperback_update_result* paperback_check_for_updates(const char* current_version, uint8_t is_installer);
void paperback_free_update_result(paperback_update_result* result);

// Utility functions - String processing
char* paperback_remove_soft_hyphens(const char* input);
char* paperback_url_decode(const char* encoded);
char* paperback_collapse_whitespace(const char* input);
char* paperback_trim_string(const char* input);

// Utility functions - Encoding conversion
char* paperback_convert_to_utf8(const uint8_t* input, size_t input_len);

// Utility functions - ZIP handling
char* paperback_read_zip_entry(const char* zip_path, const char* entry_name);
int32_t paperback_find_zip_entry(const char* zip_path, const char* entry_name, size_t* out_index);

// Memory management - Free strings returned by Rust utility functions
void paperback_free_string(char* s);

#ifdef __cplusplus
}
#endif
