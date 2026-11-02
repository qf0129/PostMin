use eframe::egui::{
    self, Align, Align2, Color32, Context, FontId, Frame, Id, Key, Layout, Margin, Modifiers,
    RichText, Rounding, Sense, Stroke, Vec2,
};
use postmin_core::http::{self, RequestSpec};
use postmin_core::model::{
    self, count_requests, find_item, find_item_mut, find_parent, insert_after, insert_item,
    move_item, remove_item, BodyKind, Environment, Folder, Item, ParentRef, SavedRequest,
    Workspace,
};
use postmin_core::storage::{self, Storage};
use postmin_core::{variables, ThemeMode};
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use crate::dialogs::{self, DeleteDialog, Notice, RenameDialog};
use crate::request_view::{self, RequestAction, RequestUiState};
use crate::response_view::{self, ReadyResponse, ResponseOutput, ResponseState, ResponseUiState};
use crate::sidebar::{self, SidebarAction, SidebarState};
use crate::theme;
use crate::widgets;

struct PendingRequest {
    request_id: String,
}

struct HttpOutcome {
    request_id: String,
    result: Result<http::ResponseData, String>,
}

#[derive(Clone, Copy)]
enum ToastKind {
    Info,
    Success,
    Error,
}

struct Toast {
    kind: ToastKind,
    message: String,
    until: Instant,
}

enum EmptyAction {
    NewRequest,
    Import,
}

pub struct PostMinApp {
    workspace: Workspace,
    storage: Storage,
    selected: Option<String>,
    request_ui: RequestUiState,
    response_ui: ResponseUiState,
    sidebar: SidebarState,
    responses: HashMap<String, ResponseState>,
    pending: Option<PendingRequest>,
    sender: Sender<HttpOutcome>,
    receiver: Receiver<HttpOutcome>,
    toast: Option<Toast>,
    save_error: Option<String>,
    save_blocked: bool,
    dirty_since: Option<Instant>,
    show_settings: bool,
    show_about: bool,
    applied_theme: ThemeMode,
    pixels_per_point: f32,
    rename: Option<RenameDialog>,
    delete: Option<DeleteDialog>,
    notice: Option<Notice>,
}

impl PostMinApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let storage = Storage::new(Storage::default_path());
        let mut notice = None;
        let mut toast = None;
        let mut save_blocked = false;
        let mut initial_save_error = None;

        let mut workspace = match storage.load() {
            Ok(Some(workspace)) => workspace,
            Ok(None) => {
                let workspace = model::sample_workspace();
                if let Err(error) = storage.save(&workspace) {
                    notice = Some(Notice {
                        title: "初始化失败".to_string(),
                        message: error.clone(),
                    });
                    initial_save_error = Some(error);
                } else {
                    toast = Some(Toast {
                        kind: ToastKind::Info,
                        message: "首次运行：已创建示例集合，点击发送即可体验。".to_string(),
                        until: Instant::now() + Duration::from_secs(6),
                    });
                }
                workspace
            }
            Err(error) => {
                let backup = storage.backup_corrupt();
                let workspace = Workspace::default();
                if backup.is_some() {
                    if let Err(error) = storage.save(&workspace) {
                        initial_save_error = Some(error);
                    }
                } else {
                    save_blocked = true;
                }
                let details = match &backup {
                    Some(path) => format!(
                        "{error}\n\n原文件已备份到：{}\n为避免数据丢失，已创建空工作区。",
                        path.display()
                    ),
                    None => format!(
                        "{error}\n\n原文件备份失败，为避免覆盖，本次运行将暂停自动保存。请先手动备份配置目录中的文件。"
                    ),
                };
                notice = Some(Notice {
                    title: "配置文件读取失败".to_string(),
                    message: details,
                });
                workspace
            }
        };
        workspace.ensure_environments();

        let selected = workspace
            .last_request_id
            .clone()
            .filter(|id| {
                find_item(&workspace.items, id)
                    .and_then(Item::as_request)
                    .is_some()
            })
            .or_else(|| first_request_id(&workspace.items));
        workspace.last_request_id = selected.clone();
        let active_folder =
            selected
                .as_deref()
                .and_then(|id| match find_parent(&workspace.items, id) {
                    Some(ParentRef::Folder(folder)) => Some(folder),
                    _ => None,
                });

        let (sender, receiver) = mpsc::channel();
        let applied_theme = workspace.settings.theme;
        theme::apply(&cc.egui_ctx, applied_theme);

        Self {
            workspace,
            storage,
            selected,
            request_ui: RequestUiState::default(),
            response_ui: ResponseUiState::default(),
            sidebar: SidebarState {
                active_folder,
                ..Default::default()
            },
            responses: HashMap::new(),
            pending: None,
            sender,
            receiver,
            toast,
            save_error: initial_save_error,
            save_blocked,
            dirty_since: None,
            show_settings: false,
            show_about: false,
            applied_theme,
            pixels_per_point: cc.egui_ctx.pixels_per_point(),
            rename: None,
            delete: None,
            notice,
        }
    }

    fn any_window_open(&self) -> bool {
        self.rename.is_some()
            || self.delete.is_some()
            || self.notice.is_some()
            || self.show_settings
            || self.show_about
    }

    fn toast(&mut self, kind: ToastKind, message: impl Into<String>) {
        let seconds = if matches!(kind, ToastKind::Error) {
            6
        } else {
            3
        };
        self.toast = Some(Toast {
            kind,
            message: message.into(),
            until: Instant::now() + Duration::from_secs(seconds),
        });
    }

    fn mark_dirty(&mut self) {
        if self.dirty_since.is_none() {
            self.dirty_since = Some(Instant::now());
        }
    }

    fn save_now(&mut self) -> bool {
        if self.save_blocked {
            self.save_error = Some(
                "自动保存已暂停：配置文件读取失败且无法备份，请手动处理配置目录。".to_string(),
            );
            return false;
        }
        self.workspace.last_request_id = self.selected.clone();
        match self.storage.save(&self.workspace) {
            Ok(()) => {
                self.dirty_since = None;
                self.save_error = None;
                true
            }
            Err(error) => {
                let is_new = self.save_error.as_deref() != Some(error.as_str());
                self.save_error = Some(error.clone());
                self.dirty_since = Some(Instant::now());
                if is_new {
                    self.toast(ToastKind::Error, format!("保存失败：{error}"));
                }
                false
            }
        }
    }

    fn autosave(&mut self, ctx: &Context) {
        if self.save_blocked {
            return;
        }
        if let Some(since) = self.dirty_since {
            if since.elapsed() >= Duration::from_millis(650) {
                self.save_now();
            } else {
                ctx.request_repaint_after(Duration::from_millis(180));
            }
        }
    }

    fn select_request(&mut self, id: String) {
        if find_item(&self.workspace.items, &id)
            .and_then(Item::as_request)
            .is_none()
        {
            return;
        }
        self.selected = Some(id.clone());
        self.workspace.last_request_id = Some(id.clone());
        self.sidebar.active_folder = match find_parent(&self.workspace.items, &id) {
            Some(ParentRef::Folder(folder)) => Some(folder),
            _ => None,
        };
        self.request_ui.format_error = None;
        self.mark_dirty();
    }

    fn create_request(&mut self, parent: Option<String>) {
        let parent = parent
            .filter(|id| matches!(find_item(&self.workspace.items, id), Some(Item::Folder(_))));
        let request = SavedRequest::new();
        let id = request.id.clone();
        let inserted = insert_item(
            &mut self.workspace.items,
            parent.as_deref(),
            Item::Request(request),
        );
        if !inserted && parent.is_some() {
            let request = SavedRequest::new();
            let id = request.id.clone();
            insert_item(&mut self.workspace.items, None, Item::Request(request));
            self.selected = Some(id);
            self.sidebar.active_folder = None;
        } else {
            self.selected = Some(id);
            self.sidebar.active_folder = parent;
        }
        if let Some(id) = &self.selected {
            self.workspace.last_request_id = Some(id.clone());
        }
        self.request_ui.focus_url = true;
        self.request_ui.format_error = None;
        self.mark_dirty();
        self.save_now();
    }

    fn create_folder(&mut self, parent: Option<String>) {
        let parent = parent
            .filter(|id| matches!(find_item(&self.workspace.items, id), Some(Item::Folder(_))));
        let mut folder = Folder::new("新建目录");
        let mut id = folder.id.clone();
        if !insert_item(
            &mut self.workspace.items,
            parent.as_deref(),
            Item::Folder(folder),
        ) {
            folder = Folder::new("新建目录");
            id = folder.id.clone();
            insert_item(&mut self.workspace.items, None, Item::Folder(folder));
        }
        self.sidebar.active_folder = Some(id.clone());
        self.rename = Some(RenameDialog::new(id, "新建目录".to_string(), true));
        self.mark_dirty();
        self.save_now();
    }

    fn handle_sidebar_actions(&mut self, actions: Vec<SidebarAction>) {
        for action in actions {
            match action {
                SidebarAction::NewRequest { parent } => self.create_request(parent),
                SidebarAction::NewFolder { parent } => self.create_folder(parent),
                SidebarAction::Rename { id, is_folder } => {
                    if let Some(item) = find_item(&self.workspace.items, &id) {
                        let name = item.name().to_string();
                        if is_folder == item.is_folder() {
                            self.rename = Some(RenameDialog::new(id, name, is_folder));
                        }
                    }
                }
                SidebarAction::Delete { id } => {
                    if let Some(item) = find_item(&self.workspace.items, &id) {
                        let name = item.name().to_string();
                        let is_folder = item.is_folder();
                        let (request_count, folder_count) = if let Item::Folder(folder) = item {
                            (
                                count_requests(&folder.children),
                                count_folders(&folder.children),
                            )
                        } else {
                            (1, 0)
                        };
                        self.delete = Some(DeleteDialog {
                            id,
                            name,
                            is_folder,
                            request_count,
                            folder_count,
                        });
                    }
                }
                SidebarAction::Duplicate { id } => {
                    if let Some(item) = find_item(&self.workspace.items, &id) {
                        let copy = model::duplicate_item(item);
                        let copy_id = copy.id().to_string();
                        let is_request = copy.as_request().is_some();
                        if insert_after(&mut self.workspace.items, &id, copy) {
                            if is_request {
                                self.select_request(copy_id);
                            } else {
                                self.sidebar.active_folder = Some(copy_id);
                            }
                            self.mark_dirty();
                            if self.save_now() {
                                self.toast(ToastKind::Success, "已复制");
                            }
                        }
                    }
                }
                SidebarAction::Export { id } => {
                    if let Some(item) = find_item(&self.workspace.items, &id) {
                        let name = item.name().to_string();
                        match storage::export_item(item) {
                            Ok(json) => {
                                let file_name = format!("{}.json", sanitize_file_name(&name));
                                self.save_json_file(&file_name, &json);
                            }
                            Err(error) => {
                                self.notice = Some(Notice {
                                    title: "导出失败".to_string(),
                                    message: error,
                                })
                            }
                        }
                    }
                }
                SidebarAction::Move { id, target } => {
                    if move_item(&mut self.workspace.items, &id, &target) {
                        self.sidebar.active_folder =
                            self.selected.as_deref().and_then(|selected| {
                                match find_parent(&self.workspace.items, selected) {
                                    Some(ParentRef::Folder(folder)) => Some(folder),
                                    _ => None,
                                }
                            });
                        self.mark_dirty();
                        if self.save_now() {
                            self.toast(ToastKind::Success, "已移动");
                        }
                    } else {
                        self.toast(ToastKind::Error, "无法移动到该位置");
                    }
                }
                SidebarAction::OpenSettings => {
                    self.show_settings = true;
                    self.show_about = false;
                }
                SidebarAction::OpenAbout => {
                    self.show_about = true;
                    self.show_settings = false;
                }
            }
        }
    }

    fn confirm_rename(&mut self, dialog: RenameDialog) {
        let name = dialog.name.trim().to_string();
        if name.is_empty() {
            return;
        }
        if let Some(item) = find_item_mut(&mut self.workspace.items, &dialog.id) {
            match item {
                Item::Folder(folder) => folder.name = name,
                Item::Request(request) => {
                    request.name = name;
                    request.auto_name = false;
                }
            }
            self.mark_dirty();
            self.save_now();
        }
    }

    fn confirm_delete(&mut self, dialog: DeleteDialog) {
        let mut removed_ids = Vec::new();
        if let Some(item) = find_item(&self.workspace.items, &dialog.id) {
            model::collect_request_ids(std::slice::from_ref(item), &mut removed_ids);
        }
        if self
            .pending
            .as_ref()
            .map(|pending| removed_ids.contains(&pending.request_id))
            .unwrap_or(false)
        {
            self.pending = None;
        }
        if remove_item(&mut self.workspace.items, &dialog.id).is_some() {
            for id in removed_ids {
                self.responses.remove(&id);
            }
            if let Some(selected) = &self.selected {
                if find_item(&self.workspace.items, selected).is_none() {
                    self.selected = None;
                    self.workspace.last_request_id = None;
                }
            }
            if let Some(active) = &self.sidebar.active_folder {
                if find_item(&self.workspace.items, active).is_none() {
                    self.sidebar.active_folder = None;
                }
            }
            self.mark_dirty();
            if self.save_now() {
                self.toast(ToastKind::Success, "已删除");
            }
        }
    }

    fn send_selected(&mut self, ctx: &Context) {
        if self.pending.is_some() {
            self.toast(ToastKind::Info, "已有请求正在执行，请稍候");
            return;
        }
        let Some(request_id) = self.selected.clone() else {
            self.toast(ToastKind::Info, "请先选择一个请求");
            return;
        };
        let environment = self.workspace.active_environment().cloned();
        let Some(request) =
            find_item(&self.workspace.items, &request_id).and_then(Item::as_request)
        else {
            return;
        };
        let spec = build_request_spec(request, environment.as_ref(), &self.workspace.settings);

        self.responses.insert(
            request_id.clone(),
            ResponseState::Sending {
                started: Instant::now(),
            },
        );
        self.pending = Some(PendingRequest {
            request_id: request_id.clone(),
        });
        self.response_ui.tab = response_view::ResponseTab::Body;

        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = http::execute(&spec);
            let _ = sender.send(HttpOutcome { request_id, result });
        });
        ctx.request_repaint();
    }

    fn poll_http(&mut self, ctx: &Context) {
        while let Ok(outcome) = self.receiver.try_recv() {
            let state = match outcome.result {
                Ok(data) => ResponseState::Ready(Box::new(ReadyResponse::from_data(data))),
                Err(message) => ResponseState::Error { message },
            };
            if find_item(&self.workspace.items, &outcome.request_id).is_some() {
                self.responses.insert(outcome.request_id.clone(), state);
            } else {
                self.responses.remove(&outcome.request_id);
            }
            if self
                .pending
                .as_ref()
                .map(|pending| pending.request_id == outcome.request_id)
                .unwrap_or(false)
            {
                self.pending = None;
            }
        }
        if self.pending.is_some() {
            ctx.request_repaint_after(Duration::from_millis(80));
        }
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        if self.any_window_open() {
            return;
        }
        if ctx.input_mut(|input| input.consume_key(Modifiers::CTRL, Key::Enter)) {
            self.send_selected(ctx);
        }
        if ctx.input_mut(|input| input.consume_key(Modifiers::CTRL, Key::S)) && self.save_now() {
            self.toast(ToastKind::Success, "已保存到本地");
        }
        if ctx.input_mut(|input| input.consume_key(Modifiers::CTRL, Key::N)) {
            self.create_request(self.sidebar.active_folder.clone());
        }
    }

    fn show_sidebar(&mut self, ctx: &Context) {
        let before = self.selected.clone();
        let output = egui::SidePanel::left("postmin-sidebar")
            .resizable(true)
            .default_width(288.0)
            .min_width(230.0)
            .max_width(430.0)
            .frame(
                Frame::none()
                    .fill(theme::panel())
                    .stroke(Stroke::new(1.0_f32, theme::border_soft())),
            )
            .show(ctx, |ui| {
                sidebar::show(ui, &mut self.workspace, &self.selected, &mut self.sidebar)
            })
            .inner;
        if output.changed {
            self.mark_dirty();
        }
        if output.selected != before {
            self.selected = output.selected;
            self.workspace.last_request_id = self.selected.clone();
            self.request_ui.format_error = None;
            self.mark_dirty();
        }
        self.handle_sidebar_actions(output.actions);
    }

    fn show_central(&mut self, ctx: &Context) {
        if self.sidebar.tab == crate::sidebar::SidebarTab::Environments {
            self.show_environment_central(ctx);
            return;
        }
        let valid_selection = self
            .selected
            .as_ref()
            .map(|id| {
                find_item(&self.workspace.items, id)
                    .and_then(Item::as_request)
                    .is_some()
            })
            .unwrap_or(false);
        if !valid_selection {
            self.selected = None;
            self.workspace.last_request_id = None;
        }

        let Some(request_id) = self.selected.clone() else {
            if let Some(action) = self.show_empty_central(ctx) {
                match action {
                    EmptyAction::NewRequest => {
                        self.create_request(self.sidebar.active_folder.clone())
                    }
                    EmptyAction::Import => self.import_file(),
                }
            }
            return;
        };

        let timeout_secs = self.workspace.settings.timeout_secs;
        let sending = self
            .pending
            .as_ref()
            .map(|pending| pending.request_id == request_id)
            .unwrap_or(false);
        let mut editor_output = request_view::RequestOutput::default();
        let mut response_output = ResponseOutput::default();

        {
            let items = &mut self.workspace.items;
            let Some(item) = find_item_mut(items, &request_id) else {
                return;
            };
            let Some(request) = item.as_request_mut() else {
                return;
            };
            let responses = &self.responses;
            let response_ui = &mut self.response_ui;
            let request_ui = &mut self.request_ui;

            egui::CentralPanel::default()
                .frame(Frame::none().fill(theme::bg()))
                .show(ctx, |ui| {
                    let response_height = (ui.available_height() * 0.46).clamp(210.0, 560.0);
                    egui::TopBottomPanel::bottom("postmin-response-panel")
                        .resizable(true)
                        .default_height(response_height)
                        .min_height(150.0)
                        .frame(
                            Frame::none()
                                .fill(theme::panel())
                                .stroke(Stroke::new(1.0_f32, theme::border_soft())),
                        )
                        .show_inside(ui, |ui| {
                            let idle = ResponseState::Idle;
                            let state = responses.get(&request_id).unwrap_or(&idle);
                            response_output = response_view::show(ui, state, response_ui);
                        });

                    editor_output =
                        request_view::show(ui, request, request_ui, sending, timeout_secs);
                });
        }

        if editor_output.changed {
            self.mark_dirty();
        }
        if editor_output.action == Some(RequestAction::Send) {
            self.send_selected(ctx);
        }
        if let Some(copy) = response_output.copy {
            ctx.copy_text(copy);
            self.toast(ToastKind::Success, "已复制到剪贴板");
        }
    }

    fn show_environment_central(&mut self, ctx: &Context) {
        let workspace = &mut self.workspace;
        let selected = &mut self.sidebar.selected_environment;
        let changed = egui::CentralPanel::default()
            .frame(Frame::none().fill(theme::bg()))
            .show(ctx, |ui| {
                crate::environment_view::show(ui, workspace, selected)
            })
            .inner;
        if changed {
            self.mark_dirty();
        }
    }

    fn show_empty_central(&mut self, ctx: &Context) -> Option<EmptyAction> {
        let mut action = None;
        egui::CentralPanel::default()
            .frame(Frame::none().fill(theme::bg()))
            .show(ctx, |ui| {
                let available = ui.available_size();
                ui.allocate_ui_with_layout(
                    available,
                    Layout::centered_and_justified(egui::Direction::TopDown),
                    |ui| {
                        ui.vertical_centered(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(56.0), Sense::hover());
                            let painter = ui.painter();
                            painter.rect_filled(rect, Rounding::same(15.0), theme::accent());
                            painter.text(
                                rect.center(),
                                Align2::CENTER_CENTER,
                                "M",
                                FontId::proportional(theme::font_size(30.0)),
                                Color32::WHITE,
                            );
                            ui.add_space(18.0);
                            ui.label(
                                RichText::new("调试接口，从这里开始")
                                    .size(theme::font_size(22.0))
                                    .strong()
                                    .color(theme::text()),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("发送 HTTP 请求，查看状态、耗时与美化后的响应。")
                                    .size(theme::font_size(13.0))
                                    .color(theme::text_dim()),
                            );
                            ui.add_space(18.0);
                            ui.horizontal(|ui| {
                                let spacing = 10.0;
                                let total = 260.0;
                                ui.add_space(
                                    ((ui.available_width() - total - spacing) / 2.0).max(0.0),
                                );
                                if ui
                                    .add_sized([130.0, 34.0], theme::primary_button("新建请求"))
                                    .clicked()
                                {
                                    action = Some(EmptyAction::NewRequest);
                                }
                                if ui
                                    .add_sized([130.0, 34.0], theme::secondary_button("导入 JSON"))
                                    .clicked()
                                {
                                    action = Some(EmptyAction::Import);
                                }
                            });
                            ui.add_space(22.0);
                            ui.horizontal(|ui| {
                                let block_width = 132.0;
                                ui.add_space(((ui.available_width() - block_width) / 2.0).max(0.0));
                                ui.vertical(|ui| {
                                    widgets::shortcut_label(ui, "Ctrl + Enter", "发送当前请求");
                                    widgets::shortcut_label(ui, "Ctrl + N", "新建请求");
                                });
                            });
                            ui.add_space(14.0);
                            widgets::hint_label(
                                ui,
                                "集合会自动保存在本地，可用右上角导出为 JSON。",
                            );
                        });
                    },
                );
            });
        action
    }

    fn show_status_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::bottom("postmin-statusbar")
            .exact_height(29.0)
            .frame(
                Frame::none()
                    .fill(theme::panel())
                    .inner_margin(Margin::symmetric(14.0, 4.0))
                    .stroke(Stroke::new(1.0_f32, theme::border_soft())),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (color, text) = if self.save_blocked {
                        (theme::red(), "自动保存已暂停".to_string())
                    } else if self.save_error.is_some() {
                        (theme::red(), "保存失败".to_string())
                    } else if self.dirty_since.is_some() {
                        (theme::yellow(), "编辑中 · 等待自动保存".to_string())
                    } else {
                        (theme::green(), "已保存到本地".to_string())
                    };
                    widgets::small_dot(ui, color);
                    let response = ui.label(
                        RichText::new(text)
                            .size(theme::font_size(11.0))
                            .color(theme::muted()),
                    );
                    if let Some(error) = &self.save_error {
                        response.on_hover_text(error);
                    } else {
                        response
                            .on_hover_text(format!("配置文件：{}", self.storage.path().display()));
                    }
                    ui.label(
                        RichText::new("·")
                            .size(theme::font_size(9.0))
                            .color(theme::border()),
                    );
                    widgets::hint_label(
                        ui,
                        if self.save_blocked {
                            "请先备份配置目录"
                        } else {
                            "自动保存已开启"
                        },
                    );
                    if self.pending.is_some() {
                        ui.add(egui::Spinner::new().size(10.0));
                        widgets::hint_label(ui, "请求进行中");
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        widgets::hint_label(ui, "Ctrl+S 保存");
                        widgets::hint_label(ui, "Ctrl+Enter 发送");
                        widgets::hint_label(ui, "Ctrl+N 新建");
                    });
                });
            });
    }

    fn show_dialogs(&mut self, ctx: &Context) {
        if let Some(mut notice) = self.notice.take() {
            if !dialogs::show_notice(ctx, &mut notice) {
                self.notice = Some(notice);
            }
        }

        if let Some(mut dialog) = self.rename.take() {
            match dialogs::show_rename(ctx, &mut dialog) {
                dialogs::RenameOutcome::None => self.rename = Some(dialog),
                dialogs::RenameOutcome::Cancel => {}
                dialogs::RenameOutcome::Confirm => self.confirm_rename(dialog),
            }
        }

        if let Some(dialog) = self.delete.take() {
            match dialogs::show_delete(ctx, &dialog) {
                dialogs::DeleteOutcome::None => self.delete = Some(dialog),
                dialogs::DeleteOutcome::Cancel => {}
                dialogs::DeleteOutcome::Confirm => self.confirm_delete(dialog),
            }
        }

        if self.show_settings {
            let outcome = dialogs::show_settings(
                ctx,
                &mut self.show_settings,
                &mut self.workspace.settings,
                self.storage.path(),
            );
            if self.workspace.settings.theme != self.applied_theme {
                self.applied_theme = self.workspace.settings.theme;
                theme::apply(ctx, self.applied_theme);
                self.mark_dirty();
                ctx.request_repaint();
            }
            match outcome {
                dialogs::SettingsOutcome::None => {}
                dialogs::SettingsOutcome::Changed => self.mark_dirty(),
                dialogs::SettingsOutcome::OpenDirectory => self.open_data_directory(),
                dialogs::SettingsOutcome::SaveNow => {
                    if self.save_now() {
                        self.toast(ToastKind::Success, "已保存到本地");
                    }
                }
                dialogs::SettingsOutcome::ImportJson => self.import_file(),
                dialogs::SettingsOutcome::ExportJson => self.export_all(),
                dialogs::SettingsOutcome::CopyJson => self.copy_all_json(ctx),
            }
        }

        if self.show_about {
            dialogs::show_about(ctx, &mut self.show_about, self.storage.path());
        }
    }

    fn show_toast(&mut self, ctx: &Context) {
        let now = Instant::now();
        let Some(toast) = &self.toast else {
            return;
        };
        if toast.until <= now {
            self.toast = None;
            return;
        }
        let remaining = toast.until.saturating_duration_since(now);
        let message = toast.message.clone();
        let color = match toast.kind {
            ToastKind::Info => theme::blue(),
            ToastKind::Success => theme::green(),
            ToastKind::Error => theme::red(),
        };

        let mut dismiss = false;
        egui::Area::new(Id::new("postmin-toast"))
            .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-18.0, -42.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_max_width(560.0);
                Frame::none()
                    .fill(theme::panel_2())
                    .rounding(Rounding::same(9.0))
                    .stroke(Stroke::new(1.0_f32, theme::tint(color, 0.45)))
                    .inner_margin(Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            widgets::small_dot(ui, color);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(message)
                                        .size(theme::font_size(12.0))
                                        .color(theme::text_dim()),
                                )
                                .wrap(true),
                            );
                        });
                    });
                let response = ui.interact(
                    ui.min_rect(),
                    Id::new("postmin-toast-click"),
                    Sense::click(),
                );
                if response.clicked() {
                    dismiss = true;
                }
            });
        if dismiss {
            self.toast = None;
        } else {
            ctx.request_repaint_after(remaining.min(Duration::from_millis(250)));
        }
    }

    fn import_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("导入 PostMin JSON")
            .add_filter("JSON 文件", &["json"])
            .pick_file()
        else {
            return;
        };
        if let Ok(metadata) = std::fs::metadata(&path) {
            if metadata.len() > 50 * 1024 * 1024 {
                self.notice = Some(Notice {
                    title: "导入失败".to_string(),
                    message: "文件超过 50 MB，已拒绝导入。".to_string(),
                });
                return;
            }
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) => {
                self.notice = Some(Notice {
                    title: "导入失败".to_string(),
                    message: format!("读取文件失败：{}\n{error}", path.display()),
                });
                return;
            }
        };
        match storage::import_items(&content) {
            Ok(mut items) => {
                let requests = count_requests(&items);
                let first = first_request_id(&items);
                let folders = count_folders(&items);
                self.workspace.items.append(&mut items);
                if let Some(id) = first {
                    self.select_request(id);
                }
                self.mark_dirty();
                if self.save_now() {
                    self.toast(
                        ToastKind::Success,
                        format!("已导入 {requests} 个请求、{folders} 个目录"),
                    );
                }
            }
            Err(error) => {
                self.notice = Some(Notice {
                    title: "导入失败".to_string(),
                    message: format!("文件：{}\n\n{error}", path.display()),
                });
            }
        }
    }

    fn export_all(&mut self) {
        match storage::export_items(&self.workspace.items) {
            Ok(json) => self.save_json_file("postmin-export.json", &json),
            Err(error) => {
                self.notice = Some(Notice {
                    title: "导出失败".to_string(),
                    message: error,
                });
            }
        }
    }

    fn copy_all_json(&mut self, ctx: &Context) {
        match storage::export_items(&self.workspace.items) {
            Ok(json) => {
                ctx.copy_text(json);
                self.toast(ToastKind::Success, "集合 JSON 已复制到剪贴板");
            }
            Err(error) => {
                self.notice = Some(Notice {
                    title: "导出失败".to_string(),
                    message: error,
                });
            }
        }
    }

    fn save_json_file(&mut self, default_name: &str, content: &str) {
        let mut dialog = rfd::FileDialog::new()
            .set_title("导出 JSON")
            .add_filter("JSON 文件", &["json"])
            .set_file_name(default_name);
        if let Some(directory) = dirs::download_dir()
            .or_else(dirs::document_dir)
            .or_else(dirs::home_dir)
        {
            dialog = dialog.set_directory(directory);
        }
        let Some(path) = dialog.save_file() else {
            return;
        };
        let path = if path.extension().is_none() {
            path.with_extension("json")
        } else {
            path
        };
        if same_path(&path, self.storage.path()) {
            self.notice = Some(Notice {
                title: "导出失败".to_string(),
                message: "该路径是 PostMin 的本地工作区文件，请选择其他文件名。".to_string(),
            });
            return;
        }
        match std::fs::write(&path, content) {
            Ok(()) => self.toast(ToastKind::Success, format!("已导出到 {}", path.display())),
            Err(error) => {
                self.notice = Some(Notice {
                    title: "导出失败".to_string(),
                    message: format!("写入文件失败：{}\n{error}", path.display()),
                });
            }
        }
    }

    fn open_data_directory(&mut self) {
        let directory = self.storage.directory();
        if let Err(error) = std::fs::create_dir_all(&directory) {
            self.notice = Some(Notice {
                title: "无法打开目录".to_string(),
                message: format!("创建目录失败：{}\n{error}", directory.display()),
            });
            return;
        }
        let result = open_directory_command(&directory);
        if let Err(error) = result {
            self.notice = Some(Notice {
                title: "无法打开目录".to_string(),
                message: format!("{}\n{error}", directory.display()),
            });
        }
    }
}

impl eframe::App for PostMinApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let pixels_per_point = ctx.pixels_per_point();
        if (pixels_per_point - self.pixels_per_point).abs() > 0.001 {
            self.pixels_per_point = pixels_per_point;
            theme::apply(ctx, self.applied_theme);
        }
        self.poll_http(ctx);
        self.handle_shortcuts(ctx);
        self.show_sidebar(ctx);
        self.show_central(ctx);
        self.show_status_bar(ctx);
        self.show_dialogs(ctx);
        self.show_toast(ctx);
        self.autosave(ctx);
    }
}

impl Drop for PostMinApp {
    fn drop(&mut self) {
        if self.dirty_since.is_some() && !self.save_blocked {
            self.workspace.last_request_id = self.selected.clone();
            let _ = self.storage.save(&self.workspace);
        }
    }
}

fn build_request_spec(
    request: &SavedRequest,
    environment: Option<&Environment>,
    settings: &postmin_core::Settings,
) -> RequestSpec {
    let resolve = |value: &str| variables::resolve(value, environment);

    let mut headers: Vec<(String, String)> = request
        .headers
        .iter()
        .filter(|header| header.enabled && !header.key.trim().is_empty())
        .map(|header| (resolve(header.key.trim()), resolve(&header.value)))
        .collect();
    let body = resolve(&request.body);
    let body_bytes = if request.body_kind != BodyKind::None && !body.is_empty() {
        Some(body.into_bytes())
    } else {
        None
    };
    if body_bytes.is_some()
        && !headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("content-type"))
    {
        if let Some(content_type) = request.body_kind.content_type() {
            headers.push(("Content-Type".to_string(), content_type.to_string()));
        }
    }

    RequestSpec {
        method: resolve(&request.method),
        url: resolve(&request.url),
        headers,
        body: body_bytes,
        timeout_secs: settings.timeout_secs,
        follow_redirects: settings.follow_redirects,
    }
}

fn first_request_id(items: &[Item]) -> Option<String> {
    for item in items {
        match item {
            Item::Request(request) => return Some(request.id.clone()),
            Item::Folder(folder) => {
                if let Some(id) = first_request_id(&folder.children) {
                    return Some(id);
                }
            }
        }
    }
    None
}

fn count_folders(items: &[Item]) -> usize {
    items
        .iter()
        .map(|item| match item {
            Item::Folder(folder) => 1 + count_folders(&folder.children),
            Item::Request(_) => 0,
        })
        .sum()
}

fn sanitize_file_name(name: &str) -> String {
    let mut sanitized: String = name
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect();
    sanitized = sanitized.trim().trim_matches('.').to_string();
    if sanitized.is_empty() {
        "postmin-export".to_string()
    } else if sanitized.chars().count() > 80 {
        sanitized.chars().take(80).collect()
    } else {
        sanitized
    }
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

#[cfg(target_os = "windows")]
fn open_directory_command(directory: &Path) -> std::io::Result<()> {
    std::process::Command::new("explorer")
        .arg(directory)
        .spawn()
        .map(|_| ())
}

#[cfg(target_os = "macos")]
fn open_directory_command(directory: &Path) -> std::io::Result<()> {
    std::process::Command::new("open")
        .arg(directory)
        .spawn()
        .map(|_| ())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_directory_command(directory: &Path) -> std::io::Result<()> {
    std::process::Command::new("xdg-open")
        .arg(directory)
        .spawn()
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use postmin_core::{EnvironmentVariable, HeaderRow};

    #[test]
    fn builds_spec_with_environment_variables() {
        let mut environment = Environment::new("test");
        environment.variables.push(EnvironmentVariable::new(
            "baseUrl",
            "https://example.com",
            "",
        ));
        environment
            .variables
            .push(EnvironmentVariable::new("token", "abc-123", ""));

        let request = SavedRequest {
            method: "{{method}}".to_string(),
            url: "{{baseUrl}}/users".to_string(),
            headers: vec![HeaderRow::new("Authorization", "Bearer {{token}}")],
            body: "{\"token\":\"{{token}}\"}".to_string(),
            body_kind: BodyKind::Json,
            ..SavedRequest::new()
        };
        environment
            .variables
            .push(EnvironmentVariable::new("method", "POST", ""));

        let settings = postmin_core::Settings::default();
        let spec = build_request_spec(&request, Some(&environment), &settings);

        assert_eq!(spec.method, "POST");
        assert_eq!(spec.url, "https://example.com/users");
        assert_eq!(
            spec.headers,
            vec![
                ("Authorization".to_string(), "Bearer abc-123".to_string()),
                (
                    "Content-Type".to_string(),
                    "application/json; charset=utf-8".to_string()
                )
            ]
        );
        assert_eq!(
            spec.body.as_deref(),
            Some("{\"token\":\"abc-123\"}".as_bytes())
        );
    }

    #[test]
    fn keeps_placeholders_without_environment() {
        let request = SavedRequest {
            url: "{{baseUrl}}/users".to_string(),
            ..SavedRequest::new()
        };
        let settings = postmin_core::Settings::default();
        let spec = build_request_spec(&request, None, &settings);
        assert_eq!(spec.url, "{{baseUrl}}/users");
        assert!(spec.body.is_none());
    }
}
