use arc_swap::ArcSwapAny;
use ocrs::TextLine;
use xcap::Window;

use crate::{
    ocr::{RecEngine, download_models},
    overlay::Overlay,
};
use std::{
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

mod ocr;
mod overlay;

const COOLDOWN: Duration = Duration::from_millis(100);
pub type ArcBuffer = Arc<ArcSwapAny<Arc<Vec<TextLine>>>>;

fn main() {
    download_models();
    egui_overlay::start(Overlay::new());
}

pub static mut CURRENT_THREAD_ID: u64 = 0;
fn thread(window: Window, buffer: ArcBuffer) -> JoinHandle<()> {
    thread::spawn(move || {
        let engine = RecEngine::new(window).expect("new err");
        let curr = unsafe {
            std::mem::transmute::<std::thread::ThreadId, u64>(std::thread::current().id())
        };
        unsafe { CURRENT_THREAD_ID = curr };

        while curr == unsafe { CURRENT_THREAD_ID } {
            let vec = engine.process().expect("process error");
            buffer.store(Arc::new(vec));

            std::thread::sleep(COOLDOWN);
        }

        println!("thread destroy [{curr}]")
    })
}
