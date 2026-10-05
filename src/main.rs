use arc_swap::ArcSwapAny;
use gag::Redirect;
use ocrs::TextLine;
use xcap::Window;

use crate::{
    ocr::{RecEngine, download_models},
    overlay::Overlay,
};
use std::{
    process::exit,
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

mod ocr;
mod overlay;

const COOLDOWN: Duration = Duration::from_millis(100);
pub type ArcBuffer = Arc<ArcSwapAny<Arc<Vec<TextLine>>>>;

fn main() {
    let _redirect = redirect();
    download_models();
    egui_overlay::start(Overlay::new());
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

pub static mut CURRENT_THREAD_ID: u64 = 0;
pub static mut ENABLE: bool = true;
fn thread(window: Window, buffer: ArcBuffer) -> JoinHandle<()> {
    thread::spawn(move || {
        let engine = handle_error!(RecEngine::new(window) => |err| format!("RecEngine try create instant {err:?}"));
        let curr = unsafe {
            std::mem::transmute::<std::thread::ThreadId, u64>(std::thread::current().id())
        };
        unsafe { CURRENT_THREAD_ID = curr };

        while curr == unsafe { CURRENT_THREAD_ID } {
            if !unsafe { ENABLE } {
                buffer.store(Arc::new(vec![]));
                continue;
            }

            let vec =
                handle_error!(engine.process() => |err| format!("OCR Engine process {err:?}"));
            buffer.store(Arc::new(vec));

            std::thread::sleep(COOLDOWN);
        }

        println!("[OK] Thread destroy [{curr}]")
    })
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
        handle_error!($i => |err| format!("Result is err({} -> {err:?})", stringify!($i)) ;; $out)
    };
    ($i:expr => $l:expr) => {
        handle_error!($i => |err| format!("Result is err({} -> {err:?})", stringify!($i)) ;; exit(1))
    };
    ($i:expr) => {
        handle_error!($i ;; exit(1))
    };
}
