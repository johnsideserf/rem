use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Instant;

use crate::app::{App, BgOperation, OpMessage, OpType, PendingAction};
use crate::throbber::{Throbber, ThrobberKind};

/// Threshold: operations with more than this many items go to background.
const BG_THRESHOLD: usize = 5;

impl App {
    pub fn do_rename(&mut self) {
        if self.rename_buf.is_empty() {
            return;
        }
        if let Some(entry) = self.current_entry() {
            let old_path = entry.path.clone();
            let new_path = old_path.parent()
                .unwrap_or(&old_path)
                .join(&self.rename_buf);

            if new_path.exists() {
                self.error = Some((
                    format!("NAME CONFLICT: {}", self.rename_buf),
                    Instant::now(),
                ));
                return;
            }

            if let Err(e) = std::fs::rename(&old_path, &new_path) {
                self.error = Some((format!("RENAME FAILED: {}", e), Instant::now()));
            } else {
                self.load_entries();
            }
        }
    }

    pub fn do_create(&mut self, is_dir: bool) {
        if self.create_buf.is_empty() {
            return;
        }
        let path = self.pane().current_dir.join(&self.create_buf);

        if path.exists() {
            self.error = Some((
                format!("ALREADY EXISTS: {}", self.create_buf),
                Instant::now(),
            ));
            return;
        }

        let result = if is_dir {
            std::fs::create_dir(&path)
        } else {
            std::fs::File::create(&path).map(|_| ())
        };

        match result {
            Ok(()) => self.load_entries(),
            Err(e) => {
                self.error = Some((format!("CREATE FAILED: {}", e), Instant::now()));
            }
        }
    }

    pub fn paste(&mut self) {
        let buffer = match self.op_buffer.take() {
            Some(b) => b,
            None => {
                self.error = Some(("BUFFER EMPTY".to_string(), Instant::now()));
                return;
            }
        };

        let dest_dir = self.pane().current_dir.clone();

        // Check for conflicts first
        for src in &buffer.paths {
            let file_name = match src.file_name() {
                Some(n) => n.to_owned(),
                None => continue,
            };
            let dest = dest_dir.join(&file_name);
            if dest.exists() {
                self.error = Some((
                    format!("CONFLICT: {}", file_name.to_string_lossy()),
                    Instant::now(),
                ));
                self.op_buffer = Some(buffer);
                return;
            }
        }

        // Decide: background or foreground
        if buffer.paths.len() >= BG_THRESHOLD {
            self.paste_background(buffer, dest_dir);
        } else {
            self.paste_foreground(buffer, dest_dir);
        }
    }

    fn paste_foreground(&mut self, buffer: crate::app::OpBuffer, dest_dir: PathBuf) {
        let mut had_error = false;

        for src in &buffer.paths {
            let file_name = match src.file_name() {
                Some(n) => n.to_owned(),
                None => continue,
            };
            let dest = dest_dir.join(&file_name);

            let result = match buffer.op {
                OpType::Copy => copy_path(src, &dest),
                OpType::Cut => std::fs::rename(src, &dest),
            };

            if let Err(e) = result {
                self.error = Some((format!("PASTE FAILED: {}", e), Instant::now()));
                had_error = true;
                break;
            }
        }

        if !had_error {
            self.visual_marks.clear();
            if buffer.op == OpType::Copy {
                self.op_buffer = Some(buffer);
            }
        }

        self.load_entries();
        // Refresh other pane if in dual mode
        if self.dual_pane {
            let other = 1 - self.active_pane;
            let other_dir = self.panes[other].current_dir.clone();
            if other_dir == dest_dir {
                let old = self.active_pane;
                self.active_pane = other;
                self.load_entries();
                self.active_pane = old;
            }
        }
    }

    fn paste_background(&mut self, buffer: crate::app::OpBuffer, dest_dir: PathBuf) {
        let (tx, rx) = mpsc::channel();
        let label = match buffer.op {
            OpType::Copy => format!("COPYING {} ITEMS", buffer.paths.len()),
            OpType::Cut => format!("MOVING {} ITEMS", buffer.paths.len()),
        };

        let op = buffer.op;
        let paths = buffer.paths.clone();
        let total = paths.len() as u64;

        // Keep copy buffer for re-paste
        if buffer.op == OpType::Copy {
            self.op_buffer = Some(buffer);
        }
        self.visual_marks.clear();

        let variant = self.palette.variant;
        self.bg_operation = Some(BgOperation {
            label: label.clone(),
            throbber: Throbber::new(ThrobberKind::DataStream, variant),
            done: 0,
            total,
            current_file: String::new(),
            receiver: rx,
            started: Instant::now(),
        });

        std::thread::spawn(move || {
            for (i, src) in paths.iter().enumerate() {
                let file_name = match src.file_name() {
                    Some(n) => n.to_string_lossy().into_owned(),
                    None => continue,
                };
                let dest = dest_dir.join(&file_name);

                let _ = tx.send(OpMessage::Progress {
                    done: i as u64,
                    total,
                    current_file: file_name.clone(),
                });

                let result = match op {
                    OpType::Copy => copy_path(src, &dest),
                    OpType::Cut => std::fs::rename(src, &dest),
                };

                if let Err(e) = result {
                    let _ = tx.send(OpMessage::Error(format!("PASTE FAILED: {} — {}", file_name, e)));
                    return;
                }
            }
            let _ = tx.send(OpMessage::Complete);
        });
    }

    pub fn execute_confirmed(&mut self, action: &PendingAction) {
        match action {
            PendingAction::Delete { paths } => {
                if paths.len() >= BG_THRESHOLD {
                    self.delete_background(paths.clone());
                } else {
                    for path in paths {
                        let result = if path.is_dir() {
                            std::fs::remove_dir_all(path)
                        } else {
                            std::fs::remove_file(path)
                        };
                        if let Err(e) = result {
                            self.error = Some((format!("DELETE FAILED: {}", e), Instant::now()));
                            break;
                        }
                    }
                    self.visual_marks.clear();
                    self.load_entries();
                }
            }
            PendingAction::Overwrite { src, dest } => {
                if let Err(e) = copy_path(src, dest) {
                    self.error = Some((format!("OVERWRITE FAILED: {}", e), Instant::now()));
                }
                self.load_entries();
            }
        }
    }

    fn delete_background(&mut self, paths: Vec<PathBuf>) {
        let (tx, rx) = mpsc::channel();
        let total = paths.len() as u64;
        let label = format!("DELETING {} ITEMS", total);

        self.visual_marks.clear();

        let variant = self.palette.variant;
        self.bg_operation = Some(BgOperation {
            label: label.clone(),
            throbber: Throbber::new(ThrobberKind::DataStream, variant),
            done: 0,
            total,
            current_file: String::new(),
            receiver: rx,
            started: Instant::now(),
        });

        std::thread::spawn(move || {
            for (i, path) in paths.iter().enumerate() {
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();

                let _ = tx.send(OpMessage::Progress {
                    done: i as u64,
                    total,
                    current_file: name.clone(),
                });

                let result = if path.is_dir() {
                    std::fs::remove_dir_all(path)
                } else {
                    std::fs::remove_file(path)
                };

                if let Err(e) = result {
                    let _ = tx.send(OpMessage::Error(format!("DELETE FAILED: {} — {}", name, e)));
                    return;
                }
            }
            let _ = tx.send(OpMessage::Complete);
        });
    }
}

fn copy_path(src: &PathBuf, dest: &PathBuf) -> std::io::Result<()> {
    if src.is_dir() {
        copy_dir_recursive(src, dest)
    } else {
        std::fs::copy(src, dest).map(|_| ())
    }
}

fn copy_dir_recursive(src: &PathBuf, dest: &PathBuf) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}
