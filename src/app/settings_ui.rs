//! About page shown inside the Settings modal.

use super::style::dim_text;
use super::*;
use crate::i18n::tr;
use crate::model::MESH_CONFIG_VERSION;
use eframe::egui::{RichText, Ui};

pub(super) fn settings_about(ui: &mut Ui) {
    ui.heading(tr!("ui.settings_modal.about"));
    ui.label(RichText::new(tr!("ui.top_bar.wireguard_mesh_wizard")).size(20.0));
    ui.label(
        RichText::new(tr!(
            "ui.settings_about.version_version",
            version = env!("CARGO_PKG_VERSION")
        ))
        .small()
        .color(dim_text(ui)),
    );
    ui.add_space(10.0);
    ui.label(tr!("ui.settings_about.a_local_first_desktop_editor_for"));

    ui.add_space(14.0);
    egui::Grid::new("about_grid")
        .num_columns(2)
        .spacing([18.0, 8.0])
        .show(ui, |ui| {
            ui.label(tr!("ui.settings_about.license"));
            ui.label("GPL-3.0-or-later");
            ui.end_row();

            ui.label(tr!("ui.settings_about.repository"));
            ui.hyperlink_to(env!("CARGO_PKG_REPOSITORY"), env!("CARGO_PKG_REPOSITORY"));
            ui.end_row();

            ui.label(tr!("ui.settings_about.author"));
            ui.label(env!("CARGO_PKG_AUTHORS"));
            ui.end_row();

            ui.label(tr!("ui.settings_about.config_format"));
            ui.label(tr!(
                "ui.settings_about.config_format_value",
                version = MESH_CONFIG_VERSION
            ));
            ui.end_row();
        });
}
