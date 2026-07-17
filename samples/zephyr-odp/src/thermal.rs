const SENSOR_EVENT_CHANNEL_SIZE: usize = 8;
const FAN_EVENT_CHANNEL_SIZE: usize = 8;

type SensorEventSender = embassy_sync::channel::Sender<'static, embedded_services::GlobalRawMutex, thermal_service_interface::sensor::Event, SENSOR_EVENT_CHANNEL_SIZE>;
type FanEventSender = embassy_sync::channel::Sender<'static, embedded_services::GlobalRawMutex, thermal_service_interface::fan::Event, FAN_EVENT_CHANNEL_SIZE>;
type SensorService = thermal_service::sensor::Service<'static, temperature_sensor::Sensor, SensorEventSender, 16>;
type FanService = thermal_service::fan::Service<'static, zephyr::device::pwm_fan::PwmFan, SensorService, FanEventSender, 16>;
pub type ThermalService = thermal_service::Service<'static, SensorService, FanService>;

pub async fn init(spawner: embassy_executor::Spawner) -> ThermalService {
    embedded_services::info!("Initializing thermal service...");

    // Create and spawn sensor service
    let sensor_service = crate::utils::spawn_service!(spawner, SensorService, |resources| thermal_service::sensor::Service::new(
        resources,
        thermal_service::sensor::InitParams {
            driver: temperature_sensor::Sensor::new(),
            event_senders: &mut [],    // List of event senders

            // Thermal service sensor config
            config: thermal_service::sensor::Config {
                sample_period: embassy_time::Duration::from_secs(2),      // Rate at which to sample the sensor when operating in normal conditions
                fast_sample_period: embassy_time::Duration::from_secs(2), // Rate at which to sample the sensor when operating in fast conditions
                ..Default::default()
            },
        },
    ))
    .expect("Failed to spawn sensor_service.");

    // Create and spawn fan service
    let fan_service = crate::utils::spawn_service!(
        spawner,
        FanService,
        |resources| thermal_service::fan::Service::new(
            resources,
            thermal_service::fan::InitParams {
                driver: zephyr::devicetree::labels::fan0::get_instance().expect("Failed to call zephyr::devicetree::labels::fan0::get_instance"),
                sensor_service,
                event_senders: &mut [],
                config: thermal_service::fan::Config {
                    auto_control: true,
                    min_temp: 22.0,
                    ramp_temp: 23.0,
                    max_temp: 24.0,
                    ..Default::default()
                },
            },
        )
    )
    .expect("Failed to spawn fan service");

    // Create the thermal service
    static SENSORS: static_cell::StaticCell<[SensorService; 1]> = static_cell::StaticCell::new();
    let sensors = SENSORS.init([sensor_service]);

    static FANS: static_cell::StaticCell<[FanService; 1]> = static_cell::StaticCell::new();
    let fans = FANS.init([fan_service]);

    static RESOURCES: static_cell::StaticCell<thermal_service::Resources<SensorService, FanService>> = static_cell::StaticCell::new();
    let resources = RESOURCES.init(thermal_service::Resources::default());

    thermal_service::Service::init(resources, thermal_service::InitParams { sensors, fans })
}

mod temperature_sensor {
    pub use embedded_sensors_hal_async::temperature::DegreesCelsius;
    use log::info;

    // Wrapper around the Zephyr temperature sensor with implementations for the generic embedded_sensors_hal_async::temperature::TemperatureSensor trait.
    pub struct Sensor(zephyr::device::temperature_sensor::TemperatureSensor);
    impl Sensor {
        pub fn new() -> Self {
            Self(zephyr::devicetree::labels::u_temperature_sensor::get_instance().expect("Failed to call zephyr::devicetree::labels::u_temperature_sensor::get_instance()"))
        }
    }
    impl thermal_service_interface::sensor::Driver for Sensor {} // Marker trait so Sensor can be used with thermal_service.

    // Error type.
    impl embedded_sensors_hal_async::sensor::ErrorType for Sensor {
        type Error = embedded_sensors_hal_async::sensor::ErrorKind;
    }

    // Returns a temperature sample in degrees Celsius.
    impl embedded_sensors_hal_async::temperature::TemperatureSensor for Sensor {
        async fn temperature(&mut self) -> Result<DegreesCelsius, Self::Error> {
            let Self(sensor) = self;
            match sensor.read_ambient_temperature() {
                Ok(temperature) => {
                    info!("Temperature read out success");
                    info!("    {}.{} Celsius", temperature.val1, temperature.val2);
                    let temperature: f32 = temperature.val1 as f32 + (temperature.val2 as f32) / 1_000_000.0;
                    Ok(temperature)
                }
                Err(e) => {
                    info!("Temperature read out failed {}", e);
                    let temperature: DegreesCelsius = 42.0;
                    Ok(temperature)
                }
            }
        }
    }
}