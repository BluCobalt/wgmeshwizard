//! The egui desktop interface.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, Sense,
    TextEdit, Ui, Vec2,
};

use crate::crypto;
use crate::i18n::{self, tr};
use crate::model::{DEFAULT_LISTEN_PORT, DEFAULT_MTU};
use crate::settings::{self, AppearanceSettings, LanguageSettings, MeshViewSettings, Settings};
use crate::storage;
use crate::wgconfig;

mod canvas;
mod settings_ui;
mod style;
mod ui;
mod widgets;

use canvas::{layout_positions, paint_connection, paint_grid, paint_node};
use style::setup_style;

const NODE_WIDTH: f32 = 200.0;
const NODE_HEIGHT: f32 = 88.0;
const MIN_ZOOM: f32 = 0.25;
const MAX_ZOOM: f32 = 2.5;
const FIELD_WIDTH: f32 = 215.0;

const CANVAS_BG: Color32 = Color32::from_rgb(12, 16, 25);
const GRID_MINOR: Color32 = Color32::from_rgb(24, 32, 47);
const GRID_MAJOR: Color32 = Color32::from_rgb(33, 44, 64);
const ACCENT: Color32 = Color32::from_rgb(86, 204, 242);
const ACCENT_WARM: Color32 = Color32::from_rgb(255, 175, 84);
const TEXT_DIM: Color32 = Color32::from_rgb(147, 162, 184);
const DANGER: Color32 = Color32::from_rgb(255, 107, 107);

fn archive_file_dialog(title: impl Into<String>) -> rfd::FileDialog {
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter(tr!("ui.file_dialog.mesh_archive_filter"), &["wgm", "zip"])
        .add_filter(tr!("ui.file_dialog.zip_archive_filter"), &["zip"])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum ThemeMode {
    #[default]
    System,
    Dark,
    Light,
}

impl ThemeMode {
    fn label(self) -> String {
        match self {
            Self::System => tr!("ui.label.system"),
            Self::Dark => tr!("ui.label.dark"),
            Self::Light => tr!("ui.label.light"),
        }
    }

    fn preference(self) -> egui::ThemePreference {
        match self {
            Self::System => egui::ThemePreference::System,
            Self::Dark => egui::ThemePreference::Dark,
            Self::Light => egui::ThemePreference::Light,
        }
    }

    fn as_storage(self) -> String {
        match self {
            Self::System => "system",
            Self::Dark => "dark",
            Self::Light => "light",
        }
        .to_owned()
    }

    fn from_storage(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "system" => Some(Self::System),
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum MeshShape {
    #[default]
    Circle,
    Square,
    Cards,
    Rows,
}

impl MeshShape {
    const ALL: [Self; 4] = [Self::Circle, Self::Square, Self::Cards, Self::Rows];

    fn label(self) -> String {
        match self {
            Self::Circle => tr!("ui.label.circle"),
            Self::Square => tr!("ui.label.square_outline"),
            Self::Cards => tr!("ui.label.card_grid"),
            Self::Rows => tr!("ui.label.vertical_rows"),
        }
    }

    fn as_storage(self) -> String {
        match self {
            Self::Circle => "circle",
            Self::Square => "square",
            Self::Cards => "cards",
            Self::Rows => "rows",
        }
        .to_owned()
    }

    fn from_storage(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "circle" => Some(Self::Circle),
            "square" => Some(Self::Square),
            "cards" | "grid" => Some(Self::Cards),
            "rows" => Some(Self::Rows),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct CanvasColors {
    bg: Color32,
    grid_minor: Color32,
    grid_major: Color32,
    node_fill: Color32,
    node_fill_hovered: Color32,
    node_fill_selected: Color32,
    node_border: Color32,
    node_border_hovered: Color32,
    node_text: Color32,
    node_subtext: Color32,
    node_shadow: Color32,
    edge: Color32,
    edge_disabled: Color32,
    selected_accent: Color32,
    label_bg: Color32,
    label_bg_hovered: Color32,
    label_bg_selected: Color32,
    label_border: Color32,
    label_text: Color32,
    label_text_selected: Color32,
    label_text_disabled: Color32,
    hint: Color32,
    empty_title: Color32,
    empty_subtitle: Color32,
}

impl CanvasColors {
    fn for_dark(dark: bool) -> Self {
        if dark {
            Self {
                bg: CANVAS_BG,
                grid_minor: GRID_MINOR,
                grid_major: GRID_MAJOR,
                node_fill: Color32::from_rgb(24, 32, 47),
                node_fill_hovered: Color32::from_rgb(30, 40, 58),
                node_fill_selected: Color32::from_rgb(35, 47, 68),
                node_border: Color32::from_rgb(48, 61, 82),
                node_border_hovered: Color32::from_rgb(84, 110, 145),
                node_text: Color32::from_rgb(235, 242, 252),
                node_subtext: Color32::from_rgb(166, 190, 219),
                node_shadow: Color32::from_black_alpha(100),
                edge: Color32::from_rgb(83, 160, 189),
                edge_disabled: Color32::from_rgb(66, 76, 92),
                selected_accent: ACCENT_WARM,
                label_bg: Color32::from_rgb(16, 22, 33),
                label_bg_hovered: Color32::from_rgb(32, 44, 62),
                label_bg_selected: Color32::from_rgb(52, 43, 28),
                label_border: Color32::from_rgb(44, 58, 78),
                label_text: Color32::from_rgb(158, 216, 238),
                label_text_selected: Color32::from_rgb(255, 214, 154),
                label_text_disabled: Color32::from_rgb(110, 120, 138),
                hint: Color32::from_rgb(75, 89, 110),
                empty_title: Color32::from_rgb(120, 138, 164),
                empty_subtitle: Color32::from_rgb(84, 100, 124),
            }
        } else {
            Self {
                bg: Color32::from_rgb(241, 245, 250),
                grid_minor: Color32::from_rgb(225, 232, 242),
                grid_major: Color32::from_rgb(211, 221, 234),
                node_fill: Color32::from_rgb(255, 255, 255),
                node_fill_hovered: Color32::from_rgb(248, 251, 255),
                node_fill_selected: Color32::from_rgb(229, 243, 252),
                node_border: Color32::from_rgb(200, 211, 225),
                node_border_hovered: Color32::from_rgb(122, 154, 186),
                node_text: Color32::from_rgb(26, 37, 52),
                node_subtext: Color32::from_rgb(69, 86, 106),
                node_shadow: Color32::from_black_alpha(30),
                edge: Color32::from_rgb(24, 122, 152),
                edge_disabled: Color32::from_rgb(170, 180, 193),
                selected_accent: Color32::from_rgb(196, 122, 28),
                label_bg: Color32::from_rgb(255, 255, 255),
                label_bg_hovered: Color32::from_rgb(237, 246, 253),
                label_bg_selected: Color32::from_rgb(255, 244, 224),
                label_border: Color32::from_rgb(205, 216, 228),
                label_text: Color32::from_rgb(13, 96, 122),
                label_text_selected: Color32::from_rgb(150, 84, 14),
                label_text_disabled: Color32::from_rgb(145, 156, 170),
                hint: Color32::from_rgb(130, 145, 165),
                empty_title: Color32::from_rgb(94, 112, 136),
                empty_subtitle: Color32::from_rgb(130, 146, 166),
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
enum Selection {
    #[default]
    None,
    Hosts(Vec<u64>),
    Connection {
        from: u64,
        to: u64,
    },
}

impl Selection {
    fn hosts(&self) -> &[u64] {
        match self {
            Self::Hosts(ids) => ids,
            _ => &[],
        }
    }

    fn contains_host(&self, id: u64) -> bool {
        self.hosts().contains(&id)
    }

    fn add_host(&mut self, id: u64) {
        match self {
            Self::Hosts(ids) => {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            _ => *self = Self::Hosts(vec![id]),
        }
    }

    fn toggle_host(&mut self, id: u64) {
        let mut empty = false;
        match self {
            Self::Hosts(ids) => {
                if let Some(index) = ids.iter().position(|&selected| selected == id) {
                    ids.remove(index);
                } else {
                    ids.push(id);
                }
                empty = ids.is_empty();
            }
            _ => *self = Self::Hosts(vec![id]),
        }
        if empty {
            *self = Self::None;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum SettingsTab {
    #[default]
    General,
    About,
}

#[derive(Clone, Copy, Debug)]
struct Camera {
    pan: Vec2,
    zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

impl Camera {
    fn world_to_screen(&self, canvas: Rect, x: f32, y: f32) -> Pos2 {
        canvas.center() + (Vec2::new(x, y) - self.pan) * self.zoom
    }

    fn screen_to_world(&self, canvas: Rect, point: Pos2) -> (f32, f32) {
        let world = (point - canvas.center()) / self.zoom + self.pan;
        (world.x, world.y)
    }
}

pub struct MeshWizardApp {
    mesh: crate::model::Mesh,
    saved: crate::model::Mesh,
    project_path: PathBuf,
    startup_prompt: bool,
    close_prompt: bool,
    close_prompt_project: bool,
    load_dirty: bool,
    selection: Selection,
    camera: Camera,
    fit_pending: bool,
    status: String,
    status_is_error: bool,
    show_settings: bool,
    settings_tab: SettingsTab,
    settings_just_opened: bool,
    ui_scale: f32,
    theme_mode: ThemeMode,
    mesh_shape: MeshShape,
    language: String,
    settings_path: PathBuf,
    settings_dirty_since: Option<Instant>,
    delete_prompt: Option<Vec<u64>>,
    delete_prompt_just_opened: bool,
    delete_prompt_dont_ask: bool,
    suppress_delete_prompt: bool,
    rotate_prompt: bool,
    rotate_prompt_just_opened: bool,
    show_private_key: bool,
    show_config: bool,
}

impl MeshWizardApp {
    pub fn new(cc: &eframe::CreationContext<'_>, project: Option<PathBuf>) -> Self {
        setup_style(&cc.egui_ctx);

        let startup_prompt = project.is_none();
        let project = project.unwrap_or_default();

        let settings_path = match settings::settings_path() {
            Some(path) => path,
            None => {
                // Extremely unusual (no HOME/XDG_CONFIG_HOME), but better than
                // panicking or silently dropping settings.
                PathBuf::from("settings.toml")
            }
        };

        let mut settings_warning = None;
        let mut needs_settings_save = false;

        let stored_settings = match settings::load(&settings_path) {
            Ok(Some(settings)) => settings,
            Ok(None) => {
                needs_settings_save = true;
                Settings::default()
            }
            Err(error) => {
                settings_warning = Some(error);
                Settings::default()
            }
        };

        let ui_scale = stored_settings.appearance.ui_scale;
        let theme_mode =
            ThemeMode::from_storage(&stored_settings.appearance.theme).unwrap_or_default();
        let mesh_shape =
            MeshShape::from_storage(&stored_settings.mesh_view.arrange_layout).unwrap_or_default();
        let language = stored_settings.language.locale.clone();

        cc.egui_ctx.set_zoom_factor(ui_scale);
        cc.egui_ctx.set_theme(theme_mode.preference());
        i18n::set_language(&language);

        let mut app = Self {
            mesh: crate::model::Mesh::new(),
            saved: crate::model::Mesh::new(),
            project_path: project,
            startup_prompt,
            close_prompt: false,
            close_prompt_project: false,
            load_dirty: false,
            selection: Selection::None,
            camera: Camera::default(),
            fit_pending: true,
            status: String::new(),
            status_is_error: false,
            show_settings: false,
            settings_tab: SettingsTab::default(),
            settings_just_opened: false,
            ui_scale,
            theme_mode,
            mesh_shape,
            language,
            settings_path,
            settings_dirty_since: None,
            delete_prompt: None,
            delete_prompt_just_opened: false,
            delete_prompt_dont_ask: false,
            suppress_delete_prompt: false,
            rotate_prompt: false,
            rotate_prompt_just_opened: false,
            show_private_key: false,
            show_config: false,
        };
        if !app.startup_prompt {
            app.load_project(true);
        }

        if needs_settings_save {
            app.save_settings();
        }
        if let Some(error) = settings_warning {
            app.set_status(
                tr!("ui.new.could_not_read_settings_error", error = error),
                true,
            );
        }

        app
    }

    fn queue_settings_save(&mut self) {
        if self.settings_dirty_since.is_none() {
            self.settings_dirty_since = Some(Instant::now());
        }
    }

    fn save_settings(&mut self) {
        self.settings_dirty_since = None;
        let settings = Settings {
            version: settings::SETTINGS_VERSION,
            appearance: AppearanceSettings {
                theme: self.theme_mode.as_storage(),
                ui_scale: self.ui_scale,
            },
            language: LanguageSettings {
                locale: self.language.clone(),
            },
            mesh_view: MeshViewSettings {
                arrange_layout: self.mesh_shape.as_storage(),
            },
        };

        if let Err(error) = settings::save(&self.settings_path, &settings) {
            self.set_status(
                tr!(
                    "ui.save_settings.could_not_save_settings_error",
                    error = error
                ),
                true,
            );
        }
    }

    fn project_dir(&self) -> PathBuf {
        self.project_path.clone()
    }

    fn project_name(&self) -> String {
        self.project_path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| self.project_path.display().to_string())
    }

    fn project_is_archive(&self) -> bool {
        storage::is_archive_project(&self.project_path)
    }

    fn project_kind(&self) -> String {
        if self.project_is_archive() {
            match self
                .project_path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref()
            {
                Some("wgm") => tr!("ui.project_overview.wgm_archive"),
                Some("zip") => tr!("ui.project_overview.zip_archive"),
                _ => tr!("ui.project_overview.archive"),
            }
        } else if self.project_path.is_dir() {
            tr!("ui.project_overview.directory")
        } else {
            tr!("ui.project_overview.directory_new")
        }
    }

    fn request_save(&mut self) {
        self.save_now(true);
    }

    fn has_unsaved_changes(&self) -> bool {
        self.load_dirty || self.mesh != self.saved
    }

    fn request_close_project(&mut self) {
        if self.has_unsaved_changes() {
            self.close_prompt = true;
            self.close_prompt_project = true;
        } else {
            self.close_project();
        }
    }

    fn close_project(&mut self) {
        self.mesh = crate::model::Mesh::new();
        self.saved = self.mesh.clone();
        self.project_path = PathBuf::new();
        self.startup_prompt = true;
        self.close_prompt = false;
        self.close_prompt_project = false;
        self.load_dirty = false;
        self.selection = Selection::None;
        self.camera = Camera::default();
        self.fit_pending = true;
        self.show_config = false;
        self.delete_prompt = None;
        self.delete_prompt_dont_ask = false;
        self.rotate_prompt = false;
        self.rotate_prompt_just_opened = false;
        self.status.clear();
        self.status_is_error = false;
    }

    fn load_from_path(&mut self, path: PathBuf) -> bool {
        let previous = self.project_path.clone();
        self.project_path = path;
        let loaded = self.load_project(false);
        if !loaded {
            self.project_path = previous;
        }
        loaded
    }

    fn pick_load_archive(&mut self) -> bool {
        if let Some(path) = archive_file_dialog(tr!("ui.file_dialog.open_archive")).pick_file() {
            self.load_from_path(path)
        } else {
            false
        }
    }

    fn pick_load_folder(&mut self) -> bool {
        if let Some(path) = rfd::FileDialog::new()
            .set_title(tr!("ui.file_dialog.open_folder"))
            .pick_folder()
        {
            self.load_from_path(path)
        } else {
            false
        }
    }

    fn pick_new_archive(&mut self) -> bool {
        if let Some(mut path) = archive_file_dialog(tr!("ui.file_dialog.save_archive"))
            .set_file_name("mesh.wgm")
            .save_file()
        {
            if !storage::is_archive_path(&path) {
                path.set_extension("wgm");
            }
            self.create_project(path)
        } else {
            false
        }
    }

    fn pick_new_folder(&mut self) -> bool {
        if let Some(path) = rfd::FileDialog::new()
            .set_title(tr!("ui.file_dialog.save_folder"))
            .pick_folder()
        {
            self.create_project(path)
        } else {
            false
        }
    }

    fn create_project(&mut self, path: PathBuf) -> bool {
        if !storage::is_archive_project(&path)
            && (path.join(storage::MESH_FILE).exists()
                || path.join(storage::HOSTS_FILE).exists()
                || path.join(storage::LINKS_FILE).exists())
        {
            self.set_status(
                tr!(
                    "ui.startup.a_project_already_exists_at_path",
                    path = path.display()
                ),
                true,
            );
            return false;
        }

        self.project_path = path;
        self.mesh = crate::model::Mesh::new();
        self.saved = self.mesh.clone();
        self.load_dirty = false;
        self.selection = Selection::None;
        self.camera = Camera::default();
        self.fit_pending = true;
        self.show_config = false;

        if !self.save_now(false) {
            return false;
        }
        self.set_status(
            tr!(
                "ui.new_project.created_a_new_project_at_path",
                path = self.project_dir().display()
            ),
            false,
        );
        true
    }

    fn pick_save_archive(&mut self) {
        let default_name = self
            .project_path
            .file_stem()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("mesh");
        let default_name = format!("{default_name}.wgm");
        if let Some(mut path) = archive_file_dialog(tr!("ui.file_dialog.save_archive"))
            .set_file_name(default_name)
            .save_file()
        {
            if !storage::is_archive_path(&path) {
                path.set_extension("wgm");
            }
            self.project_path = path;
            self.save_now(true);
        }
    }

    fn pick_save_folder(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title(tr!("ui.file_dialog.save_folder"))
            .pick_folder()
        {
            self.project_path = path;
            self.save_now(true);
        }
    }

    fn set_status(&mut self, message: impl Into<String>, is_error: bool) {
        self.status = message.into();
        self.status_is_error = is_error;
    }

    fn load_project(&mut self, startup: bool) -> bool {
        let path = self.project_dir();
        match storage::load_project(&path) {
            Ok(mut report) => {
                let generated = report.mesh.ensure_private_keys();
                report.mesh.normalize();
                let psks_changed = report.mesh.sync_preshared_keys();
                let count = report.mesh.hosts.len();
                self.mesh = report.mesh;
                self.selection = Selection::None;
                self.fit_pending = true;
                self.show_config = false;
                self.saved = self.mesh.clone();
                self.load_dirty =
                    report.created_new || report.normalized || generated > 0 || psks_changed;

                let message = if report.created_new {
                    tr!(
                        "ui.load_project.new_project_at_path_add_a",
                        path = path.display()
                    )
                } else if count == 1 {
                    tr!(
                        "ui.load_project.loaded_1_host_from_path",
                        path = path.display()
                    )
                } else {
                    tr!(
                        "ui.load_project.loaded_count_hosts_from_path",
                        count = count,
                        path = path.display()
                    )
                };
                self.set_status(message, false);
                true
            }
            Err(error) => {
                self.set_status(
                    tr!(
                        "ui.load_project.could_not_load_project_error",
                        error = error
                    ),
                    true,
                );
                if startup {
                    self.mesh = crate::model::Mesh::new();
                    self.saved = self.mesh.clone();
                    self.load_dirty = false;
                }
                false
            }
        }
    }

    fn save_now(&mut self, announce: bool) -> bool {
        let directory = self.project_dir();
        match storage::save_project(&directory, &self.mesh) {
            Ok(()) => {
                self.saved = self.mesh.clone();
                self.load_dirty = false;
                if announce {
                    self.set_status(
                        tr!(
                            "ui.save_now.saved_project_to_path",
                            path = directory.display()
                        ),
                        false,
                    );
                }
                true
            }
            Err(error) => {
                self.set_status(
                    tr!("ui.save_now.could_not_save_project_error", error = error),
                    true,
                );
                false
            }
        }
    }

    fn export_configs(&mut self) {
        let directory = storage::default_output_dir(&self.project_dir());
        match wgconfig::export_configs(&self.mesh, &directory) {
            Ok(paths) => {
                let message = if paths.len() == 1 {
                    tr!(
                        "ui.export_configs.exported_1_configuration_to_path",
                        path = directory.display()
                    )
                } else {
                    tr!(
                        "ui.export_configs.exported_count_configurations_to_path",
                        count = paths.len(),
                        path = directory.display()
                    )
                };
                self.set_status(message, false);
            }
            Err(error) => self.set_status(
                tr!("ui.export_configs.export_failed_error", error = error),
                true,
            ),
        }
    }

    fn add_host_near(&mut self, position: Option<(f32, f32)>) {
        let name = self.mesh.unique_host_name();
        let address = self.mesh.suggest_address();
        let (x, y) = position.unwrap_or_else(|| self.next_ring_position());
        let id = self.mesh.add_host(name.clone(), x, y);
        if let Some(host) = self.mesh.host_mut(id) {
            host.address = address;
            host.private_key = crypto::generate_private_key();
        }
        if self.mesh.defaults.preshared_keys {
            self.mesh.sync_preshared_keys();
        }

        // Keep the newly created host in the middle of the canvas.
        self.fit_pending = false;
        self.camera.pan = Vec2::new(x, y);
        if self.camera.zoom < 0.5 {
            self.camera.zoom = 1.0;
        }

        self.selection = Selection::Hosts(vec![id]);
        self.set_status(
            tr!(
                "ui.add_host_near.added_name_and_centered_the_canvas",
                name = name
            ),
            false,
        );
    }

    fn next_ring_position(&self) -> (f32, f32) {
        let (cx, cy) = self.mesh.centroid();
        let n = self.mesh.hosts.len() as f32;
        let angle = n * 2.399_963;
        let radius = 210.0 + (n as u32 / 8) as f32 * 90.0;
        (cx + angle.cos() * radius, cy + angle.sin() * radius)
    }

    fn delete_hosts(&mut self, ids: &[u64]) {
        let mut names = Vec::new();
        for &id in ids {
            let name = self
                .mesh
                .host(id)
                .map(|host| host.name.clone())
                .unwrap_or_else(|| tr!("ui.delete_host.host_id", id = id));
            if self.mesh.remove_host(id) {
                names.push(name);
            }
        }
        if names.is_empty() {
            self.delete_prompt = None;
            return;
        }

        let mut empty_selection = false;
        if let Selection::Hosts(selected) = &mut self.selection {
            selected.retain(|selected| !ids.contains(selected));
            empty_selection = selected.is_empty();
        }
        if empty_selection {
            self.selection = Selection::None;
        }
        if let Selection::Connection { from, to } = self.selection
            && (self.mesh.host(from).is_none() || self.mesh.host(to).is_none())
        {
            self.selection = Selection::None;
        }

        self.delete_prompt = None;
        let message = if names.len() == 1 {
            tr!("ui.delete_host.deleted_name", name = &names[0])
        } else {
            tr!("ui.delete_host.deleted_count", count = names.len())
        };
        self.set_status(message, false);
    }

    fn request_rotate_all_secrets(&mut self) {
        if self.mesh.hosts.is_empty() {
            self.set_status(tr!("ui.rotate_secrets.there_are_no_hosts_to_rotate"), false);
            return;
        }
        self.rotate_prompt = true;
        self.rotate_prompt_just_opened = true;
    }

    fn rotate_all_secrets(&mut self) {
        let (host_count, rotate_psks) = self.mesh.rotate_secrets();

        self.rotate_prompt = false;
        self.rotate_prompt_just_opened = false;
        let message = if rotate_psks {
            tr!(
                "ui.rotate_secrets.rotated_count_private_keys_and_preshared",
                count = host_count
            )
        } else {
            tr!(
                "ui.rotate_secrets.rotated_count_private_keys",
                count = host_count
            )
        };
        self.set_status(message, false);
    }

    fn request_delete_hosts(&mut self, ids: Vec<u64>) {
        let ids: Vec<u64> = ids
            .into_iter()
            .filter(|&id| self.mesh.host(id).is_some())
            .collect();
        if ids.is_empty() {
            return;
        }
        if self.suppress_delete_prompt {
            self.delete_hosts(&ids);
        } else {
            self.delete_prompt = Some(ids);
            self.delete_prompt_just_opened = true;
            self.delete_prompt_dont_ask = false;
        }
    }

    fn request_delete_host(&mut self, id: u64) {
        self.request_delete_hosts(vec![id]);
    }

    fn arrange_mesh(&mut self) {
        let mut order: Vec<usize> = (0..self.mesh.hosts.len()).collect();
        order.sort_by_key(|&index| self.mesh.hosts[index].name.to_ascii_lowercase());
        let count = order.len();
        if count == 0 {
            return;
        }

        let spacing_x = NODE_WIDTH + 70.0;
        let spacing_y = NODE_HEIGHT + 60.0;
        let positions = layout_positions(self.mesh_shape, count, spacing_x, spacing_y);

        for (index, (x, y)) in order.into_iter().zip(positions) {
            let host = &mut self.mesh.hosts[index];
            host.x = x;
            host.y = y;
        }

        self.camera.pan = Vec2::ZERO;
        self.fit_pending = true;
        let message = if count == 1 {
            tr!(
                "ui.arrange_mesh.arranged_1_host_using_the_shape",
                shape = self.mesh_shape.label()
            )
        } else {
            tr!(
                "ui.arrange_mesh.arranged_count_hosts_using_the_shape",
                count = count,
                shape = self.mesh_shape.label()
            )
        };
        self.set_status(message, false);
    }

    fn fit_to(&mut self, canvas_size: Vec2) {
        if self.mesh.hosts.is_empty() {
            self.camera.pan = Vec2::ZERO;
            self.camera.zoom = 1.0;
            return;
        }

        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for host in &self.mesh.hosts {
            min_x = min_x.min(host.x);
            max_x = max_x.max(host.x);
            min_y = min_y.min(host.y);
            max_y = max_y.max(host.y);
        }

        let width = (max_x - min_x + NODE_WIDTH).max(320.0);
        let height = (max_y - min_y + NODE_HEIGHT).max(220.0);
        let available_x = (canvas_size.x - 100.0).max(200.0);
        let available_y = (canvas_size.y - 100.0).max(150.0);
        let zoom = (available_x / width)
            .min(available_y / height)
            .clamp(MIN_ZOOM, 1.35);

        self.camera.zoom = zoom;
        self.camera.pan = Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
    }

    fn canvas(&mut self, ui: &mut Ui) {
        let (background, painter) =
            ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let canvas = background.rect;
        let colors = CanvasColors::for_dark(ui.visuals().dark_mode);
        painter.rect_filled(canvas, CornerRadius::ZERO, colors.bg);

        // Direct reads of scroll/key input ignore the modal backdrop, so avoid
        // driving the canvas while a modal, window or popup is in front of it.
        let overlay_open = self.show_settings
            || self.startup_prompt
            || self.close_prompt
            || ui.ctx().memory(|memory| memory.top_modal_layer().is_some())
            || ui.ctx().any_popup_open();

        if self.fit_pending {
            self.fit_to(canvas.size());
            self.fit_pending = false;
        }

        // Zoom around the pointer.
        if !overlay_open
            && let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
            && canvas.contains(pointer)
        {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll.abs() > 0.01 {
                let old_zoom = self.camera.zoom;
                let new_zoom = (old_zoom * (scroll * 0.0018).exp()).clamp(MIN_ZOOM, MAX_ZOOM);
                if (new_zoom - old_zoom).abs() > f32::EPSILON {
                    let before = self.camera.screen_to_world(canvas, pointer);
                    self.camera.zoom = new_zoom;
                    let after = self.camera.screen_to_world(canvas, pointer);
                    self.camera.pan += Vec2::new(before.0 - after.0, before.1 - after.1);
                }
            }
        }

        // Pan the canvas when dragging empty space.
        if background.dragged() {
            self.camera.pan -= background.drag_delta() / self.camera.zoom;
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if background.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }

        // Right click on the canvas adds a host at that position.
        let context_world = ui
            .input(|input| input.pointer.hover_pos())
            .map(|position| self.camera.screen_to_world(canvas, position));
        background.context_menu(|ui| {
            if ui.button(tr!("ui.canvas.add_host_here")).clicked() {
                self.add_host_near(context_world);
                ui.close();
            }
            ui.separator();
            if ui.button(tr!("ui.top_bar.fit_view")).clicked() {
                self.fit_pending = true;
                ui.close();
            }
        });

        paint_grid(&painter, canvas, &self.camera, &colors);

        let zoom = self.camera.zoom;

        if self.mesh.hosts.is_empty() {
            // Anchor the empty-state message to the canvas origin so it pans
            // and scales with the canvas rather than behaving like an overlay.
            let center = self.camera.world_to_screen(canvas, 0.0, 0.0);
            let scale = zoom.clamp(0.55, 1.6);
            painter.text(
                center - Vec2::new(0.0, 20.0 * scale),
                Align2::CENTER_CENTER,
                tr!("ui.canvas.no_hosts_yet"),
                FontId::proportional(24.0 * scale),
                colors.empty_title,
            );
            painter.text(
                center + Vec2::new(0.0, 18.0 * scale),
                Align2::CENTER_CENTER,
                tr!("ui.canvas.use_add_host_or_right_click"),
                FontId::proportional(14.0 * scale),
                colors.empty_subtitle,
            );
        }

        let node_rects: HashMap<u64, Rect> = self
            .mesh
            .hosts
            .iter()
            .map(|host| {
                let center = self.camera.world_to_screen(canvas, host.x, host.y);
                (
                    host.id,
                    Rect::from_center_size(center, Vec2::new(NODE_WIDTH, NODE_HEIGHT) * zoom),
                )
            })
            .collect();

        let hovered_host = if overlay_open {
            None
        } else {
            ui.ctx()
                .pointer_hover_pos()
                .filter(|position| canvas.contains(*position))
                .and_then(|position| {
                    self.mesh
                        .hosts
                        .iter()
                        .rev()
                        .find(|host| {
                            node_rects
                                .get(&host.id)
                                .is_some_and(|rect| rect.contains(position))
                        })
                        .map(|host| host.id)
                })
        };
        let selected_hosts = self.selection.hosts().to_vec();

        // Connections first so nodes sit on top of their endpoints. Paint
        // highlighted connections in a second pass so they are brought to the
        // front of the other edges.
        let mut label_rects: Vec<Rect> = node_rects.values().copied().collect();
        let mut clicked_connection: Option<(u64, u64)> = None;
        for highlighted_pass in [false, true] {
            for connection in &self.mesh.connections {
                let (Some(from_rect), Some(to_rect)) = (
                    node_rects.get(&connection.from),
                    node_rects.get(&connection.to),
                ) else {
                    continue;
                };
                let from_name = self
                    .mesh
                    .host(connection.from)
                    .map(|host| host.name.as_str())
                    .unwrap_or("?");
                let to_name = self
                    .mesh
                    .host(connection.to)
                    .map(|host| host.name.as_str())
                    .unwrap_or("?");
                let endpoint = self.mesh.resolved_endpoint(connection.from, connection.to);
                let selected = self.selection
                    == Selection::Connection {
                        from: connection.from,
                        to: connection.to,
                    };
                let highlighted = selected
                    || hovered_host == Some(connection.from)
                    || hovered_host == Some(connection.to)
                    || selected_hosts.contains(&connection.from)
                    || selected_hosts.contains(&connection.to);
                if highlighted != highlighted_pass {
                    continue;
                }

                if paint_connection(
                    ui,
                    &painter,
                    *from_rect,
                    *to_rect,
                    from_name,
                    to_name,
                    endpoint.as_deref(),
                    connection,
                    selected,
                    highlighted,
                    zoom,
                    &mut label_rects,
                    &colors,
                ) {
                    clicked_connection = Some((connection.from, connection.to));
                }
            }
        }

        // Nodes.
        let mut dragged_host: Option<(u64, Vec2)> = None;
        let mut clicked_host: Option<(u64, bool, bool)> = None;
        for host in &self.mesh.hosts {
            let Some(node_rect) = node_rects.get(&host.id).copied() else {
                continue;
            };
            let response = ui.interact(
                node_rect,
                ui.make_persistent_id(("host_node", host.id)),
                Sense::click_and_drag(),
            );
            let hovered = response.hovered();
            let selected = self.selection.contains_host(host.id);
            paint_node(
                &painter,
                node_rect,
                host,
                selected,
                hovered,
                zoom,
                host.effective_listen_port(&self.mesh.defaults),
                &colors,
            );

            if response.dragged() {
                dragged_host = Some((host.id, response.drag_delta()));
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            } else if hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
            if response.clicked() || response.drag_started() {
                let modifiers = ui.input(|input| input.modifiers);
                let additive = modifiers.shift || modifiers.ctrl || modifiers.command;
                let toggle = modifiers.ctrl || modifiers.command;
                clicked_host = Some((host.id, additive, toggle));
            }

            let endpoint = if host.endpoint.trim().is_empty() {
                tr!("ui.canvas.no_default_endpoint")
            } else {
                crate::model::format_endpoint(
                    &host.endpoint,
                    host.effective_listen_port(&self.mesh.defaults),
                )
            };
            response.on_hover_text(tr!(
                "ui.canvas.name_address_endpoint_drag_to_move",
                name = host.name,
                address = host.address,
                endpoint = endpoint
            ));
        }

        if let Some((from, to)) = clicked_connection {
            self.selection = Selection::Connection { from, to };
        }
        if let Some((id, additive, toggle)) = clicked_host {
            if !additive {
                self.selection = Selection::Hosts(vec![id]);
            } else if toggle {
                self.selection.toggle_host(id);
            } else {
                self.selection.add_host(id);
            }
        }

        // Apply mutations after the immutable iteration. Dragging any host in a
        // multi-selection moves the whole selection together.
        if let Some((dragged_id, delta)) = dragged_host {
            let world_delta = Vec2::new(delta.x / zoom, delta.y / zoom);
            let selected_hosts: Vec<u64> = self.selection.hosts().to_vec();
            if selected_hosts.contains(&dragged_id) {
                for id in selected_hosts {
                    if let Some(host) = self.mesh.host_mut(id) {
                        host.x += world_delta.x;
                        host.y += world_delta.y;
                    }
                }
            } else if let Some(host) = self.mesh.host_mut(dragged_id) {
                host.x += world_delta.x;
                host.y += world_delta.y;
            }
        }

        if background.clicked()
            && clicked_host.is_none()
            && clicked_connection.is_none()
            && self.selection != Selection::None
        {
            self.selection = Selection::None;
        }

        if !overlay_open && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.selection = Selection::None;
        }

        painter.text(
            canvas.left_bottom() + Vec2::new(12.0, -10.0),
            Align2::LEFT_BOTTOM,
            tr!("ui.canvas.right_click_empty_space_to_add"),
            FontId::proportional(11.0),
            colors.hint,
        );
    }

    fn maybe_save_settings(&mut self, ctx: &egui::Context) {
        let Some(since) = self.settings_dirty_since else {
            return;
        };

        let pointer_down = ctx.input(|input| input.pointer.any_down());
        if !pointer_down && since.elapsed() >= Duration::from_millis(300) {
            self.save_settings();
        } else {
            ctx.request_repaint_after(Duration::from_millis(150));
        }
    }
}

impl eframe::App for MeshWizardApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if ctx.input(|input| input.viewport().close_requested()) && self.has_unsaved_changes() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.close_prompt = true;
            self.close_prompt_project = false;
        }

        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.add_space(2.0);
            self.top_bar(ui);
            ui.add_space(2.0);
        });

        egui::Panel::bottom("status_bar").show(ui, |ui| {
            self.status_bar(ui);
        });

        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(380.0)
            .min_size(300.0)
            .max_size(560.0)
            .show(ui, |ui| {
                self.side_panel(ui);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                self.canvas(ui);
            });

        let overlay_open = self.show_settings
            || self.startup_prompt
            || self.close_prompt
            || self.delete_prompt.is_some()
            || ctx.memory(|memory| memory.top_modal_layer().is_some())
            || ctx.any_popup_open();
        if !overlay_open
            && !ctx.egui_wants_keyboard_input()
            && ctx.input(|input| input.key_pressed(egui::Key::Delete))
        {
            let selected_hosts = self.selection.hosts().to_vec();
            if !selected_hosts.is_empty() {
                self.request_delete_hosts(selected_hosts);
            }
        }

        self.startup_modal(&ctx);
        self.settings_modal(&ctx);
        self.delete_host_modal(&ctx);
        self.rotate_secrets_modal(&ctx);
        self.close_prompt_modal(&ctx);

        if self.mesh.defaults.preshared_keys {
            self.mesh.sync_preshared_keys();
        }

        self.maybe_save_settings(&ctx);
    }
}

impl Drop for MeshWizardApp {
    fn drop(&mut self) {
        if self.settings_dirty_since.is_some() {
            self.save_settings();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_layout_returns_centered_finite_positions() {
        for shape in MeshShape::ALL {
            let positions = layout_positions(shape, 6, 270.0, 148.0);
            assert_eq!(positions.len(), 6, "{shape:?}");
            assert!(
                positions
                    .iter()
                    .all(|(x, y)| x.is_finite() && y.is_finite()),
                "{shape:?}"
            );
            let center_x = positions.iter().map(|(x, _)| x).sum::<f32>() / 6.0;
            let center_y = positions.iter().map(|(_, y)| y).sum::<f32>() / 6.0;
            assert!(center_x.abs() < 1.0, "{shape:?} center x {center_x}");
            assert!(center_y.abs() < 1.0, "{shape:?} center y {center_y}");
        }
    }

    #[test]
    fn settings_enums_round_trip_through_storage() {
        for mode in [ThemeMode::System, ThemeMode::Dark, ThemeMode::Light] {
            assert_eq!(ThemeMode::from_storage(&mode.as_storage()), Some(mode));
        }
        for shape in MeshShape::ALL {
            assert_eq!(MeshShape::from_storage(&shape.as_storage()), Some(shape));
        }
    }
}
