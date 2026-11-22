//! Small form widgets shared by the host and mesh inspectors.

use super::*;
use crate::i18n::tr;
use eframe::egui::{self, TextEdit, Ui};

pub(super) fn inherit_hint(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        tr!("ui.inherit_hint.inherit_value", value = fallback)
    } else {
        tr!("ui.inherit_hint.inherits_value", value = value)
    }
}

pub(super) fn optional_u16_cell(
    ui: &mut Ui,
    value: &mut Option<u16>,
    inherited: Option<u16>,
    fallback: u16,
    range: std::ops::RangeInclusive<u16>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let mut overridden = value.is_some();
        if ui
            .checkbox(&mut overridden, "")
            .on_hover_text(tr!("ui.optional_u16_cell.override_the_mesh_default"))
            .changed()
        {
            *value = if overridden {
                Some(inherited.unwrap_or(fallback))
            } else {
                None
            };
            changed = true;
        }
        match value {
            Some(current) => {
                changed |= ui
                    .add(egui::DragValue::new(current).range(range).speed(1.0))
                    .changed();
            }
            None => {
                let text = match inherited {
                    Some(inherited) => tr!("ui.inherit_hint.inherit_value", value = inherited),
                    None => tr!("ui.optional_u16_cell.not_set"),
                };
                ui.weak(text);
            }
        }
    });
    changed
}

pub(super) fn optional_u32_cell(
    ui: &mut Ui,
    value: &mut Option<u32>,
    inherited: Option<u32>,
    fallback: u32,
    range: std::ops::RangeInclusive<u32>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let mut overridden = value.is_some();
        if ui
            .checkbox(&mut overridden, "")
            .on_hover_text(tr!("ui.optional_u16_cell.override_the_mesh_default"))
            .changed()
        {
            *value = if overridden {
                Some(inherited.unwrap_or(fallback))
            } else {
                None
            };
            changed = true;
        }
        match value {
            Some(current) => {
                changed |= ui
                    .add(egui::DragValue::new(current).range(range).speed(1.0))
                    .changed();
            }
            None => {
                let text = match inherited {
                    Some(inherited) => tr!("ui.inherit_hint.inherit_value", value = inherited),
                    None => tr!("ui.optional_u16_cell.not_set"),
                };
                ui.weak(text);
            }
        }
    });
    changed
}

pub(super) fn tri_state_bool_cell(
    ui: &mut Ui,
    value: &mut Option<bool>,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
) -> bool {
    let mut changed = false;
    let selected = match value {
        Some(true) => tr!("ui.connection_inspector.enabled"),
        Some(false) => tr!("ui.tri_state_bool_cell.disabled"),
        None => tr!("ui.tri_state_bool_cell.inherit"),
    };
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(selected)
        .width(130.0)
        .show_ui(ui, |ui| {
            changed |= ui
                .selectable_value(value, None, tr!("ui.tri_state_bool_cell.inherit"))
                .changed();
            changed |= ui
                .selectable_value(value, Some(true), tr!("ui.connection_inspector.enabled"))
                .changed();
            changed |= ui
                .selectable_value(value, Some(false), tr!("ui.tri_state_bool_cell.disabled"))
                .changed();
        });
    changed
}

pub(super) fn hook_row(
    ui: &mut Ui,
    label: String,
    value: &mut String,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
) {
    ui.label(label);
    ui.add(
        TextEdit::singleline(value)
            .id_source(id_salt)
            .desired_width(FIELD_WIDTH)
            .font(egui::TextStyle::Monospace)
            .hint_text(tr!("ui.hook_row.shell_command")),
    );
    ui.end_row();
}
