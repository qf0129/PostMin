use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Id, Layout, Margin, Order, Rect, Response,
    RichText, Rounding, ScrollArea, Sense, Stroke, Ui, Vec2,
};
use postmin_core::{count_requests, find_item, Environment, Item, MoveTarget, Workspace};

use crate::theme;
use crate::widgets;

const ROW_HEIGHT: f32 = 27.0;
const INDENT: f32 = 14.0;

#[derive(Debug, Clone)]
pub struct DragInfo {
    pub id: String,
    pub name: String,
    pub method: Option<String>,
    pub is_folder: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarTab {
    #[default]
    Requests,
    Environments,
}

#[derive(Debug, Default)]
pub struct SidebarState {
    pub tab: SidebarTab,
    pub active_folder: Option<String>,
    pub selected_environment: Option<String>,
    pub dragging: Option<DragInfo>,
    pub drop_target: Option<MoveTarget>,
}

#[derive(Debug, Clone)]
pub enum SidebarAction {
    NewRequest { parent: Option<String> },
    NewFolder { parent: Option<String> },
    Rename { id: String, is_folder: bool },
    Delete { id: String },
    Duplicate { id: String },
    Export { id: String },
    Move { id: String, target: MoveTarget },
    OpenSettings,
    OpenAbout,
}

#[derive(Debug, Default)]
pub struct SidebarOutput {
    pub selected: Option<String>,
    pub actions: Vec<SidebarAction>,
    pub changed: bool,
}

pub fn show(
    ui: &mut Ui,
    workspace: &mut Workspace,
    selected: &Option<String>,
    state: &mut SidebarState,
) -> SidebarOutput {
    workspace.ensure_environments();
    if let Some(active) = state.active_folder.clone() {
        if find_item(&workspace.items, &active).is_none() {
            state.active_folder = None;
        }
    }
    if let Some(dragging) = &state.dragging {
        if find_item(&workspace.items, &dragging.id).is_none() {
            state.dragging = None;
        }
    }
    if state.selected_environment.as_deref().is_some_and(|id| {
        !workspace
            .environments
            .iter()
            .any(|environment| environment.id == id)
    }) {
        state.selected_environment = None;
    }
    if state.selected_environment.is_none() {
        state.selected_environment = workspace
            .active_environment()
            .map(|environment| environment.id.clone())
            .or_else(|| {
                workspace
                    .environments
                    .first()
                    .map(|environment| environment.id.clone())
            });
    }
    state.drop_target = None;

    let total = count_requests(&workspace.items);
    let location = state
        .active_folder
        .as_deref()
        .and_then(|id| find_item(&workspace.items, id))
        .map(|item| item.name().to_string())
        .unwrap_or_else(|| "根目录".to_string());

    let mut output = SidebarOutput {
        selected: selected.clone(),
        actions: Vec::new(),
        changed: false,
    };

    egui::TopBottomPanel::bottom("postmin-sidebar-menu")
        .exact_height(42.0)
        .frame(
            egui::Frame::none()
                .fill(theme::panel_2())
                .stroke(Stroke::new(1.0_f32, theme::border_soft()))
                .inner_margin(Margin::symmetric(12.0, 6.0)),
        )
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add(theme::ghost_button("设置"))
                    .on_hover_text("外观、数据存储、网络与快捷键")
                    .clicked()
                {
                    output.actions.push(SidebarAction::OpenSettings);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .size(theme::font_size(10.0))
                            .color(theme::muted()),
                    );
                    if ui
                        .add(theme::small_ghost("关于"))
                        .on_hover_text("关于 PostMin")
                        .clicked()
                    {
                        output.actions.push(SidebarAction::OpenAbout);
                    }
                });
            });
        });

    egui::Frame::none()
        .inner_margin(Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if widgets::tab_button(ui, "所有请求", state.tab == SidebarTab::Requests).clicked()
                {
                    state.tab = SidebarTab::Requests;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let label = environment_tab_label(workspace.active_environment());
                    if widgets::tab_button(ui, label, state.tab == SidebarTab::Environments)
                        .clicked()
                    {
                        state.tab = SidebarTab::Environments;
                        state.dragging = None;
                        state.drop_target = None;
                        if state.selected_environment.is_none() {
                            state.selected_environment = workspace
                                .active_environment()
                                .map(|environment| environment.id.clone())
                                .or_else(|| {
                                    workspace
                                        .environments
                                        .first()
                                        .map(|environment| environment.id.clone())
                                });
                        }
                    }
                });
            });
        });
    ui.add(egui::Separator::default().spacing(0.0));

    if state.tab == SidebarTab::Environments {
        output.changed |= show_environments(ui, workspace, state);
    } else {
        let mut root_header_rect = None;
        egui::Frame::none()
            .inner_margin(Margin::symmetric(12.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let title = widgets::section_label(ui, "集合");
                    let badge = widgets::count_badge(ui, total);
                    root_header_rect = Some(title.rect.union(badge.rect));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("+ 新建请求")
                                        .size(theme::font_size(11.5))
                                        .color(theme::accent()),
                                )
                                .fill(theme::tint(theme::accent(), 0.12))
                                .stroke(Stroke::new(1.0_f32, theme::tint(theme::accent(), 0.25)))
                                .rounding(Rounding::same(5.0))
                                .min_size(Vec2::new(0.0, 22.0)),
                            )
                            .on_hover_text(format!("新建请求（{location}）"))
                            .clicked()
                        {
                            output.actions.push(SidebarAction::NewRequest {
                                parent: state.active_folder.clone(),
                            });
                        }
                    });
                });
            });
        if let Some(rect) = root_header_rect {
            ui.interact(rect, Id::new("postmin-root-header"), Sense::click())
                .context_menu(|ui| root_menu(ui, &mut output.actions));
        }

        ui.add(egui::Separator::default().spacing(0.0));

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Frame::none()
                    .inner_margin(Margin::symmetric(6.0, 8.0))
                    .show(ui, |ui| {
                        if workspace.items.is_empty() {
                            draw_root_zone(ui, state, &mut output.actions, true);
                        } else {
                            render_items(
                                ui,
                                0,
                                None,
                                &mut workspace.items,
                                &mut output.selected,
                                state,
                                &mut output.actions,
                                &mut output.changed,
                            );
                            draw_root_zone(ui, state, &mut output.actions, false);
                        }
                    });
            });
    }

    if let Some(drag) = state.dragging.as_ref() {
        if let Some(pointer) = ui.ctx().pointer_interact_pos() {
            draw_drag_ghost(ui.ctx(), pointer, drag);
        }
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    }

    let released = ui.input(|input| input.pointer.any_released());
    if released {
        if let (Some(drag), Some(target)) = (state.dragging.take(), state.drop_target.take()) {
            output.actions.push(SidebarAction::Move {
                id: drag.id,
                target,
            });
        }
    } else if state.dragging.is_some() && !ui.input(|input| input.pointer.any_down()) {
        state.dragging = None;
        state.drop_target = None;
    }

    output
}

fn show_environments(ui: &mut Ui, workspace: &mut Workspace, state: &mut SidebarState) -> bool {
    let mut changed = false;

    egui::Frame::none()
        .inner_margin(Margin::symmetric(12.0, 10.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                widgets::section_label(ui, "环境");
                widgets::count_badge(ui, workspace.environments.len());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(theme::accent_small("+ 新建环境"))
                        .on_hover_text("新建环境")
                        .clicked()
                    {
                        let environment =
                            Environment::new(next_environment_name(&workspace.environments));
                        state.selected_environment = Some(environment.id.clone());
                        workspace.environments.push(environment);
                        changed = true;
                    }
                });
            });
        });

    ui.add(egui::Separator::default().spacing(0.0));

    ScrollArea::vertical()
        .id_source("environment-list-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::none()
                .inner_margin(Margin::symmetric(6.0, 8.0))
                .show(ui, |ui| {
                    if workspace.environments.is_empty() {
                        ui.label(
                            RichText::new("还没有环境")
                                .size(theme::font_size(12.0))
                                .color(theme::muted()),
                        );
                        return;
                    }

                    let active_id = workspace
                        .active_environment()
                        .map(|environment| environment.id.clone());
                    let mut activate = None;

                    for environment in &workspace.environments {
                        let selected =
                            state.selected_environment.as_deref() == Some(environment.id.as_str());
                        let active = active_id.as_deref() == Some(environment.id.as_str());
                        let (response, enable) =
                            draw_environment_row(ui, environment, selected, active);
                        if enable {
                            activate = Some(environment.id.clone());
                        }
                        if response.clicked() {
                            state.selected_environment = Some(environment.id.clone());
                        }
                        if response.contains_pointer() {
                            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        }
                        response.on_hover_text("点击查看环境变量");
                    }

                    if let Some(id) = activate {
                        state.selected_environment = Some(id.clone());
                        workspace.active_environment = Some(id);
                        changed = true;
                    }
                });
        });

    changed
}

fn draw_environment_row(
    ui: &mut Ui,
    environment: &Environment,
    selected: bool,
    active: bool,
) -> (Response, bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::hover());
    let button_width = 64.0;
    let select_rect = if active {
        rect
    } else {
        Rect::from_min_max(
            rect.min,
            egui::pos2(rect.right() - button_width, rect.bottom()),
        )
    };
    let row = ui.interact(
        select_rect,
        ui.id().with(("postmin-environment", &environment.id)),
        Sense::click(),
    );
    let pointer_inside = ui
        .ctx()
        .pointer_interact_pos()
        .map(|pointer| rect.contains(pointer))
        .unwrap_or(false);
    let mut enable = false;

    if ui.is_rect_visible(rect) {
        let fill = if selected {
            theme::tint(theme::accent(), 0.12)
        } else if pointer_inside {
            theme::hover()
        } else {
            Color32::TRANSPARENT
        };
        let painter = ui.painter();
        painter.rect_filled(rect, Rounding::same(6.0), fill);

        let mut text_left = rect.left() + 10.0;
        if active {
            painter.circle_filled(
                egui::pos2(rect.left() + 10.0, rect.center().y),
                3.0,
                theme::accent(),
            );
            text_left = rect.left() + 20.0;
        }
        let text_clip = Rect::from_min_max(
            rect.left_top(),
            egui::pos2(rect.right() - button_width, rect.bottom()),
        );
        painter.with_clip_rect(text_clip).text(
            egui::pos2(text_left, rect.center().y),
            Align2::LEFT_CENTER,
            environment.display_name(),
            FontId::proportional(theme::font_size(13.0)),
            if active {
                theme::text()
            } else {
                theme::text_dim()
            },
        );
        if active {
            painter.text(
                egui::pos2(rect.right() - 10.0, rect.center().y),
                Align2::RIGHT_CENTER,
                "当前",
                FontId::proportional(theme::font_size(10.0)),
                theme::accent(),
            );
        }
    }

    if pointer_inside && !active && ui.is_rect_visible(rect) {
        let button_rect = Rect::from_center_size(
            egui::pos2(rect.right() - 34.0, rect.center().y),
            Vec2::new(52.0, 22.0),
        );
        let response = ui.put(button_rect, theme::accent_small("启用"));
        enable = response.clicked();
    }

    (row, enable)
}

fn environment_tab_label(environment: Option<&Environment>) -> String {
    let Some(environment) = environment else {
        return "环境（未启用）".to_string();
    };
    let name = environment.display_name();
    let short: String = name.chars().take(10).collect();
    if name.chars().count() > 10 {
        format!("环境（{short}…）")
    } else {
        format!("环境（{short}）")
    }
}

fn next_environment_name(environments: &[Environment]) -> String {
    let mut index = environments.len() + 1;
    loop {
        let name = format!("环境 {index}");
        if !environments
            .iter()
            .any(|environment| environment.name == name)
        {
            return name;
        }
        index += 1;
    }
}

#[allow(clippy::too_many_arguments)]
fn render_items(
    ui: &mut Ui,
    depth: usize,
    parent: Option<&str>,
    items: &mut [Item],
    selected: &mut Option<String>,
    state: &mut SidebarState,
    actions: &mut Vec<SidebarAction>,
    changed: &mut bool,
) {
    for item in items.iter_mut() {
        match item {
            Item::Request(request) => {
                let id = request.id.clone();
                let name = request.name.clone();
                let url = request.url.clone();
                let method = request.method.clone();
                let is_selected = selected.as_deref() == Some(id.as_str());
                let row = draw_row(ui, depth, &id, is_selected, false);

                if row.drag_started() && state.dragging.is_none() {
                    state.dragging = Some(DragInfo {
                        id: id.clone(),
                        name: name.clone(),
                        method: Some(method.clone()),
                        is_folder: false,
                    });
                    state.drop_target = None;
                }
                if row.clicked() && state.dragging.is_none() {
                    *selected = Some(id.clone());
                    state.active_folder = parent.map(str::to_string);
                }
                if row.hovered() && state.dragging.is_none() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }

                draw_request_content(ui, row.rect, depth, &method, &name);
                handle_drop_hover(ui, row.rect, &id, false, state);

                let row = if url.is_empty() {
                    row.on_hover_text(name)
                } else {
                    row.on_hover_text(format!("{} {url}", method.to_ascii_uppercase()))
                };
                row.context_menu(|ui| {
                    if ui.button("打开").clicked() {
                        *selected = Some(id.clone());
                        state.active_folder = parent.map(str::to_string);
                        ui.close_menu();
                    }
                    if ui.button("复制请求").clicked() {
                        actions.push(SidebarAction::Duplicate { id: id.clone() });
                        ui.close_menu();
                    }
                    if ui.button("重命名").clicked() {
                        actions.push(SidebarAction::Rename {
                            id: id.clone(),
                            is_folder: false,
                        });
                        ui.close_menu();
                    }
                    if ui.button("导出为 JSON").clicked() {
                        actions.push(SidebarAction::Export { id: id.clone() });
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .button(RichText::new("删除请求").color(theme::red()))
                        .clicked()
                    {
                        actions.push(SidebarAction::Delete { id: id.clone() });
                        ui.close_menu();
                    }
                });
            }
            Item::Folder(folder) => {
                let id = folder.id.clone();
                let name = folder.name.clone();
                let active = state.active_folder.as_deref() == Some(id.as_str());
                let count = count_requests(&folder.children);
                let row = draw_row(ui, depth, &id, false, active);

                if row.drag_started() && state.dragging.is_none() {
                    state.dragging = Some(DragInfo {
                        id: id.clone(),
                        name: name.clone(),
                        method: None,
                        is_folder: true,
                    });
                    state.drop_target = None;
                }
                if row.clicked() && state.dragging.is_none() {
                    folder.expanded = !folder.expanded;
                    *changed = true;
                    state.active_folder = Some(id.clone());
                }
                if row.hovered() && state.dragging.is_none() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }

                draw_folder_content(ui, row.rect, depth, &name, folder.expanded, count);
                handle_drop_hover(ui, row.rect, &id, true, state);

                row.context_menu(|ui| {
                    if ui.button("新建请求").clicked() {
                        actions.push(SidebarAction::NewRequest {
                            parent: Some(id.clone()),
                        });
                        state.active_folder = Some(id.clone());
                        ui.close_menu();
                    }
                    if ui.button("新建子目录").clicked() {
                        actions.push(SidebarAction::NewFolder {
                            parent: Some(id.clone()),
                        });
                        state.active_folder = Some(id.clone());
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("重命名目录").clicked() {
                        actions.push(SidebarAction::Rename {
                            id: id.clone(),
                            is_folder: true,
                        });
                        ui.close_menu();
                    }
                    if ui.button("复制目录").clicked() {
                        actions.push(SidebarAction::Duplicate { id: id.clone() });
                        ui.close_menu();
                    }
                    if ui.button("导出目录为 JSON").clicked() {
                        actions.push(SidebarAction::Export { id: id.clone() });
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .button(RichText::new("删除目录").color(theme::red()))
                        .clicked()
                    {
                        actions.push(SidebarAction::Delete { id: id.clone() });
                        ui.close_menu();
                    }
                });

                if folder.expanded {
                    if folder.children.is_empty() {
                        ui.horizontal(|ui| {
                            ui.add_space(12.0 + (depth + 1) as f32 * INDENT);
                            ui.label(
                                RichText::new("空目录")
                                    .size(theme::font_size(11.0))
                                    .color(theme::muted()),
                            );
                        });
                        ui.add_space(2.0);
                    } else {
                        render_items(
                            ui,
                            depth + 1,
                            Some(&folder.id),
                            &mut folder.children,
                            selected,
                            state,
                            actions,
                            changed,
                        );
                    }
                }
            }
        }
    }
}

fn root_menu(ui: &mut Ui, actions: &mut Vec<SidebarAction>) {
    if ui.button("新建请求").clicked() {
        actions.push(SidebarAction::NewRequest { parent: None });
        ui.close_menu();
    }
    if ui.button("新建目录").clicked() {
        actions.push(SidebarAction::NewFolder { parent: None });
        ui.close_menu();
    }
}

fn draw_root_zone(
    ui: &mut Ui,
    state: &mut SidebarState,
    actions: &mut Vec<SidebarAction>,
    empty: bool,
) {
    let available = ui.available_height();
    let height = if empty {
        available.clamp(220.0, 640.0)
    } else {
        available.clamp(100.0, 240.0)
    };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let response = ui.interact(rect, Id::new("postmin-root-zone"), Sense::click());

    let dragging = state.dragging.is_some();
    let pointer_over = dragging
        && ui
            .ctx()
            .pointer_interact_pos()
            .map(|pointer| rect.contains(pointer))
            .unwrap_or(false);
    if pointer_over {
        state.drop_target = Some(MoveTarget::RootEnd);
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if pointer_over {
            painter.rect_filled(
                rect.shrink(2.0),
                Rounding::same(7.0),
                theme::tint(theme::accent(), 0.10),
            );
            painter.rect_stroke(
                rect.shrink(2.0),
                Rounding::same(7.0),
                Stroke::new(1.0_f32, theme::accent()),
            );
        }
        if empty {
            painter.text(
                egui::pos2(rect.center().x, rect.center().y - 12.0),
                Align2::CENTER_CENTER,
                "还没有接口",
                FontId::proportional(theme::font_size(14.0)),
                theme::text_dim(),
            );
            painter.text(
                egui::pos2(rect.center().x, rect.center().y + 12.0),
                Align2::CENTER_CENTER,
                if pointer_over {
                    "释放移动到根目录"
                } else {
                    "右键空白处新建目录或请求，或点击上方“+ 新建请求”"
                },
                FontId::proportional(theme::font_size(11.5)),
                if pointer_over {
                    theme::accent()
                } else {
                    theme::muted()
                },
            );
        } else {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                if pointer_over {
                    "释放移动到根目录"
                } else {
                    "右键空白处新建目录 / 请求"
                },
                FontId::proportional(theme::font_size(11.0)),
                if pointer_over {
                    theme::accent()
                } else {
                    theme::muted()
                },
            );
        }
    }

    response.context_menu(|ui| root_menu(ui, actions));
}

fn handle_drop_hover(ui: &Ui, rect: Rect, id: &str, is_folder: bool, state: &mut SidebarState) {
    let Some(dragging) = state.dragging.as_ref() else {
        return;
    };
    if dragging.id == id {
        return;
    }
    let Some(pointer) = ui.ctx().pointer_interact_pos() else {
        return;
    };
    if !rect.expand2(Vec2::new(0.0, 3.0)).contains(pointer) {
        return;
    }
    let position = ((pointer.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
    let target = if is_folder && (0.30..=0.70).contains(&position) {
        MoveTarget::Into(id.to_string())
    } else if position < 0.5 {
        MoveTarget::Before(id.to_string())
    } else {
        MoveTarget::After(id.to_string())
    };
    draw_drop_indicator(ui, rect, &target);
    state.drop_target = Some(target);
}

fn draw_drop_indicator(ui: &Ui, rect: Rect, target: &MoveTarget) {
    let painter = ui.painter();
    match target {
        MoveTarget::Before(_) | MoveTarget::After(_) => {
            let y = match target {
                MoveTarget::Before(_) => rect.top() + 1.0,
                MoveTarget::After(_) => rect.bottom() - 1.0,
                _ => rect.center().y,
            };
            painter.line_segment(
                [
                    egui::pos2(rect.left() + 7.0, y),
                    egui::pos2(rect.right() - 7.0, y),
                ],
                Stroke::new(2.0_f32, theme::accent()),
            );
            painter.circle_filled(egui::pos2(rect.left() + 7.0, y), 3.0, theme::accent());
        }
        MoveTarget::Into(_) => {
            painter.rect_filled(
                rect.shrink(1.0),
                Rounding::same(5.0),
                theme::tint(theme::accent(), 0.13),
            );
            painter.rect_stroke(
                rect.shrink(1.0),
                Rounding::same(5.0),
                Stroke::new(1.0_f32, theme::accent()),
            );
        }
        MoveTarget::RootEnd => {}
    }
}

fn draw_row(ui: &mut Ui, depth: usize, id: &str, selected: bool, active: bool) -> Response {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, ROW_HEIGHT), Sense::hover());
    let response = ui.interact(rect, Id::new(("postmin-row", id)), Sense::click_and_drag());
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let painter = ui.painter();
    if selected {
        painter.rect_filled(
            rect,
            Rounding::same(5.0),
            theme::tint(theme::accent(), 0.13),
        );
        painter.rect_filled(
            Rect::from_min_size(rect.left_top(), Vec2::new(2.0, rect.height() - 8.0))
                .translate(Vec2::new(0.0, 4.0)),
            Rounding::same(2.0),
            theme::accent(),
        );
    } else if response.hovered() {
        painter.rect_filled(rect, Rounding::same(5.0), theme::card());
    } else if active {
        painter.rect_filled(rect, Rounding::same(5.0), theme::panel_2());
    }

    let _ = depth;
    response
}

fn draw_request_content(ui: &Ui, rect: Rect, depth: usize, method: &str, name: &str) {
    let painter = ui
        .painter()
        .with_clip_rect(rect.shrink2(Vec2::new(2.0, 0.0)));
    let indent = 8.0 + depth as f32 * INDENT;
    let badge = Rect::from_min_size(
        egui::pos2(rect.left() + indent, rect.center().y - 8.5),
        Vec2::new(widgets::METHOD_BADGE_WIDTH, 17.0),
    );
    widgets::paint_method_badge(&painter, badge, method);

    let text_x = badge.right() + 8.0;
    let color = if name.is_empty() {
        theme::muted()
    } else {
        theme::text()
    };
    let text = if name.is_empty() {
        "未命名请求"
    } else {
        name
    };
    painter.text(
        egui::pos2(text_x, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(theme::font_size(13.0)),
        color,
    );
}

fn draw_folder_content(
    ui: &Ui,
    rect: Rect,
    depth: usize,
    name: &str,
    expanded: bool,
    count: usize,
) {
    let painter = ui
        .painter()
        .with_clip_rect(rect.shrink2(Vec2::new(2.0, 0.0)));
    let indent = 8.0 + depth as f32 * INDENT;
    painter.text(
        egui::pos2(rect.left() + indent, rect.center().y),
        Align2::LEFT_CENTER,
        if expanded { "v" } else { ">" },
        FontId::proportional(theme::font_size(10.5)),
        theme::muted(),
    );
    let name_clip = Rect::from_min_max(
        egui::pos2(rect.left() + indent + 16.0, rect.top()),
        egui::pos2(
            rect.right() - if count == 0 { 8.0 } else { 26.0 },
            rect.bottom(),
        ),
    );
    ui.painter().with_clip_rect(name_clip).text(
        egui::pos2(rect.left() + indent + 16.0, rect.center().y),
        Align2::LEFT_CENTER,
        if name.is_empty() {
            "未命名目录"
        } else {
            name
        },
        FontId::proportional(theme::font_size(13.0)),
        theme::text_dim(),
    );
    painter.text(
        egui::pos2(rect.right() - 8.0, rect.center().y),
        Align2::RIGHT_CENTER,
        if count == 0 {
            String::new()
        } else {
            count.to_string()
        },
        FontId::monospace(theme::font_size(10.0)),
        theme::muted(),
    );
}

fn draw_drag_ghost(ctx: &egui::Context, pointer: egui::Pos2, drag: &DragInfo) {
    egui::Area::new(Id::new("postmin-drag-ghost"))
        .order(Order::Tooltip)
        .fixed_pos(pointer + Vec2::new(16.0, 12.0))
        .show(ctx, |ui| {
            ui.set_max_width(320.0);
            egui::Frame::none()
                .fill(theme::panel_2())
                .rounding(Rounding::same(7.0))
                .stroke(Stroke::new(1.0_f32, theme::accent()))
                .inner_margin(Margin::symmetric(10.0, 6.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if let Some(method) = &drag.method {
                            ui.label(
                                RichText::new(method.to_ascii_uppercase())
                                    .monospace()
                                    .size(theme::font_size(10.0))
                                    .color(widgets::method_color(method)),
                            );
                        } else {
                            ui.label(
                                RichText::new("目录")
                                    .size(theme::font_size(10.0))
                                    .color(theme::muted()),
                            );
                        }
                        ui.label(
                            RichText::new(if drag.name.is_empty() {
                                if drag.is_folder {
                                    "未命名目录"
                                } else {
                                    "未命名请求"
                                }
                            } else {
                                &drag.name
                            })
                            .size(theme::font_size(12.0))
                            .color(theme::text()),
                        );
                    });
                });
        });
}
