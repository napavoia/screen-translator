use std::process::exit;

use egui::{Color32, Id, Key, Modifiers, Rect, RichText, Rounding, Stroke, WidgetText};
use egui_overlay::{
    EguiOverlay, egui_render_three_d::ThreeDBackend, egui_window_glfw_passthrough::GlfwBackend,
};
use ocrs::{TextItem, TextLine};
use xcap::Window;

use crate::{
    handle_error,
    ocr::{EXPLORER, RectangleEngine, Task, focused_window_index},
    profiler,
    profiler::Profiler,
};

const TITLE: &str = "Screen Translator";

pub struct Overlay {
    engine: RectangleEngine,
    text: TextPipeline,
    settings: bool,
    borders: bool,
    debug_mode: bool,
    current: usize,
    windows: Vec<Window>,
}

impl EguiOverlay for Overlay {
    fn gui_run(&mut self, egui: &egui::Context, _: &mut ThreeDBackend, glfw: &mut GlfwBackend) {
        let w = handle_error!(self.window().width());
        let h = handle_error!(self.window().height());
        let x = handle_error!(self.window().x());
        let y = handle_error!(self.window().y());

        glfw.set_window_size([w as f32, h as f32]);
        glfw.window.set_pos(x, y);
        glfw.set_title(String::from(TITLE));

        if self.settings {
            self.settings(egui, glfw);
        }

        if self.debug_mode {
            self.debug(egui, glfw);
        }

        self.engine.process();

        match self.engine.texts() {
            Ok(v) => self.text.process(v),
            Err(_) => {}
        }

        egui::Area::new(Id::new("Text Area"))
            .interactable(false)
            .movable(false)
            .default_pos([x as f32, y as f32])
            .default_size([w as f32, h as f32])
            .order(egui::Order::Background)
            .show(egui, |ui| {
                self.draw_text(ui);
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
    pub fn new(engine: RectangleEngine, win: (Vec<Window>, usize)) -> Self {
        Self {
            // TEXT
            engine: engine,
            text: TextPipeline::new(),
            // FLAGS
            settings: true,
            borders: false,
            debug_mode: cfg!(debug_assertions),
            // WINDOWS
            windows: win.0,
            current: win.1,
        }
    }

    fn draw_text(&mut self, ui: &mut egui::Ui) {
        for val in self.text.text() {
            let stroke = match self.borders {
                true => Stroke::new(0.8, Color32::GREEN),
                false => Stroke::NONE,
            };
            let background = Color32::from_rgba_premultiplied(48, 48, 48, 220);
            let text = RichText::new(&val.text).color(Color32::from_rgb(255, 255, 255));

            ui.painter()
                .rect(val.rect, Rounding::same(10.0), background, stroke);

            ui.put(
                val.rect,
                egui::Label::new(WidgetText::RichText(text)).selectable(false),
            );
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
            struct Container {
                modifier: Modifiers,
                key: Key,
                result: Hotkey,
            }
            const HOTKEYS: [Container; 3] = [
                Container {
                    modifier: Modifiers::ALT,
                    key: Key::C,
                    result: Hotkey::SwithWindow,
                },
                Container {
                    modifier: Modifiers::ALT,
                    key: Key::T,
                    result: Hotkey::ToggleSetting,
                },
                Container {
                    modifier: Modifiers {
                        alt: true,
                        ctrl: true,
                        shift: false,
                        mac_cmd: false,
                        command: false,
                    },
                    key: Key::I,
                    result: Hotkey::Close,
                },
            ];

            egui.input_mut(|i| {
                for el in HOTKEYS {
                    if i.consume_key(el.modifier, el.key) {
                        return el.result;
                    }
                }

                Hotkey::None
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
                self.windows = windows;
                self.current = index;
                self.engine
                    .send_task(Task::SwapWindow(self.window().clone()));
            }
            Hotkey::ToggleSetting => self.settings = !self.settings,
            Hotkey::Close => exit(0),
        }
    }

    fn settings(&mut self, egui: &egui::Context, glfw: &mut GlfwBackend) {
        egui::Window::new("Settings").show(egui, |ui| {
            ui.set_width(300.0);

            let mut borders = glfw.window.is_decorated();
            if ui.checkbox(&mut borders, "Window Borders").changed() {
                glfw.window.set_decorated(borders);
            }

            let mut running = self
                .engine
                .running
                .load(std::sync::atomic::Ordering::Relaxed);
            if ui.checkbox(&mut running, "Translate").changed() {
                self.engine.send_task(Task::Toggle(running));
            };
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
                                self.engine
                                    .send_task(Task::SwapWindow(self.window().clone()));
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

    fn debug(&mut self, egui: &egui::Context, glfw: &mut GlfwBackend) {
        egui::Window::new("Debug").show(egui, |ui| {
            ui.checkbox(&mut self.borders, "Text Borders");
            ui.label(format!("Count Text Bounds: {}", self.text.text().len()));
            let size = &glfw.window_size_logical;
            ui.label(format!(
                "Window Info: {} ({}x{})",
                title(self.window()),
                size[0],
                size[1]
            ));
            ui.separator();

            Profiler::global().lock().draw(ui);
        });
    }

    fn windows(&mut self) -> Vec<Window> {
        handle_error!(Window::all() ;; vec![])
            .into_iter()
            .filter(|w| {
                w.app_name().is_ok_and(|v| v != EXPLORER) && w.is_minimized().is_ok_and(|b| !b)
            })
            .collect()
    }

    fn window(&self) -> &Window {
        &self.windows[self.current]
    }
}

fn title(win: &Window) -> String {
    handle_error!(win.app_name() ;; String::from("?"))
}

struct TextPipeline {
    inner_flag: usize,
    text: Vec<Text>,
}

impl TextPipeline {
    const Y_THRESHOLD: i32 = 10;

    pub fn new() -> Self {
        Self {
            inner_flag: 0,
            text: vec![],
        }
    }

    pub fn text(&self) -> &Vec<Text> {
        &self.text
    }

    pub fn process(&mut self, texts: &Vec<TextLine>) {
        //let inner = self.inner.load();
        if texts.len() == self.inner_flag {
            return;
        }

        self.text.clear();

        let _record = profiler!("pipeline_process");
        for line in texts.iter() {
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

        self.inner_flag = texts.len();
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
