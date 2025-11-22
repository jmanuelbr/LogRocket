use notify::{Watcher, RecommendedWatcher, RecursiveMode, Event, EventKind};
use std::path::PathBuf;
use std::sync::mpsc;

pub struct FileWatcher {
    watcher: Option<RecommendedWatcher>,
    receiver: Option<mpsc::Receiver<notify::Result<Event>>>,
    path: Option<PathBuf>,
}

impl FileWatcher {
    pub fn new() -> Self {
        Self {
            watcher: None,
            receiver: None,
            path: None,
        }
    }

    pub fn watch_file(&mut self, path: PathBuf) -> Result<(), notify::Error> {
        // Stop existing watcher
        self.stop();
        
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        
        // Watch the parent directory to catch file modifications
        if let Some(parent) = path.parent() {
            watcher.watch(parent, RecursiveMode::NonRecursive)?;
        }
        
        self.watcher = Some(watcher);
        self.receiver = Some(rx);
        self.path = Some(path);
        
        Ok(())
    }

    pub fn stop(&mut self) {
        self.watcher = None;
        self.receiver = None;
        self.path = None;
    }

    pub fn check_for_changes(&mut self) -> bool {
        if let Some(receiver) = &self.receiver {
            let mut changed = false;
            while let Ok(Ok(event)) = receiver.try_recv() {
                if let EventKind::Modify(_) = event.kind {
                    if let Some(ref path) = self.path {
                        if event.paths.iter().any(|p| p == path) {
                            changed = true;
                        }
                    }
                }
            }
            changed
        } else {
            false
        }
    }

    pub fn is_watching(&self) -> bool {
        self.watcher.is_some()
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

