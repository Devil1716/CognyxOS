//! Native CognyxOS desktop. Renders the Phase 8 shell surface and forwards
//! natural-language commands through `CognyxShell` → Agent Kernel.

#![allow(float_literal_f32_fallback)]

use cognyx_shell::{
    AgentKernelAdapter, AgentNode, ApprovalDecision, CognyxShell, NotificationKind, TaskView,
};
use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Frame, Layout, Margin, Pos2, Rect, RichText, Sense,
    Stroke, Ui, Vec2,
};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

const BG: Color32 = Color32::from_rgb(8, 12, 22);
const PANEL: Color32 = Color32::from_rgb(16, 24, 40);
const PANEL_EDGE: Color32 = Color32::from_rgb(36, 52, 78);
const ACCENT: Color32 = Color32::from_rgb(62, 224, 200);
const ACCENT_DIM: Color32 = Color32::from_rgb(36, 120, 118);
const TEXT: Color32 = Color32::from_rgb(226, 234, 244);
const MUTED: Color32 = Color32::from_rgb(148, 163, 184);
const DANGER: Color32 = Color32::from_rgb(251, 113, 133);
const WARN: Color32 = Color32::from_rgb(251, 191, 36);

enum KernelMsg {
    Submitted(TaskView),
    Updated(TaskView),
    Tree(String, AgentNode),
    Failed(String),
}

pub fn run(
    runtime: tokio::runtime::Runtime,
    shell: Arc<CognyxShell<AgentKernelAdapter>>,
    host_id: String,
    workspace_id: String,
) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1480.0, 920.0])
            .with_min_inner_size([1080.0, 700.0])
            .with_title("CognyxOS"),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "CognyxOS",
        options,
        Box::new(move |cc| {
            Ok(Box::new(CognyxDesktop::new(
                cc,
                runtime,
                shell,
                host_id,
                workspace_id,
            )))
        }),
    )
}

struct CognyxDesktop {
    runtime: tokio::runtime::Runtime,
    shell: Arc<CognyxShell<AgentKernelAdapter>>,
    host_id: String,
    workspace_id: String,
    command: String,
    search: String,
    show_command: bool,
    show_launcher: bool,
    show_agents: bool,
    busy: bool,
    status: String,
    tasks: Vec<TaskView>,
    trees: Vec<(String, AgentNode)>,
    selected_task: Option<String>,
    last_poll: Instant,
    tx: Sender<KernelMsg>,
    rx: Receiver<KernelMsg>,
}

impl CognyxDesktop {
    fn new(
        cc: &eframe::CreationContext<'_>,
        runtime: tokio::runtime::Runtime,
        shell: Arc<CognyxShell<AgentKernelAdapter>>,
        host_id: String,
        workspace_id: String,
    ) -> Self {
        apply_theme(&cc.egui_ctx);
        let (tx, rx) = mpsc::channel();
        Self {
            runtime,
            shell,
            host_id,
            workspace_id,
            command: String::new(),
            search: String::new(),
            show_command: true,
            show_launcher: true,
            show_agents: true,
            busy: false,
            status: "Desktop ready. Type a command to talk to the Agent Kernel.".into(),
            tasks: Vec::new(),
            trees: Vec::new(),
            selected_task: None,
            last_poll: Instant::now(),
            tx,
            rx,
        }
    }

    fn submit(&mut self) {
        let prompt = self.command.trim().to_string();
        if prompt.is_empty() || self.busy {
            return;
        }
        self.busy = true;
        self.status = format!("Submitting: {prompt}");
        let title: String = prompt.chars().take(42).collect();
        self.shell.open_window("command-bar", &self.host_id, &title);
        let shell = Arc::clone(&self.shell);
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            match shell.submit_intent(&prompt).await {
                Ok(task) => {
                    let _ = tx.send(KernelMsg::Submitted(task.clone()));
                    if let Ok(tree) = shell.agent_tree(&task.task_id).await {
                        let _ = tx.send(KernelMsg::Tree(task.task_id.clone(), tree));
                    }
                }
                Err(error) => {
                    let _ = tx.send(KernelMsg::Failed(error.to_string()));
                }
            }
        });
        self.command.clear();
        self.show_command = true;
        self.show_agents = true;
    }

    fn recover(&self, task_id: String) {
        let shell = Arc::clone(&self.shell);
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            match shell.recover_task(&task_id).await {
                Ok(task) => {
                    let _ = tx.send(KernelMsg::Updated(task));
                }
                Err(error) => {
                    let _ = tx.send(KernelMsg::Failed(error.to_string()));
                }
            }
        });
    }

    fn drain_kernel(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                KernelMsg::Submitted(task) | KernelMsg::Updated(task) => {
                    self.busy = false;
                    upsert_task(&mut self.tasks, task.clone());
                    self.selected_task = Some(task.task_id.clone());
                    self.status = format!("{} · {}", task.status, task.prompt);
                    if task.status == "failed" {
                        self.shell.notify(
                            NotificationKind::AgentFailed,
                            "Agent failed",
                            task.error.as_deref().unwrap_or("task failed"),
                            &task.task_id,
                        );
                    } else if task.status == "completed" {
                        self.shell.notify(
                            NotificationKind::TaskCompleted,
                            "Task completed",
                            &task.prompt,
                            &task.task_id,
                        );
                    }
                }
                KernelMsg::Tree(task_id, tree) => {
                    self.trees.retain(|(id, _)| id != &task_id);
                    self.trees.push((task_id, tree));
                }
                KernelMsg::Failed(error) => {
                    self.busy = false;
                    self.status = error;
                    self.shell.notify(
                        NotificationKind::SystemWarning,
                        "Kernel error",
                        &self.status,
                        "kernel-error",
                    );
                }
            }
        }
    }

    fn poll_tasks(&mut self) {
        if self.last_poll.elapsed() < Duration::from_millis(400) {
            return;
        }
        self.last_poll = Instant::now();
        for task in self.tasks.clone() {
            if matches!(task.status.as_str(), "completed" | "failed" | "cancelled") {
                continue;
            }
            let shell = Arc::clone(&self.shell);
            let tx = self.tx.clone();
            let task_id = task.task_id.clone();
            self.runtime.spawn(async move {
                if let Ok(updated) = shell.inspect_task(&task_id).await {
                    let _ = tx.send(KernelMsg::Updated(updated.clone()));
                    if let Ok(tree) = shell.agent_tree(&task_id).await {
                        let _ = tx.send(KernelMsg::Tree(task_id, tree));
                    }
                }
            });
        }
    }
}

impl eframe::App for CognyxDesktop {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_kernel();
        self.poll_tasks();
        ctx.request_repaint_after(Duration::from_millis(250));

        if ctx.input(|i| i.key_pressed(egui::Key::Slash) && !i.modifiers.shift) {
            self.show_command = true;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::L) && i.modifiers.command) {
            self.show_launcher = !self.show_launcher;
        }

        paint_wallpaper(ctx);

        egui::TopBottomPanel::top("menubar")
            .frame(bare_frame())
            .show(ctx, |ui| {
                self.top_bar(ui);
            });

        egui::TopBottomPanel::bottom("dock")
            .frame(bare_frame())
            .show(ctx, |ui| {
                self.dock(ui);
            });

        if self.show_launcher {
            egui::SidePanel::left("launcher")
                .resizable(true)
                .default_width(280.0)
                .frame(panel_frame())
                .show(ctx, |ui| self.launcher(ui));
        }

        if self.show_agents {
            egui::SidePanel::right("agents")
                .resizable(true)
                .default_width(340.0)
                .frame(panel_frame())
                .show(ctx, |ui| self.agent_panel(ui));
        }

        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show(ctx, |ui| {
                self.desktop(ui);
                if self.show_command {
                    self.command_bar(ui);
                }
                self.toasts(ui);
            });
    }
}

impl CognyxDesktop {
    fn top_bar(&self, ui: &mut Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.label(
                RichText::new("COGNYXOS")
                    .font(FontId::proportional(18.0))
                    .color(ACCENT)
                    .strong(),
            );
            ui.label(RichText::new("  desktop").color(MUTED));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(16.0);
                ui.label(RichText::new(clock_label()).color(TEXT));
                ui.separator();
                ui.label(RichText::new(&self.host_id).color(MUTED));
                ui.separator();
                ui.label(
                    RichText::new(format!("workspace {}", short_id(&self.workspace_id)))
                        .color(MUTED),
                );
            });
        });
        ui.add_space(8.0);
    }

    fn dock(&mut self, ui: &mut Ui) {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 210.0);
            dock_button(ui, "Launcher", self.show_launcher, || {
                self.show_launcher = !self.show_launcher;
            });
            dock_button(ui, "Command", self.show_command, || {
                self.show_command = !self.show_command;
            });
            dock_button(ui, "Agents", self.show_agents, || {
                self.show_agents = !self.show_agents;
            });
        });
        ui.add_space(10.0);
    }

    fn launcher(&mut self, ui: &mut Ui) {
        ui.add_space(6.0);
        ui.label(RichText::new("Launcher").color(ACCENT).strong());
        ui.label(RichText::new("Workspace search").color(MUTED).small());
        ui.add_space(8.0);
        let search = ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("Search files…")
                .desired_width(f32::INFINITY),
        );
        if search.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.show_command = true;
        }
        ui.add_space(10.0);

        ui.label(RichText::new("Pinned").color(MUTED).small());
        for (app, label) in [
            ("command-bar", "Command bar"),
            ("agent-panel", "Agent panel"),
            ("files", "Workspace files"),
        ] {
            if ui
                .add(egui::Button::new(label).fill(Color32::from_rgb(22, 32, 52)))
                .clicked()
            {
                match app {
                    "command-bar" => self.show_command = true,
                    "agent-panel" => self.show_agents = true,
                    _ => {
                        self.shell.open_window(app, &self.host_id, label);
                    }
                }
            }
        }

        ui.add_space(14.0);
        ui.label(RichText::new("Files").color(MUTED).small());
        let hits = if self.search.trim().is_empty() {
            self.shell.search_workspace("")
        } else {
            self.shell.search_workspace(&self.search)
        };
        egui::ScrollArea::vertical().show(ui, |ui| {
            if hits.is_empty() {
                ui.label(RichText::new("No workspace files yet.").color(MUTED));
            }
            for item in hits.into_iter().take(40) {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&item.name).color(TEXT));
                    ui.label(RichText::new(&item.location).color(MUTED).small());
                });
            }
        });
    }

    fn agent_panel(&mut self, ui: &mut Ui) {
        ui.add_space(6.0);
        ui.label(RichText::new("Agent panel").color(ACCENT).strong());
        ui.label(
            RichText::new("Tasks go to the Agent Kernel. The shell does not execute them.")
                .color(MUTED)
                .small(),
        );
        ui.add_space(8.0);

        let pending = self.shell.pending_approvals();
        if !pending.is_empty() {
            ui.label(RichText::new("Approvals").color(WARN).strong());
            let mut decisions = Vec::new();
            for req in pending {
                ui.group(|ui| {
                    ui.label(RichText::new(&req.capability).color(TEXT).strong());
                    ui.label(RichText::new(&req.reason).color(MUTED).small());
                    ui.horizontal(|ui| {
                        if ui.button("Allow once").clicked() {
                            decisions.push((req.id.clone(), ApprovalDecision::AllowOnce));
                        }
                        if ui.button("Allow for task").clicked() {
                            decisions.push((req.id.clone(), ApprovalDecision::AllowForTask));
                        }
                        if ui.button("Deny").clicked() {
                            decisions.push((req.id.clone(), ApprovalDecision::Deny));
                        }
                    });
                });
            }
            for (id, decision) in decisions {
                let _ = self.shell.decide_approval(&id, decision);
            }
            ui.add_space(8.0);
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.tasks.is_empty() {
                ui.label(RichText::new("No tasks yet.").color(MUTED));
            }
            for task in self.tasks.clone() {
                let selected = self.selected_task.as_deref() == Some(task.task_id.as_str());
                let fill = if selected {
                    Color32::from_rgb(24, 48, 52)
                } else {
                    Color32::from_rgb(18, 28, 44)
                };
                Frame::new()
                    .fill(fill)
                    .stroke(Stroke::new(1.0_f32, PANEL_EDGE))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        if ui
                            .add(
                                egui::Label::new(RichText::new(&task.prompt).color(TEXT))
                                    .sense(Sense::click()),
                            )
                            .clicked()
                        {
                            self.selected_task = Some(task.task_id.clone());
                        }
                        ui.horizontal(|ui| {
                            ui.label(status_text(&task.status));
                            ui.label(RichText::new(short_id(&task.task_id)).color(MUTED).small());
                        });
                        if let Some(error) = &task.error {
                            ui.label(RichText::new(error).color(DANGER).small());
                        }
                        if task.status == "failed" && ui.button("Recover").clicked() {
                            self.recover(task.task_id.clone());
                        }
                    });
                ui.add_space(6.0);
            }

            if let Some(task_id) = &self.selected_task {
                if let Some((_, tree)) = self.trees.iter().find(|(id, _)| id == task_id) {
                    ui.separator();
                    ui.label(RichText::new("Agent tree").color(ACCENT).small());
                    draw_tree(ui, tree, 0);
                }
            }
        });
    }

    fn desktop(&self, ui: &mut Ui) {
        let windows = self.shell.windows();
        if windows.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("Welcome to CognyxOS")
                            .font(FontId::proportional(28.0))
                            .color(TEXT),
                    );
                    ui.label(
                        RichText::new("Ask the kernel from the command bar. Try “list files”.")
                            .color(MUTED),
                    );
                });
            });
            return;
        }
        let mut origin = Pos2::new(36.0, 36.0);
        for window in windows {
            let size = Vec2::new(320.0, 170.0);
            let rect = Rect::from_min_size(ui.min_rect().min + origin.to_vec2(), size);
            ui.painter()
                .rect_filled(rect, CornerRadius::same(14), PANEL);
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(14),
                Stroke::new(1.0_f32, PANEL_EDGE),
                egui::StrokeKind::Outside,
            );
            ui.painter().text(
                rect.min + Vec2::new(16.0, 14.0),
                egui::Align2::LEFT_TOP,
                &window.title,
                FontId::proportional(16.0),
                TEXT,
            );
            ui.painter().text(
                rect.min + Vec2::new(16.0, 40.0),
                egui::Align2::LEFT_TOP,
                format!("{} · {}", window.application_id, window.runtime_id),
                FontId::proportional(12.0),
                MUTED,
            );
            origin.x += 28.0;
            origin.y += 28.0;
            if origin.x > 420.0 {
                origin.x = 36.0;
                origin.y += 160.0;
            }
        }
    }

    fn command_bar(&mut self, ui: &mut Ui) {
        let width = ui.available_width().min(760.0);
        let bar_rect = Rect::from_center_size(
            Pos2::new(ui.max_rect().center().x, ui.min_rect().top() + 88.0),
            Vec2::new(width, 118.0),
        );
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(bar_rect), |ui| {
            Frame::new()
                .fill(PANEL)
                .stroke(Stroke::new(1.0_f32, ACCENT_DIM))
                .corner_radius(CornerRadius::same(18))
                .inner_margin(Margin::same(16))
                .shadow(egui::Shadow {
                    offset: [0, 12],
                    blur: 28,
                    spread: 0,
                    color: Color32::from_black_alpha(90),
                })
                .show(ui, |ui| {
                    ui.label(RichText::new("Command bar").color(MUTED).small());
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut self.command)
                            .hint_text("Ask CognyxOS…")
                            .font(FontId::proportional(20.0))
                            .desired_width(f32::INFINITY),
                    );
                    if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.submit();
                    }
                    ui.horizontal(|ui| {
                        let busy = if self.busy { "running" } else { "idle" };
                        ui.label(RichText::new(busy).color(ACCENT).small());
                        ui.label(RichText::new(&self.status).color(MUTED).small());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Send").clicked() {
                                self.submit();
                            }
                        });
                    });
                });
        });
    }

    fn toasts(&self, ui: &mut Ui) {
        let notes = self.shell.notifications();
        if notes.is_empty() {
            return;
        }
        let mut y = ui.max_rect().bottom() - 88.0;
        for note in notes.iter().rev().take(3) {
            let rect = Rect::from_min_size(
                Pos2::new(ui.max_rect().right() - 360.0, y - 72.0),
                Vec2::new(330.0, 64.0),
            );
            ui.painter()
                .rect_filled(rect, CornerRadius::same(12), PANEL);
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(12),
                Stroke::new(1.0_f32, ACCENT_DIM),
                egui::StrokeKind::Outside,
            );
            ui.painter().text(
                rect.min + Vec2::new(14.0, 10.0),
                egui::Align2::LEFT_TOP,
                &note.title,
                FontId::proportional(14.0),
                TEXT,
            );
            ui.painter().text(
                rect.min + Vec2::new(14.0, 32.0),
                egui::Align2::LEFT_TOP,
                &note.body,
                FontId::proportional(12.0),
                MUTED,
            );
            y -= 76.0;
        }
    }
}

fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::TRANSPARENT;
    visuals.window_fill = PANEL;
    visuals.override_text_color = Some(TEXT);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(24, 36, 56);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(32, 52, 74);
    visuals.widgets.active.bg_fill = ACCENT_DIM;
    visuals.selection.bg_fill = ACCENT_DIM;
    visuals.hyperlink_color = ACCENT;
    ctx.set_visuals(visuals);
    ctx.set_pixels_per_point(1.15);
}

fn paint_wallpaper(ctx: &egui::Context) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let rect = ctx.screen_rect();
    painter.rect_filled(rect, 0.0, BG);
    let mut y = rect.top() + 48.0;
    while y < rect.bottom() {
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(1.0_f32, Color32::from_rgb(16, 26, 42)),
        );
        y += 48.0;
    }
    painter.circle_filled(
        Pos2::new(rect.right() - 180.0, rect.top() + 140.0),
        220.0,
        Color32::from_rgba_unmultiplied(20, 80, 90, 35),
    );
}

fn bare_frame() -> Frame {
    Frame::new()
        .fill(Color32::from_rgba_unmultiplied(10, 16, 28, 220))
        .inner_margin(Margin::ZERO)
}

fn panel_frame() -> Frame {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, PANEL_EDGE))
        .inner_margin(Margin::same(12))
}

fn dock_button(ui: &mut Ui, label: &str, active: bool, on_click: impl FnOnce()) {
    let fill = if active {
        ACCENT_DIM
    } else {
        Color32::from_rgb(22, 32, 52)
    };
    let text = if active { ACCENT } else { TEXT };
    if ui
        .add(
            egui::Button::new(RichText::new(label).color(text))
                .fill(fill)
                .min_size(Vec2::new(120.0, 36.0))
                .corner_radius(CornerRadius::same(18)),
        )
        .clicked()
    {
        on_click();
    }
}

fn draw_tree(ui: &mut Ui, node: &AgentNode, depth: usize) {
    ui.horizontal(|ui| {
        ui.add_space(depth as f32 * 12.0);
        ui.label(RichText::new(format!("{} · {}", node.role, node.status)).color(TEXT));
    });
    if let Some(op) = &node.operation {
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 12.0 + 12.0);
            ui.label(RichText::new(op).color(MUTED).small());
        });
    }
    for child in &node.children {
        draw_tree(ui, child, depth + 1);
    }
}

fn upsert_task(tasks: &mut Vec<TaskView>, task: TaskView) {
    if let Some(existing) = tasks.iter_mut().find(|t| t.task_id == task.task_id) {
        *existing = task;
    } else {
        tasks.insert(0, task);
    }
}

fn short_id(id: &str) -> String {
    if id.len() <= 14 {
        id.to_string()
    } else {
        format!("{}…", &id[..12])
    }
}

fn clock_label() -> String {
    let secs = CognyxShell::<AgentKernelAdapter>::now_secs();
    format!("{:02}:{:02} UTC", (secs / 3600) % 24, (secs / 60) % 60)
}

fn status_text(status: &str) -> RichText {
    let color = match status {
        "completed" => ACCENT,
        "failed" => DANGER,
        "running" | "pending" | "recovering" => WARN,
        _ => MUTED,
    };
    RichText::new(status).color(color).small().strong()
}
