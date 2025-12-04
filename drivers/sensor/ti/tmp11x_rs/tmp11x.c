/*
 * Copyright (c) 2019 Centaur Analytics, Inc
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#define DT_DRV_COMPAT ti_tmp11x_rs

#include <zephyr/device.h>
#include <zephyr/drivers/i2c.h>
#include <zephyr/drivers/sensor.h>
#include <zephyr/pm/device.h>
#include <zephyr/sys/byteorder.h>


/*
 * Forward declarations of Rust-generated symbols.
 * These are created by the sensor_ffi_exports! macro with prefix "tmp11x_rs"
 */
extern const struct sensor_driver_api tmp11x_rs_driver_api;
extern int tmp11x_rs_init(const struct device *dev);

/*
 * Data and config structures.
 * These must match the Rust #[repr(C)] structs if Rust needs to access them.
 */
struct tmp11x_data {
	uint16_t sample;
	uint16_t id;
};

struct tmp11x_dev_config {
	struct i2c_dt_spec bus;
	uint16_t odr;
	uint16_t oversampling;
	bool alert_pin_polarity;
	bool alert_mode;
	bool alert_dr_sel;
	bool store_attr_values;
#ifdef CONFIG_TMP11X_RS_TRIGGER
	struct gpio_dt_spec alert_gpio;
#endif
};

/*
 * This is a I2C wrapper then currently is not available in Zephyr.
 */
int tmp11x_reg_read(const struct device *dev, uint8_t reg, uint16_t *val)
{
    const struct tmp11x_dev_config *cfg = dev->config;

    if (i2c_burst_read_dt(&cfg->bus, reg, (uint8_t *)val, 2) < 0) {
        return -EIO;
    }

    *val = sys_be16_to_cpu(*val);

    return 0;
}

int tmp11x_reg_read_wrapper(void *ptr, uint8_t reg, uint16_t *val)
{
    return tmp11x_reg_read((const struct device *)ptr, reg, val);
}

/*
 * Trigger configuration (optional)
 */
#ifdef CONFIG_TMP11X_RS_TRIGGER
#define DEFINE_TMP11X_RS_TRIGGER(_num) \
	.alert_gpio = GPIO_DT_SPEC_INST_GET_OR(_num, alert_gpios, {}),
#else
#define DEFINE_TMP11X_RS_TRIGGER(_num)
#endif

/*
 * Per-instance definition macro.
 * Creates static data/config and registers the device.
 * All driver logic is handled by Rust via tmp11x_rs_driver_api.
 */
#define DEFINE_TMP11X_RS(_num)                                                                     \
	static struct tmp11x_data tmp11x_rs_data_##_num;                                           \
	static const struct tmp11x_dev_config tmp11x_rs_config_##_num = {                          \
		.bus = I2C_DT_SPEC_INST_GET(_num),                                                 \
		.odr = DT_INST_PROP(_num, odr),                                                    \
		.oversampling = DT_INST_PROP(_num, oversampling),                                  \
		.alert_pin_polarity = DT_INST_PROP(_num, alert_polarity),                          \
		.alert_mode = DT_INST_PROP(_num, alert_mode),                                      \
		.alert_dr_sel = DT_INST_PROP(_num, alert_dr_sel),                                  \
		.store_attr_values = DT_INST_PROP(_num, store_attr_values),                        \
		DEFINE_TMP11X_RS_TRIGGER(_num)};                                                   \
                                                                                                   \
	SENSOR_DEVICE_DT_INST_DEFINE(_num, tmp11x_rs_init, PM_DEVICE_DT_INST_GET(_num),            \
				     &tmp11x_rs_data_##_num, &tmp11x_rs_config_##_num,             \
				     POST_KERNEL, CONFIG_SENSOR_INIT_PRIORITY,                     \
				     &tmp11x_rs_driver_api);

/* Expand for all enabled instances */
DT_INST_FOREACH_STATUS_OKAY(DEFINE_TMP11X_RS)
