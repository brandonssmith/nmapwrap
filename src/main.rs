use eframe::egui::{self, Visuals};
use serde::Deserialize;
use std::net::IpAddr;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

/// ---------------------------------------------------------------------------
/// Entry point
/// ---------------------------------------------------------------------------
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 720.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Nmap GUI Wrapper",
        options,
        Box::new(|cc| {
            // Load theme
            let mut visuals = Visuals::dark();
            if let Some(stored) = cc.storage.and_then(|s| s.get_string("theme")) {
                if stored == "light" {
                    visuals = Visuals::light();
                }
            }
            cc.egui_ctx.set_visuals(visuals);

            // Auto-detect gateway
            let gateway = detect_default_gateway();

            Ok(Box::new(NmapApp {
                gateway,
                ..Default::default()
            }))
        }),
    )
}

/// ---------------------------------------------------------------------------
/// Application state
/// ---------------------------------------------------------------------------
#[derive(Default)]
struct NmapApp {
    gateway: Option<IpAddr>,
    scan_running: bool,
    hosts: Vec<Host>,
    raw_output: String,

    scan_rx: Option<mpsc::Receiver<ScanResult>>,
}

#[derive(Clone, Debug)]
struct Host {
    ip: IpAddr,
    hostname: Option<String>,
}

#[derive(Debug)]
struct ScanResult {
    raw: String,
    hosts: Vec<Host>,
}

impl eframe::App for NmapApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --------------------------------------------------- Top bar (theme)
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Nmap GUI Wrapper");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut is_dark = ctx.style().visuals.dark_mode;
                    if ui.checkbox(&mut is_dark, "Dark mode").changed() {
                        let visuals = if is_dark { Visuals::dark() } else { Visuals::light() };
                        ctx.set_visuals(visuals.clone());

                        if let Some(storage) = _frame.storage_mut() {
                            storage.set_string("theme", if is_dark { "dark" } else { "light" }.to_string());
                        }
                    }
                });
            });
        });

        // --------------------------------------------------- Central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            // ---- 1. Default gateway
            ui.horizontal(|ui| {
                ui.label("Default gateway:");
                match self.gateway {
                    Some(ip) => ui.monospace(ip.to_string()),
                    None => ui.label("not found"),
                }
            });

            ui.separator();

            // ---- 2. Scan button
            let can_scan = self.gateway.is_some() && !self.scan_running;
            if can_scan && ui.button("Scan /24").clicked() {
                self.start_scan(ctx.clone());
            }
            if self.scan_running {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Scanning + resolving DNS…");
                });
            }

            ui.separator();

            // ---- 3. Host list
            ui.label("Live hosts:");
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if self.hosts.is_empty() {
                        ui.label("(none yet)");
                    } else {
                        ui.horizontal(|ui| {
                            if ui.button("Copy all").clicked() {
                                let text = self
                                    .hosts
                                    .iter()
                                    .map(|h| match &h.hostname {
                                        Some(name) => format!("{}  ({})", h.ip, name),
                                        None => h.ip.to_string(),
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                ui.output_mut(|o| o.copied_text = text);
                            }
                            ui.label(format!("{} host(s)", self.hosts.len()));
                        });

                        for h in &self.hosts {
                            let line = match &h.hostname {
                                Some(name) => format!("{}  ({})", h.ip, name),
                                None => format!("{}  (no hostname)", h.ip),
                            };
                            ui.monospace(&line);
                        }
                    }
                });

            // ---- Raw output (for debugging)
            if !self.raw_output.is_empty() {
                ui.collapsing("Raw nmap output", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(150.0)
                        .show(ui, |ui| ui.monospace(&self.raw_output));
                });
            }
        });

        // --------------------------------------------------- Poll scan result
        if let Some(rx) = &self.scan_rx {
            if let Ok(result) = rx.try_recv() {
                self.hosts = result.hosts;
                self.raw_output = result.raw;
                self.scan_running = false;
                self.scan_rx = None;
            }
        }

        // Keep UI responsive during scan
        if self.scan_running {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}
}

impl NmapApp {
    fn start_scan(&mut self, ctx: egui::Context) {
        let gw = match self.gateway {
            Some(ip) => ip,
            None => return,
        };
        let subnet = format!("{}/24", gw);

        self.scan_running = true;
        self.hosts.clear();
        self.raw_output.clear();

        let (tx, rx) = mpsc::channel::<ScanResult>();
        self.scan_rx = Some(rx);

        thread::spawn(move || {
            let result = run_nmap(&subnet);
            let _ = tx.send(result);
        });

        ctx.request_repaint();
    }
}

// -----------------------------------------------------------------------------
// 1. Detect default gateway
// -----------------------------------------------------------------------------
fn detect_default_gateway() -> Option<IpAddr> {
    let output = if cfg!(target_os = "windows") {
        Command::new("netsh")
            .args(["interface", "ip", "show", "address"])
            .stdout(Stdio::piped())
            .output()
            .ok()?
            .stdout
    } else {
        Command::new("ip")
            .args(["route", "show", "default"])
            .stdout(Stdio::piped())
            .output()
            .ok()?
            .stdout
    };

    let text = String::from_utf8_lossy(&output);
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        for (i, token) in parts.iter().enumerate() {
            if *token == "via" || token.contains("Gateway") {
                if let Some(ip_str) = parts.get(i + 1).or_else(|| parts.get(i + 2)) {
                    let cleaned = ip_str.trim_matches(|c: char| !c.is_numeric() && c != '.');
                    if let Ok(ip) = cleaned.parse() {
                        return Some(ip);
                    }
                }
            }
        }
    }
    None
}

// -----------------------------------------------------------------------------
// 2. Run nmap with DNS resolution
// -----------------------------------------------------------------------------
fn run_nmap(subnet: &str) -> ScanResult {
    let output = Command::new("nmap")
        .args([
            "-sn",                     // ping scan
            "-R",                      // always do reverse DNS
            "--dns-servers", "1.1.1.1", // fast public DNS
            "--host-timeout", "5s",
            "-oX", "-",
            subnet,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let out = match output {
        Ok(o) => o,
        Err(e) => return ScanResult {
            raw: format!("Failed to start nmap: {e}"),
            hosts: vec![],
        },
    };

    let xml = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let raw = format!("STDOUT:\n{xml}\n\nSTDERR:\n{stderr}");

    let nmaprun: NmapRun = match serde_xml_rs::from_str(&xml) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("XML parse error: {e}");
            return ScanResult { raw, hosts: vec![] };
        }
    };

    let hosts = nmaprun
        .hosts
        .into_iter()
        .filter(|h| h.status.state == "up")
        .filter_map(|h| {
            let ipv4 = h.addresses.iter().find(|a| a.addrtype == "ipv4")?;
            let ip: IpAddr = ipv4.addr.parse().ok()?;
            let hostname = h.hostnames.and_then(|hn| {
                hn.hostname.into_iter().next().map(|n| n.name)
            });
            Some(Host { ip, hostname })
        })
        .collect();

    ScanResult { raw, hosts }
}

// -----------------------------------------------------------------------------
// XML structures – fully robust
// -----------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
struct NmapRun {
    #[serde(rename = "host")]
    hosts: Vec<NmapHost>,
}

#[derive(Debug, Deserialize)]
struct NmapHost {
    status: Status,
    #[serde(rename = "address", default = "Vec::new")]
    addresses: Vec<Address>,
    #[serde(default)]
    hostnames: Option<Hostnames>,
}

#[derive(Debug, Deserialize)]
struct Status {
    state: String,
}

#[derive(Debug, Deserialize)]
struct Address {
    addr: String,
    addrtype: String,
}

#[derive(Debug, Deserialize, Default)]
struct Hostnames {
    #[serde(rename = "hostname", default = "Vec::new")]
    hostname: Vec<Hostname>,
}

#[derive(Debug, Deserialize)]
struct Hostname {
    name: String,
}