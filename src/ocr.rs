use std::{
    process::{Command, exit},
    sync::{
        Arc,
        atomic::AtomicBool,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

use ocrs::{OcrEngine, OcrEngineParams, TextLine};
use rten::Model;
use xcap::{Window, XCapError, XCapResult, image::DynamicImage};

use crate::profiler;

pub struct RectangleEngine {
    pub running: Arc<AtomicBool>,
    task: Sender<Task>,
    recv: Receiver<anyhow::Result<Vec<TextLine>>>,
    texts: anyhow::Result<Vec<TextLine>>,
}

impl RectangleEngine {
    const URL_RECOGNITION_MODEL: &str =
        "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.onnx";
    const URL_DETECTION_MODEL: &str =
        "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.onnx";

    const RECOGNITION_MODEL: &str = "models/text-recognition.onnx";
    const DETECTION_MODEL: &str = "models/text-detection.onnx";

    pub fn new(window: Window, sleep_time: Duration) -> Self {
        download_models();

        let running = Arc::new(AtomicBool::new(true));
        let (task, recv) = Self::create_thread(window.clone(), running.clone(), sleep_time);

        Self {
            running,
            recv,
            task,
            texts: Ok(vec![]),
        }
    }

    pub fn send_task(&self, task: Task) {
        let _ = self.task.send(task);
    }

    pub fn process(&mut self) {
        match self.recv.try_recv() {
            Ok(v) => self.texts = v,
            Err(_) => {}
        }
    }

    pub fn texts(&self) -> Result<&Vec<TextLine>, &anyhow::Error> {
        self.texts.as_ref()
    }

    fn create_thread(
        window: Window,
        running: Arc<AtomicBool>,
        time: Duration,
    ) -> (Sender<Task>, Receiver<anyhow::Result<Vec<TextLine>>>) {
        let (input, output) = mpsc::channel::<anyhow::Result<Vec<TextLine>>>();
        let (input_task, output_task) = mpsc::channel::<Task>();
        thread::spawn(move || {
            let running = running;
            let mut window = window;
            // TODO: Delete unwrap
            let detection_model = Model::load_file("models/text-detection.onnx").unwrap();
            let recognition_model = Model::load_file("models/text-recognition.onnx").unwrap();

            let engine = OcrEngine::new(OcrEngineParams {
                detection_model: Some(detection_model),
                recognition_model: Some(recognition_model),
                ..Default::default()
            })
            .unwrap();

            loop {
                if !running.load(std::sync::atomic::Ordering::Relaxed) {
                    continue;
                }

                if let Ok(task) = output_task.try_recv() {
                    match task {
                        Task::SwapWindow(w) => window = w,
                        Task::Toggle(b) => running.store(b, std::sync::atomic::Ordering::Relaxed),
                    }
                }

                let result = process(&window, &engine);
                let _ = input.send(result);

                std::thread::sleep(time);
            }
        });

        (input_task, output)
    }
}

#[derive(Debug, Clone)]
pub enum Task {
    SwapWindow(Window),
    Toggle(bool),
}

pub const EXPLORER: &str = "Windows Explorer";
pub fn focused_window_index(windows: &[Window]) -> XCapResult<usize> {
    let win = windows
        .iter()
        .enumerate()
        .filter(|(_, win)| win.app_name().is_ok_and(|s| s != EXPLORER))
        .max_by_key(|(_, win)| {
            let focused = win.is_focused().unwrap_or(false);
            let z = win.z().unwrap_or_default();
            (focused, z)
        });

    win.map(|(i, _)| i)
        .ok_or(XCapError::Error(String::from("Focused window not found")))
}

fn process(window: &Window, ocr: &OcrEngine) -> anyhow::Result<Vec<TextLine>> {
    let line_texts = {
        let _record = profiler!("ocr_process");

        let capture = DynamicImage::ImageRgba8(window.capture_image()?).to_rgb8();
        let img_source = ocrs::ImageSource::from_bytes(capture.as_raw(), capture.dimensions())?;

        let ocr_input = ocr.prepare_input(img_source)?;
        let word_rects = ocr.detect_words(&ocr_input)?;
        let line_rects = ocr.find_text_lines(&ocr_input, &word_rects);
        ocr.recognize_text(&ocr_input, &line_rects)?
    };

    Ok(line_texts.into_iter().flatten().collect())
}

fn download_models() {
    use std::fs::exists;
    let _ = std::fs::create_dir("models");

    fn dnld(url: &str, path: &str, msg: &str) {
        match exists(path) {
            Ok(false) => {
                println!("[OK] {msg}");
                let _ = Command::new("curl").args([url, "-o", path]).status();
            }
            Ok(true) => {}
            Err(err) => {
                println!("[ERR] download error; {err}");
                exit(1)
            }
        }
    }

    dnld(
        RectangleEngine::URL_DETECTION_MODEL,
        RectangleEngine::DETECTION_MODEL,
        "Download detection model...",
    );

    dnld(
        RectangleEngine::URL_RECOGNITION_MODEL,
        RectangleEngine::RECOGNITION_MODEL,
        "Download detection model...",
    );
}
