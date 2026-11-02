use eframe::egui::{
    self, Align, Align2, Context, Key, Layout, Margin, RichText, Rounding, ScrollArea, Stroke,
    TextEdit, Vec2,
};
use postmin_core::{Settings, ThemeMode};
use std::path::Path;

use crate::theme;
use crate::widgets;

#[derive(Debug, Clone)]
pub struct RenameDialog {
    pub id: String,
    pub name: String,
    pub is_folder: bool,
    pub focus: bool,
}

impl RenameDialog {
    pub fn new(id: String, name: String, is_folder: bool) -> Self {
        Self {
            id,
            name,
            is_folder,
            focus: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameOutcome {
    None,
    Cancel,
    Confirm,
}

pub fn show_rename(ctx: &Context, dialog: &mut RenameDialog) -> RenameOutcome {
    let mut outcome = RenameOutcome::None;
    let title = if dialog.is_folder {
        "重命名目录"
    } else {
        "重命名请求"
    };
    egui::Window::new(title)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .collapsible(false)
        .resizable(false)
        .default_width(400.0)
        .show(ctx, |ui| {
            widgets::hint_label(ui, "名称会显示在左侧集合树中。");
            ui.add_space(8.0);
            let response = ui.add_sized(
                [ui.available_width(), 30.0],
                TextEdit::singleline(&mut dialog.name)
                    .hint_text("输入名称")
                    .margin(Vec2::new(8.0, 6.0)),
            );
            if dialog.focus {
                response.request_focus();
                dialog.focus = false;
            }
            let enter = ui.input(|input| input.key_pressed(Key::Enter));
            let escape = ui.input(|input| input.key_pressed(Key::Escape));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.add(theme::ghost_button("取消")).clicked() {
                    outcome = RenameOutcome::Cancel;
                }
                uiset_right(ui, |ui| {
                    let can_confirm = !dialog.name.trim().is_empty();
                    if ui
                        .add_enabled(can_confirm, theme::primary_button("确定"))
                        .clicked()
                    {
                        outcome = RenameOutcome::Confirm;
                    }
                });
            });
            if enter && !dialog.name.trim().is_empty() {
                outcome = RenameOutcome::Confirm;
            }
            if escape {
                outcome = RenameOutcome::Cancel;
            }
        });
    outcome
}

#[derive(Debug, Clone)]
pub struct DeleteDialog {
    pub id: String,
    pub name: String,
    pub is_folder: bool,
    pub request_count: usize,
    pub folder_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteOutcome {
    None,
    Cancel,
    Confirm,
}

pub fn show_delete(ctx: &Context, dialog: &DeleteDialog) -> DeleteOutcome {
    let mut outcome = DeleteOutcome::None;
    egui::Window::new("确认删除")
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .collapsible(false)
        .resizable(false)
        .default_width(420.0)
        .show(ctx, |ui| {
            ui.label(
                RichText::new(format!("确定删除“{}”吗？", dialog.name))
                    .size(theme::font_size(14.0))
                    .color(theme::text()),
            );
            ui.add_space(6.0);
            if dialog.is_folder {
                widgets::hint_label(
                    ui,
                    &format!(
                        "该目录包含 {} 个请求、{} 个子目录，删除后无法恢复。",
                        dialog.request_count, dialog.folder_count
                    ),
                );
            } else {
                widgets::hint_label(ui, "删除后无法恢复。");
            }
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if ui.add(theme::ghost_button("取消")).clicked() {
                    outcome = DeleteOutcome::Cancel;
                }
                uiset_right(ui, |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("删除").color(egui::Color32::WHITE).strong(),
                            )
                            .fill(theme::red())
                            .rounding(Rounding::same(6.0))
                            .min_size(Vec2::new(0.0, 30.0)),
                        )
                        .clicked()
                    {
                        outcome = DeleteOutcome::Confirm;
                    }
                });
            });
            if ui.input(|input| input.key_pressed(Key::Escape)) {
                outcome = DeleteOutcome::Cancel;
            }
        });
    outcome
}

#[derive(Debug, Clone)]
pub struct Notice {
    pub title: String,
    pub message: String,
}

pub fn show_notice(ctx: &Context, notice: &mut Notice) -> bool {
    let mut closed = false;
    egui::Window::new(&notice.title)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .collapsible(false)
        .resizable(true)
        .default_width(520.0)
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .max_height(320.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(&notice.message)
                                .size(theme::font_size(12.0))
                                .color(theme::text_dim())
                                .monospace(),
                        )
                        .selectable(true)
                        .wrap(true),
                    );
                });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.add(theme::small_ghost("复制内容")).clicked() {
                    ui.ctx().copy_text(notice.message.clone());
                }
                uiset_right(ui, |ui| {
                    if ui.add(theme::primary_button("关闭")).clicked() {
                        closed = true;
                    }
                });
            });
            if ui.input(|input| input.key_pressed(Key::Escape)) {
                closed = true;
            }
        });
    closed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsOutcome {
    None,
    Changed,
    OpenDirectory,
    SaveNow,
    ImportJson,
    ExportJson,
    CopyJson,
}

pub fn show_settings(
    ctx: &Context,
    show: &mut bool,
    settings: &mut Settings,
    data_path: &Path,
) -> SettingsOutcome {
    let mut outcome = SettingsOutcome::None;
    let mut open = *show;
    let scroll_height = (ctx.screen_rect().height() - 170.0).clamp(320.0, 640.0);
    egui::Window::new("设置")
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(600.0)
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .id_source("settings-scroll")
                .max_height(scroll_height)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.set_min_width(548.0);

                    widgets::section_label(ui, "外观");
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if theme_option(ui, ThemeMode::Light, settings.theme) {
                            settings.theme = ThemeMode::Light;
                            outcome = SettingsOutcome::Changed;
                        }
                        if theme_option(ui, ThemeMode::Dark, settings.theme) {
                            settings.theme = ThemeMode::Dark;
                            outcome = SettingsOutcome::Changed;
                        }
                    });
                    ui.add_space(2.0);
                    widgets::hint_label(ui, "切换后立即生效，并随工作区一起保存。");

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(10.0);

                    widgets::section_label(ui, "数据存储");
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        widgets::small_dot(ui, theme::green());
                        ui.label(
                            RichText::new("本地存储")
                                .size(theme::font_size(13.0))
                                .color(theme::text()),
                        );
                        widgets::hint_label(ui, "自动保存到本机 JSON 文件");
                    });
                    ui.add_space(5.0);
                    egui::Frame::none()
                        .fill(theme::input())
                        .rounding(Rounding::same(6.0))
                        .stroke(Stroke::new(1.0_f32, theme::border_soft()))
                        .inner_margin(Margin::symmetric(10.0, 8.0))
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(data_path.display().to_string())
                                        .monospace()
                                        .size(theme::font_size(11.0))
                                        .color(theme::text_dim()),
                                )
                                .selectable(true)
                                .wrap(true),
                            );
                        });
                    ui.add_space(7.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(theme::secondary_button("导入 JSON"))
                            .on_hover_text("从 PostMin 导出的 JSON 合并导入")
                            .clicked()
                        {
                            outcome = SettingsOutcome::ImportJson;
                        }
                        if ui
                            .add(theme::secondary_button("导出 JSON"))
                            .on_hover_text("导出全部集合")
                            .clicked()
                        {
                            outcome = SettingsOutcome::ExportJson;
                        }
                        if ui.add(theme::small_ghost("复制全部 JSON")).clicked() {
                            outcome = SettingsOutcome::CopyJson;
                        }
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.add(theme::small_ghost("打开配置目录")).clicked() {
                            outcome = SettingsOutcome::OpenDirectory;
                        }
                        if ui.add(theme::small_ghost("立即保存")).clicked() {
                            outcome = SettingsOutcome::SaveNow;
                        }
                        widgets::hint_label(ui, "编辑会自动保存");
                    });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(10.0);

                    widgets::section_label(ui, "网络设置");
                    ui.add_space(6.0);
                    egui::Grid::new("settings-network-grid")
                        .num_columns(2)
                        .spacing(egui::Vec2::new(18.0, 10.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new("请求超时").size(theme::font_size(13.0)));
                            ui.horizontal(|ui| {
                                let response = ui.add(
                                    egui::DragValue::new(&mut settings.timeout_secs)
                                        .clamp_range(1..=300)
                                        .speed(1.0)
                                        .suffix(" 秒"),
                                );
                                if response.changed() {
                                    outcome = SettingsOutcome::Changed;
                                }
                                widgets::hint_label(ui, "范围 1 - 300 秒");
                            });
                            ui.end_row();
                        });
                    if ui
                        .checkbox(
                            &mut settings.follow_redirects,
                            "自动跟随 3xx 重定向（最多 10 次）",
                        )
                        .changed()
                    {
                        outcome = SettingsOutcome::Changed;
                    }
                    widgets::hint_label(ui, "HTTPS 使用系统证书库；本地代理请在系统中配置。");

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(10.0);

                    widgets::section_label(ui, "快捷键");
                    ui.add_space(6.0);
                    widgets::shortcut_label(ui, "Ctrl + Enter", "发送当前请求");
                    widgets::shortcut_label(ui, "Ctrl + S", "立即保存到本地");
                    widgets::shortcut_label(ui, "Ctrl + N", "新建请求");
                    widgets::shortcut_label(ui, "Esc", "关闭设置 / 弹窗");
                    ui.add_space(2.0);
                    widgets::hint_label(ui, "当前版本快捷键为固定组合。");
                });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                widgets::hint_label(
                    ui,
                    concat!("所有设置自动保存 · PostMin v", env!("CARGO_PKG_VERSION")),
                );
                uiset_right(ui, |ui| {
                    if ui.add(theme::primary_button("完成")).clicked() {
                        *show = false;
                    }
                });
            });
            if ui.input(|input| input.key_pressed(Key::Escape)) {
                *show = false;
            }
        });
    if !open {
        *show = false;
    }
    outcome
}

fn theme_option(ui: &mut egui::Ui, mode: ThemeMode, current: ThemeMode) -> bool {
    let selected = mode == current;
    let label = mode.label();
    let fill = if selected {
        theme::tint(theme::accent(), 0.12)
    } else {
        theme::card()
    };
    let stroke = if selected {
        Stroke::new(1.0_f32, theme::accent())
    } else {
        Stroke::new(1.0_f32, theme::border_soft())
    };
    let response = egui::Frame::none()
        .fill(fill)
        .rounding(Rounding::same(8.0))
        .stroke(stroke)
        .inner_margin(Margin::symmetric(12.0, 9.0))
        .show(ui, |ui| {
            ui.set_min_width(220.0);
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::hover());
                let preview_bg = match mode {
                    ThemeMode::Dark => egui::Color32::from_rgb(17, 18, 22),
                    ThemeMode::Light => egui::Color32::from_rgb(244, 245, 248),
                };
                let preview_panel = match mode {
                    ThemeMode::Dark => egui::Color32::from_rgb(33, 35, 42),
                    ThemeMode::Light => egui::Color32::WHITE,
                };
                ui.painter()
                    .rect_filled(rect, Rounding::same(5.0), preview_bg);
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        rect.min + Vec2::new(2.0, 2.0),
                        Vec2::new(8.0, rect.height() - 4.0),
                    ),
                    Rounding::same(2.0),
                    preview_panel,
                );
                ui.painter().circle_filled(
                    rect.center() + Vec2::new(4.0, 0.0),
                    3.0,
                    theme::accent(),
                );
                ui.label(
                    RichText::new(label)
                        .size(theme::font_size(13.0))
                        .color(if selected {
                            theme::accent()
                        } else {
                            theme::text()
                        })
                        .strong(),
                );
                if selected {
                    ui.label(
                        RichText::new("已启用")
                            .size(theme::font_size(11.0))
                            .color(theme::muted()),
                    );
                }
            });
        })
        .response
        .interact(egui::Sense::click());
    response.clicked()
}

pub fn show_about(ctx: &Context, show: &mut bool, data_path: &Path) {
    let mut open = *show;
    egui::Window::new("关于 PostMin")
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(480.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("MIN")
                        .size(theme::font_size(22.0))
                        .color(theme::text())
                        .strong(),
                );
                ui.label(
                    RichText::new("POST")
                        .size(theme::font_size(22.0))
                        .color(theme::accent())
                        .strong(),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .size(theme::font_size(11.5))
                        .color(theme::muted()),
                );
            });
            ui.add_space(2.0);
            widgets::hint_label(ui, "轻量、原生的 API 调试工具 · Rust + egui");
            ui.add_space(12.0);
            widgets::section_label(ui, "能力");
            ui.add_space(4.0);
            for line in [
                "HTTP 请求：自定义 Method、URL、Header 与 Body",
                "响应预览：状态码、耗时、大小、响应头",
                "JSON / XML 自动美化与语法高亮",
                "可拖拽的目录树与右键菜单",
                "深色 / 浅色外观，设置自动保存",
                "全部 / 目录 / 单请求 JSON 导入导出",
            ] {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("·")
                            .size(theme::font_size(12.0))
                            .color(theme::accent()),
                    );
                    ui.label(
                        RichText::new(line)
                            .size(theme::font_size(12.0))
                            .color(theme::text_dim()),
                    );
                });
            }
            ui.add_space(12.0);
            widgets::section_label(ui, "数据文件");
            ui.add_space(4.0);
            ui.add(
                egui::Label::new(
                    RichText::new(data_path.display().to_string())
                        .monospace()
                        .size(theme::font_size(11.0))
                        .color(theme::muted()),
                )
                .selectable(true)
                .wrap(true),
            );
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                uiset_right(ui, |ui| {
                    if ui.add(theme::primary_button("关闭")).clicked() {
                        *show = false;
                    }
                });
            });
        });
    if !open {
        *show = false;
    }
}

fn uiset_right<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.with_layout(Layout::right_to_left(Align::Center), add)
        .inner
}
