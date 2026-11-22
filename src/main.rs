mod app;
mod crypto;
mod deploy;
mod i18n;
mod model;
mod settings;
mod storage;
mod timestamp;
mod wgconfig;

rust_i18n::i18n!("res/i18n", fallback = "en");

use std::ffi::OsString;
use std::path::PathBuf;

use crate::i18n::tr;

fn main() {
    if let Err(error) = run() {
        eprintln!("{}: {error}", tr!("cli.error_prefix"));
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    init_locale();
    let mut arguments = std::env::args_os().skip(1);
    let first = arguments.next();
    if let Some(argument) = &first {
        match argument.to_string_lossy().as_ref() {
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            "--version" => {
                println!("wgmeshwizard {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "export" => return export_command(arguments.collect()),
            _ => {}
        }
    }

    let project = first.map(PathBuf::from);

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("WireGuard Mesh Wizard")
            .with_inner_size([1360.0, 860.0])
            .with_min_inner_size([980.0, 640.0]),
        persistence_path: settings::persistence_path(),
        ..Default::default()
    };

    eframe::run_native(
        "WireGuard Mesh Wizard",
        native_options,
        Box::new(move |cc| Ok(Box::new(app::MeshWizardApp::new(cc, project)))),
    )
    .map_err(|error| tr!("cli.could_not_start_gui", error = error))
}

fn init_locale() {
    let language = settings::settings_path()
        .and_then(|path| settings::load(&path).ok().flatten())
        .map(|settings| settings.language.locale)
        .unwrap_or_else(|| "system".to_owned());
    i18n::set_language(&language);
}

fn export_command(arguments: Vec<OsString>) -> Result<(), String> {
    let mut project = None;
    let mut output = None;
    let mut arguments = arguments.into_iter();

    while let Some(argument) = arguments.next() {
        match argument.to_string_lossy().as_ref() {
            "-h" | "--help" => {
                print_export_help();
                return Ok(());
            }
            "-o" | "--output" => {
                output = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| tr!("cli.output_requires_directory"))?,
                ));
            }
            value if value.starts_with('-') => {
                return Err(tr!(
                    "cli.unknown_export_option",
                    value = format!("{value:?}")
                ));
            }
            _ => {
                if project.is_some() {
                    return Err(tr!("cli.export_accepts_one_project"));
                }
                project = Some(PathBuf::from(argument));
            }
        }
    }

    let project = project.ok_or_else(|| tr!("cli.export_needs_project"))?;
    let mut report = storage::load_project(&project)?;
    let generated = report.mesh.ensure_private_keys();
    report.normalized |= report.mesh.normalize();
    let psks_generated = report.mesh.sync_preshared_keys();

    if report.normalized || generated > 0 || psks_generated {
        storage::save_project(&project, &report.mesh)?;
    }

    let output = output.unwrap_or_else(|| storage::default_output_dir(&project));
    let paths = wgconfig::export_configs(&report.mesh, &output)?;

    if paths.len() == 1 {
        println!("{}", tr!("cli.exported_1", path = output.display()));
    } else {
        println!(
            "{}",
            tr!(
                "cli.exported_many",
                count = paths.len(),
                path = output.display()
            )
        );
    }
    for path in paths {
        println!("  {}", path.display());
    }
    Ok(())
}

fn print_help() {
    println!("{}", tr!("cli.help"));
}

fn print_export_help() {
    println!("{}", tr!("cli.export_help"));
}
