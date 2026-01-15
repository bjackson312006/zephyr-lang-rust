/*
 * Copyright (c) 2024 Linaro LTD
 * SPDX-License-Identifier: Apache-2.0
 */

/* This main is brought into the Rust driver. */
#include <zephyr/kernel.h>

#ifdef CONFIG_RUST

#if defined(CONFIG_LOG) && !defined(CONFIG_LOG_MINIMAL)
// Logging, to see how things things are expanded.
#include <zephyr/logging/log.h>
LOG_MODULE_REGISTER(rust, 3);

__attribute__((weak))
void rust_log_message(uint32_t level, char *msg) {
	// Ok.  The log macros in Zephyr perform all kinds of macro stitching, etc, on the
	// arguments.  As such, we can't just pass the level to something, but actually need to
	// expand things here.  This puts the file and line information of the log in this file,
	// rather than where we came from.
	switch (level) {
	case LOG_LEVEL_ERR:
		LOG_ERR("%s", msg);
		break;
	case LOG_LEVEL_WRN:
		LOG_WRN("%s", msg);
		break;
	case LOG_LEVEL_INF:
		LOG_INF("%s", msg);
		break;
	case LOG_LEVEL_DBG:
	default:
		LOG_DBG("%s", msg);
		break;
	}
}
#endif /* defined(CONFIG_LOG) && !defined(CONFIG_LOG_MINIMAL) */

/*
 * Expecting several rust drivers built.
 */
__attribute__((weak))
void rust_panic_wrap(void)
{
	k_panic();
}

#endif
