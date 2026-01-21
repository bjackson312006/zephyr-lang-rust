/*
 * Copyright (c) 2019 Centaur Analytics, Inc
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#define DT_DRV_COMPAT ti_tmp108_rs

#include <zephyr/device.h>
#include <zephyr/drivers/i2c.h>
#include <zephyr/drivers/sensor.h>
#include <zephyr/pm/device.h>
#include <zephyr/sys/byteorder.h>


/*
 * Forward declarations of Rust-generated symbols.
 * These are created by the sensor_ffi_exports! macro with prefix "tmp108_rs"
 */
extern const struct sensor_driver_api tmp108_rs_driver_api;
extern int tmp108_rs_init(const struct device *dev);

/*
 * Data and config structures.
 * These must match the Rust #[repr(C)] structs if Rust needs to access them.
 */
struct tmp108_rs_data {
	uint16_t sample;
	uint16_t id;
};

struct tmp108_rs_config {
	const struct i2c_dt_spec bus;
};

#define TMP_RS_DEFINE(inst, t)                                                       \
	static struct tmp108_rs_data tmp108_rs_prv_data_##inst##t;                       \
	static const struct tmp108_rs_config tmp108_rs_config_##inst##t = {              \
		.bus = I2C_DT_SPEC_INST_GET(inst)                                            \
		};                                                                           \
	SENSOR_DEVICE_DT_INST_DEFINE(inst, &tmp108_rs_init, NULL, &tmp108_rs_prv_data_##inst##t,    \
				     &tmp108_rs_config_##inst##t, POST_KERNEL,                                  \
				     CONFIG_SENSOR_INIT_PRIORITY, &tmp108_rs_driver_api);

#define TMP_RS_INIT(n) TMP_RS_DEFINE(n, TI_TMP108)
#undef DT_DRV_COMPAT
#define DT_DRV_COMPAT ti_tmp108
DT_INST_FOREACH_STATUS_OKAY(TMP_RS_INIT)

