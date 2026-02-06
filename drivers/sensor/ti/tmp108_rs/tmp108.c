/*
 * Copyright (c) 2019 Centaur Analytics, Inc
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#define DT_DRV_COMPAT ti_tmp108_rs

#include <zephyr/device.h>
#include <zephyr/drivers/gpio.h>
#include <zephyr/drivers/i2c.h>
#include <zephyr/drivers/sensor.h>
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

/* Storage for Rust Tmp108RsData struct.
 * We use a pointer to heap-allocated Rust data.
 * The Rust code allocates the data during init() using Box::new()
 * and stores the raw pointer here.
 *
 * This approach completely decouples the C and Rust struct layouts,
 * eliminating any ABI compatibility concerns.
 */
struct tmp108_rs_data {
void *rust_ptr;
};

struct tmp108_rs_config {
    // This holds the device tree for the i2c bus connected to the sensor.
	const struct i2c_dt_spec bus;
    // This is the GPIO spec for the alert pin, if used.
    const struct gpio_dt_spec alert_gpio;
};

#define CONFIG_TMP108_RS_ALERT_INTERRUPTS 1

#define TMP_RS_DEFINE(inst, t)                                                       \
	static struct tmp108_rs_data tmp108_rs_prv_data_##inst##t;                       \
	static const struct tmp108_rs_config tmp108_rs_config_##inst##t = {              \
		.bus = I2C_DT_SPEC_INST_GET(inst),                                           \
        IF_ENABLED(CONFIG_TMP108_RS_ALERT_INTERRUPTS,                                \
			   (.alert_gpio = GPIO_DT_SPEC_INST_GET(inst, alert_gpios),))            \
		};                                                                           \
	SENSOR_DEVICE_DT_INST_DEFINE(inst, &tmp108_rs_init, NULL, &tmp108_rs_prv_data_##inst##t,    \
				     &tmp108_rs_config_##inst##t, POST_KERNEL,                                  \
				     CONFIG_SENSOR_INIT_PRIORITY, &tmp108_rs_driver_api);

#define TMP10_RS_INIT(n) TMP_RS_DEFINE(n, TI_TMP108_RS)
#undef DT_DRV_COMPAT
#define DT_DRV_COMPAT ti_tmp108_rs
DT_INST_FOREACH_STATUS_OKAY(TMP10_RS_INIT)
