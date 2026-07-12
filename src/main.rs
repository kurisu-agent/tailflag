// tailflag — SNI tray icon showing Tailscale exit-node status.
//
// Icon states:
//   - tailscale not running (stopped / NeedsLogin / no daemon): all-dim dot grid
//   - running, no exit node: dot grid with bright bottom row (the Tailscale mark)
//   - exit node active, country known: that country's flag
//   - exit node active, country unknown: dot grid with green bottom row
//
// Country resolution order for the active exit node:
//   1. Peer.Location.CountryCode (set for Mullvad nodes)
//   2. TAILFLAG_LOCATIONS env map ("host=cc,host2=cc2", matched on HostName)
//   3. Mullvad hostname convention (cc-city-wg-NNN on *.mullvad.ts.net)
//
// Flag PNGs are read from $TAILFLAG_FLAG_DIR/<size>/<cc>.png (sizes 24/48),
// baked at build time from lipis/flag-icons.

use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;

use ksni::blocking::TrayMethods;
use ksni::menu::{MenuItem, StandardItem};
use ksni::{Icon, ToolTip, Tray};

const ICON_SIZES: [u32; 2] = [24, 48];
const POLL_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, PartialEq)]
enum State {
    /// Daemon unreachable, stopped, or logged out.
    Stopped(String),
    /// Running with no exit node configured.
    NoExit,
    /// Running with an exit node configured.
    Exit(ExitInfo),
}

#[derive(Clone, Debug, PartialEq)]
struct ExitInfo {
    host: String,
    country: Option<String>, // lowercase ISO 3166-1 alpha-2
    city: Option<String>,
    ip: Option<String>,
    online: bool,
}

/// Parse TAILFLAG_LOCATIONS ("host=cc,host2=cc2") into a lookup map.
fn location_overrides() -> HashMap<String, String> {
    std::env::var("TAILFLAG_LOCATIONS")
        .unwrap_or_default()
        .split(',')
        .filter_map(|kv| {
            let (host, cc) = kv.split_once('=')?;
            let (host, cc) = (host.trim(), cc.trim());
            (!host.is_empty() && cc.len() == 2)
                .then(|| (host.to_lowercase(), cc.to_lowercase()))
        })
        .collect()
}

fn poll_state(overrides: &HashMap<String, String>) -> State {
    if let Ok(demo) = std::env::var("TAILFLAG_DEMO") {
        return demo_state(&demo);
    }
    let out = match Command::new("tailscale").args(["status", "--json"]).output() {
        Ok(o) if o.status.success() => o.stdout,
        Ok(o) => {
            return State::Stopped(format!(
                "tailscale status failed: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            ))
        }
        Err(e) => return State::Stopped(format!("tailscale not available: {e}")),
    };
    let status: serde_json::Value = match serde_json::from_slice(&out) {
        Ok(v) => v,
        Err(e) => return State::Stopped(format!("bad status JSON: {e}")),
    };

    let backend = status["BackendState"].as_str().unwrap_or("");
    if backend != "Running" {
        return State::Stopped(format!("tailscale is {}", backend.to_lowercase()));
    }

    // ExitNodeStatus is non-null iff an exit node is configured; the matching
    // Peer entry (by ID) carries hostname and Mullvad Location metadata.
    let exit = &status["ExitNodeStatus"];
    if exit.is_null() {
        return State::NoExit;
    }
    let exit_id = exit["ID"].as_str().unwrap_or("");
    let online = exit["Online"].as_bool().unwrap_or(false);

    let peer = status["Peer"]
        .as_object()
        .into_iter()
        .flat_map(|peers| peers.values())
        .find(|p| p["ID"].as_str() == Some(exit_id));

    let info = match peer {
        None => ExitInfo {
            host: format!("unknown peer ({exit_id})"),
            country: None,
            city: None,
            ip: exit["TailscaleIPs"][0]
                .as_str()
                .map(|ip| ip.split('/').next().unwrap_or(ip).to_string()),
            online,
        },
        Some(p) => {
            let host = p["HostName"].as_str().unwrap_or(exit_id).to_string();
            let dns = p["DNSName"].as_str().unwrap_or("").trim_end_matches('.');
            let country = p["Location"]["CountryCode"]
                .as_str()
                .map(|c| c.to_lowercase())
                .or_else(|| overrides.get(&host.to_lowercase()).cloned())
                .or_else(|| mullvad_country(dns));
            ExitInfo {
                country,
                city: p["Location"]["City"].as_str().map(str::to_string),
                ip: p["TailscaleIPs"][0].as_str().map(str::to_string),
                host,
                online,
            }
        }
    };
    State::Exit(info)
}

/// Mullvad exit nodes are named `<cc>-<city>-wg-NNN.mullvad.ts.net`; only
/// trust the two-letter prefix when the suffix proves it's a Mullvad node.
fn mullvad_country(dns_name: &str) -> Option<String> {
    let (host, domain) = dns_name.split_once('.')?;
    if !domain.ends_with("mullvad.ts.net") {
        return None;
    }
    let cc = host.get(..2)?;
    (host.as_bytes().get(2) == Some(&b'-') && cc.chars().all(|c| c.is_ascii_alphabetic()))
        .then(|| cc.to_lowercase())
}

fn demo_state(demo: &str) -> State {
    match demo {
        "stopped" => State::Stopped("tailscale is stopped".into()),
        "none" => State::NoExit,
        cc => State::Exit(ExitInfo {
            host: format!("demo-{cc}"),
            country: (cc != "unknown").then(|| cc.to_lowercase()),
            city: Some("Demoville".into()),
            ip: Some("100.64.0.1".into()),
            online: true,
        }),
    }
}

// ---------------------------------------------------------------------------
// Icon rendering

/// Draw the Tailscale-style 3x3 dot grid: top two rows in `dim`, bottom row
/// in `bottom` (RGB). Returns an ARGB32 (network byte order) SNI icon.
fn grid_icon(size: u32, dim: [u8; 3], bottom: [u8; 3]) -> Icon {
    let s = size as f32;
    let radius = s * 0.13;
    let centers = [s * 0.18, s * 0.5, s * 0.82];
    let mut data = vec![0u8; (size * size * 4) as usize];
    for (row, &cy) in centers.iter().enumerate() {
        let rgb = if row == 2 { bottom } else { dim };
        for &cx in &centers {
            for y in 0..size {
                for x in 0..size {
                    let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                    let dist = (dx * dx + dy * dy).sqrt();
                    // 1px-wide smoothstep edge for cheap antialiasing
                    let cov = (radius - dist + 0.5).clamp(0.0, 1.0);
                    let px = ((y * size + x) * 4) as usize;
                    let a = (cov * 255.0) as u8;
                    if a > data[px] {
                        data[px..px + 4].copy_from_slice(&[a, rgb[0], rgb[1], rgb[2]]);
                    }
                }
            }
        }
    }
    Icon {
        width: size as i32,
        height: size as i32,
        data,
    }
}

const DIM: [u8; 3] = [0x9a, 0x9a, 0x9a];
const BRIGHT: [u8; 3] = [0xff, 0xff, 0xff];
const GREEN: [u8; 3] = [0x4c, 0xd9, 0x64];

fn unknown_exit_icons() -> Vec<Icon> {
    ICON_SIZES.iter().map(|&s| grid_icon(s, DIM, GREEN)).collect()
}

/// Load `$TAILFLAG_FLAG_DIR/<size>/<cc>.png` for each icon size, converting
/// RGBA rows to the ARGB32 byte order SNI wants. None if any size is missing.
fn load_flag_icons(cc: &str) -> Option<Vec<Icon>> {
    let dir = std::env::var("TAILFLAG_FLAG_DIR").ok()?;
    if cc.len() != 2 || !cc.chars().all(|c| c.is_ascii_lowercase()) {
        return None;
    }
    let radius_fraction = corner_radius_fraction();
    ICON_SIZES
        .iter()
        .map(|size| {
            let path = format!("{dir}/{size}/{cc}.png");
            let mut decoder = png::Decoder::new(std::fs::File::open(path).ok()?);
            // rsvg-convert emits RGB for fully-opaque flags; expand to RGBA8
            decoder.set_transformations(
                png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
            );
            let mut reader = decoder.read_info().ok()?;
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).ok()?;
            if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
                return None;
            }
            let mut data: Vec<u8> = buf[..info.buffer_size()]
                .chunks_exact(4)
                .flat_map(|p| [p[3], p[0], p[1], p[2]])
                .collect();
            round_corners(&mut data, info.width, info.height, radius_fraction);
            Some(Icon {
                width: info.width as i32,
                height: info.height as i32,
                data,
            })
        })
        .collect()
}

/// Flag border style from TAILFLAG_STYLE, as a corner-radius fraction of
/// the icon size: "square" (0.0), "rounded" (0.25), "circle" (0.5, the
/// default), or a bare fraction like "0.35".
fn corner_radius_fraction() -> f32 {
    match std::env::var("TAILFLAG_STYLE").as_deref() {
        Ok("square") => 0.0,
        Ok("rounded") => 0.25,
        Err(_) | Ok("circle") => 0.5,
        Ok(other) => other.parse().map(|f: f32| f.clamp(0.0, 0.5)).unwrap_or(0.5),
    }
}

/// Soften the square flags: multiply alpha by an antialiased
/// rounded-rectangle coverage mask (signed-distance based).
fn round_corners(argb: &mut [u8], width: u32, height: u32, radius_fraction: f32) {
    if radius_fraction <= 0.0 {
        return;
    }
    let (w, h) = (width as f32, height as f32);
    let r = w.min(h) * radius_fraction;
    // half-extents of the inner rect whose corners the radius wraps
    let (bx, by) = (w / 2.0 - r, h / 2.0 - r);
    for y in 0..height {
        for x in 0..width {
            let dx = ((x as f32 + 0.5 - w / 2.0).abs() - bx).max(0.0);
            let dy = ((y as f32 + 0.5 - h / 2.0).abs() - by).max(0.0);
            let sdf = (dx * dx + dy * dy).sqrt() - r;
            let cov = (0.5 - sdf).clamp(0.0, 1.0);
            if cov < 1.0 {
                let px = ((y * width + x) * 4) as usize;
                argb[px] = (argb[px] as f32 * cov) as u8;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tray

struct Tailflag {
    state: State,
    icons: Vec<Icon>,
    overrides: HashMap<String, String>,
    flag_cache: HashMap<String, Option<Vec<Icon>>>,
}

impl Tailflag {
    fn new(overrides: HashMap<String, String>) -> Self {
        let mut tray = Tailflag {
            state: State::Stopped("starting".into()),
            icons: Vec::new(),
            overrides,
            flag_cache: HashMap::new(),
        };
        tray.refresh();
        tray
    }

    fn set_state(&mut self, state: State) {
        self.icons = match &state {
            State::Stopped(_) => ICON_SIZES.iter().map(|&s| grid_icon(s, DIM, DIM)).collect(),
            State::NoExit => ICON_SIZES
                .iter()
                .map(|&s| grid_icon(s, DIM, BRIGHT))
                .collect(),
            State::Exit(info) => match &info.country {
                Some(cc) => self
                    .flag_cache
                    .entry(cc.clone())
                    .or_insert_with(|| load_flag_icons(cc))
                    .clone()
                    .unwrap_or_else(unknown_exit_icons),
                None => unknown_exit_icons(),
            },
        };
        self.state = state;
    }

    fn refresh(&mut self) {
        let state = poll_state(&self.overrides);
        self.set_state(state);
    }

    /// Short human line for the current state: state text, or just the
    /// exit node's hostname (no flag emoji / IP noise).
    fn status_line(&self) -> String {
        match &self.state {
            State::Stopped(why) => why.clone(),
            State::NoExit => "no exit node — routing directly".into(),
            State::Exit(info) => {
                let mut line = info.host.clone();
                if !info.online {
                    line.push_str(" — OFFLINE");
                }
                line
            }
        }
    }
}

impl Tray for Tailflag {
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        "tailflag".into()
    }

    fn title(&self) -> String {
        "Tailscale exit node".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        self.icons.clone()
    }

    fn tool_tip(&self) -> ToolTip {
        // Tooltip carries the location detail the icon can't
        let description = match &self.state {
            State::Exit(info) => {
                let loc = [info.city.as_deref(), info.country.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(", ");
                if loc.is_empty() {
                    self.status_line()
                } else {
                    format!("{} — {loc}", self.status_line())
                }
            }
            _ => self.status_line(),
        };
        ToolTip {
            title: match &self.state {
                State::Stopped(_) => "Tailscale: not running".into(),
                State::NoExit => "Tailscale: no exit node".into(),
                State::Exit(_) => "Tailscale exit node".into(),
            },
            description,
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![StandardItem {
            label: self.status_line(),
            enabled: false,
            ..Default::default()
        }
        .into()]
    }
}

fn main() {
    let overrides = location_overrides();

    if std::env::args().any(|a| a == "--status") {
        let state = poll_state(&overrides);
        println!("{state:#?}");
        if let State::Exit(ExitInfo {
            country: Some(cc), ..
        }) = &state
        {
            match load_flag_icons(cc) {
                Some(icons) => println!("flag '{cc}': loaded ({} sizes)", icons.len()),
                None => println!("flag '{cc}': NOT FOUND — will show green fallback"),
            }
        }
        return;
    }

    let tray = Tailflag::new(overrides);

    // assume_sni_available: we may start before the COSMIC panel's status
    // area registers the watcher; keep waiting instead of exiting.
    let handle = tray
        .assume_sni_available(true)
        .spawn()
        .expect("failed to start SNI tray service");

    loop {
        std::thread::sleep(POLL_INTERVAL);
        let alive = handle.update(Tailflag::refresh).is_some();
        if !alive {
            break;
        }
    }
}
