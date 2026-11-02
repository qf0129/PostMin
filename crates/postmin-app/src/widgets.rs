use eframe::egui::{
    self, Align2, Color32, FontId, Rect, Response, RichText, Rounding, Sense, Stroke, Ui, Vec2,
};

use crate::theme;

pub const METHOD_BADGE_WIDTH: f32 = 46.0;

pub fn method_color(method: &str) -> Color32 {
    match method.trim().to_ascii_uppercase().as_str() {
        "GET" => theme::green(),
        "POST" => theme::accent(),
        "PUT" => theme::blue(),
        "PATCH" => theme::purple(),
        "DELETE" => theme::red(),
        "HEAD" => theme::cyan(),
        "OPTIONS" => theme::yellow(),
        _ => theme::muted(),
    }
}

fn clipped_method(method: &str) -> String {
    let upper = method.trim().to_ascii_uppercase();
    if upper.chars().count() > 5 {
        let mut text: String = upper.chars().take(4).collect();
        text.push('…');
        text
    } else if upper.is_empty() {
        "GET".to_string()
    } else {
        upper
    }
}

pub fn paint_method_badge(painter: &egui::Painter, rect: Rect, method: &str) {
    let color = method_color(method);
    painter.rect_filled(rect, Rounding::same(4.0), theme::tint(color, 0.15));
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        clipped_method(method),
        FontId::monospace(theme::font_size(10.0)),
        color,
    );
}

pub fn status_color(status: u16) -> Color32 {
    match status {
        200..=299 => theme::green(),
        300..=399 => theme::blue(),
        400..=499 => theme::yellow(),
        _ => theme::red(),
    }
}

pub fn status_pill(ui: &mut Ui, status: u16, status_text: &str) -> Response {
    let color = status_color(status);
    let label = format!("{status} {status_text}");
    let width = 28.0 + label.chars().count() as f32 * 6.4;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 24.0), Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, Rounding::same(12.0), theme::tint(color, 0.16));
        painter.rect_stroke(
            rect,
            Rounding::same(12.0),
            Stroke::new(1.0_f32, theme::tint(color, 0.45)),
        );
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::monospace(theme::font_size(12.0)),
            color,
        );
    }
    response
}

pub fn count_badge(ui: &mut Ui, count: usize) -> Response {
    let text = count.to_string();
    let width = 18.0 + text.chars().count() as f32 * 6.5;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 16.0), Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, Rounding::same(8.0), theme::card());
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            FontId::monospace(theme::font_size(10.0)),
            theme::text_dim(),
        );
    }
    response
}

pub fn tab_button(ui: &mut Ui, label: impl Into<String>, selected: bool) -> Response {
    let label = label.into();
    let color = if selected {
        theme::accent()
    } else {
        theme::text_dim()
    };
    let response = ui.add(
        egui::Button::new(
            RichText::new(label)
                .color(color)
                .size(theme::font_size(13.0))
                .strong(),
        )
        .fill(if selected {
            theme::tint(theme::accent(), 0.10)
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::NONE)
        .rounding(Rounding::same(6.0))
        .min_size(Vec2::new(0.0, 28.0)),
    );
    if selected && ui.is_rect_visible(response.rect) {
        let y = response.rect.bottom() - 1.5;
        ui.painter().line_segment(
            [
                egui::pos2(response.rect.left() + 9.0, y),
                egui::pos2(response.rect.right() - 9.0, y),
            ],
            Stroke::new(2.0_f32, theme::accent()),
        );
    }
    response
}

pub fn section_label(ui: &mut Ui, text: &str) -> Response {
    ui.label(
        RichText::new(text)
            .size(theme::font_size(11.0))
            .color(theme::muted())
            .strong(),
    )
}

pub fn empty_state(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(14.0);
        ui.label(
            RichText::new(title)
                .size(theme::font_size(14.0))
                .color(theme::text_dim()),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(subtitle)
                .size(theme::font_size(12.0))
                .color(theme::muted()),
        );
    });
}

pub fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub fn format_duration(milliseconds: u128) -> String {
    if milliseconds < 1000 {
        format!("{milliseconds} ms")
    } else {
        format!("{:.2} s", milliseconds as f64 / 1000.0)
    }
}

pub fn warning_bar(ui: &mut Ui, text: &str) {
    egui::Frame::none()
        .fill(theme::tint(theme::yellow(), 0.09))
        .rounding(Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(10.0, 7.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(theme::font_size(12.0))
                    .color(theme::yellow()),
            );
        });
}

pub fn error_bar(ui: &mut Ui, text: &str) {
    egui::Frame::none()
        .fill(theme::tint(theme::red(), 0.09))
        .rounding(Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(10.0, 7.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(theme::font_size(12.0))
                    .color(theme::red()),
            );
        });
}

pub fn hint_label(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(theme::font_size(11.5))
            .color(theme::muted()),
    );
}

pub fn shortcut_label(ui: &mut Ui, key: &str, action: &str) {
    ui.horizontal(|ui| {
        egui::Frame::none()
            .fill(theme::card())
            .rounding(Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(6.0, 2.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(key)
                        .monospace()
                        .size(theme::font_size(11.0))
                        .color(theme::text_dim()),
                );
            });
        ui.label(
            RichText::new(action)
                .size(theme::font_size(12.0))
                .color(theme::muted()),
        );
    });
}

pub fn small_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.5, color);
}
