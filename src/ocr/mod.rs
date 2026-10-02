use std::{fmt::{ format}, io};

use rust_ocr::Coordinates;
use xcap::{Window, XCapError, XCapResult};

const TEMP_TITLE: &str = "tmp_capture.png";

fn focused_window() -> XCapResult<Window> {
    let windows = Window::all()?;
    for win in windows {
        if win.is_focused()? {
            return Ok(win);
        }
    }

    Err(XCapError::Error(String::from("Focused window not found")))
}

fn save_capture(window: Window) -> XCapResult<()> {
    let img = window.capture_image()?;
    if let Err(err) = img.save(TEMP_TITLE) {
        return match err {
            xcap::image::ImageError::IoError(error) => Err(XCapError::Error(format("IO error: {error}")))
            _ => Err(XCapError::NotSupported),
        };
    }

    Ok(())
}

fn parse() -> Result<Vec<Coordinates>, String>{
    rust_ocr::ocr_with_bounds(TEMP_TITLE, None).map_err(|e| format!("ocr err{e}"))
}


pub struct Stated{
    target: Window,
    is_old: bool,
    pub text: Vec<Coordinates>
}

impl Stated {
    pub fn new(window: Window) -> Stated{
        Self { target: window, is_old: false, text: vec![] }
    }


    pub fn save_picture(&mut self) -> Result<(), Error>{
        let img = self.target.capture_image().map_err(|e| Error::Text(format!("Err: {e}")))?;
        img.save(TEMP_TITLE).map_err(|e| Error::Text(format!("Err: {e}")))?;

        self.is_old = false;
        Ok(())
    }

    pub fn parse(&mut self) -> Result<(), Error>{
        if self.is_old {
            return Err(Error::Old);
        }

        self.text = parse()?;
        Ok(())
    }
}

enum Error {
    Old,
    Io(io::Error),
    Text(String),
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}


impl From<String> for Error {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}
