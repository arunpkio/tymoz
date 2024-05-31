slint::include_modules!();
mod time_zones;
use time_zones::TIME_ZONES;
use std::cell::RefCell;
use std::rc::Rc;
use chrono::{Local, Offset, Utc};
use slint::{Timer, TimerMode};

fn format_time_diff(value: f64) -> String {
    match value {
        0.0 => String::from("GMT"),
        offset if offset > 0.0 => format!("GMT +{offset}"),
        offset => format!("GMT {offset}",),
    }
}

struct AppState {
    available_cities_model: Rc<slint::VecModel<TimeZoneInfo>>,
}

impl AppState {
    fn initialize(&mut self) {
        let utc_now: chrono::prelude::DateTime<Utc> = Utc::now();
        for zone in TIME_ZONES.iter() {
            let zone_time = utc_now.with_timezone(&zone.timezone);
            let time_str = zone_time.format("%H:%M").to_string();
            let local_offset: f64 = zone_time.offset().fix().local_minus_utc() as f64 / 3600.0;
            let timezone_date = zone_time.format("%d. %B").to_string();

            self.available_cities_model.push(TimeZoneInfo {
                city: zone.name.to_string().into(),
                timenow: time_str.into(),
                offset: format_time_diff(local_offset).into(),
                date: timezone_date.into(),
                is_ahead: local_offset >= 0.0,
                timezone: zone.timezone.to_string().into(),
            });
        }
    }
}

pub fn update(window: &AppWindow) -> Timer {
    let update_timer = Timer::default();

    update_timer.start(
        TimerMode::Repeated,
        std::time::Duration::from_millis(250),
        {
            let weak_window = window.as_weak();
            move || {
                let now = Local::now();

                if let Some(window) = weak_window.upgrade() {
                    let data_adapter = window.global::<DataAdapter>();
                    let time_format = if data_adapter.get_use_12h_format() {
                        "%a, %b %d | %I:%M:%S %p"
                    } else {
                        "%a, %b %d | %H:%M:%S"
                    };
                    data_adapter.set_date_time(slint::format!("{}", now.format(time_format)));
                }
            }
        },
    );

    update_timer
}

fn main() -> Result<(), slint::PlatformError> {

    let app_window = AppWindow::new().unwrap();

    let app_state = Rc::new(RefCell::new(AppState {
        available_cities_model: Rc::new(slint::VecModel::default()),
    }));

    app_state.borrow_mut().initialize();

    let data_adapter = app_window.global::<DataAdapter>();
    data_adapter.set_available_cities_model(app_state.borrow().available_cities_model.clone().into());

    // _ to keep the timer alive
    let _timer = update(&app_window);

    app_window.run()
}
