use gag::Redirect;
use xcap::Window;

use crate::{
    ocr::{RectangleEngine, focused_window_index},
    overlay::Overlay,
};
use std::{process::exit, time::Duration};

mod ocr;
mod overlay;
mod profiler;

fn main() {
    let _redirect = redirect();

    let windows = handle_error!(Window::all() ;; vec![]);
    let current = handle_error!(focused_window_index(&windows));
    let engine = RectangleEngine::new(windows[current].clone(), Duration::from_secs(1));

    egui_overlay::start(Overlay::new(engine, (windows, current)));
}

fn redirect() -> Redirect<std::fs::File> {
    use std::fs::File;
    use std::time::SystemTime;
    let _ = std::fs::create_dir("logs");

    let log = File::create(format!(
        "logs/{}.log",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs()
    ))
    .unwrap();
    Redirect::stdout(log).unwrap()
}

#[macro_export]
macro_rules! handle_error {
    ($i:expr => $l:expr ;; $out:expr) => {
        match $i {
            Ok(v) => v,
            Err(err) => {
                println!("[ERR] {}", $l(err));
                $out
            }
        }
    };
    ($i:expr ;; $out:expr) => {
        handle_error!($i => |err| format!("Result is {} -> {err:?}", stringify!($i)) ;; $out)
    };
    ($i:expr => $l:expr) => {
        handle_error!($i => |err| format!("Result is {} -> {err:?}", stringify!($i)) ;; exit(1))
    };
    ($i:expr) => {
        handle_error!($i ;; exit(1))
    };
}
