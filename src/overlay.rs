use std::{process::exit, sync::Arc, thread::JoinHandle};

use arc_swap::ArcSwap;
use egui::{Color32, Id, Rect, Stroke, pos2};
use egui_overlay::{
    EguiOverlay, egui_render_three_d::ThreeDBackend, egui_window_glfw_passthrough::GlfwBackend,
};
use ocrs::{TextItem, TextLine};
use xcap::Window;

use crate::{
    ocr::{ELAPSED, EXPLORER, focused_window_index},
    thread,
};

const TITLE: &str = "Screen Translator";

pub struct Overlay {
    thread: JoinHandle<()>,
    texts: Arc<ArcSwap<Vec<TextLine>>>,
    settings: bool,
    borders: bool,
    debug_mode: bool,
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

        if self.borders {
            egui::Area::new(Id::new("Text Area"))
                .interactable(false)
                .movable(false)
                .default_pos([x as f32, y as f32])
                .default_size([w as f32, h as f32])
                .order(egui::Order::Background)
                .show(egui, |ui| self.draw_text(ui));
        }

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
        let texts = Arc::new(ArcSwap::default());
        let cloned = texts.clone();
        let windows = Window::all().unwrap_or_default();
        let current = focused_window_index(&windows).unwrap();
        //let window = WrapWindow(windows[index].clone());

        Self {
            texts,
            settings: true,
            borders: false,
            debug_mode: false,
            thread: thread(windows[current].clone(), cloned),
            windows,
            current,
        }
    }

    fn draw_text(&mut self, ui: &mut egui::Ui) {
        for val in self.texts.load().iter() {
            let rect = val.bounding_rect();

            let rect = Rect::from_min_max(
                pos2(rect.left() as f32, rect.top() as f32),
                pos2(rect.right() as f32, rect.bottom() as f32),
            );

            ui.painter()
                .rect_stroke(rect, 0.0, Stroke::new(0.8, Color32::GREEN));
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

            ui.checkbox(&mut self.borders, "Text Borders");
            ui.checkbox(&mut self.debug_mode, "Debug Mode");

            ui.separator();
            ui.horizontal(|ui| {
                egui::ComboBox::from_label("")
                    .selected_text(format!("{}", title(self.window())))
                    .show_ui(ui, |ui| {
                        let windows = self.windows();
                        for (i, win) in windows.iter().enumerate() {
                            let text = format!("{}", title(&win));
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
        ui.label(format!("Count Text Bounds: {}", self.texts.load().len()));
        ui.label(format!("Elapsed time on recognize text: {:?}", unsafe {
            ELAPSED
        }));

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
                w.app_name().is_ok_and(|v| &v != EXPLORER) && w.is_minimized().is_ok_and(|b| !b)
            })
            .collect()
    }

    fn window(&self) -> &Window {
        &self.windows[self.current]
    }

    fn refresh_thread(&mut self) {
        self.thread = thread(self.window().clone(), self.texts.clone())
    }
}

fn title(win: &Window) -> String {
    win.app_name().unwrap_or(String::from("?"))
}
