use crate::{counters::Counters, state::State};

/// Abstraction over the `.workflow/` directory.
///
/// Separates I/O from logic so that CLI commands can be tested
/// with an in-memory mock.
pub trait Store {
    type Error: std::error::Error;

    fn read_state(&self) -> Result<State, Self::Error>;
    fn write_state(&self, state: &State) -> Result<(), Self::Error>;

    fn read_counters(&self) -> Result<Counters, Self::Error>;
    fn write_counters(&self, counters: &Counters) -> Result<(), Self::Error>;

    fn append_approval(&self, record: &str) -> Result<(), Self::Error>;
    fn read_approvals(&self) -> Result<Vec<String>, Self::Error>;

    /// Check whether the workflow directory exists (project is initialized).
    fn is_initialized(&self) -> bool;

    /// Create the `.workflow/` directory and initial files.
    fn initialize(&self, state: &State, counters: &Counters) -> Result<(), Self::Error>;

    /// Write a file at a path relative to the project root.
    fn write_file(&self, rel_path: &str, content: &str) -> Result<(), Self::Error>;

    /// Read a file at a path relative to the project root.
    fn read_file(&self, rel_path: &str) -> Result<String, Self::Error>;

    /// Ensure a directory exists at a path relative to the project root.
    fn ensure_dir(&self, rel_path: &str) -> Result<(), Self::Error>;

    /// Check if a file exists at a path relative to the project root.
    fn file_exists(&self, rel_path: &str) -> bool;

    /// Find a directory matching a prefix pattern (e.g., "milestones/M-001-*").
    /// Returns the first match, or None.
    fn find_dir(&self, pattern: &str) -> Option<String>;

    /// Find a file matching a prefix pattern (e.g., "requirements/REQ-001-*").
    /// Returns the first match, or None.
    fn find_file(&self, pattern: &str) -> Option<String>;

    /// Find a file by name prefix anywhere under a root directory.
    /// E.g., find "T-001-" under "milestones/M-001-mvp/epics/E-001-auth".
    fn find_file_deep(&self, root: &str, name_prefix: &str) -> Option<String>;

    /// Find all files by name prefix anywhere under a root directory.
    fn find_all_files_deep(&self, root: &str, name_prefix: &str) -> Vec<String>;
}

/// File-system backed store that reads/writes to `.workflow/` under a project
/// root.
pub struct FsStore {
    root: std::path::PathBuf,
}

impl FsStore {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn workflow_dir(&self) -> std::path::PathBuf {
        self.root.join(".workflow")
    }

    fn state_path(&self) -> std::path::PathBuf {
        self.workflow_dir().join("state.yml")
    }

    fn counters_path(&self) -> std::path::PathBuf {
        self.workflow_dir().join("counters.yml")
    }

    fn approvals_path(&self) -> std::path::PathBuf {
        self.workflow_dir().join("approvals.ndjson")
    }
}

#[derive(Debug)]
pub enum FsStoreError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
}

impl std::fmt::Display for FsStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Yaml(e) => write!(f, "yaml error: {e}"),
        }
    }
}

impl std::error::Error for FsStoreError {}

impl From<std::io::Error> for FsStoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_yaml::Error> for FsStoreError {
    fn from(e: serde_yaml::Error) -> Self {
        Self::Yaml(e)
    }
}

impl Store for FsStore {
    type Error = FsStoreError;

    fn read_state(&self) -> Result<State, Self::Error> {
        let content = std::fs::read_to_string(self.state_path())?;
        Ok(serde_yaml::from_str(&content)?)
    }

    fn write_state(&self, state: &State) -> Result<(), Self::Error> {
        let yaml = serde_yaml::to_string(state)?;
        std::fs::write(self.state_path(), yaml)?;
        Ok(())
    }

    fn read_counters(&self) -> Result<Counters, Self::Error> {
        let path = self.counters_path();
        if !path.exists() {
            return Ok(Counters::new());
        }
        let content = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content)?)
    }

    fn write_counters(&self, counters: &Counters) -> Result<(), Self::Error> {
        let yaml = serde_yaml::to_string(counters)?;
        std::fs::write(self.counters_path(), yaml)?;
        Ok(())
    }

    fn append_approval(&self, record: &str) -> Result<(), Self::Error> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.approvals_path())?;
        writeln!(file, "{record}")?;
        Ok(())
    }

    fn read_approvals(&self) -> Result<Vec<String>, Self::Error> {
        let path = self.approvals_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(path)?;
        Ok(content.lines().map(String::from).collect())
    }

    fn is_initialized(&self) -> bool {
        self.workflow_dir().is_dir() && self.state_path().exists()
    }

    fn initialize(&self, state: &State, counters: &Counters) -> Result<(), Self::Error> {
        std::fs::create_dir_all(self.workflow_dir())?;
        self.write_state(state)?;
        self.write_counters(counters)?;
        std::fs::write(self.approvals_path(), "")?;
        std::fs::write(self.workflow_dir().join("unresolved.ndjson"), "")?;
        Ok(())
    }

    fn write_file(&self, rel_path: &str, content: &str) -> Result<(), Self::Error> {
        let path = self.root.join(rel_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    fn read_file(&self, rel_path: &str) -> Result<String, Self::Error> {
        Ok(std::fs::read_to_string(self.root.join(rel_path))?)
    }

    fn ensure_dir(&self, rel_path: &str) -> Result<(), Self::Error> {
        std::fs::create_dir_all(self.root.join(rel_path))?;
        Ok(())
    }

    fn file_exists(&self, rel_path: &str) -> bool {
        self.root.join(rel_path).exists()
    }

    fn find_dir(&self, pattern: &str) -> Option<String> {
        self.find_entry(pattern, true)
    }

    fn find_file(&self, pattern: &str) -> Option<String> {
        self.find_entry(pattern, false)
    }

    fn find_file_deep(&self, root: &str, name_prefix: &str) -> Option<String> {
        walk_for_file(&self.root.join(root), root, name_prefix)
    }

    fn find_all_files_deep(&self, root: &str, name_prefix: &str) -> Vec<String> {
        let mut results = Vec::new();
        walk_for_all_files(&self.root.join(root), root, name_prefix, &mut results);
        results
    }
}

fn walk_for_file(dir: &std::path::Path, rel: &str, prefix: &str) -> Option<String> {
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let rel_path = format!("{rel}/{name}");
        if entry.file_type().ok()?.is_dir() {
            if let Some(found) = walk_for_file(&entry.path(), &rel_path, prefix) {
                return Some(found);
            }
        } else if name.starts_with(prefix) {
            return Some(rel_path);
        }
    }
    None
}

fn walk_for_all_files(dir: &std::path::Path, rel: &str, prefix: &str, results: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let rel_path = format!("{rel}/{name}");
        if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            walk_for_all_files(&entry.path(), &rel_path, prefix, results);
        } else if name.starts_with(prefix) {
            results.push(rel_path);
        }
    }
}

impl FsStore {
    fn find_entry(&self, pattern: &str, want_dir: bool) -> Option<String> {
        let (parent, prefix) = pattern.rsplit_once('/')?;
        let prefix = prefix.trim_end_matches('*');
        let parent_path = self.root.join(parent);
        let entries = std::fs::read_dir(parent_path).ok()?;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().ok()?.is_dir();
            if name.starts_with(prefix) && is_dir == want_dir {
                return Some(format!("{parent}/{name}"));
            }
        }
        None
    }
}

/// In-memory store for testing. No file system access.
#[cfg(any(test, feature = "test-support"))]
pub struct MemStore {
    pub state: std::cell::RefCell<Option<State>>,
    pub counters: std::cell::RefCell<Counters>,
    pub approvals: std::cell::RefCell<Vec<String>>,
    pub initialized: std::cell::Cell<bool>,
    pub files: std::cell::RefCell<std::collections::HashMap<String, String>>,
}

#[cfg(any(test, feature = "test-support"))]
impl Default for MemStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "test-support"))]
impl MemStore {
    pub fn new() -> Self {
        Self {
            state: std::cell::RefCell::new(None),
            counters: std::cell::RefCell::new(Counters::new()),
            approvals: std::cell::RefCell::new(Vec::new()),
            initialized: std::cell::Cell::new(false),
            files: std::cell::RefCell::new(std::collections::HashMap::new()),
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
#[derive(Debug)]
pub struct MemStoreError(pub String);

#[cfg(any(test, feature = "test-support"))]
impl std::fmt::Display for MemStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(any(test, feature = "test-support"))]
impl std::error::Error for MemStoreError {}

#[cfg(any(test, feature = "test-support"))]
impl Store for MemStore {
    type Error = MemStoreError;

    fn read_state(&self) -> Result<State, Self::Error> {
        self.state
            .borrow()
            .clone()
            .ok_or_else(|| MemStoreError("no state".to_owned()))
    }

    fn write_state(&self, state: &State) -> Result<(), Self::Error> {
        *self.state.borrow_mut() = Some(state.clone());
        Ok(())
    }

    fn read_counters(&self) -> Result<Counters, Self::Error> {
        Ok(self.counters.borrow().clone())
    }

    fn write_counters(&self, counters: &Counters) -> Result<(), Self::Error> {
        *self.counters.borrow_mut() = counters.clone();
        Ok(())
    }

    fn append_approval(&self, record: &str) -> Result<(), Self::Error> {
        self.approvals.borrow_mut().push(record.to_owned());
        Ok(())
    }

    fn read_approvals(&self) -> Result<Vec<String>, Self::Error> {
        Ok(self.approvals.borrow().clone())
    }

    fn is_initialized(&self) -> bool {
        self.initialized.get()
    }

    fn initialize(&self, state: &State, counters: &Counters) -> Result<(), Self::Error> {
        self.write_state(state)?;
        self.write_counters(counters)?;
        self.write_file(".workflow/unresolved.ndjson", "")?;
        self.initialized.set(true);
        Ok(())
    }

    fn write_file(&self, rel_path: &str, content: &str) -> Result<(), Self::Error> {
        self.files
            .borrow_mut()
            .insert(rel_path.to_owned(), content.to_owned());
        Ok(())
    }

    fn read_file(&self, rel_path: &str) -> Result<String, Self::Error> {
        self.files
            .borrow()
            .get(rel_path)
            .cloned()
            .ok_or_else(|| MemStoreError(format!("file not found: {rel_path}")))
    }

    fn ensure_dir(&self, _rel_path: &str) -> Result<(), Self::Error> {
        Ok(()) // no-op for in-memory store
    }

    fn file_exists(&self, rel_path: &str) -> bool {
        self.files.borrow().contains_key(rel_path)
    }

    fn find_dir(&self, pattern: &str) -> Option<String> {
        self.find_entry(pattern, true)
    }

    fn find_file(&self, pattern: &str) -> Option<String> {
        self.find_entry(pattern, false)
    }

    fn find_file_deep(&self, root: &str, name_prefix: &str) -> Option<String> {
        let files = self.files.borrow();
        for key in files.keys() {
            if key.starts_with(root) {
                let file_name = key.rsplit_once('/').map_or(key.as_str(), |(_, n)| n);
                if file_name.starts_with(name_prefix) {
                    return Some(key.clone());
                }
            }
        }
        None
    }

    fn find_all_files_deep(&self, root: &str, name_prefix: &str) -> Vec<String> {
        let files = self.files.borrow();
        let mut results = Vec::new();
        for key in files.keys() {
            if key.starts_with(root) {
                let file_name = key.rsplit_once('/').map_or(key.as_str(), |(_, n)| n);
                if file_name.starts_with(name_prefix) {
                    results.push(key.clone());
                }
            }
        }
        results.sort();
        results
    }
}

#[cfg(any(test, feature = "test-support"))]
impl MemStore {
    /// Find an entry matching a prefix pattern.
    /// For dirs (`want_dir=true`): looks for paths with more components after
    /// the match. For files (`want_dir=false`): looks for paths that end at the
    /// match.
    fn find_entry(&self, pattern: &str, want_dir: bool) -> Option<String> {
        let (parent, prefix) = pattern.rsplit_once('/')?;
        let prefix = prefix.trim_end_matches('*');
        let files = self.files.borrow();
        for key in files.keys() {
            let Some(rest) = key.strip_prefix(parent).and_then(|r| r.strip_prefix('/')) else {
                continue;
            };
            if !rest.starts_with(prefix) {
                continue;
            }
            let Some(first_component) = rest.split('/').next() else {
                continue;
            };
            let has_more = rest.contains('/');
            if want_dir && has_more {
                return Some(format!("{parent}/{first_component}"));
            }
            if !want_dir && !has_more {
                return Some(format!("{parent}/{first_component}"));
            }
        }
        None
    }
}
