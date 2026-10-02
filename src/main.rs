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
    time::{Duration, Instant},
};

mod ocr;
mod overlay;

const DELAY: Duration = Duration::from_millis(100);

fn main() {
    download_models();
    egui_overlay::start(Overlay::new());
}

fn thread(window: Window, buffer: Arc<ArcSwapAny<Arc<Vec<TextLine>>>>) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut now = Instant::now();
        let engine = RecEngine::new(window).expect("new err");

        loop {
            if now.elapsed() > DELAY {
                let vec = engine.process().expect("process error");
                buffer.store(Arc::new(vec));

                now = Instant::now()
            }
        }
    })
}
