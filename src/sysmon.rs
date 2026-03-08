use std::time::Instant;

use sysinfo::{Disks, Networks, System};

use crate::throbber::PaletteVariant;

const SPARKLINE_LEN: usize = 30;

/// Per-drive info snapshot.
pub struct DiskInfo {
    pub mount: String,
    pub total: u64,
    pub used: u64,
}

/// Per-interface network snapshot.
pub struct NetSnapshot {
    pub tx_bytes_sec: f64,
    pub rx_bytes_sec: f64,
    pub tx_sparkline: Vec<f64>,
    pub rx_sparkline: Vec<f64>,
}

/// All system telemetry data.
pub struct SysMon {
    sys: System,
    disks: Disks,
    networks: Networks,
    pub disk_info: Vec<DiskInfo>,
    pub net: NetSnapshot,
    pub cpu_pct: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    last_refresh: Instant,
    last_net_refresh: Instant,
    prev_tx: u64,
    prev_rx: u64,
    first_net: bool,
}

impl SysMon {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_all();
        sys.refresh_memory();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        let mut mon = Self {
            sys,
            disks,
            networks,
            disk_info: Vec::new(),
            net: NetSnapshot {
                tx_bytes_sec: 0.0,
                rx_bytes_sec: 0.0,
                tx_sparkline: vec![0.0; SPARKLINE_LEN],
                rx_sparkline: vec![0.0; SPARKLINE_LEN],
            },
            cpu_pct: 0.0,
            mem_used: 0,
            mem_total: 0,
            last_refresh: Instant::now(),
            last_net_refresh: Instant::now(),
            prev_tx: 0,
            prev_rx: 0,
            first_net: true,
        };
        mon.refresh_disks();
        mon.refresh_vitals();
        mon.snapshot_net_totals();
        mon
    }

    /// Call every tick (~100ms). Only actually refreshes data every 2 seconds.
    pub fn tick(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refresh).as_millis();
        if elapsed >= 2000 {
            self.last_refresh = now;
            self.refresh_disks();
            self.refresh_vitals();
            self.refresh_network();
        }
    }

    fn refresh_disks(&mut self) {
        self.disks.refresh(true);
        self.disk_info = self.disks.iter().map(|d| {
            let mount = d.mount_point().to_string_lossy().into_owned();
            let total = d.total_space();
            let avail = d.available_space();
            DiskInfo {
                mount,
                total,
                used: total.saturating_sub(avail),
            }
        }).collect();

        // Deduplicate by mount point (Windows can report same drive multiple times)
        self.disk_info.sort_by(|a, b| a.mount.cmp(&b.mount));
        self.disk_info.dedup_by(|a, b| a.mount == b.mount);
    }

    fn refresh_vitals(&mut self) {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.cpu_pct = self.sys.global_cpu_usage();
        self.mem_used = self.sys.used_memory();
        self.mem_total = self.sys.total_memory();
    }

    fn snapshot_net_totals(&mut self) {
        let (tx, rx) = self.net_totals();
        self.prev_tx = tx;
        self.prev_rx = rx;
        self.last_net_refresh = Instant::now();
    }

    fn net_totals(&self) -> (u64, u64) {
        let mut tx: u64 = 0;
        let mut rx: u64 = 0;
        for (_name, data) in self.networks.iter() {
            tx += data.total_transmitted();
            rx += data.total_received();
        }
        (tx, rx)
    }

    fn refresh_network(&mut self) {
        self.networks.refresh(true);

        if self.first_net {
            self.first_net = false;
            self.snapshot_net_totals();
            return;
        }

        let now = Instant::now();
        let dt = now.duration_since(self.last_net_refresh).as_secs_f64();
        if dt < 0.01 {
            return;
        }

        let (tx, rx) = self.net_totals();
        let tx_delta = tx.saturating_sub(self.prev_tx);
        let rx_delta = rx.saturating_sub(self.prev_rx);
        self.net.tx_bytes_sec = tx_delta as f64 / dt;
        self.net.rx_bytes_sec = rx_delta as f64 / dt;

        // Push to sparkline ring buffers
        self.net.tx_sparkline.push(self.net.tx_bytes_sec);
        if self.net.tx_sparkline.len() > SPARKLINE_LEN {
            self.net.tx_sparkline.remove(0);
        }
        self.net.rx_sparkline.push(self.net.rx_bytes_sec);
        if self.net.rx_sparkline.len() > SPARKLINE_LEN {
            self.net.rx_sparkline.remove(0);
        }

        self.prev_tx = tx;
        self.prev_rx = rx;
        self.last_net_refresh = now;
    }
}

/// Render a sparkline from a slice of values using palette-appropriate characters.
pub fn sparkline_str(values: &[f64], variant: PaletteVariant) -> String {
    if values.is_empty() {
        return String::new();
    }

    let max = values.iter().cloned().fold(0.0f64, f64::max);
    let max = if max < 1.0 { 1.0 } else { max };

    match variant {
        PaletteVariant::Green => {
            // Braille vertical: use lower dots for low values, upper for high
            // Map 0..1 to braille patterns ⡀ ⡄ ⡆ ⡇ ⣇ ⣧ ⣷ ⣿
            const BRAILLE: &[char] = &[' ', '⡀', '⡄', '⡆', '⡇', '⣇', '⣧', '⣷', '⣿'];
            values.iter().map(|&v| {
                let idx = ((v / max) * 8.0).round() as usize;
                BRAILLE[idx.min(BRAILLE.len() - 1)]
            }).collect()
        }
        PaletteVariant::Amber => {
            // Block elements
            const BLOCKS: &[char] = &[' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
            values.iter().map(|&v| {
                let idx = ((v / max) * 8.0).round() as usize;
                BLOCKS[idx.min(BLOCKS.len() - 1)]
            }).collect()
        }
        PaletteVariant::Cyan => {
            // Glitchy sparse — randomly skip some frames for degradation feel
            const GLITCH: &[char] = &[' ', '⠁', '⠃', '⠇', '⡇', '⡏', '⡟', '⡿', '⣿'];
            values.iter().enumerate().map(|(i, &v)| {
                // Every 7th sample "drops out" for signal degradation
                if i % 7 == 3 {
                    ' '
                } else {
                    let idx = ((v / max) * 8.0).round() as usize;
                    GLITCH[idx.min(GLITCH.len() - 1)]
                }
            }).collect()
        }
    }
}

/// Format bytes/sec as human-readable throughput.
pub fn format_throughput(bytes_sec: f64) -> String {
    if bytes_sec < 1024.0 {
        format!("{:.0} B/s", bytes_sec)
    } else if bytes_sec < 1024.0 * 1024.0 {
        format!("{:.1} KB/s", bytes_sec / 1024.0)
    } else {
        format!("{:.1} MB/s", bytes_sec / (1024.0 * 1024.0))
    }
}

/// Format bytes as human-readable capacity.
pub fn format_capacity(bytes: u64) -> String {
    if bytes < 1024 * 1024 * 1024 {
        format!("{} MB", bytes / (1024 * 1024))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
