use crate::{Config, Error, Result};
use fs2::FileExt;
use std::{fs::{self, File, OpenOptions}, io::Write, path::{Path, PathBuf}};

pub struct Store {
    path: PathBuf,
    _lock: File,
}

impl Store {
    pub fn default_path() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("io", "infinuts", "EPF")
            .ok_or_else(|| Error::Storage("Could not find your configuration directory.".into()))?;
        Ok(dirs.config_dir().join("config.json"))
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(storage)?;
        let lock = OpenOptions::new().create(true).truncate(false).read(true).write(true)
            .open(path.with_extension("lock")).map_err(storage)?;
        lock.try_lock_exclusive().map_err(|_| Error::Busy)?;
        Ok(Self { path, _lock: lock })
    }

    pub fn load(&self) -> Result<Config> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
            Err(e) => return Err(storage(e)),
        };
        let config: Config = serde_json::from_slice(&bytes).map_err(|e|
            Error::Storage(format!("Could not read {}: {e}. The file has been kept for recovery.", self.path.display())))?;
        if config.schema_version != 1 {
            return Err(Error::Storage(format!("Configuration version {} is not supported. Update EPF before using this file.", config.schema_version)));
        }
        Ok(config)
    }

    pub fn save(&self, config: &Config) -> Result<()> {
        let parent = self.path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(storage)?;
        let json = serde_json::to_vec_pretty(config).map_err(storage)?;
        temp.write_all(&json).map_err(storage)?;
        temp.as_file().sync_all().map_err(storage)?;
        temp.persist(&self.path).map_err(storage)?;
        Ok(())
    }
}

fn storage(e: impl std::fmt::Display) -> Error {
    Error::Storage(format!("Could not access EPF's configuration: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locks_and_atomically_replaces_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let store = Store::open(&path).unwrap();
        assert!(matches!(Store::open(&path), Err(Error::Busy)));
        store.save(&Config::default()).unwrap();
        store.save(&Config::default()).unwrap();
        assert_eq!(store.load().unwrap().schema_version, 1);
        drop(store);
        assert!(Store::open(&path).is_ok());
    }

    #[test]
    fn preserves_corrupt_and_future_configs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let store = Store::open(&path).unwrap();
        for content in ["{broken", r#"{"schemaVersion":99}"#] {
            fs::write(&path, content).unwrap();
            assert!(store.load().is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), content);
        }
    }
}
