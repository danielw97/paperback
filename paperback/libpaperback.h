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

#ifdef __cplusplus
}
#endif
