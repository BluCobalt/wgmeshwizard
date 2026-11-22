//! Inspector, toolbar, settings modal and dialog UI.

use super::settings_ui::settings_about;
use super::style::{accent_text, dim_text, warm_text};
use super::widgets::{
    hook_row, inherit_hint, optional_u16_cell, optional_u32_cell, tri_state_bool_cell,
};
use super::*;
use crate::i18n::tr;

impl MeshWizardApp {
    pub(super) fn top_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading(
                RichText::new(tr!("ui.top_bar.wireguard_mesh_wizard")).color(accent_text(ui)),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .button(tr!("ui.top_bar.settings"))
                    .on_hover_text(tr!("ui.top_bar.wizard_settings_and_about"))
                    .clicked()
                {
                    self.show_settings = true;
                    self.settings_just_opened = true;
                }
                if self.has_unsaved_changes() {
                    ui.label(
                        RichText::new(tr!("ui.top_bar.unsaved_changes"))
                            .color(warm_text(ui))
                            .small(),
                    );
                } else {
                    ui.label(
                        RichText::new(tr!("ui.top_bar.saved"))
                            .color(dim_text(ui))
                            .small(),
                    );
                }
            });
        });

        ui.horizontal_wrapped(|ui| {
            if ui
                .button(tr!("ui.top_bar.add_host"))
                .on_hover_text(tr!("ui.top_bar.add_a_host_near_the_centre"))
                .clicked()
            {
                self.add_host_near(None);
            }
            if ui
                .button(tr!("ui.top_bar.fit_view"))
                .on_hover_text(tr!("ui.top_bar.fit_every_host_into_the_canvas"))
                .clicked()
            {
                self.fit_pending = true;
            }
            if ui
                .button(tr!("ui.top_bar.arrange"))
                .on_hover_text(tr!(
                    "ui.top_bar.sort_hosts_by_name_and_arrange",
                    shape = self.mesh_shape.label()
                ))
                .clicked()
            {
                self.arrange_mesh();
            }
            if ui
                .button(tr!("ui.top_bar.export_configs"))
                .on_hover_text(tr!("ui.top_bar.write_one_wg_quick_conf_per"))
                .clicked()
            {
                self.export_configs();
            }

            ui.separator();
            ui.label(tr!("ui.top_bar.project"));
            ui.label(RichText::new(self.project_name()).strong())
                .on_hover_text(self.project_path.display().to_string());

            ui.menu_button(tr!("ui.top_bar.save"), |ui| {
                if ui.button(tr!("ui.top_bar.save_current")).clicked() {
                    self.request_save();
                    ui.close();
                }
                if ui.button(tr!("ui.top_bar.save_as_archive")).clicked() {
                    self.pick_save_archive();
                    ui.close();
                }
                if ui.button(tr!("ui.top_bar.save_as_folder")).clicked() {
                    self.pick_save_folder();
                    ui.close();
                }
            });
            if ui.button(tr!("ui.top_bar.close")).clicked() {
                self.request_close_project();
            }
        });
    }

    pub(super) fn status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            if self.status.is_empty() {
                ui.label(RichText::new(tr!("ui.status_bar.ready")).color(dim_text(ui)));
            } else {
                let color = if self.status_is_error {
                    DANGER
                } else {
                    dim_text(ui)
                };
                ui.label(RichText::new(&self.status).color(color));
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(tr!(
                        "ui.status_bar.zoom_percent",
                        percent = format!("{:.0}", self.camera.zoom * 100.0)
                    ))
                    .small(),
                );
                ui.separator();
                ui.label(
                    RichText::new(tr!(
                        "ui.status_bar.hosts_hosts_connections_connections",
                        hosts = self.mesh.hosts.len(),
                        connections = self.mesh.connections.len()
                    ))
                    .small(),
                );
                ui.separator();
                ui.label(
                    RichText::new(tr!("ui.status_bar.scroll_to_zoom_drag_background_to")).small(),
                );
            });
        });
    }

    fn project_overview(&self, ui: &mut Ui) {
        egui::CollapsingHeader::new(tr!("ui.project_overview.project"))
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new("project_overview_grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(tr!("ui.project_overview.location"));
                        ui.vertical(|ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(self.project_path.display().to_string())
                                        .monospace()
                                        .small(),
                                )
                                .selectable(true)
                                .wrap(),
                            );
                            if ui.small_button(tr!("ui.project_overview.copy")).clicked() {
                                ui.ctx().copy_text(self.project_path.display().to_string());
                            }
                        });
                        ui.end_row();

                        ui.label(tr!("ui.project_overview.format"));
                        ui.label(self.project_kind());
                        ui.end_row();

                        ui.label(tr!("ui.project_overview.mesh"));
                        ui.label(&self.mesh.defaults.name);
                        ui.end_row();

                        ui.label(tr!("ui.project_overview.hosts"));
                        ui.label(self.mesh.hosts.len().to_string());
                        ui.end_row();

                        let enabled = self
                            .mesh
                            .connections
                            .iter()
                            .filter(|connection| connection.enabled)
                            .count();
                        ui.label(tr!("ui.project_overview.links"));
                        ui.label(tr!(
                            "ui.project_overview.links_value",
                            total = self.mesh.connections.len(),
                            enabled = enabled
                        ));
                        ui.end_row();

                        let mut psk_pairs = std::collections::HashSet::new();
                        for connection in &self.mesh.connections {
                            if !connection.preshared_key.trim().is_empty() {
                                psk_pairs.insert((
                                    connection.from.min(connection.to),
                                    connection.from.max(connection.to),
                                ));
                            }
                        }
                        ui.label(tr!("ui.project_overview.psk_pairs"));
                        ui.label(psk_pairs.len().to_string());
                        ui.end_row();

                        ui.label(tr!("ui.project_overview.output"));
                        ui.add(
                            egui::Label::new(
                                RichText::new(
                                    storage::default_output_dir(&self.project_path)
                                        .display()
                                        .to_string(),
                                )
                                .monospace()
                                .small(),
                            )
                            .selectable(true)
                            .wrap(),
                        );
                        ui.end_row();
                    });
            });
    }

    pub(super) fn side_panel(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(tr!("ui.side_panel.inspector"));
                    if self.selection != Selection::None
                        && ui.small_button(tr!("ui.side_panel.deselect")).clicked()
                    {
                        self.selection = Selection::None;
                    }
                });
                ui.add_space(4.0);
                self.project_overview(ui);
                ui.separator();

                match self.selection.clone() {
                    Selection::None => {
                        ui.label(
                            RichText::new(tr!("ui.side_panel.click_a_host_to_edit_it"))
                                .color(dim_text(ui)),
                        );
                        ui.add_space(8.0);
                    }
                    Selection::Hosts(ids) => match ids.as_slice() {
                        [id] => {
                            if self.mesh.host(*id).is_some() {
                                self.host_inspector(ui, *id);
                            } else {
                                self.selection = Selection::None;
                            }
                            ui.add_space(10.0);
                            ui.separator();
                        }
                        [] => {
                            self.selection = Selection::None;
                        }
                        _ => {
                            ui.heading(tr!("ui.side_panel.hosts_selected", count = ids.len()));
                            ui.add_space(4.0);
                            for id in ids {
                                if let Some(host) = self.mesh.host(id) {
                                    ui.label(&host.name);
                                }
                            }
                            ui.add_space(10.0);
                            ui.separator();
                        }
                    },
                    Selection::Connection { from, to } => {
                        if self.mesh.connection(from, to).is_some() {
                            self.connection_inspector(ui, from, to);
                        } else {
                            self.selection = Selection::None;
                        }
                        ui.add_space(10.0);
                        ui.separator();
                    }
                }

                let mesh_defaults_open = self.selection == Selection::None;
                egui::CollapsingHeader::new(tr!("ui.side_panel.mesh_defaults"))
                    .default_open(mesh_defaults_open)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(tr!(
                                "ui.side_panel.empty_host_fields_inherit_these_values"
                            ))
                            .small()
                            .color(dim_text(ui)),
                        );
                        ui.add_space(4.0);
                        self.mesh_defaults_inspector(ui);
                    });
            });
    }

    pub(super) fn mesh_defaults_inspector(&mut self, ui: &mut Ui) {
        let next_address = self.mesh.suggest_address();
        let has_hosts = !self.mesh.hosts.is_empty();
        let mut psk_toggled = None;
        let mut clear_psks = false;
        let mut rotate_requested = false;

        {
            let defaults = &mut self.mesh.defaults;

            egui::Grid::new("mesh_defaults_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label(tr!("ui.mesh_defaults_inspector.mesh_name"));
                    ui.add(
                        TextEdit::singleline(&mut defaults.name)
                            .desired_width(FIELD_WIDTH)
                            .hint_text(tr!("ui.mesh_defaults_inspector.my_mesh")),
                    );
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.address_prefix"))
                        .on_hover_text(tr!(
                            "ui.mesh_defaults_inspector.new_hosts_get_the_next_free"
                        ));
                    ui.add(
                        TextEdit::singleline(&mut defaults.address_prefix)
                            .desired_width(FIELD_WIDTH)
                            .hint_text(tr!("ui.mesh_defaults_inspector.10_100_0_0_24")),
                    );
                    ui.end_row();

                    ui.label("");
                    ui.label(
                        RichText::new(tr!(
                            "ui.mesh_defaults_inspector.next_suggested_address_address",
                            address = next_address
                        ))
                        .small()
                        .color(dim_text(ui)),
                    );
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.listen_port"));
                    optional_u16_cell(
                        ui,
                        &mut defaults.listen_port,
                        None,
                        DEFAULT_LISTEN_PORT,
                        1..=65535,
                    );
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.persistent_keepalive"));
                    optional_u16_cell(ui, &mut defaults.persistent_keepalive, None, 25, 1..=65535);
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.dns"));
                    ui.add(
                        TextEdit::singleline(&mut defaults.dns)
                            .desired_width(FIELD_WIDTH)
                            .hint_text(tr!("ui.mesh_defaults_inspector.1_1_1_1_8_8")),
                    );
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.mtu"));
                    optional_u32_cell(ui, &mut defaults.mtu, None, DEFAULT_MTU, 576..=9200);
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.fwmark"));
                    ui.add(
                        TextEdit::singleline(&mut defaults.fwmark)
                            .desired_width(FIELD_WIDTH)
                            .hint_text(tr!("ui.mesh_defaults_inspector.0xca6c")),
                    );
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.table"));
                    ui.add(
                        TextEdit::singleline(&mut defaults.table)
                            .desired_width(FIELD_WIDTH)
                            .hint_text(tr!("ui.mesh_defaults_inspector.auto_off_number")),
                    );
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.allowedips"));
                    ui.add(
                        TextEdit::singleline(&mut defaults.allowed_ips)
                            .desired_width(FIELD_WIDTH)
                            .hint_text(tr!(
                                "ui.mesh_defaults_inspector.extra_routes_for_every_peer"
                            )),
                    );
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.preshared_keys"))
                        .on_hover_text(tr!(
                            "ui.mesh_defaults_inspector.generate_and_maintain_one_key_per"
                        ));
                    if ui
                        .checkbox(
                            &mut defaults.preshared_keys,
                            tr!("ui.mesh_defaults_inspector.enable_mesh_wide"),
                        )
                        .changed()
                    {
                        psk_toggled = Some(defaults.preshared_keys);
                    }
                    ui.end_row();

                    ui.label("");
                    ui.label(
                        RichText::new(tr!(
                            "ui.mesh_defaults_inspector.both_directions_of_each_link_share"
                        ))
                        .small()
                        .color(dim_text(ui)),
                    );
                    ui.end_row();

                    ui.label("");
                    if ui
                        .small_button(tr!("ui.mesh_defaults_inspector.clear_all_pair_psks"))
                        .on_hover_text(tr!(
                            "ui.mesh_defaults_inspector.disable_mesh_wide_psks_and_remove"
                        ))
                        .clicked()
                    {
                        clear_psks = true;
                    }
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.secrets"));
                    if ui
                        .add_enabled(
                            has_hosts,
                            egui::Button::new(tr!("ui.mesh_defaults_inspector.rotate_all_secrets")),
                        )
                        .on_hover_text(tr!(
                            "ui.mesh_defaults_inspector.rotate_every_host_private_key_and"
                        ))
                        .clicked()
                    {
                        rotate_requested = true;
                    }
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.saveconfig"));
                    tri_state_bool_cell(ui, &mut defaults.save_config, "mesh_save_config");
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.hosts_file"))
                        .on_hover_text(tr!(
                            "ui.mesh_defaults_inspector.generate_an_etc_hosts_compatible_file"
                        ));
                    ui.checkbox(
                        &mut defaults.generate_hosts_file,
                        tr!("ui.mesh_defaults_inspector.generate_on_export"),
                    );
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.search_domain"))
                        .on_hover_text(tr!(
                            "ui.mesh_defaults_inspector.append_the_search_domain_to_each"
                        ));
                    ui.add(
                        TextEdit::singleline(&mut defaults.search_domain)
                            .desired_width(FIELD_WIDTH)
                            .hint_text(tr!("ui.mesh_defaults_inspector.example_tld")),
                    );
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.deployment"))
                        .on_hover_text(tr!(
                            "ui.mesh_defaults_inspector.write_a_static_deploy_sh_script"
                        ));
                    ui.checkbox(
                        &mut defaults.enable_deployment,
                        tr!("ui.mesh_defaults_inspector.generate_deploy_sh_on_export"),
                    );
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.ssh_user"))
                        .on_hover_text(tr!(
                            "ui.mesh_defaults_inspector.default_ssh_user_for_deploy"
                        ));
                    ui.add(
                        TextEdit::singleline(&mut defaults.ssh_user)
                            .desired_width(FIELD_WIDTH)
                            .hint_text(tr!("ui.mesh_defaults_inspector.deploy")),
                    );
                    ui.end_row();

                    ui.label(tr!("ui.mesh_defaults_inspector.ssh_port"))
                        .on_hover_text(tr!(
                            "ui.mesh_defaults_inspector.default_ssh_port_for_deploy"
                        ));
                    optional_u16_cell(ui, &mut defaults.ssh_port, None, 22, 1..=65535);
                    ui.end_row();
                });

            ui.add_space(8.0);
            egui::CollapsingHeader::new(tr!("ui.mesh_defaults_inspector.hooks"))
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("mesh_hooks_grid")
                        .num_columns(2)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            hook_row(
                                ui,
                                tr!("ui.mesh_defaults_inspector.preup"),
                                &mut defaults.pre_up,
                                "mesh_pre_up",
                            );
                            hook_row(
                                ui,
                                tr!("ui.mesh_defaults_inspector.postup"),
                                &mut defaults.post_up,
                                "mesh_post_up",
                            );
                            hook_row(
                                ui,
                                tr!("ui.mesh_defaults_inspector.predown"),
                                &mut defaults.pre_down,
                                "mesh_pre_down",
                            );
                            hook_row(
                                ui,
                                tr!("ui.mesh_defaults_inspector.postdown"),
                                &mut defaults.post_down,
                                "mesh_post_down",
                            );
                        });
                });
        }

        if rotate_requested {
            self.request_rotate_all_secrets();
        }

        if clear_psks {
            self.mesh.defaults.preshared_keys = false;
            for connection in &mut self.mesh.connections {
                connection.preshared_key.clear();
            }
            self.set_status(
                tr!("ui.mesh_defaults_inspector.disabled_mesh_wide_preshared_keys_and"),
                false,
            );
        } else if let Some(enabled) = psk_toggled {
            if enabled {
                self.mesh.sync_preshared_keys();
                self.set_status(
                    tr!("ui.mesh_defaults_inspector.mesh_wide_preshared_keys_enabled_existing"),
                    false,
                );
            } else {
                self.set_status(
                    tr!("ui.mesh_defaults_inspector.mesh_wide_preshared_key_generation_disabled"),
                    false,
                );
            }
        }
    }

    pub(super) fn host_inspector(&mut self, ui: &mut Ui, id: u64) {
        let suggested_address = self.mesh.suggest_address();
        let defaults = self.mesh.defaults.clone();
        let host_pos = self.mesh.host(id).map(|host| Vec2::new(host.x, host.y));
        let mut reveal = self.show_private_key;
        let mut show_config = self.show_config;
        let mut delete_requested = false;
        let mut focus = false;

        {
            let Some(host) = self.mesh.host_mut(id) else {
                return;
            };

            ui.heading(&host.name);
            ui.horizontal(|ui| {
                if ui.button(tr!("ui.host_inspector.focus")).clicked() {
                    focus = true;
                }
                if ui
                    .button(RichText::new(tr!("ui.host_inspector.delete_host")).color(DANGER))
                    .clicked()
                {
                    delete_requested = true;
                }
            });

            ui.add_space(6.0);
            egui::CollapsingHeader::new(tr!("ui.host_inspector.identity"))
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new(("host_identity_grid", id))
                        .num_columns(2)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(tr!("ui.host_inspector.name"));
                            ui.add(
                                TextEdit::singleline(&mut host.name)
                                    .desired_width(FIELD_WIDTH)
                                    .hint_text(tr!("ui.host_inspector.host_name")),
                            );
                            ui.end_row();

                            ui.label(tr!("ui.host_inspector.address"));
                            ui.horizontal(|ui| {
                                ui.add(
                                    TextEdit::singleline(&mut host.address)
                                        .desired_width(150.0)
                                        .hint_text(tr!("ui.host_inspector.10_100_0_1_32")),
                                );
                                if ui
                                    .small_button(tr!("ui.host_inspector.auto"))
                                    .on_hover_text(tr!(
                                        "ui.host_inspector.use_address",
                                        address = suggested_address
                                    ))
                                    .clicked()
                                {
                                    host.address = suggested_address.clone();
                                }
                            });
                            ui.end_row();
                        });
                });

            egui::CollapsingHeader::new(tr!("ui.host_inspector.connectivity"))
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new(("host_connectivity_grid", id))
                        .num_columns(2)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(tr!("ui.host_inspector.endpoint"))
                                .on_hover_text(tr!(
                                    "ui.host_inspector.public_address_other_hosts_use_by"
                                ));
                            ui.add(
                                TextEdit::singleline(&mut host.endpoint)
                                    .desired_width(FIELD_WIDTH)
                                    .hint_text(tr!("ui.host_inspector.vpn_example_com_port")),
                            );
                            ui.end_row();

                            ui.label(tr!("ui.host_inspector.ssh_deploy_target"))
                                .on_hover_text(tr!(
                                    "ui.host_inspector.override_the_ssh_destination_used_by"
                                ));
                            ui.add(
                                TextEdit::singleline(&mut host.ssh_target)
                                    .desired_width(FIELD_WIDTH)
                                    .hint_text(tr!("ui.host_inspector.user_host_port_optional")),
                            );
                            ui.end_row();

                            ui.label(tr!("ui.mesh_defaults_inspector.listen_port"));
                            optional_u16_cell(
                                ui,
                                &mut host.listen_port,
                                defaults.listen_port,
                                DEFAULT_LISTEN_PORT,
                                1..=65535,
                            );
                            ui.end_row();

                            ui.label(tr!("ui.mesh_defaults_inspector.allowedips"))
                                .on_hover_text(tr!(
                                    "ui.host_inspector.extra_routes_advertised_to_every_other"
                                ));
                            ui.add(
                                TextEdit::singleline(&mut host.allowed_ips)
                                    .desired_width(FIELD_WIDTH)
                                    .hint_text(tr!("ui.host_inspector.192_168_1_0_24")),
                            );
                            ui.end_row();

                            ui.label(tr!("ui.mesh_defaults_inspector.persistent_keepalive"));
                            optional_u16_cell(
                                ui,
                                &mut host.persistent_keepalive,
                                defaults.persistent_keepalive,
                                25,
                                1..=65535,
                            );
                            ui.end_row();
                        });
                });

            egui::CollapsingHeader::new(tr!("ui.host_inspector.interface_options"))
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new(("host_interface_grid", id))
                        .num_columns(2)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(tr!("ui.mesh_defaults_inspector.dns"));
                            ui.add(
                                TextEdit::singleline(&mut host.dns)
                                    .desired_width(FIELD_WIDTH)
                                    .hint_text(inherit_hint(&defaults.dns, "1.1.1.1")),
                            );
                            ui.end_row();

                            ui.label(tr!("ui.mesh_defaults_inspector.mtu"));
                            optional_u32_cell(
                                ui,
                                &mut host.mtu,
                                defaults.mtu,
                                DEFAULT_MTU,
                                576..=9200,
                            );
                            ui.end_row();

                            ui.label(tr!("ui.mesh_defaults_inspector.fwmark"));
                            ui.add(
                                TextEdit::singleline(&mut host.fwmark)
                                    .desired_width(FIELD_WIDTH)
                                    .hint_text(inherit_hint(&defaults.fwmark, "0xca6c")),
                            );
                            ui.end_row();

                            ui.label(tr!("ui.mesh_defaults_inspector.table"));
                            ui.add(
                                TextEdit::singleline(&mut host.table)
                                    .desired_width(FIELD_WIDTH)
                                    .hint_text(inherit_hint(&defaults.table, "auto / off")),
                            );
                            ui.end_row();

                            ui.label(tr!("ui.mesh_defaults_inspector.saveconfig"));
                            tri_state_bool_cell(
                                ui,
                                &mut host.save_config,
                                ("host_save_config", id),
                            );
                            ui.end_row();
                        });
                });

            egui::CollapsingHeader::new(tr!("ui.mesh_defaults_inspector.hooks"))
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new(("host_hooks_grid", id))
                        .num_columns(2)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            hook_row(
                                ui,
                                tr!("ui.mesh_defaults_inspector.preup"),
                                &mut host.pre_up,
                                ("host_pre_up", id),
                            );
                            hook_row(
                                ui,
                                tr!("ui.mesh_defaults_inspector.postup"),
                                &mut host.post_up,
                                ("host_post_up", id),
                            );
                            hook_row(
                                ui,
                                tr!("ui.mesh_defaults_inspector.predown"),
                                &mut host.pre_down,
                                ("host_pre_down", id),
                            );
                            hook_row(
                                ui,
                                tr!("ui.mesh_defaults_inspector.postdown"),
                                &mut host.post_down,
                                ("host_post_down", id),
                            );
                        });
                });

            egui::CollapsingHeader::new(tr!("ui.host_inspector.keys"))
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(
                            TextEdit::singleline(&mut host.private_key)
                                .password(!reveal)
                                .desired_width(225.0)
                                .font(egui::TextStyle::Monospace)
                                .hint_text(tr!("ui.host_inspector.base64_x25519_private_key")),
                        );
                        if ui.small_button(tr!("ui.host_inspector.copy")).clicked() {
                            ui.ctx().copy_text(host.private_key.clone());
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui
                            .button(tr!("ui.host_inspector.generate_new_key"))
                            .on_hover_text(tr!(
                                "ui.host_inspector.replace_this_host_s_private_public"
                            ))
                            .clicked()
                        {
                            host.private_key = crypto::generate_private_key();
                            reveal = true;
                        }
                        ui.checkbox(&mut reveal, tr!("ui.host_inspector.show_key"));
                    });

                    if host.private_key.trim().is_empty() {
                        ui.colored_label(DANGER, tr!("ui.host_inspector.no_private_key_set"));
                    } else {
                        match crypto::public_key(&host.private_key) {
                            Ok(public_key) => {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(tr!("ui.host_inspector.public_key"))
                                        .small()
                                        .color(dim_text(ui)),
                                );
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(public_key.clone()).monospace().small(),
                                        )
                                        .selectable(true),
                                    );
                                    if ui.small_button(tr!("ui.host_inspector.copy")).clicked() {
                                        ui.ctx().copy_text(public_key);
                                    }
                                });
                            }
                            Err(error) => {
                                ui.colored_label(DANGER, error);
                            }
                        }
                    }
                });

            ui.add_space(8.0);
            ui.checkbox(
                &mut show_config,
                tr!("ui.host_inspector.preview_generated_config"),
            );
        }

        self.show_private_key = reveal;
        self.show_config = show_config;

        if focus && let Some(position) = host_pos {
            self.camera.pan = position;
            if self.camera.zoom < 0.8 {
                self.camera.zoom = 1.0;
            }
        }
        if delete_requested {
            self.request_delete_host(id);
            return;
        }
        if self.show_config {
            ui.add_space(6.0);
            self.config_preview(ui, id);
        }
    }

    pub(super) fn config_preview(&mut self, ui: &mut Ui, id: u64) {
        match wgconfig::generate_host_config(&self.mesh, id) {
            Ok(config) => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr!("ui.config_preview.generated_config")).strong());
                    if ui.small_button(tr!("ui.host_inspector.copy")).clicked() {
                        ui.ctx().copy_text(config.clone());
                    }
                });
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(RichText::new(config).monospace().small())
                                .selectable(true)
                                .wrap(),
                        );
                    });
            }
            Err(error) => {
                ui.colored_label(
                    DANGER,
                    tr!(
                        "ui.config_preview.cannot_generate_config_error",
                        error = error
                    ),
                );
            }
        }
    }

    pub(super) fn connection_inspector(&mut self, ui: &mut Ui, from: u64, to: u64) {
        let Some(from_host) = self.mesh.host(from) else {
            self.selection = Selection::None;
            return;
        };
        let Some(to_host) = self.mesh.host(to) else {
            self.selection = Selection::None;
            return;
        };

        let from_name = from_host.name.clone();
        let to_name = to_host.name.clone();
        let automatic = self.mesh.default_endpoint_for(to);
        let resolved = self.mesh.resolved_endpoint(from, to);
        let mesh_wide_psks = self.mesh.defaults.preshared_keys;
        let mut reveal = self.show_private_key;
        let mut select_source = false;
        let mut select_target = false;
        let mut swap = false;

        {
            let Some(connection) = self.mesh.connection_mut(from, to) else {
                return;
            };

            ui.horizontal(|ui| {
                ui.heading(tr!(
                    "ui.connection_inspector.from_to",
                    from = from_name,
                    to = to_name
                ));
            });
            ui.label(
                RichText::new(tr!(
                    "ui.connection_inspector.endpoint_used_by_the_source_host"
                ))
                .small()
                .color(dim_text(ui)),
            );

            ui.add_space(6.0);
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr!("ui.connection_inspector.effective_endpoint"))
                            .color(dim_text(ui)),
                    );
                    match &resolved {
                        Some(endpoint) => {
                            ui.label(RichText::new(endpoint).monospace().color(accent_text(ui)));
                        }
                        None => {
                            ui.label(
                                RichText::new(tr!("ui.connection_inspector.none_no_endpoint_line"))
                                    .color(warm_text(ui)),
                            );
                        }
                    }
                });
                match &automatic {
                    Some(endpoint) => {
                        ui.label(
                            RichText::new(tr!(
                                "ui.connection_inspector.automatic_value_from_to_s_endpoint",
                                to = to_name,
                                endpoint = endpoint
                            ))
                            .small()
                            .color(dim_text(ui)),
                        );
                    }
                    None => {
                        ui.label(
                            RichText::new(tr!(
                                "ui.connection_inspector.to_has_no_default_endpoint_set",
                                to = to_name
                            ))
                            .small()
                            .color(dim_text(ui)),
                        );
                    }
                }
            });

            ui.add_space(6.0);
            egui::Grid::new(("connection_grid", from, to))
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label(tr!("ui.connection_inspector.enabled"));
                    ui.checkbox(
                        &mut connection.enabled,
                        tr!("ui.connection_inspector.include_in_generated_configs"),
                    );
                    ui.end_row();

                    ui.label(tr!("ui.connection_inspector.override"));
                    ui.add(
                        TextEdit::singleline(&mut connection.endpoint)
                            .desired_width(FIELD_WIDTH)
                            .hint_text(
                                automatic
                                    .as_deref()
                                    .map(|endpoint| {
                                        tr!(
                                            "ui.connection_inspector.automatic_endpoint",
                                            endpoint = endpoint
                                        )
                                    })
                                    .unwrap_or_else(|| {
                                        tr!("ui.connection_inspector.automatic_no_default")
                                    }),
                            ),
                    );
                    ui.end_row();

                    ui.label(tr!("ui.connection_inspector.presharedkey"));
                    ui.horizontal(|ui| {
                        ui.add(
                            TextEdit::singleline(&mut connection.preshared_key)
                                .password(!reveal)
                                .desired_width(150.0)
                                .font(egui::TextStyle::Monospace)
                                .hint_text(tr!("ui.connection_inspector.optional")),
                        );
                        if ui
                            .small_button(tr!("ui.connection_inspector.generate"))
                            .clicked()
                        {
                            connection.preshared_key = crypto::generate_preshared_key();
                            reveal = true;
                        }
                        if !mesh_wide_psks
                            && ui
                                .small_button(tr!("ui.connection_inspector.clear"))
                                .clicked()
                        {
                            connection.preshared_key.clear();
                        }
                    });
                    ui.end_row();

                    if mesh_wide_psks {
                        ui.label("");
                        ui.label(
                            RichText::new(tr!(
                                "ui.connection_inspector.mesh_wide_psks_are_enabled_this"
                            ))
                            .small()
                            .color(dim_text(ui)),
                        );
                        ui.end_row();
                    }

                    ui.label("");
                    ui.checkbox(&mut reveal, tr!("ui.connection_inspector.show_secret"));
                    ui.end_row();
                });

            if !connection.preshared_key.trim().is_empty()
                && !crypto::is_valid_key(&connection.preshared_key)
            {
                ui.colored_label(
                    DANGER,
                    tr!("ui.connection_inspector.presharedkey_is_not_a_valid_32"),
                );
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("ui.connection_inspector.use_automatic"))
                    .clicked()
                {
                    connection.endpoint.clear();
                }
                if ui
                    .button(tr!("ui.connection_inspector.omit_endpoint"))
                    .on_hover_text(tr!(
                        "ui.connection_inspector.write_no_endpoint_line_for_this"
                    ))
                    .clicked()
                {
                    connection.endpoint = "none".to_owned();
                }
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("ui.connection_inspector.edit_source_host"))
                    .clicked()
                {
                    select_source = true;
                }
                if ui
                    .button(tr!("ui.connection_inspector.edit_target_host"))
                    .clicked()
                {
                    select_target = true;
                }
                if ui
                    .button(tr!("ui.connection_inspector.swap_direction"))
                    .clicked()
                {
                    swap = true;
                }
            });
        }

        self.show_private_key = reveal;
        if mesh_wide_psks {
            let key = self
                .mesh
                .connection(from, to)
                .map(|connection| connection.preshared_key.clone())
                .unwrap_or_default();
            if !key.is_empty()
                && let Some(reverse) = self.mesh.connection_mut(to, from)
            {
                reverse.preshared_key = key;
            }
        }
        if select_source {
            self.selection = Selection::Hosts(vec![from]);
        } else if select_target {
            self.selection = Selection::Hosts(vec![to]);
        } else if swap {
            self.selection = Selection::Connection { from: to, to: from };
        }
    }

    pub(super) fn settings_modal(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }

        let mut close_clicked = false;
        let just_opened = std::mem::take(&mut self.settings_just_opened);
        let modal = egui::Modal::new(egui::Id::unique("wizard_settings_modal"))
            .backdrop_color(Color32::from_black_alpha(170))
            .show(ctx, |ui| {
                let available = ui.ctx().content_rect().size();
                ui.set_width((available.x - 80.0).clamp(360.0, 680.0));
                let content_height = (available.y - 190.0).clamp(180.0, 540.0);

                ui.horizontal(|ui| {
                    ui.heading(tr!("ui.top_bar.settings"));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button(tr!("ui.settings_modal.close")).clicked() {
                            close_clicked = true;
                        }
                    });
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.settings_tab,
                        SettingsTab::General,
                        tr!("ui.settings_modal.general"),
                    );
                    ui.selectable_value(
                        &mut self.settings_tab,
                        SettingsTab::About,
                        tr!("ui.settings_modal.about"),
                    );
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(content_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.settings_tab {
                        SettingsTab::General => self.settings_general(ui),
                        SettingsTab::About => settings_about(ui),
                    });
            });

        if close_clicked || (!just_opened && modal.should_close()) {
            self.show_settings = false;
        }
    }

    pub(super) fn settings_general(&mut self, ui: &mut Ui) {
        ui.heading(tr!("ui.settings_general.appearance"));

        ui.horizontal(|ui| {
            ui.label(tr!("ui.settings_general.language"));
            let previous = self.language.clone();
            let selected = if self.language == "system" {
                tr!("ui.label.system")
            } else {
                self.language.clone()
            };
            egui::ComboBox::from_id_salt("language")
                .selected_text(selected)
                .width(150.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.language,
                        "system".to_owned(),
                        tr!("ui.label.system"),
                    );
                    let mut languages = i18n::available_languages();
                    if languages.is_empty() {
                        languages.push("en".to_owned());
                    }
                    for language in languages {
                        let label = language.clone();
                        ui.selectable_value(&mut self.language, language, label);
                    }
                });
            if self.language != previous {
                i18n::set_language(&self.language);
                self.queue_settings_save();
                ui.ctx().request_repaint();
            }
        });
        ui.label(
            RichText::new(tr!(
                "ui.settings_general.system_follows_the_desktop_language_translations"
            ))
            .small()
            .color(dim_text(ui)),
        );

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(tr!("ui.settings_general.theme"));
            let previous = self.theme_mode;
            egui::ComboBox::from_id_salt("theme_mode")
                .selected_text(self.theme_mode.label())
                .width(150.0)
                .show_ui(ui, |ui| {
                    for mode in [ThemeMode::System, ThemeMode::Dark, ThemeMode::Light] {
                        ui.selectable_value(&mut self.theme_mode, mode, mode.label());
                    }
                });
            if self.theme_mode != previous {
                ui.ctx().set_theme(self.theme_mode.preference());
                self.queue_settings_save();
            }
        });
        ui.label(
            RichText::new(tr!(
                "ui.settings_general.system_follows_the_desktop_theme_dark"
            ))
            .small()
            .color(dim_text(ui)),
        );

        ui.add_space(12.0);
        ui.label(tr!("ui.settings_general.ui_scale"));
        ui.label(
            RichText::new(tr!(
                "ui.settings_general.scales_all_text_controls_and_spacing"
            ))
            .small()
            .color(dim_text(ui)),
        );

        ui.add_space(6.0);
        let mut scale = self.ui_scale;
        if ui
            .add(
                egui::Slider::new(&mut scale, 0.75..=2.0)
                    .step_by(0.05)
                    .fixed_decimals(2)
                    .suffix("×"),
            )
            .on_hover_text(tr!("ui.settings_general.scale_the_entire_wizard_ui"))
            .changed()
        {
            self.apply_ui_scale(ui, scale);
        }

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            for percent in [75u32, 100, 125, 150, 175, 200] {
                if ui
                    .button(format!("{percent}%"))
                    .on_hover_text(tr!(
                        "ui.settings_general.set_ui_scale_to_percent",
                        percent = percent
                    ))
                    .clicked()
                {
                    self.apply_ui_scale(ui, percent as f32 / 100.0);
                }
            }
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button(tr!("ui.settings_general.reset_to_100")).clicked() {
                self.apply_ui_scale(ui, 1.0);
            }
            ui.label(
                RichText::new(tr!(
                    "ui.settings_general.current_scale_percent",
                    percent = format!("{:.0}", self.ui_scale * 100.0)
                ))
                .small()
                .color(dim_text(ui)),
            );
        });

        ui.add_space(14.0);
        ui.separator();
        ui.heading(tr!("ui.settings_general.mesh_view"));
        ui.horizontal(|ui| {
            ui.label(tr!("ui.settings_general.auto_arrange_layout"));
            let previous = self.mesh_shape;
            egui::ComboBox::from_id_salt("mesh_shape")
                .selected_text(self.mesh_shape.label())
                .width(170.0)
                .show_ui(ui, |ui| {
                    for shape in MeshShape::ALL {
                        ui.selectable_value(&mut self.mesh_shape, shape, shape.label());
                    }
                });
            if self.mesh_shape != previous {
                self.queue_settings_save();
            }
        });
        ui.label(
            RichText::new(tr!("ui.settings_general.the_arrange_button_sorts_hosts_by"))
                .small()
                .color(dim_text(ui)),
        );
        if ui
            .button(tr!("ui.settings_general.arrange_hosts_now"))
            .clicked()
        {
            self.arrange_mesh();
        }

        ui.add_space(14.0);
        ui.separator();
        ui.strong(tr!("ui.settings_general.settings_file"));
        ui.horizontal(|ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(self.settings_path.display().to_string())
                        .monospace()
                        .small(),
                )
                .selectable(true),
            );
            if ui.small_button(tr!("ui.host_inspector.copy")).clicked() {
                ui.ctx().copy_text(self.settings_path.display().to_string());
            }
        });
        ui.label(
            RichText::new(tr!(
                "ui.settings_general.window_geometry_and_egui_state_remain"
            ))
            .small()
            .color(dim_text(ui)),
        );

        ui.add_space(14.0);
        ui.separator();
        ui.strong(tr!("ui.settings_general.keyboard_and_mouse"));
        ui.label(tr!("ui.settings_general.drag_a_host_to_move_it"));
        ui.label(tr!("ui.settings_general.scroll_over_the_canvas_to_zoom"));
        ui.label(tr!("ui.settings_general.right_click_empty_canvas_space_to"));
        ui.label(tr!("ui.settings_general.the_canvas_centers_on_each_newly"));
        ui.label(tr!("ui.settings_general.press_delete_with_a_host_selected"));
        ui.label(tr!("ui.settings_general.press_escape_to_clear_the_current"));
    }

    pub(super) fn apply_ui_scale(&mut self, ui: &Ui, scale: f32) {
        let scale = scale.clamp(0.75, 2.0);
        self.ui_scale = scale;
        ui.ctx().set_zoom_factor(scale);
        self.queue_settings_save();
    }

    pub(super) fn delete_host_modal(&mut self, ctx: &egui::Context) {
        let Some(ids) = self.delete_prompt.clone() else {
            return;
        };
        let names: Vec<String> = ids
            .iter()
            .filter_map(|&id| self.mesh.host(id).map(|host| host.name.clone()))
            .collect();
        if names.is_empty() {
            self.delete_prompt = None;
            return;
        }
        let just_opened = std::mem::take(&mut self.delete_prompt_just_opened);
        let multiple = names.len() > 1;
        let heading = if multiple {
            tr!("ui.delete_host_modal.delete_hosts", count = names.len())
        } else {
            tr!("ui.delete_host_modal.delete_host")
        };
        let body = if multiple {
            tr!(
                "ui.delete_host_modal.remove_count_hosts_and_all_of_their",
                count = names.len()
            )
        } else {
            tr!(
                "ui.delete_host_modal.remove_name_and_all_of_its",
                name = &names[0]
            )
        };

        let mut dont_ask = self.delete_prompt_dont_ask;
        let mut confirmed = false;
        let mut cancelled = false;

        let modal = egui::Modal::new(egui::Id::unique("delete_host_modal"))
            .backdrop_color(Color32::from_black_alpha(170))
            .show(ctx, |ui| {
                ui.set_width(430.0);
                ui.heading(heading);
                ui.add_space(4.0);
                ui.label(body);
                ui.label(
                    RichText::new(tr!("ui.delete_host_modal.this_cannot_be_undone")).color(DANGER),
                );
                ui.add_space(10.0);
                ui.checkbox(
                    &mut dont_ask,
                    tr!("ui.delete_host_modal.don_t_ask_again_for_this"),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button(tr!("ui.confirm_new_window.cancel")).clicked() {
                        cancelled = true;
                    }
                    if ui
                        .button(RichText::new(tr!("ui.host_inspector.delete_host")).color(DANGER))
                        .clicked()
                    {
                        confirmed = true;
                    }
                });
            });

        self.delete_prompt_dont_ask = dont_ask;
        if confirmed {
            self.suppress_delete_prompt = dont_ask;
            self.delete_prompt_dont_ask = false;
            self.delete_hosts(&ids);
        } else if cancelled || (!just_opened && modal.should_close()) {
            self.delete_prompt = None;
            self.delete_prompt_dont_ask = false;
        }
    }
    pub(super) fn rotate_secrets_modal(&mut self, ctx: &egui::Context) {
        if !self.rotate_prompt {
            return;
        }

        let just_opened = std::mem::take(&mut self.rotate_prompt_just_opened);
        let host_count = self.mesh.hosts.len();
        let rotate_psks = self.mesh.defaults.preshared_keys;
        let mut confirmed = false;
        let mut cancelled = false;

        let modal = egui::Modal::new(egui::Id::unique("rotate_secrets_modal"))
            .backdrop_color(Color32::from_black_alpha(170))
            .show(ctx, |ui| {
                ui.set_width(460.0);
                ui.heading(tr!("ui.rotate_secrets.rotate_all_secrets"));
                ui.add_space(4.0);
                ui.label(tr!(
                    "ui.rotate_secrets.this_replaces_every_host_private_key",
                    count = host_count
                ));
                if rotate_psks {
                    ui.label(tr!(
                        "ui.rotate_secrets.mesh_wide_preshared_keys_are_enabled"
                    ));
                }
                ui.label(
                    RichText::new(tr!(
                        "ui.rotate_secrets.existing_connections_will_stop_working"
                    ))
                    .color(DANGER),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button(tr!("ui.confirm_new_window.cancel")).clicked() {
                        cancelled = true;
                    }
                    if ui
                        .button(RichText::new(tr!("ui.rotate_secrets.rotate_all")).color(DANGER))
                        .clicked()
                    {
                        confirmed = true;
                    }
                });
            });

        if confirmed {
            self.rotate_all_secrets();
        } else if cancelled || (!just_opened && modal.should_close()) {
            self.rotate_prompt = false;
            self.rotate_prompt_just_opened = false;
        }
    }

    pub(super) fn startup_modal(&mut self, ctx: &egui::Context) {
        if !self.startup_prompt {
            return;
        }

        let mut open_archive = false;
        let mut open_folder = false;
        let mut new_archive = false;
        let mut new_folder = false;

        egui::Modal::new(egui::Id::unique("startup_modal"))
            .backdrop_color(Color32::from_black_alpha(170))
            .show(ctx, |ui| {
                ui.set_width(520.0);
                ui.heading(tr!("ui.startup.create_a_project"));
                ui.add_space(4.0);
                ui.label(tr!(
                    "ui.startup.create_a_new_project_as_a_folder_or_wgm_archive"
                ));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button(tr!("ui.startup.new_archive")).clicked() {
                        new_archive = true;
                    }
                    if ui.button(tr!("ui.startup.new_folder")).clicked() {
                        new_folder = true;
                    }
                });
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                ui.heading(tr!("ui.startup.open_a_project"));
                ui.add_space(4.0);
                ui.label(tr!(
                    "ui.startup.open_an_existing_project_as_a_folder_or_wgm_archive"
                ));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button(tr!("ui.top_bar.open_archive")).clicked() {
                        open_archive = true;
                    }
                    if ui.button(tr!("ui.top_bar.open_folder")).clicked() {
                        open_folder = true;
                    }
                });
            });

        let succeeded = if open_archive {
            self.pick_load_archive()
        } else if open_folder {
            self.pick_load_folder()
        } else if new_archive {
            self.pick_new_archive()
        } else if new_folder {
            self.pick_new_folder()
        } else {
            false
        };

        if succeeded {
            self.startup_prompt = false;
        }
    }

    pub(super) fn close_prompt_modal(&mut self, ctx: &egui::Context) {
        if !self.close_prompt {
            return;
        }

        let mut save = false;
        let mut discard = false;
        let mut cancel = false;

        egui::Modal::new(egui::Id::unique("close_prompt_modal"))
            .backdrop_color(Color32::from_black_alpha(170))
            .show(ctx, |ui| {
                ui.set_width(430.0);
                ui.heading(tr!("ui.close_prompt.save_changes_before_closing"));
                ui.add_space(4.0);
                ui.label(tr!(
                    "ui.close_prompt.your_changes_will_be_lost_if_you_close"
                ));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button(tr!("ui.top_bar.save")).clicked() {
                        save = true;
                    }
                    if ui.button(tr!("ui.close_prompt.discard_changes")).clicked() {
                        discard = true;
                    }
                    if ui.button(tr!("ui.confirm_new_window.cancel")).clicked() {
                        cancel = true;
                    }
                });
            });

        let close_project = self.close_prompt_project;
        if save {
            if self.save_now(true) {
                self.close_prompt = false;
                self.close_prompt_project = false;
                if close_project {
                    self.close_project();
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        } else if discard {
            self.saved = self.mesh.clone();
            self.load_dirty = false;
            self.close_prompt = false;
            self.close_prompt_project = false;
            if close_project {
                self.close_project();
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        } else if cancel {
            self.close_prompt = false;
            self.close_prompt_project = false;
        }
    }
}
