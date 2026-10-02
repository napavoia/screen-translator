use std::{
    cmp::Ordering,
    process::Command,
    time::{Duration, Instant},
};

use ocrs::{OcrEngine, OcrEngineParams, TextLine};
use rten::Model;
use xcap::{Window, XCapError, XCapResult, image::DynamicImage};

pub const EXPLORER: &str = "Windows Explorer";

pub static mut ELAPSED: Duration = Duration::from_secs(0);

pub fn focused_window_index(windows: &[Window]) -> XCapResult<usize> {
    let win = windows
        .iter()
        .enumerate()
        .filter(|(_, win)| win.app_name().is_ok_and(|s| s != EXPLORER))
        .max_by(|(_, win), (_, win2)| {
            if win.is_focused().is_ok_and(|b| b) {
                Ordering::Greater
            } else {
                win.z()
                    .unwrap_or_default()
                    .cmp(&win2.z().unwrap_or_default())
            }
        });

    win.map(|(i, _)| i)
        .ok_or(XCapError::Error(String::from("Focused window not found")))
}

pub struct RecEngine {
    target: Window,
    engine: OcrEngine,
}

impl RecEngine {
    const URL_RECOGNITION_MODEL: &str =
        "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.onnx";
    const URL_DETECTION_MODEL: &str =
        "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.onnx";

    const RECOGNITION_MODEL: &str = "models/text-recognition.onnx";
    const DETECTION_MODEL: &str = "models/text-detection.onnx";

    pub fn new(window: Window) -> anyhow::Result<Self> {
        let detection_model = Model::load_file("models/text-detection.onnx")?;
        let recognition_model = Model::load_file("models/text-recognition.onnx")?;

        let engine = OcrEngine::new(OcrEngineParams {
            detection_model: Some(detection_model),
            recognition_model: Some(recognition_model),
            ..Default::default()
        })?;

        Ok(Self {
            target: window,
            engine,
        })
    }

    pub fn process(&self) -> anyhow::Result<Vec<TextLine>> {
        let now = Instant::now();
        let capture = DynamicImage::ImageRgba8(self.target.capture_image()?).to_rgb8();
        let img_source = ocrs::ImageSource::from_bytes(capture.as_raw(), capture.dimensions())?;

        let ocr_input = self.engine.prepare_input(img_source)?;
        let word_rects = self.engine.detect_words(&ocr_input)?;
        let line_rects = self.engine.find_text_lines(&ocr_input, &word_rects);
        let line_texts = self.engine.recognize_text(&ocr_input, &line_rects)?;

        unsafe {
            ELAPSED = now.elapsed();
        }

        Ok(line_texts.into_iter().flatten().collect())
    }
}

pub fn download_models() {
    use std::fs::exists;
    let _ = std::fs::create_dir("models");

    fn dnld(url: &str, path: &str, msg: &str) {
        if !exists(path).is_ok_and(|b| b) {
            println!("{msg}");
            let _ = Command::new("curl").args([url, "-o", path]).spawn();
        }
    }

    dnld(
        RecEngine::URL_DETECTION_MODEL,
        RecEngine::DETECTION_MODEL,
        "Download detection model...",
    );

    dnld(
        RecEngine::URL_RECOGNITION_MODEL,
        RecEngine::RECOGNITION_MODEL,
        "Download detection model...",
    );
}
