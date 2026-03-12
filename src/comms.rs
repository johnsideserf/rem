use std::time::Instant;

const MESSAGES: &[&str] = &[
    "LV-426 BEACON DETECTED",
    "COMPANY DIRECTIVE 937 UPDATED",
    "CREW EXPENDABLE — PRIORITY OVERRIDE",
    "HYPERDRIVE COOLANT AT 47%",
    "GATEWAY STATION RELAY NOMINAL",
    "SPECIAL ORDER 937 — SCIENCE OFFICER EYES ONLY",
    "BIOSCAN ANOMALY — SECTOR 7G",
    "CORPORATE QUARTERLY REVIEW — ALL TERMINALS",
    "TOWING VESSEL NOSTROMO — REROUTING",
    "WEYLAND-YUTANI MEMO: PRODUCT RECALL XM-310",
    "DEEP SPACE RELAY — SIGNAL DEGRADED",
    "ATMOSPHERIC PROCESSOR — PRESSURE NOMINAL",
    "COLONY STATUS: LV-426 — NO RESPONSE",
    "SHUTTLE NARCISSUS — BEACON ACTIVE",
    "FLIGHT RECORDER RECOVERED — ANALYSIS PENDING",
    "CLASSIFIED: XENOMORPH SPECIMEN REQUEST",
    "MU-TH-UR UPLINK — HANDSHAKE COMPLETE",
    "TERRAFORMING PERMIT 2039-B APPROVED",
    "CRYO DECK MALFUNCTION — NON-CRITICAL",
    "SUPPLY DROP COORDINATES RECEIVED",
];

pub struct CommsState {
    shown: Vec<usize>,
    pub current: Option<(String, Instant)>,
    last_comms: Instant,
    rng_counter: u32,
}

impl CommsState {
    pub fn new() -> Self {
        Self { shown: Vec::new(), current: None, last_comms: Instant::now(), rng_counter: 0 }
    }
    pub fn tick(&mut self, idle_secs: u64) {
        if idle_secs < 20 || idle_secs >= 45 {
            self.current = None;
            return;
        }
        if let Some((_, ts)) = &self.current {
            if ts.elapsed().as_secs() >= 3 { self.current = None; }
        }
        if self.current.is_none() && self.last_comms.elapsed().as_secs() >= 8 {
            self.last_comms = Instant::now();
            self.pick_message();
        }
    }
    fn pick_message(&mut self) {
        if self.shown.len() >= MESSAGES.len() { self.shown.clear(); }
        let available: Vec<usize> = (0..MESSAGES.len()).filter(|i| !self.shown.contains(i)).collect();
        if available.is_empty() { return; }
        self.rng_counter = self.rng_counter.wrapping_mul(1103515245).wrapping_add(12345);
        let idx = available[(self.rng_counter as usize) % available.len()];
        self.shown.push(idx);
        self.current = Some((MESSAGES[idx].to_string(), Instant::now()));
    }
    pub fn dismiss(&mut self) {
        self.current = None;
        self.last_comms = Instant::now();
    }
}
