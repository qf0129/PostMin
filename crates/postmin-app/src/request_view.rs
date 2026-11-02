use eframe::egui::{
    self, Align, FontId, Layout, Margin, RichText, Rounding, ScrollArea, Stroke, TextEdit, Ui, Vec2,
};
use postmin_core::{format::BodyFormat, name_from_url, BodyKind, HeaderRow, SavedRequest};

use crate::highlight;
use crate::theme;
use crate::widgets;

const COMMON_METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RequestTab {
    #[default]
    Headers,
    Body,
}

#[derive(Debug, Default)]
pub struct RequestUiState {
    pub tab: RequestTab,
    pub format_error: Option<String>,
    pub focus_url: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestAction {
    Send,
}

#[derive(Debug, Default)]
pub struct RequestOutput {
    pub action: Option<RequestAction>,
    pub changed: bool,
}

pub fn show(
    ui: &mut Ui,
    request: &mut SavedRequest,
    state: &mut RequestUiState,
    sending: bool,
    timeout_secs: u64,
) -> RequestOutput {
    let mut output = RequestOutput::default();

    egui::Frame::none()
        .inner_margin(Margin::symmetric(18.0, 12.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let name_width = (ui.available_width() * 0.18).clamp(120.0, 190.0);
                let name_response = ui.add_sized(
                    [name_width, 30.0],
                    TextEdit::singleline(&mut request.name)
                        .hint_text("请求名称")
                        .font(FontId::proportional(theme::font_size(14.0)))
                        .margin(Vec2::new(8.0, 5.0)),
                );
                if name_response.changed() {
                    request.auto_name = false;
                    output.changed = true;
                }

                let method_response = ui.add_sized(
                    [76.0, 30.0],
                    TextEdit::singleline(&mut request.method)
                        .char_limit(20)
                        .font(FontId::monospace(theme::font_size(12.0)))
                        .horizontal_align(Align::Center)
                        .margin(Vec2::new(4.0, 6.0)),
                );
                if method_response.changed() {
                    request.method = request.method.trim().to_ascii_uppercase();
                    output.changed = true;
                }
                ui.menu_button(
                    RichText::new("v")
                        .size(theme::font_size(9.0))
                        .color(theme::muted()),
                    |ui| {
                        for method in COMMON_METHODS {
                            if ui
                                .selectable_label(
                                    request.method.eq_ignore_ascii_case(method),
                                    method,
                                )
                                .clicked()
                            {
                                request.method = method.to_string();
                                output.changed = true;
                                ui.close_menu();
                            }
                        }
                    },
                )
                .response
                .on_hover_text("选择常用 Method（也可直接输入自定义 Method）");

                let send_width = 92.0;
                let url_width = (ui.available_width() - send_width - 8.0).max(180.0);
                let url_response = ui.add_sized(
                    [url_width, 30.0],
                    TextEdit::singleline(&mut request.url)
                        .hint_text("https://api.example.com/endpoint")
                        .font(FontId::monospace(theme::font_size(12.0)))
                        .margin(Vec2::new(9.0, 6.0)),
                );
                if state.focus_url {
                    url_response.request_focus();
                    state.focus_url = false;
                }
                if url_response.changed() {
                    if request.auto_name {
                        if let Some(name) = name_from_url(&request.url) {
                            request.name = name;
                        }
                    }
                    output.changed = true;
                }
                let enter_pressed = url_response.lost_focus()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if enter_pressed && !sending {
                    output.action = Some(RequestAction::Send);
                }

                let label = if sending { "请求中…" } else { "发送" };
                let send_button =
                    theme::primary_button(label).min_size(Vec2::new(send_width, 32.0));
                let send = ui.add_enabled(!sending, send_button);
                let send =
                    send.on_hover_text(format!("发送请求（Ctrl+Enter）· 超时 {timeout_secs} 秒"));
                if send.clicked() && !sending {
                    output.action = Some(RequestAction::Send);
                }
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                let headers_label = format!(
                    "Headers  {}",
                    request
                        .headers
                        .iter()
                        .filter(|header| header.enabled && !header.key.trim().is_empty())
                        .count()
                );
                if widgets::tab_button(ui, headers_label, state.tab == RequestTab::Headers)
                    .clicked()
                {
                    state.tab = RequestTab::Headers;
                }
                let body_label = match request.body_kind {
                    BodyKind::None => "Body".to_string(),
                    kind => format!("Body · {}", kind.label()),
                };
                if widgets::tab_button(ui, body_label, state.tab == RequestTab::Body).clicked() {
                    state.tab = RequestTab::Body;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    widgets::hint_label(ui, "Ctrl+Enter 发送");
                });
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(10.0);

            match state.tab {
                RequestTab::Headers => {
                    output.changed |= show_headers(ui, request);
                }
                RequestTab::Body => {
                    output.changed |= show_body(ui, request, state);
                }
            }
        });

    output
}

fn show_headers(ui: &mut Ui, request: &mut SavedRequest) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        widgets::section_label(ui, "请求头");
        widgets::count_badge(ui, request.headers.len());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.add(theme::small_ghost("+ 添加")).clicked() {
                request.headers.push(HeaderRow::new("", ""));
                changed = true;
            }
            ui.menu_button(
                RichText::new("常用")
                    .size(theme::font_size(12.0))
                    .color(theme::text_dim()),
                |ui| {
                    for (key, value) in [
                        ("Content-Type", "application/json; charset=utf-8"),
                        ("Accept", "application/json"),
                        ("Authorization", "Bearer <token>"),
                        ("User-Agent", "PostMin/0.1"),
                        ("Accept-Encoding", "gzip, deflate"),
                    ] {
                        if ui.button(format!("{key}: {value}")).clicked() {
                            request.headers.push(HeaderRow::new(key, value));
                            changed = true;
                            ui.close_menu();
                        }
                    }
                },
            );
        });
    });
    ui.add_space(6.0);

    if request.headers.is_empty() {
        egui::Frame::none()
            .fill(theme::card())
            .rounding(Rounding::same(7.0))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("没有额外请求头")
                        .size(theme::font_size(13.0))
                        .color(theme::text_dim()),
                );
                ui.add_space(2.0);
                widgets::hint_label(ui, "发送时如需自定义 Content-Type、鉴权等，可在此添加。");
            });
        return changed;
    }

    let has_content_type = request.headers.iter().any(|header| {
        header.enabled
            && !header.key.trim().is_empty()
            && header.key.trim().eq_ignore_ascii_case("content-type")
    });
    if request.body_kind != BodyKind::None && !has_content_type {
        if let Some(content_type) = request.body_kind.content_type() {
            ui.add_space(2.0);
            widgets::warning_bar(
                ui,
                &format!("未设置 Content-Type，发送时会自动添加 {content_type}"),
            );
            ui.add_space(6.0);
        }
    }

    let mut remove_index = None;
    ScrollArea::vertical()
        .id_source("headers-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (index, header) in request.headers.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let enabled = ui
                        .add(egui::Checkbox::new(&mut header.enabled, ""))
                        .on_hover_text("启用 / 禁用该请求头");
                    changed |= enabled.changed();

                    let key = ui.add_sized(
                        [180.0, 26.0],
                        TextEdit::singleline(&mut header.key)
                            .hint_text("Header 名称")
                            .font(FontId::monospace(theme::font_size(12.0)))
                            .margin(Vec2::new(6.0, 4.0)),
                    );
                    changed |= key.changed();

                    let value_width = (ui.available_width() - 34.0).max(120.0);
                    let value = ui.add_sized(
                        [value_width, 26.0],
                        TextEdit::singleline(&mut header.value)
                            .hint_text("值")
                            .font(FontId::monospace(theme::font_size(12.0)))
                            .margin(Vec2::new(6.0, 4.0)),
                    );
                    changed |= value.changed();

                    if ui
                        .add(theme::small_ghost("×"))
                        .on_hover_text("删除该请求头")
                        .clicked()
                    {
                        remove_index = Some(index);
                    }
                });
            }
        });
    if let Some(index) = remove_index {
        request.headers.remove(index);
        changed = true;
    }

    changed
}

fn show_body(ui: &mut Ui, request: &mut SavedRequest, state: &mut RequestUiState) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        widgets::section_label(ui, "请求体");
        ui.add_space(4.0);
        for (kind, label) in [
            (BodyKind::None, "无"),
            (BodyKind::Json, "JSON"),
            (BodyKind::Xml, "XML"),
            (BodyKind::Text, "文本"),
        ] {
            let selected = request.body_kind == kind;
            let text = RichText::new(label)
                .size(theme::font_size(12.0))
                .color(if selected {
                    theme::accent()
                } else {
                    theme::text_dim()
                });
            let response = ui.add(
                egui::Button::new(text)
                    .fill(if selected {
                        theme::tint(theme::accent(), 0.12)
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .stroke(Stroke::NONE)
                    .rounding(Rounding::same(5.0))
                    .min_size(Vec2::new(44.0, 24.0)),
            );
            if response.clicked() && !selected {
                request.body_kind = kind;
                state.format_error = None;
                changed = true;
            }
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            widgets::hint_label(ui, &format!("{} 字节", request.body.len()));
            let format_kind = match request.body_kind {
                BodyKind::Json => Some((true, "美化 JSON 请求体")),
                BodyKind::Xml => Some((false, "美化 XML 请求体")),
                _ => None,
            };
            if let Some((is_json, hover)) = format_kind {
                if !request.body.trim().is_empty()
                    && ui
                        .add(theme::small_ghost("格式化"))
                        .on_hover_text(hover)
                        .clicked()
                {
                    let result = if is_json {
                        postmin_core::format::pretty_json(&request.body)
                    } else {
                        postmin_core::format::pretty_xml(&request.body)
                    };
                    match result {
                        Ok(pretty) => {
                            request.body = pretty;
                            state.format_error = None;
                            changed = true;
                        }
                        Err(error) => state.format_error = Some(error),
                    }
                }
            }
            if !request.body.is_empty() && ui.add(theme::small_ghost("清空")).clicked() {
                request.body.clear();
                state.format_error = None;
                changed = true;
            }
        });
    });

    ui.add_space(8.0);

    if request.body_kind == BodyKind::None {
        egui::Frame::none()
            .fill(theme::card())
            .rounding(Rounding::same(7.0))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("该请求没有请求体")
                        .size(theme::font_size(13.0))
                        .color(theme::text_dim()),
                );
                ui.add_space(2.0);
                widgets::hint_label(
                    ui,
                    "GET/HEAD 通常不需要请求体；需要时选择 JSON、XML 或文本。",
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add(theme::small_ghost("使用 JSON")).clicked() {
                        request.body_kind = BodyKind::Json;
                        changed = true;
                    }
                    if ui.add(theme::small_ghost("使用 XML")).clicked() {
                        request.body_kind = BodyKind::Xml;
                        changed = true;
                    }
                    if ui.add(theme::small_ghost("使用文本")).clicked() {
                        request.body_kind = BodyKind::Text;
                        changed = true;
                    }
                });
            });
        return changed;
    }

    let editor_height = (ui.available_height() - 34.0).max(140.0);
    let kind = request.body_kind;
    let hint = match kind {
        BodyKind::Json => "{\n  \"key\": \"value\"\n}",
        BodyKind::Xml => "<root>\n  <item>value</item>\n</root>",
        BodyKind::Text => "请求体内容…",
        BodyKind::None => "",
    };

    egui::Frame::none()
        .fill(theme::input())
        .rounding(Rounding::same(7.0))
        .stroke(Stroke::new(1.0_f32, theme::border_soft()))
        .inner_margin(Margin::same(2.0))
        .show(ui, |ui| {
            ui.set_min_height(editor_height);
            ui.set_max_height(editor_height);
            ScrollArea::vertical()
                .id_source("body-editor-scroll")
                .max_height(editor_height - 6.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let body_format = match kind {
                        BodyKind::Json => BodyFormat::Json,
                        BodyKind::Xml => BodyFormat::Xml,
                        _ => BodyFormat::Text,
                    };
                    let mut layouter = |ui: &Ui, text: &str, wrap_width: f32| {
                        let job = highlight::layout(text, body_format, theme::MONO, wrap_width);
                        ui.fonts(|fonts| fonts.layout_job(job))
                    };
                    let response = ui.add(
                        TextEdit::multiline(&mut request.body)
                            .hint_text(hint)
                            .desired_width(f32::INFINITY)
                            .desired_rows(16)
                            .margin(Margin::same(10.0))
                            .frame(false)
                            .layouter(&mut layouter),
                    );
                    changed |= response.changed();
                });
        });

    if let Some(error) = &state.format_error {
        ui.add_space(6.0);
        widgets::error_bar(ui, error);
    } else if let Some(content_type) = request.body_kind.content_type() {
        ui.add_space(6.0);
        widgets::hint_label(
            ui,
            &format!("自动 Content-Type：{content_type}（若 Headers 中未显式设置）"),
        );
    }

    changed
}
