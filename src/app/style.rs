//! Shared egui styling and semantic text colors.

use super::*;
use eframe::egui::{self, Color32, CornerRadius, Ui, Vec2};

pub(super) fn dim_text(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        TEXT_DIM
    } else {
        Color32::from_rgb(91, 107, 128)
    }
}

pub(super) fn accent_text(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        ACCENT
    } else {
        Color32::from_rgb(18, 112, 143)
    }
}

pub(super) fn warm_text(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        ACCENT_WARM
    } else {
        Color32::from_rgb(175, 103, 18)
    }
}

pub(super) fn setup_style(ctx: &egui::Context) {
    let mut dark = (*ctx.style_of(egui::Theme::Dark)).clone();
    dark.spacing.item_spacing = Vec2::new(8.0, 6.0);
    dark.spacing.button_padding = Vec2::new(8.0, 4.0);

    let mut dark_visuals = egui::Visuals::dark();
    dark_visuals.panel_fill = Color32::from_rgb(18, 23, 34);
    dark_visuals.window_fill = Color32::from_rgb(22, 28, 41);
    dark_visuals.extreme_bg_color = Color32::from_rgb(11, 15, 23);
    dark_visuals.faint_bg_color = Color32::from_rgb(25, 32, 46);
    dark_visuals.selection.bg_fill = Color32::from_rgb(44, 92, 118);
    dark_visuals.hyperlink_color = ACCENT;
    dark_visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    dark_visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    dark_visuals.widgets.active.corner_radius = CornerRadius::same(6);
    dark_visuals.widgets.inactive.bg_fill = Color32::from_rgb(30, 39, 55);
    dark_visuals.widgets.hovered.bg_fill = Color32::from_rgb(42, 55, 76);
    dark_visuals.widgets.active.bg_fill = Color32::from_rgb(53, 70, 96);
    dark.visuals = dark_visuals;

    let mut light = (*ctx.style_of(egui::Theme::Light)).clone();
    light.spacing.item_spacing = Vec2::new(8.0, 6.0);
    light.spacing.button_padding = Vec2::new(8.0, 4.0);

    let mut light_visuals = egui::Visuals::light();
    light_visuals.panel_fill = Color32::from_rgb(246, 248, 252);
    light_visuals.window_fill = Color32::from_rgb(252, 253, 255);
    light_visuals.extreme_bg_color = Color32::WHITE;
    light_visuals.faint_bg_color = Color32::from_rgb(238, 242, 248);
    light_visuals.selection.bg_fill = Color32::from_rgb(168, 216, 238);
    light_visuals.hyperlink_color = Color32::from_rgb(16, 110, 140);
    light_visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    light_visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    light_visuals.widgets.active.corner_radius = CornerRadius::same(6);
    light_visuals.widgets.inactive.bg_fill = Color32::from_rgb(238, 242, 248);
    light_visuals.widgets.hovered.bg_fill = Color32::from_rgb(224, 235, 246);
    light_visuals.widgets.active.bg_fill = Color32::from_rgb(208, 226, 242);
    light.visuals = light_visuals;

    ctx.set_style_of(egui::Theme::Dark, dark);
    ctx.set_style_of(egui::Theme::Light, light);
}
