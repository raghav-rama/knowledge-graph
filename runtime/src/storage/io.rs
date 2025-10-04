use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use super::StorageResult;

pub async fn ensure_parent_dir(path: &Path) -> StorageResult<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).await?;
    }
    Ok(())
}

pub async fn read_json_file<T>(path: &Path) -> StorageResult<Option<T>>
where
    T: DeserializeOwned,
{
    match fs::read(path).await {
        Ok(bytes) => {
            if bytes.is_empty() {
                Ok(None)
            } else {
                let value = serde_json::from_slice::<T>(&bytes)?;
                Ok(Some(value))
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

/// Atomically write json to disk using a temp file + rename.
///
/// The write is fsync'd to ensure durability.
pub async fn write_json_file<T>(path: &Path, value: &T) -> StorageResult<()>
where
    T: Serialize,
{
    ensure_parent_dir(path).await?;

    let tmp_path = temp_path(path);

    let mut file = fs::File::create(&tmp_path).await?;
    let json = serde_json::to_vec_pretty(value)?;
    file.write_all(&json).await?;
    file.sync_all().await?;

    fs::rename(&tmp_path, path).await?;
    Ok(())
}

/// load json or default to an empty value.
pub async fn load_or_default<T>(path: &Path) -> StorageResult<T>
where
    T: DeserializeOwned + Default,
{
    match read_json_file::<T>(path).await? {
        Some(v) => Ok(v),
        None => Ok(T::default()),
    }
}

fn temp_path(path: &Path) -> PathBuf {
    let mut tmp = path.to_path_buf();
    let file_name = path
        .file_name()
        .map(|name| format!("{}.tmp", name.to_string_lossy()))
        .unwrap_or_else(|| "tmp.json".to_string());
    tmp.set_file_name(file_name);
    tmp
}
