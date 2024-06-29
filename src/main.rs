mod time_zones;
use chrono::{Local, Offset, Utc};
use chrono_tz::Tz;
use directories::ProjectDirs;
use serde_json;
use slint::{CloseRequestResponse, Model, Timer, TimerMode, VecModel};
use std::cell::RefCell;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;
use time_zones::TIME_ZONES;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

slint::include_modules!();

pub const CACHE_FILE_NAME: &str = "cache.json";
fn get_app_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "apk", "tymoz") {
        let app_dir = proj_dirs.config_dir();
        fs::create_dir_all(app_dir).expect("Failed to create application directory");
        app_dir.to_path_buf()
    } else {
        panic!("Could not determine application directory");
    }
}

fn format_time_diff(value: f64) -> String {
    match value {
        0.0 => String::from("GMT"),
        offset if offset > 0.0 => format!("GMT +{offset}"),
        offset => format!("GMT {offset}",),
    }
}

struct AppState {
    available_cities_model: Rc<VecModel<TimeZoneInfo>>,
    selected_cities_model: Rc<VecModel<TimeZoneInfo>>,
    timer: Timer,
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
                timenow: time_str.clone().into(),
                offset: format_time_diff(local_offset).into(),
                date: timezone_date.clone().into(),
                is_ahead: local_offset >= 0.0,
                timezone: zone.timezone.to_string().into(),
            });
        }

        let utc_now: chrono::prelude::DateTime<Utc> = Utc::now();
        let local_time_str: String = utc_now.format("%H:%M").to_string();
        let local_offset: f64 = utc_now.offset().fix().local_minus_utc() as f64 / 3600.0;
        let utc_date = utc_now.format("%d, %B").to_string();
        let utc_zone_info = &TIME_ZONES[0]; // By default, show UTC time

        #[cfg(not(target_arch = "wasm32"))]
        {
            let app_dir = get_app_dir();
            let settings_file = app_dir.join(CACHE_FILE_NAME);
            if settings_file.exists() {
                let file = File::open(settings_file).expect("Unable to open file");
                let reader = BufReader::new(file);

                for line in reader.lines() {
                    let line = line.expect("Unable to read line");
                    let mut info: TimeZoneInfo =
                        serde_json::from_str(&line).expect("Unable to parse JSON");
                    if let Ok(tz) = Tz::from_str(&info.timezone) {
                        let zone_time = utc_now.with_timezone(&tz);
                        info.timenow = zone_time
                            .format(if true { "%I:%M %p" } else { "%H:%M" })
                            .to_string()
                            .into();
                        info.date = zone_time.format("%d, %B").to_string().into();
                        self.selected_cities_model.push(info);
                    }
                }
            } else {
                self.selected_cities_model.insert(
                    0,
                    TimeZoneInfo {
                        city: utc_zone_info.name.into(),
                        timenow: local_time_str.into(),
                        offset: format_time_diff(local_offset).into(),
                        date: utc_date.into(),
                        is_ahead: local_offset >= 0.0,
                        timezone: utc_zone_info.location.into(),
                    },
                );
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.selected_cities_model.insert(
                0,
                TimeZoneInfo {
                    city: utc_zone_info.name.into(),
                    timenow: local_time_str.into(),
                    offset: crate::format_time_diff(local_offset).into(),
                    date: utc_date.into(),
                    is_ahead: local_offset >= 0.0,
                    timezone: utc_zone_info.location.into(),
                },
            );
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

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
fn main() {
    let app_window = AppWindow::new().unwrap();

    let app_state = Rc::new(RefCell::new(AppState {
        available_cities_model: Rc::new(slint::VecModel::default()),
        selected_cities_model: Rc::new(slint::VecModel::default()),
        timer: Timer::default(),
    }));

    app_state.borrow_mut().initialize();

    let data_adapter = app_window.global::<DataAdapter>();
    data_adapter
        .set_available_cities_model(app_state.borrow().available_cities_model.clone().into());
    data_adapter.set_selected_cities_model(app_state.borrow().selected_cities_model.clone().into());

    let state_copy = app_state.clone();
    let app_window_weak = app_window.as_weak();
    state_copy.borrow().timer.start(
        TimerMode::Repeated,
        std::time::Duration::from_millis(250),
        move || {
            if let Some(window) = app_window_weak.upgrade() {
                let data_adapter = window.global::<DataAdapter>();
                let use_12h_format = data_adapter.get_use_12h_format();
                let utc_now = Utc::now();
                for (index, mut info) in app_state.borrow().selected_cities_model.iter().enumerate()
                {
                    if let Ok(tz) = Tz::from_str(&info.timezone) {
                        let zone_time = utc_now.with_timezone(&tz);
                        info.timenow = zone_time
                            .format(if use_12h_format { "%I:%M %p" } else { "%H:%M" })
                            .to_string()
                            .into();
                        info.date = zone_time.format("%d, %B").to_string().into();
                        app_state
                            .borrow()
                            .selected_cities_model
                            .set_row_data(index, info);
                    } else {
                        println!("Invalid timezone string: {}", info.timezone);
                    }
                }
            }
        },
    );

    let selected_cities_model_clone = state_copy.borrow().selected_cities_model.clone();
    let available_cities_model_clone = state_copy.borrow().available_cities_model.clone();
    data_adapter.on_add_city(move |idx| {
        let val = available_cities_model_clone.row_data(idx as usize).unwrap();
        selected_cities_model_clone.push(val);
    });

    // _ to keep the timer alive
    let _timer = update(&app_window);

    let selected_cities_model_clone2 = state_copy.borrow().selected_cities_model.clone();

    app_window.window().on_close_requested(move || {
        let app_dir = get_app_dir();
        let cache_file = app_dir.join(CACHE_FILE_NAME);
        let file = File::create(cache_file).unwrap();
        let mut binding = BufWriter::new(&file);
        let writer: &mut &File = binding.get_mut();
        for info in selected_cities_model_clone2.iter() {
            let serialized = serde_json::to_string(&info).unwrap();
            writeln!(writer, "{}", serialized).unwrap();
        }
        writer.flush().unwrap();
        CloseRequestResponse::HideWindow
    });

    app_window.run().unwrap()
}
