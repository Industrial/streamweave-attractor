//! Checkpoint save/load to run directory (JSON).

use crate::types::Checkpoint;
use std::path::Path;
use tracing::instrument;

/// Default filename for checkpoint under a run directory.
pub const CHECKPOINT_FILENAME: &str = "checkpoint.json";

/// Saves a checkpoint to `path` as JSON.
#[instrument(level = "trace", skip(path, cp))]
pub fn save_checkpoint(path: &Path, cp: &Checkpoint) -> Result<(), std::io::Error> {
  let json = serde_json::to_string_pretty(cp)
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent)?;
  }
  std::fs::write(path, json)
}

/// Loads a checkpoint from `path`. Returns error if file is missing or invalid JSON.
#[instrument(level = "trace", skip(path))]
pub fn load_checkpoint(path: &Path) -> Result<Checkpoint, std::io::Error> {
  let bytes = std::fs::read(path)?;
  serde_json::from_slice(&bytes)
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
