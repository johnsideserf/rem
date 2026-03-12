use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

pub struct DirWatcher {
    dir: PathBuf,
    last_check: Instant,
    last_mtime: Option<SystemTime>,
    tx: mpsc::Sender<()>,
}

impl DirWatcher {
    pub fn new(dir: PathBuf, tx: mpsc::Sender<()>) -> Self {
        let last_mtime = dir_mtime(&dir);
        Self { dir, last_check: Instant::now(), last_mtime, tx }
    }

    pub fn set_dir(&mut self, dir: PathBuf) {
        self.dir = dir.clone();
        self.last_mtime = dir_mtime(&dir);
        self.last_check = Instant::now();
    }

    pub fn poll(&mut self) {
        if self.last_check.elapsed() < Duration::from_secs(2) { return; }
        self.last_check = Instant::now();
        let current = dir_mtime(&self.dir);
        if current != self.last_mtime {
            self.last_mtime = current;
            let _ = self.tx.send(());
        }
    }
}

fn dir_mtime(dir: &PathBuf) -> Option<SystemTime> {
    std::fs::metadata(dir).ok().and_then(|m| m.modified().ok())
}

pub fn create_watcher(dir: PathBuf) -> (DirWatcher, mpsc::Receiver<()>) {
    let (tx, rx) = mpsc::channel();
    (DirWatcher::new(dir, tx), rx)
}
