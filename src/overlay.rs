use std::{
    process::exit,
    sync::Arc,
    thread::JoinHandle,
    time::{Duration, Instant},
};

use egui::{Color32, Id, Key, Modifiers, Rect, Stroke};
use egui_overlay::{
    EguiOverlay, egui_render_three_d::ThreeDBackend, egui_window_glfw_passthrough::GlfwBackend,
};
use ocrs::{TextItem, TextLine};
use xcap::Window;

use crate::{
    ArcBuffer,
    ocr::{ELAPSED, EXPLORER, focused_window_index},
    thread,
};

const TITLE: &str = "Screen Translator";

pub struct Overlay {
    thread: JoinHandle<()>,
    text: TextPipeline,
    settings: bool,
    borders: bool,
    debug_mode: bool,
    enable_ocr: bool,
    current: usize,
    windows: Vec<Window>,
}

impl EguiOverlay for Overlay {
    fn gui_run(&mut self, egui: &egui::Context, _: &mut ThreeDBackend, glfw: &mut GlfwBackend) {
        let w = self.window().width().unwrap();
        let h = self.window().height().unwrap();
        let x = self.window().x().unwrap();
        let y = self.window().y().unwrap();

        glfw.set_window_size([w as f32, h as f32]);
        glfw.window.set_pos(x, y);
        glfw.set_title(String::from(TITLE));

        if self.settings {
            self.settings(egui, glfw);
        }

        self.text.process();
        egui::Area::new(Id::new("Text Area"))
            .interactable(false)
            .movable(false)
            .default_pos([x as f32, y as f32])
            .default_size([w as f32, h as f32])
            .order(egui::Order::Background)
            .show(egui, |ui| {
                if self.enable_ocr {
                    self.draw_text(ui);
                }

                if self.borders {
                    self.draw_borders(ui)
                }
            });

        self.hotkeys(egui);

        if egui.wants_pointer_input() || egui.wants_keyboard_input() {
            glfw.set_passthrough(false);
        } else {
            glfw.set_passthrough(true)
        }

        egui.request_repaint();
    }
}

impl Overlay {
    pub fn new() -> Self {
        let texts: ArcBuffer = Arc::default();
        let cloned = texts.clone();
        let windows = Window::all().unwrap_or_default();
        let current = focused_window_index(&windows).unwrap();

        Self {
            // TEXT
            thread: thread(windows[current].clone(), cloned),
            text: TextPipeline::new(texts),
            // FLAGS
            settings: true,
            borders: false,
            debug_mode: false,
            enable_ocr: true,
            // WINDOWS
            windows,
            current,
        }
    }

    fn draw_text(&mut self, ui: &mut egui::Ui) {
        for val in self.text.text() {
            ui.put(val.rect, egui::Label::new(&val.text).selectable(false));
        }
    }

    fn draw_borders(&mut self, ui: &mut egui::Ui) {
        for val in self.text.text() {
            ui.painter()
                .rect_stroke(val.rect, 0.0, Stroke::new(0.8, Color32::GREEN));
        }
    }

    fn hotkeys(&mut self, egui: &egui::Context) {
        enum Hotkey {
            SwithWindow,
            ToggleSetting,
            Close,
            None,
        }

        fn hotkeys(egui: &egui::Context) -> Hotkey {
            egui.input_mut(|i| {
                if i.consume_key(Modifiers::ALT, Key::C) {
                    Hotkey::SwithWindow
                } else if i.consume_key(Modifiers::ALT, Key::I) {
                    Hotkey::ToggleSetting
                } else if i.consume_key(
                    Modifiers {
                        alt: true,
                        ctrl: true,
                        ..Default::default()
                    },
                    Key::I,
                ) {
                    Hotkey::Close
                } else {
                    Hotkey::None
                }
            })
        }

        match hotkeys(egui) {
            Hotkey::None => {}
            Hotkey::SwithWindow => {
                let windows = self.windows();
                let index = focused_window_index(&windows).unwrap();
                if self.current == index {
                    return;
                }
                self.current = index;
                self.refresh_thread();
            }
            Hotkey::ToggleSetting => self.settings = !self.settings,
            Hotkey::Close => exit(0),
        }
    }

    fn settings(&mut self, egui: &egui::Context, glfw: &mut GlfwBackend) {
        egui::Window::new("Settings").show(egui, |ui| {
            ui.set_width(300.0);

            if self.debug_mode {
                self.debug(ui, glfw);
            }

            let mut borders = glfw.window.is_decorated();
            if ui.checkbox(&mut borders, "Window Borders").changed() {
                glfw.window.set_decorated(borders);
            }

            ui.checkbox(&mut self.enable_ocr, "Translate");
            ui.checkbox(&mut self.borders, "Text Borders");
            ui.checkbox(&mut self.debug_mode, "Debug Mode");

            ui.separator();
            ui.horizontal(|ui| {
                egui::ComboBox::from_label("")
                    .selected_text(title(self.window()))
                    .show_ui(ui, |ui| {
                        let windows = self.windows();
                        for (i, win) in windows.iter().enumerate() {
                            let text = title(win);
                            if ui.selectable_value(&mut self.current, i, text).changed() {
                                self.windows = windows;
                                self.refresh_thread();
                                break;
                            };
                        }
                    });

                if ui.button("Hide Settings").clicked() {
                    self.settings = false
                }

                if ui.button("EXIT").clicked() {
                    exit(0)
                }
            })
        });
    }

    fn debug(&mut self, ui: &mut egui::Ui, glfw: &mut GlfwBackend) {
        ui.label(format!("Count Text Bounds: {}", self.text.text().len()));
        ui.label(format!("Elapsed time on recognize text: {:?}", unsafe {
            ELAPSED
        }));
        ui.label(format!(
            "Elapsed time on merge text: {:?}",
            self.text.elapsed
        ));

        let size = &glfw.window_size_logical;
        ui.label(format!(
            "Window Info: {} ({}x{})",
            title(self.window()),
            size[0],
            size[1]
        ));

        ui.separator();
    }

    fn windows(&mut self) -> Vec<Window> {
        Window::all()
            .unwrap_or_default()
            .into_iter()
            .filter(|w| {
                w.app_name().is_ok_and(|v| v != EXPLORER) && w.is_minimized().is_ok_and(|b| !b)
            })
            .collect()
    }

    fn window(&self) -> &Window {
        &self.windows[self.current]
    }

    fn refresh_thread(&mut self) {
        self.thread = thread(self.window().clone(), self.text.inner.clone());
    }
}

fn title(win: &Window) -> String {
    win.app_name().unwrap_or(String::from("?"))
}

struct TextPipeline {
    inner: ArcBuffer,
    inner_flag: usize,
    text: Vec<Text>,
    elapsed: Duration,
}

impl TextPipeline {
    const Y_THRESHOLD: i32 = 10;

    pub fn new(buff: ArcBuffer) -> Self {
        Self {
            inner: buff,
            inner_flag: 0,
            text: vec![],
            elapsed: Duration::from_secs(0),
        }
    }

    pub fn text(&self) -> &Vec<Text> {
        &self.text
    }

    pub fn process(&mut self) {
        let inner = self.inner.load();
        if inner.len() == self.inner_flag {
            return;
        }

        let now = Instant::now();

        self.text.clear();

        for line in inner.iter() {
            let text = line.to_string();
            let rect = line.bounding_rect();

            if text.parse::<f32>().is_ok() {
                continue;
            }

            if let Some(last) = self.text.last_mut() {
                let overlap_x = (last.rect.left() as i32) < rect.right()
                    && rect.left() < (last.rect.right() as i32);
                let overlap_y = (last.rect.top() as i32) < rect.bottom() + Self::Y_THRESHOLD
                    && rect.top() < (last.rect.bottom() as i32) + Self::Y_THRESHOLD;

                if overlap_x && overlap_y {
                    let x = last.rect.left().min(rect.right() as f32);
                    let y = last.rect.top().min(rect.top() as f32);
                    let width = last.rect.right().max(rect.right() as f32);
                    let height = last.rect.bottom().max(rect.bottom() as f32);

                    last.rect = Rect {
                        min: [x, y].into(),
                        max: [width, height].into(),
                    };
                    last.text = format!("{}\n{}", last.text, text);
                    continue;
                }
            }

            self.text.push(Text::from(line));
        }

        self.elapsed = now.elapsed();
        self.inner_flag = inner.len();
    }
}

struct Text {
    text: String,
    rect: Rect,
}

impl Text {
    pub fn from(text: &TextLine) -> Self {
        let rect = text.bounding_rect();
        Self {
            text: text.to_string(),
            rect: Rect {
                min: [rect.left() as f32, rect.top() as f32].into(),
                max: [rect.right() as f32, rect.bottom() as f32].into(),
            },
        }
    }
}
