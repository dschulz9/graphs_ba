use std::{
    f32::consts::PI,
    sync::mpsc::{self, Receiver},
};

use egui::{Color32, Pos2, Rect, Shape, Slider, emath, epaint::RectShape, pos2, vec2};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct GraphVisualizer {
    // Example stuff:
    graph: String,

    #[serde(skip)]
    rec_channel: Option<Receiver<Vec<u8>>>,
    #[serde(skip)]
    circle_center: Pos2,
    #[serde(skip)]
    circle_radius: f32,
    #[serde(skip)]
    scene_rect: Rect,
}

impl Default for GraphVisualizer {
    fn default() -> Self {
        Self {
            // Example stuff:
            graph: String::new(),
            rec_channel: None,
            circle_center: Pos2::ZERO,
            circle_radius: 0.5,
            scene_rect: Rect::from_min_max(pos2(-1.0, -1.0), pos2(1.0, 1.0)),
        }
    }
}

impl GraphVisualizer {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        }
    }

    fn paint(&mut self, painter: &egui::Painter) {
        let mut shapes: Vec<Shape> = Vec::new();
        fn flip_y_axis(mut point: Pos2) -> Pos2 {
            point.y = -point.y;
            return point;
        }
        shapes.push(Shape::circle_filled(
            flip_y_axis(self.circle_center),
            0.25 * self.circle_radius,
            Color32::WHITE,
        ));
        let mut paint_line = |points: [Pos2; 2], color: Color32, width: f32| {
            let line = [flip_y_axis(points[0]), flip_y_axis(points[1])];

            // culling
            // if rect.intersects(Rect::from_two_pos(line[0], line[1])) {
            shapes.push(Shape::line_segment(line, (width, color)));
            // }
        };

        for i in 0..10 {
            let center = pos2(0.0, 0.0);
            let phi = PI * i as f32 / 5.0;
            let end = center
                + vec2(
                    self.circle_radius * f32::cos(phi),
                    self.circle_radius * f32::sin(phi),
                );
            paint_line([center, end], Color32::from_additive_luminance(255), 0.005);
        }

        painter.extend(shapes);
    }
}

impl eframe::App for GraphVisualizer {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        ui.request_repaint();

        if let Some(rec) = &self.rec_channel {
            match rec.try_recv() {
                Ok(result) => match String::from_utf8(result) {
                    Ok(string) => {
                        self.graph = string;
                    }
                    Err(err) => {
                        self.graph = format!("Failed to load file: {err}");
                    }
                },
                Err(mpsc::TryRecvError::Disconnected) => {}
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }

        egui::Panel::top("top_panel").show(ui, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::Panel::left("SidePannel").show(ui, |ui| {
            egui::Panel::bottom("GraphControls").show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Plot").clicked() {}
                    if ui.button("Clear").clicked() {
                        self.graph.clear();
                    }

                    if ui.button("Load File").clicked() {
                        let ctx = ui.ctx().clone();
                        let (sc, rc) = mpsc::channel();
                        self.rec_channel = Some(rc);
                        wasm_bindgen_futures::spawn_local(async move {
                            if let Some(file) = rfd::AsyncFileDialog::new()
                                .add_filter("Dot Files", &["dot"])
                                .pick_file()
                                .await
                            {
                                let content = file.read().await;
                                let _ = sc.send(content);
                                ctx.request_repaint();
                            }
                        });
                    }
                });
            });
            ui.label("Input Graph");

            let rem_size = ui.available_size();
            egui::ScrollArea::vertical()
                .max_height(rem_size.y)
                .show(ui, |ui| {
                    ui.add_sized(
                        rem_size,
                        egui::TextEdit::multiline(&mut self.graph)
                            .hint_text("graph g { a -- b }")
                            .code_editor()
                            .desired_width(300.0),
                    );
                });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.horizontal(|ui| {
                ui.add(Slider::new(&mut self.circle_center.x, -4.0..=4.0));
                ui.add(Slider::new(&mut self.circle_center.y, -4.0..=4.0));
                ui.add(Slider::new(&mut self.circle_radius, 0.0..=4.0));
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
                ui.separator();

                web_sys::console::log_1(&format!("scene_rect: {:?}", self.scene_rect).into());
                let mut scene_rect = self.scene_rect;
                egui::Scene::new().zoom_range(0.0..=f32::INFINITY).show(
                    ui,
                    &mut scene_rect,
                    |scene_ui| {
                        self.paint(&mut scene_ui.painter());
                    },
                );

                self.scene_rect = scene_rect;
                web_sys::console::log_1(&format!("scene_rect: {:?}", self.scene_rect).into());
            });
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
