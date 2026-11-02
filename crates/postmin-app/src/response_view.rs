use eframe::egui::{self, Align, Layout, Margin, RichText, Rounding, ScrollArea, Stroke, Ui};
use postmin_core::format::{self, BodyFormat};
use postmin_core::http::ResponseData;
use std::time::Instant;

use crate::highlight;
use crate::theme;
use crate::widgets;

const MAX_PRETTY_BYTES: usize = 2 * 1024 * 1024;
const MAX_DISPLAY_BYTES: usize = 1024 * 1024;
const MAX_HIGHLIGHT_BYTES: usize = 320 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseTab {
    #[default]
    Body,
    Headers,
}

#[derive(Debug)]
pub struct ResponseUiState {
    pub tab: ResponseTab,
    pub pretty: bool,
}

impl Default for ResponseUiState {
    fn default() -> Self {
        Self {
            tab: ResponseTab::Body,
            pretty: true,
        }
    }
}

#[derive(Debug)]
pub enum ResponseState {
    Idle,
    Sending { started: Instant },
    Ready(Box<ReadyResponse>),
    Error { message: String },
}

#[derive(Debug)]
pub struct ReadyResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub final_url: String,
    pub duration_ms: u128,
    pub size: usize,
    pub body: Vec<u8>,
    pub truncated: bool,
    pub binary: bool,
    pub charset: Option<String>,
    pub content_type: String,
    pub format: BodyFormat,
    pub text: String,
    pub pretty: Option<String>,
    pub pretty_error: Option<String>,
    pub pretty_skipped: bool,
}

impl ReadyResponse {
    pub fn from_data(data: ResponseData) -> Self {
        let content_type = format::header_value(&data.headers, "content-type")
            .unwrap_or_default()
            .to_string();
        let decoded = format::decode_body(&data.headers, &data.body);
        let format = if decoded.binary {
            BodyFormat::Text
        } else {
            format::detect_format(&content_type, &decoded.text)
        };

        let mut pretty = None;
        let mut pretty_error = None;
        let mut pretty_skipped = false;
        if !decoded.binary && !decoded.text.trim().is_empty() && format != BodyFormat::Text {
            if decoded.text.len() > MAX_PRETTY_BYTES {
                pretty_skipped = true;
            } else {
                let result = match format {
                    BodyFormat::Json => format::pretty_json(&decoded.text),
                    BodyFormat::Xml => format::pretty_xml(&decoded.text),
                    BodyFormat::Text => Ok(decoded.text.clone()),
                };
                match result {
                    Ok(value) => pretty = Some(value),
                    Err(error) => pretty_error = Some(error),
                }
            }
        }

        Self {
            status: data.status,
            status_text: data.status_text,
            headers: data.headers,
            final_url: data.final_url,
            duration_ms: data.duration_ms,
            size: data.body.len(),
            body: data.body,
            truncated: data.truncated,
            binary: decoded.binary,
            charset: decoded.charset,
            content_type,
            format,
            text: decoded.text,
            pretty,
            pretty_error,
            pretty_skipped,
        }
    }

    fn display_text(&self, pretty_enabled: bool) -> &str {
        if pretty_enabled && self.pretty.is_some() {
            self.pretty.as_deref().unwrap_or(&self.text)
        } else {
            &self.text
        }
    }
}

#[derive(Debug, Default)]
pub struct ResponseOutput {
    pub copy: Option<String>,
}

pub fn show(ui: &mut Ui, state: &ResponseState, ui_state: &mut ResponseUiState) -> ResponseOutput {
    let mut output = ResponseOutput::default();

    egui::Frame::none()
        .inner_margin(Margin::symmetric(18.0, 10.0))
        .show(ui, |ui| {
            let mut copy_body: Option<String> = None;

            ui.horizontal(|ui| {
                widgets::section_label(ui, "响应");
                ui.add_space(4.0);
                match state {
                    ResponseState::Idle => {
                        widgets::hint_label(ui, "等待发送");
                    }
                    ResponseState::Sending { started } => {
                        ui.add(egui::Spinner::new().size(13.0));
                        widgets::hint_label(
                            ui,
                            &format!("请求中 · {:.1}s", started.elapsed().as_secs_f32()),
                        );
                    }
                    ResponseState::Ready(response) => {
                        widgets::status_pill(ui, response.status, &response.status_text)
                            .on_hover_text(format!("最终地址：{}", response.final_url));
                        ui.add_space(2.0);
                        widgets::hint_label(ui, &widgets::format_duration(response.duration_ms));
                        ui.label(RichText::new("·").size(theme::font_size(10.5)).color(theme::muted()));
                        widgets::hint_label(ui, &widgets::format_size(response.size));
                        if response.truncated {
                            ui.label(
                                RichText::new("已截断 10 MB")
                                    .size(theme::font_size(11.0))
                                    .color(theme::yellow()),
                            );
                        }
                        if response.binary {
                            ui.label(RichText::new("二进制").size(theme::font_size(11.0)).color(theme::purple()));
                        } else if response.format != BodyFormat::Text {
                            ui.label(
                                RichText::new(response.format.label())
                                    .size(theme::font_size(10.5))
                                    .color(theme::text_dim()),
                            );
                        }
                    }
                    ResponseState::Error { .. } => {
                        ui.label(
                            RichText::new("请求失败")
                                .size(theme::font_size(13.0))
                                .color(theme::red())
                                .strong(),
                        );
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let ResponseState::Ready(response) = state {
                        if ui_state.tab == ResponseTab::Body
                            && !response.binary
                            && ui
                                .add(theme::small_ghost("复制响应体"))
                                .on_hover_text("复制当前显示的内容")
                                .clicked()
                        {
                            copy_body = Some(response.display_text(ui_state.pretty).to_string());
                        }
                        if ui_state.tab == ResponseTab::Headers
                            && ui
                                .add(theme::small_ghost("复制全部"))
                                .on_hover_text("复制全部响应头")
                                .clicked()
                        {
                            copy_body = Some(
                                response
                                    .headers
                                    .iter()
                                    .map(|(key, value)| format!("{key}: {value}"))
                                    .collect::<Vec<_>>()
                                    .join("\n"),
                            );
                        }
                    }
                });
            });

            ui.add_space(6.0);

            match state {
                ResponseState::Idle => {
                    ui.separator();
                    ui.add_space(6.0);
                    let remaining = ui.available_size();
                    ui.allocate_ui_with_layout(
                        remaining,
                        Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new("还没有响应")
                                        .size(theme::font_size(14.0))
                                        .color(theme::text_dim()),
                                );
                                ui.add_space(5.0);
                                widgets::hint_label(ui, "点击发送，或按 Ctrl+Enter 发送当前请求");
                            })
                        },
                    );
                }
                ResponseState::Sending { started } => {
                    ui.separator();
                    let remaining = ui.available_size();
                    ui.allocate_ui_with_layout(
                        remaining,
                        Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add(egui::Spinner::new().size(22.0));
                                ui.add_space(8.0);
                                widgets::hint_label(
                                    ui,
                                    &format!("正在等待响应 · {:.1}s", started.elapsed().as_secs_f32()),
                                );
                            })
                        },
                    );
                }
                ResponseState::Error { message } => {
                    ui.separator();
                    ui.add_space(8.0);
                    ScrollArea::vertical()
                        .id_source("response-error-scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                    egui::Frame::none()
                        .fill(theme::tint(theme::red(), 0.07))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::same(12.0))
                        .stroke(Stroke::new(1.0_f32, theme::tint(theme::red(), 0.22)))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.label(
                                RichText::new("无法完成请求")
                                    .size(theme::font_size(13.0))
                                    .color(theme::red())
                                    .strong(),
                            );
                            ui.add_space(4.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(message)
                                        .size(theme::font_size(12.0))
                                        .color(theme::text_dim())
                                        .monospace(),
                                )
                                .selectable(true)
                                .wrap(true),
                            );
                            ui.add_space(8.0);
                            if ui.add(theme::small_ghost("复制错误信息")).clicked() {
                                copy_body = Some(message.clone());
                            }
                            ui.add_space(6.0);
                            widgets::hint_label(
                                ui,
                                "请检查 URL、网络连接、代理或证书配置；可在右上角“设置”中调整超时时间。",
                            );
                        });
                        ui.add_space(8.0);
                        });
                }
                ResponseState::Ready(response) => {
                    ui.horizontal(|ui| {
                        let headers_label = format!("Headers  {}", response.headers.len());
                        if widgets::tab_button(
                            ui,
                            "Body",
                            ui_state.tab == ResponseTab::Body,
                        )
                        .clicked()
                        {
                            ui_state.tab = ResponseTab::Body;
                        }
                        if widgets::tab_button(
                            ui,
                            headers_label,
                            ui_state.tab == ResponseTab::Headers,
                        )
                        .clicked()
                        {
                            ui_state.tab = ResponseTab::Headers;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui_state.tab == ResponseTab::Body
                                && !response.binary
                                && response.format != BodyFormat::Text
                            {
                                ui.checkbox(&mut ui_state.pretty, "美化");
                            }
                            if response.charset.is_some() {
                                widgets::hint_label(
                                    ui,
                                    response.charset.as_deref().unwrap_or_default(),
                                );
                            }
                        });
                    });
                    ui.add_space(3.0);
                    ui.separator();
                    ui.add_space(6.0);

                    match ui_state.tab {
                        ResponseTab::Body => show_body(ui, response, ui_state),
                        ResponseTab::Headers => show_headers(ui, response),
                    }
                }
            }

            if let Some(copy) = copy_body {
                output.copy = Some(copy);
            }
        });

    output
}

fn show_body(ui: &mut Ui, response: &ReadyResponse, ui_state: &ResponseUiState) {
    if response.truncated {
        widgets::warning_bar(ui, "响应体超过 10 MB，仅保留前 10 MB。");
        ui.add_space(6.0);
    }
    if response.pretty_skipped {
        widgets::warning_bar(ui, "响应体较大，已跳过美化，直接显示原文。");
        ui.add_space(6.0);
    }
    if let Some(error) = &response.pretty_error {
        widgets::warning_bar(ui, &format!("{error}（已显示原文）"));
        ui.add_space(6.0);
    }

    if response.binary {
        let content_type = if response.content_type.is_empty() {
            "未知类型"
        } else {
            &response.content_type
        };
        widgets::hint_label(
            ui,
            &format!(
                "二进制响应 · {content_type} · {}",
                widgets::format_size(response.size)
            ),
        );
        ui.add_space(6.0);
        let dump = format::hex_dump(&response.body, 512);
        ScrollArea::both()
            .id_source("response-binary-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Frame::none()
                    .fill(theme::input())
                    .rounding(Rounding::same(6.0))
                    .inner_margin(Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(if dump.is_empty() {
                                    "响应体为空".to_string()
                                } else {
                                    dump
                                })
                                .monospace()
                                .size(theme::font_size(11.5))
                                .color(theme::text_dim()),
                            )
                            .wrap(false),
                        );
                        ui.add_space(8.0);
                    });
            });
        return;
    }

    let display_source = response.display_text(ui_state.pretty);
    if display_source.trim().is_empty() {
        widgets::empty_state(ui, "响应体为空", "服务器返回了空内容");
        return;
    }
    let (display, display_truncated) = safe_prefix(display_source, MAX_DISPLAY_BYTES);
    if display_truncated {
        widgets::warning_bar(
            ui,
            "内容过大，仅显示前 1 MB；使用“复制响应体”可获取完整内容。",
        );
        ui.add_space(6.0);
    }

    let format = if response.pretty_error.is_some() {
        BodyFormat::Text
    } else {
        response.format
    };
    let highlight_enabled = display.len() <= MAX_HIGHLIGHT_BYTES;
    let font_size = theme::font_size(theme::MONO);

    ScrollArea::both()
        .id_source("response-body-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(2.0);
            if highlight_enabled && format != BodyFormat::Text {
                let job = highlight::layout(display, format, font_size, f32::INFINITY);
                ui.add(egui::Label::new(job).selectable(true).wrap(false));
                ui.add_space(10.0);
            } else {
                ui.add(
                    egui::Label::new(
                        RichText::new(display)
                            .monospace()
                            .size(font_size)
                            .color(theme::text()),
                    )
                    .selectable(true)
                    .wrap(false),
                );
                ui.add_space(10.0);
            }
        });
}

fn show_headers(ui: &mut Ui, response: &ReadyResponse) {
    if response.headers.is_empty() {
        widgets::empty_state(ui, "没有响应头", "服务器未返回任何 Header");
        return;
    }
    ScrollArea::vertical()
        .id_source("response-headers-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let full_width = ui.available_width();
            let key_width = (full_width * 0.26).clamp(150.0, 240.0);
            for (index, (key, value)) in response.headers.iter().enumerate() {
                let stripe = if index % 2 == 0 {
                    theme::panel_2()
                } else {
                    egui::Color32::TRANSPARENT
                };
                egui::Frame::none()
                    .fill(stripe)
                    .rounding(Rounding::same(4.0))
                    .inner_margin(Margin::symmetric(8.0, 5.0))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal_top(|ui| {
                            let (key_rect, _) = ui.allocate_exact_size(
                                egui::Vec2::new(key_width, 18.0),
                                egui::Sense::hover(),
                            );
                            let painter = ui.painter().with_clip_rect(key_rect);
                            painter.text(
                                key_rect.left_center(),
                                egui::Align2::LEFT_CENTER,
                                key,
                                egui::FontId::monospace(theme::font_size(11.5)),
                                theme::blue(),
                            );
                            ui.add_space(10.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(value)
                                        .monospace()
                                        .size(theme::font_size(11.5))
                                        .color(theme::text_dim()),
                                )
                                .selectable(true)
                                .wrap(true),
                            );
                        });
                    });
                ui.add_space(2.0);
            }
            ui.add_space(6.0);
        });
}

fn safe_prefix(text: &str, max_bytes: usize) -> (&str, bool) {
    if text.len() <= max_bytes {
        return (text, false);
    }
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    (&text[..end], true)
}
