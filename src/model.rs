//! In-memory model of a WireGuard mesh.
//!
//! The persistent representation is a set of three CSV files (see
//! [`crate::storage`]).  This module only contains the data and the pure
//! operations on it, which keeps it easy to test without a GUI.

use std::collections::HashSet;
use std::net::Ipv4Addr;

/// Version of the `mesh.csv` layout. The first column stores this value so a
/// future build can recognize data it understands.
pub const MESH_CONFIG_VERSION: u32 = 1;

/// Default WireGuard listen port.
pub const DEFAULT_LISTEN_PORT: u16 = 51820;

/// WireGuard's conventional MTU for a tunnel that runs over Ethernet/IPv4.
pub const DEFAULT_MTU: u32 = 1420;

/// Values that mean "do not write an Endpoint line" when entered explicitly.
pub const NO_ENDPOINT_VALUES: &[&str] = &["none", "null", "-", "~"];

/// Mesh-wide defaults.  A host field that is left empty falls back to the
/// corresponding value here.  `None` means "inherit the application default"
/// for the numeric / boolean fields.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshDefaults {
    /// Version of the surrounding `mesh.csv` layout.
    pub version: u32,
    pub name: String,
    /// Prefix used to suggest addresses for new hosts, for example
    /// `10.100.0.0/24`.  New hosts receive the next free host address from
    /// this network as a `/32`.
    pub address_prefix: String,
    pub listen_port: Option<u16>,
    pub persistent_keepalive: Option<u16>,
    pub fwmark: String,
    pub dns: String,
    pub mtu: Option<u32>,
    pub table: String,
    pub pre_up: String,
    pub post_up: String,
    pub pre_down: String,
    pub post_down: String,
    pub save_config: Option<bool>,
    /// Extra AllowedIPs added to every peer entry when the host does not
    /// define its own.
    pub allowed_ips: String,
    /// Generate and maintain one preshared key per host pair.
    pub preshared_keys: bool,
    /// Write an /etc/hosts-compatible file when configs are exported.
    pub generate_hosts_file: bool,
    /// Domain appended as a fully-qualified alias in the generated hosts file.
    pub search_domain: String,
    /// Write a `deploy.sh` script when configs are exported.
    pub enable_deployment: bool,
    /// Default SSH user used by the deploy script when a host does not name one.
    pub ssh_user: String,
    /// Default SSH port used by the deploy script when a host does not name one.
    pub ssh_port: Option<u16>,
}

impl Default for MeshDefaults {
    fn default() -> Self {
        Self {
            version: MESH_CONFIG_VERSION,
            name: "my-mesh".to_owned(),
            address_prefix: "10.100.0.0/24".to_owned(),
            listen_port: Some(DEFAULT_LISTEN_PORT),
            persistent_keepalive: None,
            fwmark: String::new(),
            dns: String::new(),
            mtu: None,
            table: String::new(),
            pre_up: String::new(),
            post_up: String::new(),
            pre_down: String::new(),
            post_down: String::new(),
            save_config: None,
            allowed_ips: String::new(),
            preshared_keys: false,
            generate_hosts_file: false,
            search_domain: String::new(),
            enable_deployment: false,
            ssh_user: String::new(),
            ssh_port: None,
        }
    }
}

/// A single WireGuard host.
#[derive(Clone, Debug, PartialEq)]
pub struct Host {
    pub id: u64,
    pub name: String,
    /// Position on the canvas, persisted with the project.
    pub x: f32,
    pub y: f32,
    pub address: String,
    /// Public endpoint other hosts can use by default.  A port may be
    /// included; otherwise the host's effective listen port is appended.
    pub endpoint: String,
    /// Optional `user@host:port` destination used by the generated deploy
    /// script instead of the public endpoint.
    pub ssh_target: String,
    pub listen_port: Option<u16>,
    /// WireGuard X25519 private key, base64 encoded.
    pub private_key: String,
    /// Extra AllowedIPs advertised for this host (comma separated).
    pub allowed_ips: String,
    pub persistent_keepalive: Option<u16>,
    pub fwmark: String,
    pub dns: String,
    pub mtu: Option<u32>,
    pub table: String,
    pub pre_up: String,
    pub post_up: String,
    pub pre_down: String,
    pub post_down: String,
    pub save_config: Option<bool>,
}

impl Host {
    pub fn empty(id: u64, name: String, x: f32, y: f32) -> Self {
        Self {
            id,
            name,
            x,
            y,
            address: String::new(),
            endpoint: String::new(),
            ssh_target: String::new(),
            listen_port: None,
            private_key: String::new(),
            allowed_ips: String::new(),
            persistent_keepalive: None,
            fwmark: String::new(),
            dns: String::new(),
            mtu: None,
            table: String::new(),
            pre_up: String::new(),
            post_up: String::new(),
            pre_down: String::new(),
            post_down: String::new(),
            save_config: None,
        }
    }

    pub fn effective_listen_port(&self, defaults: &MeshDefaults) -> u16 {
        self.listen_port
            .or(defaults.listen_port)
            .unwrap_or(DEFAULT_LISTEN_PORT)
    }

    pub fn effective_persistent_keepalive(&self, defaults: &MeshDefaults) -> Option<u16> {
        self.persistent_keepalive.or(defaults.persistent_keepalive)
    }

    pub fn effective_mtu(&self, defaults: &MeshDefaults) -> Option<u32> {
        self.mtu.or(defaults.mtu)
    }

    pub fn effective_save_config(&self, defaults: &MeshDefaults) -> bool {
        self.save_config.or(defaults.save_config).unwrap_or(false)
    }

    pub fn effective_fwmark<'a>(&'a self, defaults: &'a MeshDefaults) -> &'a str {
        inherited(&self.fwmark, &defaults.fwmark)
    }

    pub fn effective_dns<'a>(&'a self, defaults: &'a MeshDefaults) -> &'a str {
        inherited(&self.dns, &defaults.dns)
    }

    pub fn effective_table<'a>(&'a self, defaults: &'a MeshDefaults) -> &'a str {
        inherited(&self.table, &defaults.table)
    }

    pub fn effective_pre_up<'a>(&'a self, defaults: &'a MeshDefaults) -> &'a str {
        inherited(&self.pre_up, &defaults.pre_up)
    }

    pub fn effective_post_up<'a>(&'a self, defaults: &'a MeshDefaults) -> &'a str {
        inherited(&self.post_up, &defaults.post_up)
    }

    pub fn effective_pre_down<'a>(&'a self, defaults: &'a MeshDefaults) -> &'a str {
        inherited(&self.pre_down, &defaults.pre_down)
    }

    pub fn effective_post_down<'a>(&'a self, defaults: &'a MeshDefaults) -> &'a str {
        inherited(&self.post_down, &defaults.post_down)
    }

    pub fn effective_allowed_ips<'a>(&'a self, defaults: &'a MeshDefaults) -> &'a str {
        inherited(&self.allowed_ips, &defaults.allowed_ips)
    }
}

fn inherited<'a>(own: &'a str, default: &'a str) -> &'a str {
    if own.trim().is_empty() { default } else { own }
}

/// A directed connection from one host to another.
///
/// A full mesh has two connections for every pair of hosts, because each host
/// has its own endpoint for reaching the other.
#[derive(Clone, Debug, PartialEq)]
pub struct Connection {
    pub from: u64,
    pub to: u64,
    pub enabled: bool,
    /// Endpoint override used by `from` to reach `to`.  Empty means "use the
    /// target host's default endpoint".  `none` suppresses the Endpoint line.
    pub endpoint: String,
    pub preshared_key: String,
}

/// The complete project.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    pub defaults: MeshDefaults,
    pub hosts: Vec<Host>,
    pub connections: Vec<Connection>,
    pub next_id: u64,
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            defaults: MeshDefaults::default(),
            hosts: Vec::new(),
            connections: Vec::new(),
            next_id: 1,
        }
    }

    pub fn host(&self, id: u64) -> Option<&Host> {
        self.hosts.iter().find(|host| host.id == id)
    }

    pub fn host_mut(&mut self, id: u64) -> Option<&mut Host> {
        self.hosts.iter_mut().find(|host| host.id == id)
    }

    pub fn connection(&self, from: u64, to: u64) -> Option<&Connection> {
        self.connections
            .iter()
            .find(|conn| conn.from == from && conn.to == to)
    }

    pub fn connection_mut(&mut self, from: u64, to: u64) -> Option<&mut Connection> {
        self.connections
            .iter_mut()
            .find(|conn| conn.from == from && conn.to == to)
    }

    /// Add a host at the given position and connect it to every existing host
    /// in both directions.  Returns the new host's id.
    pub fn add_host(&mut self, name: String, x: f32, y: f32) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.hosts.push(Host::empty(id, name, x, y));

        let other_ids: Vec<u64> = self
            .hosts
            .iter()
            .map(|host| host.id)
            .filter(|other| *other != id)
            .collect();

        for other in other_ids {
            self.connections.push(Connection {
                from: id,
                to: other,
                enabled: true,
                endpoint: String::new(),
                preshared_key: String::new(),
            });
            self.connections.push(Connection {
                from: other,
                to: id,
                enabled: true,
                endpoint: String::new(),
                preshared_key: String::new(),
            });
        }

        id
    }

    pub fn remove_host(&mut self, id: u64) -> bool {
        let before = self.hosts.len();
        self.hosts.retain(|host| host.id != id);
        self.connections
            .retain(|conn| conn.from != id && conn.to != id);
        before != self.hosts.len()
    }

    /// Make sure the project is a full mesh and that ids / links are
    /// consistent.  Missing links are created enabled.  Returns true when the
    /// mesh was modified.
    pub fn normalize(&mut self) -> bool {
        let mut changed = false;

        // Remove duplicate host ids and duplicate connections.
        let mut seen_hosts = HashSet::new();
        let before = self.hosts.len();
        self.hosts.retain(|host| seen_hosts.insert(host.id));
        changed |= self.hosts.len() != before;

        let ids: HashSet<u64> = self.hosts.iter().map(|host| host.id).collect();
        let mut seen_links = HashSet::new();
        let before = self.connections.len();
        self.connections.retain(|conn| {
            conn.from != conn.to
                && ids.contains(&conn.from)
                && ids.contains(&conn.to)
                && seen_links.insert((conn.from, conn.to))
        });
        changed |= self.connections.len() != before;

        let max_id = self.hosts.iter().map(|host| host.id).max().unwrap_or(0);
        if self.next_id <= max_id {
            self.next_id = max_id + 1;
            changed = true;
        }

        changed |= self.ensure_full_mesh();
        changed
    }

    /// Add any missing directed links.  Existing (possibly disabled) links are
    /// left untouched.
    pub fn ensure_full_mesh(&mut self) -> bool {
        let mut changed = false;
        let ids: Vec<u64> = self.hosts.iter().map(|host| host.id).collect();
        for &from in &ids {
            for &to in &ids {
                if from == to {
                    continue;
                }
                if self.connection(from, to).is_none() {
                    self.connections.push(Connection {
                        from,
                        to,
                        enabled: true,
                        endpoint: String::new(),
                        preshared_key: String::new(),
                    });
                    changed = true;
                }
            }
        }
        changed
    }

    /// Endpoint that `from` uses to reach `to` after applying the connection
    /// override and the target's default endpoint and listen port.
    pub fn resolved_endpoint(&self, from: u64, to: u64) -> Option<String> {
        let connection = self.connection(from, to)?;
        let target = self.host(to)?;
        let explicit = connection.endpoint.trim();

        let raw = if explicit.is_empty() {
            target.endpoint.trim()
        } else {
            explicit
        };

        if raw.is_empty() || NO_ENDPOINT_VALUES.contains(&raw.to_ascii_lowercase().as_str()) {
            return None;
        }

        Some(format_endpoint(
            raw,
            target.effective_listen_port(&self.defaults),
        ))
    }

    /// Suggestions for the host inspector: the endpoint that would be used if
    /// the override were empty.
    pub fn default_endpoint_for(&self, to: u64) -> Option<String> {
        let target = self.host(to)?;
        let raw = target.endpoint.trim();
        if raw.is_empty() || NO_ENDPOINT_VALUES.contains(&raw.to_ascii_lowercase().as_str()) {
            return None;
        }
        Some(format_endpoint(
            raw,
            target.effective_listen_port(&self.defaults),
        ))
    }

    /// Pick a free host address from the mesh's address prefix.
    pub fn suggest_address(&self) -> String {
        let used: HashSet<u32> = self
            .hosts
            .iter()
            .filter_map(|host| first_ipv4(&host.address))
            .collect();

        let prefix = self.defaults.address_prefix.trim();
        if let Some((base, len)) = prefix.split_once('/')
            && let (Ok(base), Ok(prefix_len)) = (base.parse::<Ipv4Addr>(), len.parse::<u8>())
            && prefix_len <= 30
        {
            let host_bits = 32 - u32::from(prefix_len);
            let network = u32::from(base) & (!0u32 << host_bits);
            let max_offset = if host_bits >= 16 {
                65_534
            } else {
                (1u32 << host_bits).saturating_sub(2)
            };
            for offset in 1..=max_offset {
                let candidate = network.wrapping_add(offset);
                if !used.contains(&candidate) {
                    return format!("{}/32", Ipv4Addr::from(candidate));
                }
            }
        }

        // A partially typed prefix such as "10.100.0." is still useful.
        let mut n = 1u32;
        loop {
            let candidate = if prefix.ends_with('.') {
                format!("{prefix}{n}/32")
            } else {
                format!("10.100.0.{n}/32")
            };
            let already_used = self
                .hosts
                .iter()
                .any(|host| host.address.trim() == candidate);
            if !already_used {
                return candidate;
            }
            n += 1;
        }
    }

    pub fn unique_host_name(&self) -> String {
        let used: HashSet<&str> = self.hosts.iter().map(|host| host.name.as_str()).collect();
        let mut n = self.hosts.len() + 1;
        loop {
            let name = format!("host-{n}");
            if !used.contains(name.as_str()) {
                return name;
            }
            n += 1;
        }
    }

    pub fn centroid(&self) -> (f32, f32) {
        if self.hosts.is_empty() {
            return (0.0, 0.0);
        }
        let n = self.hosts.len() as f32;
        (
            self.hosts.iter().map(|host| host.x).sum::<f32>() / n,
            self.hosts.iter().map(|host| host.y).sum::<f32>() / n,
        )
    }

    /// Generate private keys for hosts that do not have one.  Returns the
    /// number of keys that were generated.
    pub fn ensure_private_keys(&mut self) -> usize {
        let mut generated = 0;
        for host in &mut self.hosts {
            if host.private_key.trim().is_empty() {
                host.private_key = crate::crypto::generate_private_key();
                generated += 1;
            }
        }
        generated
    }

    /// Replace every host private key and, when mesh-wide preshared keys are
    /// enabled, replace every pair key.  Returns the number of host keys
    /// rotated and whether preshared keys were rotated too.
    pub fn rotate_secrets(&mut self) -> (usize, bool) {
        let host_count = self.hosts.len();
        for host in &mut self.hosts {
            host.private_key = crate::crypto::generate_private_key();
        }

        let rotate_psks = self.defaults.preshared_keys;
        if rotate_psks {
            for connection in &mut self.connections {
                connection.preshared_key.clear();
            }
            self.sync_preshared_keys();
        }

        (host_count, rotate_psks)
    }

    /// Keep one preshared key per unordered host pair when mesh-wide PSKs are
    /// enabled.  Both directions always receive the same key.  Existing keys
    /// are preserved (the first direction with a key wins).
    pub fn sync_preshared_keys(&mut self) -> bool {
        if !self.defaults.preshared_keys {
            return false;
        }

        let ids: Vec<u64> = self.hosts.iter().map(|host| host.id).collect();
        let mut changed = false;

        for left in 0..ids.len() {
            for right in (left + 1)..ids.len() {
                let a = ids[left];
                let b = ids[right];
                let existing = self
                    .connection(a, b)
                    .map(|connection| connection.preshared_key.trim())
                    .filter(|key| !key.is_empty())
                    .map(str::to_owned)
                    .or_else(|| {
                        self.connection(b, a)
                            .map(|connection| connection.preshared_key.trim())
                            .filter(|key| !key.is_empty())
                            .map(str::to_owned)
                    });
                let key = existing.unwrap_or_else(crate::crypto::generate_preshared_key);

                for (from, to) in [(a, b), (b, a)] {
                    if let Some(connection) = self.connection_mut(from, to)
                        && connection.preshared_key != key
                    {
                        connection.preshared_key = key.clone();
                        changed = true;
                    }
                }
            }
        }

        changed
    }
}

fn first_ipv4(address: &str) -> Option<u32> {
    address
        .split(',')
        .next()?
        .trim()
        .split('/')
        .next()?
        .parse::<Ipv4Addr>()
        .ok()
        .map(u32::from)
}

/// Append a port to an endpoint if it does not already have one.
///
/// Bare IPv6 addresses are bracketed before the port is appended, matching the
/// syntax used by WireGuard's `Endpoint` option.
pub fn format_endpoint(endpoint: &str, port: u16) -> String {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return String::new();
    }

    if endpoint.starts_with('[') {
        if endpoint.ends_with(']') {
            return format!("{endpoint}:{port}");
        }
        return endpoint.to_owned();
    }

    let colon_count = endpoint.matches(':').count();
    if colon_count == 1 {
        if let Some((host, possible_port)) = endpoint.rsplit_once(':') {
            if !possible_port.is_empty() && possible_port.chars().all(|c| c.is_ascii_digit()) {
                return endpoint.to_owned();
            }
            if possible_port.is_empty() {
                return format!("{host}:{port}");
            }
            // A non-numeric port is invalid, but do not mangle it further.
            return endpoint.to_owned();
        }
    } else if colon_count > 1 {
        // Bare IPv6 address.
        return format!("[{endpoint}]:{port}");
    }

    format!("{endpoint}:{port}")
}

/// Split a comma or newline separated list into trimmed, non-empty items.
pub fn split_list(value: &str) -> Vec<String> {
    value
        .split([',', '\n'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_formatting() {
        assert_eq!(format_endpoint("example.com", 51820), "example.com:51820");
        assert_eq!(
            format_endpoint("example.com:1234", 51820),
            "example.com:1234"
        );
        assert_eq!(format_endpoint("2001:db8::1", 51820), "[2001:db8::1]:51820");
        assert_eq!(
            format_endpoint("[2001:db8::1]", 51820),
            "[2001:db8::1]:51820"
        );
        assert_eq!(
            format_endpoint("[2001:db8::1]:1234", 51820),
            "[2001:db8::1]:1234"
        );
    }

    #[test]
    fn full_mesh_is_created() {
        let mut mesh = Mesh::new();
        let a = mesh.add_host("a".into(), 0.0, 0.0);
        let b = mesh.add_host("b".into(), 100.0, 0.0);
        let c = mesh.add_host("c".into(), 0.0, 100.0);

        assert_eq!(mesh.connections.len(), 6);
        assert!(mesh.connection(a, b).is_some());
        assert!(mesh.connection(b, a).is_some());
        assert!(mesh.connection(a, c).is_some());
        assert!(mesh.connection(b, c).is_some());
    }

    #[test]
    fn resolving_uses_override_and_target_port() {
        let mut mesh = Mesh::new();
        let a = mesh.add_host("a".into(), 0.0, 0.0);
        let b = mesh.add_host("b".into(), 100.0, 0.0);
        mesh.host_mut(b).unwrap().endpoint = "b.example.com".into();
        mesh.host_mut(b).unwrap().listen_port = Some(5555);

        assert_eq!(
            mesh.resolved_endpoint(a, b).as_deref(),
            Some("b.example.com:5555")
        );

        mesh.connection_mut(a, b).unwrap().endpoint = "10.0.0.2".into();
        assert_eq!(
            mesh.resolved_endpoint(a, b).as_deref(),
            Some("10.0.0.2:5555")
        );

        mesh.connection_mut(a, b).unwrap().endpoint = "none".into();
        assert_eq!(mesh.resolved_endpoint(a, b), None);
    }

    #[test]
    fn address_suggestions_skip_used_addresses() {
        let mut mesh = Mesh::new();
        mesh.defaults.address_prefix = "10.42.0.0/24".into();
        assert_eq!(mesh.suggest_address(), "10.42.0.1/32");

        let a = mesh.add_host("a".into(), 0.0, 0.0);
        mesh.host_mut(a).unwrap().address = "10.42.0.1/32".into();
        assert_eq!(mesh.suggest_address(), "10.42.0.2/32");

        mesh.host_mut(a).unwrap().address = "10.42.0.7/32".into();
        assert_eq!(mesh.suggest_address(), "10.42.0.1/32");
    }

    #[test]
    fn no_endpoint_markers_and_empty_endpoint() {
        let mut mesh = Mesh::new();
        let a = mesh.add_host("a".into(), 0.0, 0.0);
        let b = mesh.add_host("b".into(), 0.0, 0.0);
        assert_eq!(mesh.resolved_endpoint(a, b), None);

        mesh.host_mut(b).unwrap().endpoint = "none".into();
        assert_eq!(mesh.resolved_endpoint(a, b), None);
    }

    #[test]
    fn rotating_secrets_replaces_keys_and_enabled_preshared_keys() {
        let mut mesh = Mesh::new();
        mesh.defaults.preshared_keys = true;
        let a = mesh.add_host("a".into(), 0.0, 0.0);
        let b = mesh.add_host("b".into(), 0.0, 0.0);
        mesh.ensure_private_keys();
        mesh.sync_preshared_keys();

        let old_a = mesh.host(a).unwrap().private_key.clone();
        let old_b = mesh.host(b).unwrap().private_key.clone();
        let old_psk = mesh.connection(a, b).unwrap().preshared_key.clone();

        let (count, rotated_psks) = mesh.rotate_secrets();
        assert_eq!(count, 2);
        assert!(rotated_psks);
        assert_ne!(mesh.host(a).unwrap().private_key, old_a);
        assert_ne!(mesh.host(b).unwrap().private_key, old_b);
        assert_ne!(mesh.connection(a, b).unwrap().preshared_key, old_psk);
        assert_eq!(
            mesh.connection(a, b).unwrap().preshared_key,
            mesh.connection(b, a).unwrap().preshared_key
        );

        mesh.defaults.preshared_keys = false;
        let existing_psk = mesh.connection(a, b).unwrap().preshared_key.clone();
        let (_, rotated_psks) = mesh.rotate_secrets();
        assert!(!rotated_psks);
        assert_eq!(mesh.connection(a, b).unwrap().preshared_key, existing_psk);
    }

    #[test]
    fn mesh_wide_preshared_keys_are_shared_by_both_directions() {
        let mut mesh = Mesh::new();
        mesh.defaults.preshared_keys = true;
        let a = mesh.add_host("a".into(), 0.0, 0.0);
        let b = mesh.add_host("b".into(), 0.0, 0.0);
        let c = mesh.add_host("c".into(), 0.0, 0.0);

        assert!(mesh.sync_preshared_keys());
        let ab = mesh.connection(a, b).unwrap().preshared_key.clone();
        let ba = mesh.connection(b, a).unwrap().preshared_key.clone();
        assert!(!ab.is_empty());
        assert_eq!(ab, ba);
        assert_eq!(
            mesh.connection(a, c).unwrap().preshared_key,
            mesh.connection(c, a).unwrap().preshared_key
        );

        // Running the sync again is a no-op once all pairs have keys.
        assert!(!mesh.sync_preshared_keys());

        // Editing one direction is propagated to the other.
        mesh.connection_mut(b, a).unwrap().preshared_key = crate::crypto::generate_preshared_key();
        assert!(mesh.sync_preshared_keys());
        // The first direction (a -> b) still has the old key and wins.
        assert_eq!(
            mesh.connection(a, b).unwrap().preshared_key,
            mesh.connection(b, a).unwrap().preshared_key
        );
    }

    #[test]
    fn private_keys_are_generated_only_when_missing() {
        let mut mesh = Mesh::new();
        let a = mesh.add_host("a".into(), 0.0, 0.0);
        let b = mesh.add_host("b".into(), 0.0, 0.0);
        mesh.host_mut(a).unwrap().private_key = crate::crypto::generate_private_key();
        let existing = mesh.host(a).unwrap().private_key.clone();

        assert_eq!(mesh.ensure_private_keys(), 1);
        assert_eq!(mesh.host(a).unwrap().private_key, existing);
        assert!(!mesh.host(b).unwrap().private_key.is_empty());
        assert_eq!(mesh.ensure_private_keys(), 0);
    }
}
