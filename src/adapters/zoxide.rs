use crate::domain::entry::Entry;
use crate::domain::error::AdapterError;
use crate::domain::path_name::basename_from_path;
use std::process::Command;

pub trait ZoxideSource {
    fn query(&self, limit: usize) -> Result<Vec<String>, AdapterError>;
    fn directories(&self, limit: usize) -> Result<Vec<Entry>, AdapterError>;
}

pub struct ZoxideAdapter;

impl ZoxideAdapter {
    pub fn new() -> Self {
        Self
    }

    #[cfg(test)]
    fn parse_dirs(output: &str) -> Vec<Entry> {
        output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let path = line.trim();
                let name = basename_from_path(path);
                Entry::zoxide(name, path.to_string())
            })
            .collect()
    }
}

impl Default for ZoxideAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ZoxideSource for ZoxideAdapter {
    fn query(&self, limit: usize) -> Result<Vec<String>, AdapterError> {
        let output = Command::new("zoxide")
            .args(["query", "-l"])
            .output()
            .map_err(|e| AdapterError::ZoxideCommand {
                command: "query -l".to_string(),
                detail: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                return Ok(vec![]);
            }
            return Err(AdapterError::ZoxideCommand {
                command: "query -l".to_string(),
                detail: stderr,
            });
        }

        let paths: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(String::from)
            .take(limit)
            .collect();
        Ok(paths)
    }

    fn directories(&self, limit: usize) -> Result<Vec<Entry>, AdapterError> {
        let paths = self.query(limit)?;
        Ok(paths
            .into_iter()
            .map(|path| {
                let name = basename_from_path(&path);
                Entry::zoxide(name, path)
            })
            .collect())
    }
}

pub struct FakeZoxideSource {
    pub paths: Vec<String>,
}

impl FakeZoxideSource {
    pub fn new() -> Self {
        Self { paths: vec![] }
    }

    pub fn with_dirs(dirs: &[&str]) -> Self {
        Self {
            paths: dirs.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl Default for FakeZoxideSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ZoxideSource for FakeZoxideSource {
    fn query(&self, _limit: usize) -> Result<Vec<String>, AdapterError> {
        Ok(self.paths.clone())
    }

    fn directories(&self, limit: usize) -> Result<Vec<Entry>, AdapterError> {
        let entries: Vec<Entry> = self
            .paths
            .iter()
            .take(limit)
            .map(|path| {
                let name = basename_from_path(path);
                Entry::zoxide(name, path.clone())
            })
            .collect();
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entry::EntryType;

    #[test]
    fn parse_dirs_extracts_name_from_path() {
        let input = "/home/user/project1\n/home/user/project2";
        let entries = ZoxideAdapter::parse_dirs(input);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].display, "▤ project1");
        assert_eq!(entries[1].display, "▤ project2");
        assert_eq!(entries[0].target, "/home/user/project1");
        assert_eq!(entries[0].entry_type, EntryType::Zoxide);
    }

    #[test]
    fn parse_dirs_skips_empty_lines() {
        let input = "/home/user/a\n\n/home/user/b";
        let entries = ZoxideAdapter::parse_dirs(input);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn fake_zoxide_returns_configured_paths() {
        let fake = FakeZoxideSource::with_dirs(&["/home/a", "/home/b"]);
        let entries = fake.directories(10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].target, "/home/a");
    }

    #[test]
    fn fake_zoxide_respects_limit() {
        let fake = FakeZoxideSource::with_dirs(&["/a", "/b", "/c"]);
        let entries = fake.directories(2).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn fake_zoxide_empty() {
        let fake = FakeZoxideSource::new();
        let entries = fake.directories(10).unwrap();
        assert!(entries.is_empty());
    }
}
