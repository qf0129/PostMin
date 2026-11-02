use eframe::egui::{
    self, Align, Align2, FontId, Layout, Margin, RichText, ScrollArea, Sense, TextEdit, Ui, Vec2,
};
use postmin_core::{EnvironmentVariable, Workspace};

use crate::theme;
use crate::widgets;

pub fn show(ui: &mut Ui, workspace: &mut Workspace, selected: &mut Option<String>) -> bool {
    workspace.ensure_environments();
    if selected.is_none() {
        *selected = workspace
            .active_environment()
            .map(|environment| environment.id.clone())
            .or_else(|| {
                workspace
                    .environments
                    .first()
                    .map(|environment| environment.id.clone())
            });
    }

    let Some(index) = selected.as_deref().and_then(|id| {
        workspace
            .environments
            .iter()
            .position(|environment| environment.id == id)
    }) else {
        widgets::empty_state(
            ui,
            "还没有环境",
            "在左侧环境标签中新建环境后，可在这里编辑变量。",
        );
        return false;
    };

    let environment_id = workspace.environments[index].id.clone();
    let is_active = workspace.active_environment.as_deref() == Some(environment_id.as_str());
    let mut changed = false;

    egui::Frame::none()
        .inner_margin(Margin::symmetric(18.0, 14.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("环境变量")
                        .size(theme::font_size(19.0))
                        .strong()
                        .color(theme::text()),
                );
                widgets::count_badge(ui, workspace.environments[index].variables.len());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if is_active {
                        egui::Frame::none()
                            .fill(theme::tint(theme::accent(), 0.12))
                            .rounding(egui::Rounding::same(5.0))
                            .inner_margin(Margin::symmetric(8.0, 3.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("✓ 当前环境")
                                        .size(theme::font_size(11.0))
                                        .strong()
                                        .color(theme::accent()),
                                );
                            });
                    } else if ui
                        .add(theme::accent_small("设为当前环境"))
                        .on_hover_text("之后发送请求时使用该环境的变量")
                        .clicked()
                    {
                        workspace.active_environment = Some(environment_id.clone());
                        changed = true;
                    }
                });
            });

            ui.add_space(2.0);
            widgets::hint_label(
                ui,
                "在请求的 URL、Header、Body 中用 {{变量名}} 引用当前环境变量，例如 {{baseUrl}}/users。",
            );
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("环境名称")
                        .size(theme::font_size(11.0))
                        .color(theme::muted()),
                );
                let name = ui.add_sized(
                    [320.0, 30.0],
                    TextEdit::singleline(&mut workspace.environments[index].name)
                        .hint_text("环境名称")
                        .font(FontId::proportional(theme::font_size(13.0)))
                        .margin(Vec2::new(8.0, 5.0)),
                );
                if name.changed() {
                    changed = true;
                }
            });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                widgets::section_label(ui, "变量");
                widgets::count_badge(ui, workspace.environments[index].variables.len());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(theme::accent_small("+ 添加变量"))
                        .on_hover_text("新增一个环境变量")
                        .clicked()
                    {
                        workspace.environments[index]
                            .variables
                            .push(EnvironmentVariable::default());
                        changed = true;
                    }
                });
            });

            ui.add_space(6.0);

            let available = ui.available_width();
            let name_width = 200.0;
            let value_width = (available * 0.28).clamp(150.0, 300.0);
            let remove_width = 32.0;
            let spacing = ui.spacing().item_spacing.x * 3.0;
            let description_width =
                (available - name_width - value_width - remove_width - spacing).max(120.0);

            ui.horizontal(|ui| {
                header_label(ui, "名称", name_width);
                header_label(ui, "值", value_width);
                header_label(ui, "描述", description_width);
            });
            ui.add_space(4.0);

            let mut remove_index = None;
            ScrollArea::vertical()
                .id_source("environment-variables-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if workspace.environments[index].variables.is_empty() {
                        egui::Frame::none()
                            .fill(theme::card())
                            .rounding(egui::Rounding::same(7.0))
                            .inner_margin(Margin::symmetric(12.0, 10.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("还没有变量")
                                        .size(theme::font_size(12.0))
                                        .color(theme::text_dim()),
                                );
                                ui.add_space(2.0);
                                widgets::hint_label(
                                    ui,
                                    "点击右上角“+ 添加变量”，然后在请求中用 {{名称}} 引用。",
                                );
                            });
                        ui.add_space(8.0);
                    }

                    for (variable_index, variable) in workspace.environments[index]
                        .variables
                        .iter_mut()
                        .enumerate()
                    {
                        ui.horizontal(|ui| {
                            let name = ui.add_sized(
                                [name_width, 28.0],
                                TextEdit::singleline(&mut variable.name)
                                    .hint_text("变量名")
                                    .font(FontId::monospace(theme::font_size(12.0)))
                                    .margin(Vec2::new(6.0, 4.0)),
                            );
                            changed |= name.changed();

                            let value = ui.add_sized(
                                [value_width, 28.0],
                                TextEdit::singleline(&mut variable.value)
                                    .hint_text("值")
                                    .font(FontId::monospace(theme::font_size(12.0)))
                                    .margin(Vec2::new(6.0, 4.0)),
                            );
                            changed |= value.changed();

                            let description = ui.add_sized(
                                [description_width, 28.0],
                                TextEdit::singleline(&mut variable.description)
                                    .hint_text("描述（可选）")
                                    .font(FontId::proportional(theme::font_size(12.0)))
                                    .margin(Vec2::new(6.0, 4.0)),
                            );
                            changed |= description.changed();

                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("×")
                                            .size(theme::font_size(14.0))
                                            .color(theme::muted()),
                                    )
                                    .frame(false)
                                    .min_size(Vec2::new(remove_width, 28.0)),
                                )
                                .on_hover_text("删除该变量")
                                .clicked()
                            {
                                remove_index = Some(variable_index);
                            }
                        });
                    }
                });

            if let Some(variable_index) = remove_index {
                workspace.environments[index]
                    .variables
                    .remove(variable_index);
                changed = true;
            }
        });

    changed
}

fn header_label(ui: &mut Ui, text: &str, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 20.0), Sense::hover());
    ui.painter().text(
        rect.left_center(),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(theme::font_size(10.5)),
        theme::muted(),
    );
}
