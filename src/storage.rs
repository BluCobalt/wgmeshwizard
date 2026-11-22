//! CSV persistence.
//!
//! A project contains three plain CSV files:
//!
//! * `mesh.csv`   – mesh-wide defaults (one data row)
//! * `hosts.csv`  – one row per host, including canvas position
//! * `links.csv`  – one row per directed host-to-host connection
//!
//! The files are deliberately simple and independent so they can be edited by
//! hand or by a spreadsheet.  All values are quoted, and a UTF-8 BOM is written
//! so Excel/LibreOffice detect the encoding correctly.
//!
//! A project can be either a directory containing those files or a ZIP archive
//! containing them at the root.  `.zip` and the friendlier `.wgm` extension are
//! both accepted; a file with ZIP magic bytes is also recognized.

use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::model::{Connection, Host, MESH_CONFIG_VERSION, Mesh, MeshDefaults};

pub const MESH_FILE: &str = "mesh.csv";
pub const HOSTS_FILE: &str = "hosts.csv";
pub const LINKS_FILE: &str = "links.csv";

const MESH_HEADERS: &[&str] = &[
    "Version",
    "MeshName",
    "AddressPrefix",
    "ListenPort",
    "PersistentKeepalive",
    "FwMark",
    "DNS",
    "MTU",
    "Table",
    "PreUp",
    "PostUp",
    "PreDown",
    "PostDown",
    "SaveConfig",
    "AllowedIPs",
    "PresharedKeys",
    "GenerateHostsFile",
    "SearchDomain",
    "EnableDeployment",
    "SSHUser",
    "SSHPort",
];

const HOST_HEADERS: &[&str] = &[
    "ID",
    "Name",
    "X",
    "Y",
    "Address",
    "Endpoint",
    "SSHTarget",
    "ListenPort",
    "PrivateKey",
    "AllowedIPs",
    "PersistentKeepalive",
    "FwMark",
    "DNS",
    "MTU",
    "Table",
    "PreUp",
    "PostUp",
    "PreDown",
    "PostDown",
    "SaveConfig",
];

const LINK_HEADERS: &[&str] = &["FromID", "ToID", "Enabled", "Endpoint", "PresharedKey"];

#[derive(Debug)]
pub struct LoadReport {
    pub mesh: Mesh,
    /// Whether loading itself filled in missing data (links, ids, ...).
    pub normalized: bool,
    /// True when the project did not contain `hosts.csv`.
    pub created_new: bool,
}

/// True when the path has a `.zip` or `.wgm` extension.
pub fn is_archive_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            let extension = extension.to_ascii_lowercase();
            extension == "zip" || extension == "wgm"
        })
        .unwrap_or(false)
}

/// True when the path should be treated as an archive project.
pub fn is_archive_project(path: &Path) -> bool {
    !path.is_dir() && (is_archive_path(path) || looks_like_zip(path))
}

pub fn default_output_dir(project_path: &Path) -> PathBuf {
    if is_archive_project(project_path) {
        let parent = project_path.parent().unwrap_or_else(|| Path::new("."));
        let stem = project_path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "mesh".to_owned());
        parent.join(format!("{stem}-configs"))
    } else {
        project_path.join("output")
    }
}

/// Load a project from a directory or a `.zip`/`.wgm` archive.
pub fn load_project(path: &Path) -> Result<LoadReport, String> {
    if is_archive_project(path) {
        load_archive(path)
    } else if path.is_file() {
        Err(format!(
            "{} is a file but not a .zip/.wgm archive",
            path.display()
        ))
    } else {
        load_directory(path)
    }
}

/// Save a project, using the same representation as the path implies.
pub fn save_project(path: &Path, mesh: &Mesh) -> Result<(), String> {
    if is_archive_project(path) {
        save_archive(path, mesh)
    } else {
        save_directory(path, mesh)
    }
}

fn load_directory(directory: &Path) -> Result<LoadReport, String> {
    let created_new = !directory.join(HOSTS_FILE).exists();
    let normalized;
    let mut mesh;

    if directory.join(HOSTS_FILE).exists() {
        let defaults = if directory.join(MESH_FILE).exists() {
            parse_defaults(
                &read_text(&directory.join(MESH_FILE))?,
                &path_label(&directory.join(MESH_FILE)),
            )?
        } else {
            MeshDefaults::default()
        };
        mesh = Mesh {
            defaults,
            hosts: Vec::new(),
            connections: Vec::new(),
            next_id: 1,
        };
        parse_hosts(
            &read_text(&directory.join(HOSTS_FILE))?,
            &path_label(&directory.join(HOSTS_FILE)),
            &mut mesh,
        )?;
        if directory.join(LINKS_FILE).exists() {
            parse_links(
                &read_text(&directory.join(LINKS_FILE))?,
                &path_label(&directory.join(LINKS_FILE)),
                &mut mesh,
            )?;
        }
        normalized = mesh.normalize();
    } else if directory.join(MESH_FILE).exists() || directory.join(LINKS_FILE).exists() {
        let defaults = if directory.join(MESH_FILE).exists() {
            parse_defaults(
                &read_text(&directory.join(MESH_FILE))?,
                &path_label(&directory.join(MESH_FILE)),
            )?
        } else {
            MeshDefaults::default()
        };
        mesh = Mesh {
            defaults,
            hosts: Vec::new(),
            connections: Vec::new(),
            next_id: 1,
        };
        normalized = mesh.normalize();
    } else {
        mesh = Mesh::new();
        normalized = false;
    }

    Ok(LoadReport {
        mesh,
        normalized,
        created_new,
    })
}

fn load_archive(path: &Path) -> Result<LoadReport, String> {
    let file =
        File::open(path).map_err(|error| format!("could not open {}: {error}", path.display()))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|error| format!("could not read zip {}: {error}", path.display()))?;

    let mut mesh_text = None;
    let mut hosts_text = None;
    let mut links_text = None;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("could not read zip entry in {}: {error}", path.display()))?;
        if entry.is_dir() {
            continue;
        }
        let entry_name = entry
            .name()
            .map(|name| name.into_owned())
            .unwrap_or_default();
        let name = Path::new(&entry_name)
            .file_name()
            .map(|name| name.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let target = match name.as_str() {
            "mesh.csv" => &mut mesh_text,
            "hosts.csv" => &mut hosts_text,
            "links.csv" => &mut links_text,
            _ => continue,
        };
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|error| format!("could not read {entry_name}: {error}"))?;
        *target = Some(text_from_bytes(&bytes));
    }

    if mesh_text.is_none() && hosts_text.is_none() && links_text.is_none() {
        return Err(format!(
            "{} does not contain mesh.csv, hosts.csv or links.csv",
            path.display()
        ));
    }

    let mut mesh = Mesh {
        defaults: mesh_text
            .as_deref()
            .map(|text| parse_defaults(text, &format!("{}/mesh.csv", path.display())))
            .transpose()?
            .unwrap_or_default(),
        hosts: Vec::new(),
        connections: Vec::new(),
        next_id: 1,
    };

    if let Some(text) = hosts_text.as_deref() {
        parse_hosts(text, &format!("{}/hosts.csv", path.display()), &mut mesh)?;
    }
    if let Some(text) = links_text.as_deref() {
        parse_links(text, &format!("{}/links.csv", path.display()), &mut mesh)?;
    }

    let normalized = mesh.normalize();
    Ok(LoadReport {
        mesh,
        normalized,
        created_new: hosts_text.is_none(),
    })
}

fn save_directory(directory: &Path, mesh: &Mesh) -> Result<(), String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;

    let mesh_csv = render_defaults(&mesh.defaults)?;
    let hosts_csv = render_hosts(&mesh.hosts)?;
    let links_csv = render_links(&mesh.connections)?;

    write_atomic(&directory.join(MESH_FILE), &mesh_csv)?;
    write_atomic(&directory.join(HOSTS_FILE), &hosts_csv)?;
    write_atomic(&directory.join(LINKS_FILE), &links_csv)?;
    Ok(())
}

fn save_archive(path: &Path, mesh: &Mesh) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }

    let mesh_csv = render_defaults(&mesh.defaults)?;
    let hosts_csv = render_hosts(&mesh.hosts)?;
    let links_csv = render_links(&mesh.connections)?;

    let temporary = temporary_path(path);
    let file = File::create(&temporary)
        .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    for (name, bytes) in [
        (MESH_FILE, mesh_csv.as_slice()),
        (HOSTS_FILE, hosts_csv.as_slice()),
        (LINKS_FILE, links_csv.as_slice()),
    ] {
        writer
            .start_file(name, options)
            .map_err(|error| format!("could not add {name}: {error}"))?;
        writer
            .write_all(bytes)
            .map_err(|error| format!("could not write {name}: {error}"))?;
    }

    let file = writer
        .finish()
        .map_err(|error| format!("could not finish zip: {error}"))?;
    drop(file);
    fs::rename(&temporary, path)
        .map_err(|error| format!("could not replace {}: {error}", path.display()))?;
    Ok(())
}

fn parse_defaults(text: &str, label: &str) -> Result<MeshDefaults, String> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(detect_delimiter(text))
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());

    let headers = reader
        .headers()
        .map_err(|error| format!("{label}: {error}"))?
        .clone();

    let record = match reader.records().next() {
        Some(Ok(record)) => record,
        Some(Err(error)) => return Err(format!("{label}: {error}")),
        None => return Ok(MeshDefaults::default()),
    };

    let get = |name: &str| field(&headers, &record, name);
    let mut defaults = MeshDefaults::default();
    let version = parse_optional::<u32>(get("Version"), "Version")?.unwrap_or(MESH_CONFIG_VERSION);
    if version > MESH_CONFIG_VERSION {
        return Err(format!(
            "{label}: mesh config version {version} is newer than this build supports ({MESH_CONFIG_VERSION})"
        ));
    }
    defaults.version = version.max(1);
    defaults.name = non_empty(get("MeshName"), &defaults.name);
    defaults.address_prefix = non_empty(get("AddressPrefix"), &defaults.address_prefix);
    defaults.listen_port = parse_optional(get("ListenPort"), "ListenPort")?;
    defaults.persistent_keepalive =
        parse_optional(get("PersistentKeepalive"), "PersistentKeepalive")?;
    defaults.fwmark = get("FwMark").to_owned();
    defaults.dns = get("DNS").to_owned();
    defaults.mtu = parse_optional(get("MTU"), "MTU")?;
    defaults.table = get("Table").to_owned();
    defaults.pre_up = get("PreUp").to_owned();
    defaults.post_up = get("PostUp").to_owned();
    defaults.pre_down = get("PreDown").to_owned();
    defaults.post_down = get("PostDown").to_owned();
    defaults.save_config = parse_optional_bool(get("SaveConfig"), "SaveConfig")?;
    defaults.allowed_ips = get("AllowedIPs").to_owned();
    defaults.preshared_keys =
        parse_optional_bool(get("PresharedKeys"), "PresharedKeys")?.unwrap_or(false);
    defaults.generate_hosts_file =
        parse_optional_bool(get("GenerateHostsFile"), "GenerateHostsFile")?.unwrap_or(false);
    defaults.search_domain = get("SearchDomain").to_owned();
    defaults.enable_deployment =
        parse_optional_bool(get("EnableDeployment"), "EnableDeployment")?.unwrap_or(false);
    defaults.ssh_user = get("SSHUser").to_owned();
    defaults.ssh_port = parse_optional(get("SSHPort"), "SSHPort")?;

    Ok(defaults)
}

fn parse_hosts(text: &str, label: &str, mesh: &mut Mesh) -> Result<(), String> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(detect_delimiter(text))
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());

    let headers = reader
        .headers()
        .map_err(|error| format!("{label}: {error}"))?
        .clone();

    for (row, result) in reader.records().enumerate() {
        let record = result.map_err(|error| format!("{label} row {}: {error}", row + 2))?;
        let get = |name: &str| field(&headers, &record, name);

        let id = parse_optional::<u64>(get("ID"), "ID")?
            .ok_or_else(|| format!("{label} row {}: missing ID", row + 2))?;

        let name = get("Name");
        if name.is_empty() {
            continue;
        }

        let host = Host {
            id,
            name: name.to_owned(),
            x: parse_f32(get("X")),
            y: parse_f32(get("Y")),
            address: get("Address").to_owned(),
            endpoint: get("Endpoint").to_owned(),
            ssh_target: get("SSHTarget").to_owned(),
            listen_port: parse_optional(get("ListenPort"), "ListenPort")?,
            private_key: get("PrivateKey").to_owned(),
            allowed_ips: get("AllowedIPs").to_owned(),
            persistent_keepalive: parse_optional(
                get("PersistentKeepalive"),
                "PersistentKeepalive",
            )?,
            fwmark: get("FwMark").to_owned(),
            dns: get("DNS").to_owned(),
            mtu: parse_optional(get("MTU"), "MTU")?,
            table: get("Table").to_owned(),
            pre_up: get("PreUp").to_owned(),
            post_up: get("PostUp").to_owned(),
            pre_down: get("PreDown").to_owned(),
            post_down: get("PostDown").to_owned(),
            save_config: parse_optional_bool(get("SaveConfig"), "SaveConfig")?,
        };

        mesh.hosts.push(host);
        mesh.next_id = mesh.next_id.max(id.saturating_add(1));
    }

    Ok(())
}

fn parse_links(text: &str, label: &str, mesh: &mut Mesh) -> Result<(), String> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(detect_delimiter(text))
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());

    let headers = reader
        .headers()
        .map_err(|error| format!("{label}: {error}"))?
        .clone();

    for (row, result) in reader.records().enumerate() {
        let record = result.map_err(|error| format!("{label} row {}: {error}", row + 2))?;
        let get = |name: &str| field(&headers, &record, name);

        let from = parse_optional::<u64>(get("FromID"), "FromID")?
            .ok_or_else(|| format!("{label} row {}: missing FromID", row + 2))?;
        let to = parse_optional::<u64>(get("ToID"), "ToID")?
            .ok_or_else(|| format!("{label} row {}: missing ToID", row + 2))?;

        mesh.connections.push(Connection {
            from,
            to,
            enabled: parse_optional_bool(get("Enabled"), "Enabled")?.unwrap_or(true),
            endpoint: get("Endpoint").to_owned(),
            preshared_key: get("PresharedKey").to_owned(),
        });
    }

    Ok(())
}

fn render_defaults(defaults: &MeshDefaults) -> Result<Vec<u8>, String> {
    let values = [
        defaults.version.to_string(),
        defaults.name.clone(),
        defaults.address_prefix.clone(),
        opt_string(defaults.listen_port),
        opt_string(defaults.persistent_keepalive),
        defaults.fwmark.clone(),
        defaults.dns.clone(),
        opt_string(defaults.mtu),
        defaults.table.clone(),
        defaults.pre_up.clone(),
        defaults.post_up.clone(),
        defaults.pre_down.clone(),
        defaults.post_down.clone(),
        opt_bool_string(defaults.save_config),
        defaults.allowed_ips.clone(),
        defaults.preshared_keys.to_string(),
        defaults.generate_hosts_file.to_string(),
        defaults.search_domain.clone(),
        defaults.enable_deployment.to_string(),
        defaults.ssh_user.clone(),
        opt_string(defaults.ssh_port),
    ];
    render_csv(MESH_HEADERS, &[values])
}

fn render_hosts(hosts: &[Host]) -> Result<Vec<u8>, String> {
    let rows: Vec<[String; 20]> = hosts
        .iter()
        .map(|host| {
            [
                host.id.to_string(),
                host.name.clone(),
                format!("{:.1}", host.x),
                format!("{:.1}", host.y),
                host.address.clone(),
                host.endpoint.clone(),
                host.ssh_target.clone(),
                opt_string(host.listen_port),
                host.private_key.clone(),
                host.allowed_ips.clone(),
                opt_string(host.persistent_keepalive),
                host.fwmark.clone(),
                host.dns.clone(),
                opt_string(host.mtu),
                host.table.clone(),
                host.pre_up.clone(),
                host.post_up.clone(),
                host.pre_down.clone(),
                host.post_down.clone(),
                opt_bool_string(host.save_config),
            ]
        })
        .collect();

    render_csv(HOST_HEADERS, &rows)
}

fn render_links(connections: &[Connection]) -> Result<Vec<u8>, String> {
    let rows: Vec<[String; 5]> = connections
        .iter()
        .map(|conn| {
            [
                conn.from.to_string(),
                conn.to.to_string(),
                if conn.enabled { "true" } else { "false" }.to_owned(),
                conn.endpoint.clone(),
                conn.preshared_key.clone(),
            ]
        })
        .collect();

    render_csv(LINK_HEADERS, &rows)
}

fn render_csv<const N: usize>(headers: &[&str], rows: &[[String; N]]) -> Result<Vec<u8>, String> {
    let mut writer = csv::WriterBuilder::new()
        .quote_style(csv::QuoteStyle::Always)
        .from_writer(Vec::new());

    writer
        .write_record(headers)
        .map_err(|error| format!("could not write CSV header: {error}"))?;
    for row in rows {
        writer
            .write_record(row)
            .map_err(|error| format!("could not write CSV row: {error}"))?;
    }

    let bytes = writer
        .into_inner()
        .map_err(|error| format!("could not finish CSV: {error}"))?;

    let mut with_bom = Vec::with_capacity(bytes.len() + 3);
    with_bom.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    with_bom.extend_from_slice(&bytes);
    Ok(with_bom)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = temporary_path(path);
    fs::write(&temporary, bytes)
        .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("could not replace {}: {error}", path.display()))?;
    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    name.push(".tmp");
    path.with_file_name(name)
}

fn read_text(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
    Ok(text_from_bytes(&bytes))
}

fn text_from_bytes(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    String::from_utf8_lossy(bytes).into_owned()
}

fn path_label(path: &Path) -> String {
    path.display().to_string()
}

fn looks_like_zip(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let mut prefix = [0u8; 4];
    match File::open(path).and_then(|mut file| file.read_exact(&mut prefix)) {
        Ok(()) => prefix.starts_with(b"PK"),
        Err(_) => false,
    }
}

/// Accept comma-separated files as well as the semicolon/tab variants that
/// Excel and LibreOffice sometimes produce for non-English locales.
fn detect_delimiter(text: &str) -> u8 {
    let header = text.lines().next().unwrap_or("");
    let candidates = [
        (b',', header.matches(',').count()),
        (b';', header.matches(';').count()),
        (b'\t', header.matches('\t').count()),
    ];
    candidates
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .filter(|(_, count)| *count > 0)
        .map(|(delimiter, _)| delimiter)
        .unwrap_or(b',')
}

fn field<'a>(headers: &csv::StringRecord, record: &'a csv::StringRecord, name: &str) -> &'a str {
    headers
        .iter()
        .position(|header| header.eq_ignore_ascii_case(name))
        .and_then(|index| record.get(index))
        .unwrap_or("")
}

fn non_empty(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_owned()
    } else {
        value.to_owned()
    }
}

fn parse_f32(value: &str) -> f32 {
    value.trim().parse::<f32>().unwrap_or(0.0)
}

fn parse_optional<T>(value: &str, label: &str) -> Result<Option<T>, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<T>()
        .map(Some)
        .map_err(|error| format!("invalid {label} value {value:?}: {error}"))
}

fn parse_optional_bool(value: &str, label: &str) -> Result<Option<bool>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "y" => Ok(Some(true)),
        "false" | "0" | "no" | "n" | "off" => Ok(Some(false)),
        _ => Err(format!("invalid {label} value {value:?}")),
    }
}

fn opt_string<T: ToString>(value: Option<T>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn opt_bool_string(value: Option<bool>) -> String {
    match value {
        Some(true) => "true".to_owned(),
        Some(false) => "false".to_owned(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("wgmeshwizard-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn demo_mesh() -> Mesh {
        let mut mesh = Mesh::new();
        mesh.defaults.name = "lab".into();
        mesh.defaults.dns = "1.1.1.1".into();
        mesh.defaults.mtu = Some(1380);
        mesh.defaults.preshared_keys = true;
        mesh.defaults.generate_hosts_file = true;
        mesh.defaults.search_domain = "example.tld".into();
        mesh.defaults.enable_deployment = true;
        mesh.defaults.ssh_user = "deploy".into();
        mesh.defaults.ssh_port = Some(2222);
        let a = mesh.add_host("alpha".into(), 12.5, -9.0);
        let b = mesh.add_host("bravo".into(), 300.0, 20.0);
        {
            let host = mesh.host_mut(a).unwrap();
            host.address = "10.50.0.1/32".into();
            host.endpoint = "203.0.113.10".into();
            host.listen_port = Some(51821);
            host.private_key = crate::crypto::generate_private_key();
            host.pre_up = "echo up".into();
            host.save_config = Some(true);
            host.ssh_target = "root@203.0.113.10:2222".into();
        }
        {
            let conn = mesh.connection_mut(a, b).unwrap();
            conn.endpoint = "192.168.1.2".into();
            conn.enabled = false;
            conn.preshared_key = crate::crypto::generate_preshared_key();
        }
        mesh
    }

    #[test]
    fn round_trip_preserves_everything() {
        let dir = temp_dir("roundtrip");
        let mesh = demo_mesh();
        let a = mesh.hosts[0].id;
        let b = mesh.hosts[1].id;

        save_project(&dir, &mesh).unwrap();
        let report = load_project(&dir).unwrap();

        assert_eq!(report.mesh.defaults, mesh.defaults);
        assert_eq!(report.mesh.hosts.len(), 2);
        assert_eq!(report.mesh.hosts[0], mesh.hosts[0]);
        assert_eq!(report.mesh.hosts[1], mesh.hosts[1]);
        assert_eq!(
            report.mesh.connection(a, b).unwrap(),
            mesh.connection(a, b).unwrap()
        );
        assert!(!report.normalized);

        let raw = fs::read(dir.join(HOSTS_FILE)).unwrap();
        assert_eq!(&raw[..3], &[0xEF, 0xBB, 0xBF]);

        let mesh_raw = fs::read(dir.join(MESH_FILE)).unwrap();
        assert_eq!(&mesh_raw[..3], &[0xEF, 0xBB, 0xBF]);
        assert!(mesh_raw[3..].starts_with(b"\"Version\",\"MeshName\""));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn wgm_archive_round_trip() {
        let dir = temp_dir("wgm");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("lab.wgm");
        let mesh = demo_mesh();
        let a = mesh.hosts[0].id;
        let b = mesh.hosts[1].id;

        save_project(&path, &mesh).unwrap();
        assert!(path.is_file());
        let report = load_project(&path).unwrap();

        assert_eq!(report.mesh.defaults, mesh.defaults);
        assert_eq!(report.mesh.hosts, mesh.hosts);
        assert_eq!(
            report.mesh.connection(a, b).unwrap(),
            mesh.connection(a, b).unwrap()
        );
        assert!(!report.created_new);
        assert!(!report.normalized);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn zip_output_directory_is_sibling_folder() {
        assert!(is_archive_path(Path::new("network.zip")));
        assert!(is_archive_path(Path::new("network.WGM")));
        assert!(!is_archive_path(Path::new("network")));
        assert_eq!(
            default_output_dir(Path::new("/tmp/network.wgm")),
            PathBuf::from("/tmp/network-configs")
        );
        assert_eq!(
            default_output_dir(Path::new("/tmp/network.zip")),
            PathBuf::from("/tmp/network-configs")
        );
        assert_eq!(
            default_output_dir(Path::new("/tmp/network")),
            PathBuf::from("/tmp/network/output")
        );
    }

    #[test]
    fn mesh_without_a_version_column_uses_the_current_version() {
        let dir = temp_dir("old-version");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(MESH_FILE),
            "\"MeshName\",\"AddressPrefix\"\n\"legacy\",\"10.9.0.0/24\"\n",
        )
        .unwrap();
        fs::write(
            dir.join(HOSTS_FILE),
            "\"ID\",\"Name\",\"Address\"\n1,\"alpha\",\"10.9.0.1/32\"\n",
        )
        .unwrap();

        let report = load_project(&dir).unwrap();
        assert_eq!(report.mesh.defaults.version, MESH_CONFIG_VERSION);
        assert_eq!(report.mesh.defaults.name, "legacy");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn newer_mesh_versions_are_rejected() {
        let dir = temp_dir("new-version");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(MESH_FILE),
            format!(
                "\"Version\",\"MeshName\"\n\"{}\",\"future\"\n",
                MESH_CONFIG_VERSION + 1
            ),
        )
        .unwrap();
        fs::write(
            dir.join(HOSTS_FILE),
            "\"ID\",\"Name\",\"Address\"\n1,\"alpha\",\"10.9.0.1/32\"\n",
        )
        .unwrap();

        let error = load_project(&dir).unwrap_err();
        assert!(error.contains("newer"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_files_load_as_new_mesh() {
        let dir = temp_dir("missing");
        let report = load_project(&dir).unwrap();
        assert!(report.created_new);
        assert!(report.mesh.hosts.is_empty());
        assert_eq!(report.mesh.defaults.address_prefix, "10.100.0.0/24");
    }

    #[test]
    fn headers_are_case_insensitive_and_extra_columns_are_ignored() {
        let dir = temp_dir("headers");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(HOSTS_FILE),
            "id,name,x,y,address,unknown\n1,alpha,10,20,10.0.0.1/32,ignored\n",
        )
        .unwrap();

        let report = load_project(&dir).unwrap();
        assert_eq!(report.mesh.hosts.len(), 1);
        assert_eq!(report.mesh.hosts[0].name, "alpha");
        assert_eq!(report.mesh.hosts[0].x, 10.0);
        assert_eq!(report.mesh.defaults.address_prefix, "10.100.0.0/24");
        assert!(!report.mesh.defaults.generate_hosts_file);
        assert!(report.mesh.defaults.search_domain.is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn semicolon_delimited_exports_load() {
        let dir = temp_dir("semicolon");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(HOSTS_FILE),
            "ID;Name;X;Y;Address\n1;alpha;10;20;10.0.0.1/32\n",
        )
        .unwrap();

        let report = load_project(&dir).unwrap();
        assert_eq!(report.mesh.hosts.len(), 1);
        assert_eq!(report.mesh.hosts[0].name, "alpha");
        assert_eq!(report.mesh.hosts[0].address, "10.0.0.1/32");
        fs::remove_dir_all(&dir).unwrap();
    }
}
