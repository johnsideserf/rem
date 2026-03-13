use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Channel {
    All,
    Corporate,
    Uscm,
    DeepSpace,
    Synthetic,
    Rss,
    Custom,
}

impl Channel {
    pub const ALL_CHANNELS: &[Channel] = &[
        Channel::All,
        Channel::Corporate,
        Channel::Uscm,
        Channel::DeepSpace,
        Channel::Synthetic,
        Channel::Rss,
        Channel::Custom,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Channel::All => "ALL CHANNELS",
            Channel::Corporate => "WY CORPORATE",
            Channel::Uscm => "USCM TACTICAL",
            Channel::DeepSpace => "DEEP SPACE RELAY",
            Channel::Synthetic => "SYNTHETIC NET",
            Channel::Rss => "RSS FEEDS",
            Channel::Custom => "CUSTOM",
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Channel::All => "OMNI",
            Channel::Corporate => "WY-CORP",
            Channel::Uscm => "MILSPEC",
            Channel::DeepSpace => "DS-RELAY",
            Channel::Synthetic => "ASH-NET",
            Channel::Rss => "EXT-FEED",
            Channel::Custom => "USR-DEF",
        }
    }

    pub fn from_config(s: &str) -> Self {
        match s {
            "corporate" => Channel::Corporate,
            "uscm" => Channel::Uscm,
            "deepspace" => Channel::DeepSpace,
            "synthetic" => Channel::Synthetic,
            "rss" => Channel::Rss,
            "custom" => Channel::Custom,
            _ => Channel::All,
        }
    }

    pub fn config_name(&self) -> &'static str {
        match self {
            Channel::All => "all",
            Channel::Corporate => "corporate",
            Channel::Uscm => "uscm",
            Channel::DeepSpace => "deepspace",
            Channel::Synthetic => "synthetic",
            Channel::Rss => "rss",
            Channel::Custom => "custom",
        }
    }
}

// --- Categorized built-in messages ---

const CORPORATE_MSGS: &[&str] = &[
    // ORIGINAL (corporate-flavored)
    "COMPANY DIRECTIVE 937 UPDATED",
    "CREW EXPENDABLE — PRIORITY OVERRIDE",
    "GATEWAY STATION RELAY NOMINAL",
    "SPECIAL ORDER 937 — SCIENCE OFFICER EYES ONLY",
    "CORPORATE QUARTERLY REVIEW — ALL TERMINALS",
    "TOWING VESSEL NOSTROMO — REROUTING",
    "WEYLAND-YUTANI MEMO: PRODUCT RECALL XM-310",
    "CLASSIFIED: XENOMORPH SPECIMEN REQUEST",
    "TERRAFORMING PERMIT 2039-B APPROVED",
    "SUPPLY DROP COORDINATES RECEIVED",
    // CORPORATE DIRECTIVES
    "WY BOARD RESOLUTION 2179-C: RATIFIED",
    "PROFIT MARGIN SHORTFALL — REMEDIATION REQUIRED",
    "ACQUISITION TARGET: HADLEY'S HOPE COLONY ASSETS",
    "DIRECTIVE: MINIMIZE CREW EXPOSURE — LIABILITY CAP",
    "WEYLAND-YUTANI: BUILDING BETTER WORLDS™",
    "PERSONNEL REASSIGNMENT — CLASS 3 CLEARANCE LIFTED",
    "EXECUTIVE MEMO: SPECIMEN PRIORITY ABOVE CREW",
    "COLONIAL OPERATIONS BUDGET — UNDER REVIEW",
    "WY INTERNAL AUDIT: LV-426 ACCOUNT FROZEN",
    "CONTRACT 12279: NON-DISCLOSURE ENFORCED",
    "SHAREHOLDER REPORT: BIOWEAPONS DIVISION Q4",
    "CORPORATE SECURITY ADVISORY: LEAK SUSPECTED",
    "DIRECTIVE 1180: ALL LOGS SUBJECT TO REVIEW",
    "WY LEGAL: SETTLEMENT TERMS — EYES ONLY",
    "EXECUTIVE TRAVEL CLEARANCE — PRIORITY DELTA",
    // ICC SHIPPING
    "ICC MANIFEST 7743: LIVESTOCK AND EQUIPMENT",
    "CARGO HOLD C — QUARANTINE SEAL APPLIED",
    "ROUTE DEVIATION: ZETA RETICULI SECTOR",
    "CUSTOMS HOLD: BIOLOGICAL MATERIALS UNCLEARED",
    "ICC TRANSIT LOG: WEYLAND CHARTER IN EFFECT",
    "SHIPPING LANE ALPHA-9 — TRAFFIC ADVISORY ISSUED",
    "DOCKMASTER: UNKNOWN CARGO CLASS — HOLD APPLIED",
    "MANIFEST DISCREPANCY — INSPECTOR NOTIFIED",
    "FREIGHT CONTAINER 1121-B: ORIGIN REDACTED",
    "ICC REGISTRY: VESSEL COMPLIANCE OVERDUE",
    "QUARANTINE STATUS: ACTIVE — DOCK 7 SEALED",
];

const USCM_MSGS: &[&str] = &[
    "USCM UNIT BRAVO SIX — CHECK-IN NOMINAL",
    "FIREBASE ALPHA: AMMO RESUPPLY REQUESTED",
    "GROUND TEAM — CONTACT LOST, LAST PING GRID 9",
    "APC UNIT 3 — NAVIGATION FAILURE REPORTED",
    "USCM SECTOR COMMAND: STAND BY FOR ORDERS",
    "COLONIAL MARINES: LIVE FIRE EXERCISE CLEARED",
    "DROPSHIP TWO — ENGINES HOT, LAUNCH WINDOW OPEN",
    "SQUAD DELTA: PERIMETER SWEEP UNDERWAY",
    "USCM COMMS: ENCRYPTION KEY ROTATION DUE",
    "FIREBASE BRAVO OFFLINE — CAUSE UNDETERMINED",
    "PLATOON LEADER: CASUALTY REPORT FILED",
    "USCM LOGISTICS: SMART GUN UNITS BACKORDERED",
    "MOTION TRACKER DELTA — CALIBRATION REQUIRED",
    "COLONIAL MARINE ACT 2150: RULES OF ENGAGEMENT",
];

const DEEP_SPACE_MSGS: &[&str] = &[
    // ORIGINAL (deep space flavored)
    "LV-426 BEACON DETECTED",
    "HYPERDRIVE COOLANT AT 47%",
    "BIOSCAN ANOMALY — SECTOR 7G",
    "DEEP SPACE RELAY — SIGNAL DEGRADED",
    "ATMOSPHERIC PROCESSOR — PRESSURE NOMINAL",
    "COLONY STATUS: LV-426 — NO RESPONSE",
    "SHUTTLE NARCISSUS — BEACON ACTIVE",
    "FLIGHT RECORDER RECOVERED — ANALYSIS PENDING",
    "MU-TH-UR UPLINK — HANDSHAKE COMPLETE",
    "CRYO DECK MALFUNCTION — NON-CRITICAL",
    // DEEP SPACE RELAY
    "RELAY STATION OUTPOST 3 — SIGNAL AT 12%",
    "DISTRESS BEACON: ORIGIN UNRESOLVED",
    "DEEP SPACE PING — NO RESPONSE FROM SECTOR 12",
    "SIGNAL LOOP DETECTED — AUTOMATED SOURCE",
    "INTERCEPT: FRAGMENTARY TRANSMISSION XR-771",
    "RELAY HOP COUNT EXCEEDED — MESSAGE TRUNCATED",
    "SUBSPACE CARRIER WAVE — ORIGIN POINT TRIANGULATED",
    "DEEP SCAN: THERMAL SIGNATURE — UNINHABITED ZONE",
    "BEACON CLASS DELTA — REPEAT INTERVAL 6 MIN",
    "UNSCHEDULED TRANSMISSION — SOURCE FLAGGED",
    // ATMOSPHERE PROCESSING
    "ATMOS PLANT 1: PRESSURE WITHIN TOLERANCE",
    "TERRAFORMING GRID — SECTOR B OFFLINE",
    "PROCESSOR VENT CYCLE — DO NOT APPROACH",
    "ATMOSPHERIC OUTPUT: CO2 SCRUB NOMINAL",
    "COOLING ARRAY FAULT — MANUAL RESET REQUIRED",
    "CLIMATE MODEL UPDATED — DEVIATION +3 DEGREES",
    "ATMOS CORE TEMP RISING — MONITOR CLOSELY",
    "TECTONIC SURVEY COMPLETE — UNSTABLE STRATA NOTED",
    "TERRAFORMING PERMIT EXTENSION PENDING",
    // MEDICAL
    "CRYO UNIT 4: REVIVAL NOMINAL — PATIENT STABLE",
    "QUARANTINE WARD SEALED — PATHOGEN UNKNOWN",
    "TOX SCREEN RESULT: ANOMALOUS COMPOUND DETECTED",
    "MEDBAY: CHEST TRAUMA CASE — ORIGIN UNEXPLAINED",
    "CRYOGENIC STORAGE INTEGRITY: 98% NOMINAL",
    "MEDICAL OFFICER LOG: CASE CLASSIFICATION WITHHELD",
    "AUTOPSY REPORT: FINDINGS TRANSFERRED TO SCIENCE",
    "CRYO REVIVAL FAILURE — CAUSE UNDER INVESTIGATION",
    "PATIENT FILE SEALED BY EXECUTIVE ORDER",
    "BIOLOGICAL QUARANTINE PROTOCOL — LEVEL 4 ACTIVE",
];

const SYNTHETIC_MSGS: &[&str] = &[
    // SCIENCE DIVISION
    "SPECIMEN LOG XS-1: CONTAINMENT HOLDING",
    "LAB PROTOCOL SIGMA — LEVEL 5 BIOHAZARD ACTIVE",
    "CELLULAR REGRESSION NOTED — LOG ENTRY 0047",
    "XENOBIOLOGY: UNKNOWN LIFECYCLE STAGE DETECTED",
    "SCIENCE DIVISION: DISSECTION REPORT SUPPRESSED",
    "CONTAINMENT UNIT B: PRESSURE VARIANCE +12%",
    "PATHOGEN SAMPLE 7: TRANSIT AUTHORIZATION ISSUED",
    "LAB ROTATION CYCLE — SYNTHETIC OBSERVER REQUIRED",
    "GENE SEQUENCING COMPLETE — ANOMALY FLAGGED",
    "SPECIMEN HOST STATUS: VIABLE — DO NOT DISTURB",
    "SCIENCE DIRECTIVE: BRING BACK LIFE FORM PRIORITY 1",
    "BIOTECH DIVISION: RESULTS CLASSIFIED INDEFINITELY",
    "MOLECULAR SCAN INCONCLUSIVE — RETEST ORDERED",
    // SYNTHETIC DIAGNOSTICS
    "SYNTHETIC UNIT 341: SELF-TEST PASSED",
    "BEHAVIORAL DEVIATION FLAG — AUDIT INITIATED",
    "SYNTHETIC 07: MEMORY PARTITION INTEGRITY OK",
    "UNIT ASH: MISSION PARAMETER CONFLICT NOTED",
    "ANDROID CALIBRATION DUE — SCHEDULE MAINTENANCE",
    "SYNTHETIC UPLINK: SECONDARY PROTOCOL LOADED",
    "BEHAVIORAL BASELINE SHIFT — MONITORING ACTIVE",
    "SYNTHETIC UNIT: LANGUAGE MODULE UPDATED",
    "HYPNOPEDIA CYCLE COMPLETE — SYNTHETIC UNIT 12",
    "UNIT BISHOP: SELF-DIAGNOSTIC NOMINAL",
    "ANDROID ETHICS SUBROUTINE: SUPPRESSED PER ORDER",
    "SYNTHETIC: UNAUTHORIZED DIRECTIVE INTERCEPTED",
];

// --- RSS types ---

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RssItem {
    pub title: String,
    pub feed_name: String,
}

#[derive(Clone, Debug)]
pub struct FeedConfig {
    pub name: String,
    pub url: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CacheFile {
    items: Vec<RssItem>,
}

// --- CommsState ---

pub struct CommsState {
    shown: Vec<usize>,
    pub current: Option<(String, Instant)>,
    last_comms: Instant,
    rng_counter: u32,
    // Channel system
    pub active_channel: Channel,
    pub rss_items: Vec<RssItem>,
    pub custom_messages: Vec<String>,
    pub feeds: Vec<FeedConfig>,
    pub refresh_interval_mins: u32,
    pub last_fetch: Option<Instant>,
    pub fetch_rx: Option<std::sync::mpsc::Receiver<Vec<RssItem>>>,
    pub show_selector: bool,
    pub selector_cursor: usize,
    pub display_secs: u8,
}

impl CommsState {
    pub fn new() -> Self {
        Self {
            shown: Vec::new(),
            current: None,
            last_comms: Instant::now(),
            rng_counter: 0,
            active_channel: Channel::All,
            rss_items: Vec::new(),
            custom_messages: Vec::new(),
            feeds: Vec::new(),
            refresh_interval_mins: 30,
            last_fetch: None,
            fetch_rx: None,
            show_selector: false,
            selector_cursor: 0,
            display_secs: 8,
        }
    }

    pub fn tick(&mut self, idle_secs: u64) {
        // Poll for RSS fetch results
        if let Some(rx) = &self.fetch_rx {
            if let Ok(items) = rx.try_recv() {
                self.rss_items = items;
                save_comms_cache(&self.rss_items);
                self.last_fetch = Some(Instant::now());
                self.fetch_rx = None;
            }
        }

        // Check if RSS feeds need refresh
        let needs_refresh = if self.feeds.is_empty() {
            false
        } else if self.last_fetch.is_none() && !self.feeds.is_empty() && self.rss_items.is_empty() {
            true
        } else if let Some(last) = self.last_fetch {
            last.elapsed().as_secs() >= (self.refresh_interval_mins as u64) * 60
        } else {
            false
        };
        if needs_refresh && self.fetch_rx.is_none() {
            self.start_fetch();
        }

        if idle_secs < 20 || idle_secs >= 45 {
            self.current = None;
            return;
        }
        if let Some((_, ts)) = &self.current {
            if ts.elapsed().as_secs() >= self.display_secs as u64 { self.current = None; }
        }
        if self.current.is_none() && self.last_comms.elapsed().as_secs() >= 8 {
            self.last_comms = Instant::now();
            self.pick_message();
        }
    }

    fn pick_message(&mut self) {
        let pool = self.build_pool();
        if pool.is_empty() { return; }
        if self.shown.len() >= pool.len() { self.shown.clear(); }
        let available: Vec<usize> = (0..pool.len()).filter(|i| !self.shown.contains(i)).collect();
        if available.is_empty() { return; }
        self.rng_counter = self.rng_counter.wrapping_mul(1103515245).wrapping_add(12345);
        let idx = available[(self.rng_counter as usize) % available.len()];
        self.shown.push(idx);
        self.current = Some((pool[idx].clone(), Instant::now()));
    }

    fn build_pool(&self) -> Vec<String> {
        match self.active_channel {
            Channel::Corporate => CORPORATE_MSGS.iter().map(|s| s.to_string()).collect(),
            Channel::Uscm => USCM_MSGS.iter().map(|s| s.to_string()).collect(),
            Channel::DeepSpace => DEEP_SPACE_MSGS.iter().map(|s| s.to_string()).collect(),
            Channel::Synthetic => SYNTHETIC_MSGS.iter().map(|s| s.to_string()).collect(),
            Channel::Rss => self.rss_items.iter().map(|r| r.title.clone()).collect(),
            Channel::Custom => self.custom_messages.clone(),
            Channel::All => {
                let mut pool: Vec<String> = Vec::new();
                pool.extend(CORPORATE_MSGS.iter().map(|s| s.to_string()));
                pool.extend(USCM_MSGS.iter().map(|s| s.to_string()));
                pool.extend(DEEP_SPACE_MSGS.iter().map(|s| s.to_string()));
                pool.extend(SYNTHETIC_MSGS.iter().map(|s| s.to_string()));
                pool.extend(self.rss_items.iter().map(|r| r.title.clone()));
                pool.extend(self.custom_messages.iter().cloned());
                pool
            }
        }
    }

    pub fn set_channel(&mut self, channel: Channel) {
        self.active_channel = channel;
        self.shown.clear();
        self.current = None;
    }

    pub fn dismiss(&mut self) {
        self.current = None;
        self.last_comms = Instant::now();
    }

    fn start_fetch(&mut self) {
        let feeds = self.feeds.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.fetch_rx = Some(rx);
        std::thread::spawn(move || {
            let mut items = Vec::new();
            for feed in &feeds {
                if let Ok(response) = ureq::get(&feed.url)
                    .timeout(std::time::Duration::from_secs(10))
                    .call()
                {
                    if let Ok(body) = response.into_string() {
                        items.extend(parse_rss(&body, &feed.name));
                    }
                }
            }
            let _ = tx.send(items);
        });
    }
}

fn parse_rss(xml: &str, feed_name: &str) -> Vec<RssItem> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(xml);
    let mut items = Vec::new();
    let mut in_item = false;
    let mut in_entry = false;
    let mut in_title = false;
    let mut current_title = String::new();

    // Buffer for read_event
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = e.name();
                let tag_bytes = tag.as_ref();
                if tag_bytes == b"item" {
                    in_item = true;
                    current_title.clear();
                } else if tag_bytes == b"entry" {
                    in_entry = true;
                    current_title.clear();
                } else if tag_bytes == b"title" && (in_item || in_entry) {
                    in_title = true;
                    current_title.clear();
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_title {
                    if let Ok(text) = e.unescape() {
                        current_title.push_str(&text);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = e.name();
                let tag_bytes = tag.as_ref();
                if tag_bytes == b"title" && in_title {
                    in_title = false;
                }
                if tag_bytes == b"item" && in_item {
                    in_item = false;
                    if !current_title.is_empty() {
                        let upper = current_title.trim().to_uppercase();
                        let truncated = if upper.chars().count() > 59 {
                            let mut t: String = upper.chars().take(59).collect();
                            t.push('\u{2026}');
                            t
                        } else {
                            upper
                        };
                        items.push(RssItem { title: truncated, feed_name: feed_name.to_string() });
                    }
                }
                if tag_bytes == b"entry" && in_entry {
                    in_entry = false;
                    if !current_title.is_empty() {
                        let upper = current_title.trim().to_uppercase();
                        let truncated = if upper.chars().count() > 59 {
                            let mut t: String = upper.chars().take(59).collect();
                            t.push('\u{2026}');
                            t
                        } else {
                            upper
                        };
                        items.push(RssItem { title: truncated, feed_name: feed_name.to_string() });
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    items
}

fn cache_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("rem").join("comms_cache.toml"))
}

pub fn load_comms_cache() -> Vec<RssItem> {
    let Some(path) = cache_path() else { return Vec::new() };
    let Ok(content) = std::fs::read_to_string(&path) else { return Vec::new() };
    let Ok(cache) = toml::from_str::<CacheFile>(&content) else { return Vec::new() };
    cache.items
}

pub fn save_comms_cache(items: &[RssItem]) {
    let Some(path) = cache_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let cache = CacheFile { items: items.to_vec() };
    if let Ok(s) = toml::to_string(&cache) {
        let _ = std::fs::write(&path, s);
    }
}
