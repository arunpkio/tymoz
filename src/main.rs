use chrono::Local;
use slint::{Timer, TimerMode};
slint::include_modules!();

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
    let ui = AppWindow::new().unwrap();

    // _ to keep the timer alive
    let _timer = update(&ui);

    ui.run()
}
