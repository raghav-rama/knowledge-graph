use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;

#[async_trait]
pub trait FileRepository: Send + Sync {
    async fn create_dir_all(&self, path: &Path) -> Result<()>;
    async fn rename(&self, from: &Path, to: &Path) -> Result<()>;
    async fn read(&self, path: &Path) -> Result<Vec<u8>>;
    fn exists(&self, path: &Path) -> bool;
}

#[derive(Debug, Default, Clone)]
pub struct FsFileRepository;

#[async_trait]
impl FileRepository for FsFileRepository {
    async fn create_dir_all(&self, path: &Path) -> Result<()> {
        tokio::fs::create_dir_all(path)
            .await
            .with_context(|| format!("failed to create directory {}", path.display()))
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        tokio::fs::rename(from, to)
            .await
            .with_context(|| format!("failed to move {} to {}", from.display(), to.display()))
    }

    async fn read(&self, path: &Path) -> Result<Vec<u8>> {
        tokio::fs::read(path)
            .await
            .with_context(|| format!("failed to read file {}", path.display()))
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

#[derive(Clone)]
pub struct DocumentManager {
    base_input_dir: PathBuf,
    workspace: Option<String>,
    supported_extensions: HashSet<String>,
    file_repo: Arc<dyn FileRepository>,
}

impl DocumentManager {
    pub async fn new<P>(
        input_dir: P,
        workspace: Option<String>,
        supported_extensions: &[&str],
    ) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        Self::with_repository(
            input_dir,
            workspace,
            supported_extensions,
            Arc::new(FsFileRepository::default()),
        )
        .await
    }

    pub async fn with_repository<P>(
        input_dir: P,
        workspace: Option<String>,
        supported_extensions: &[&str],
        file_repo: Arc<dyn FileRepository>,
    ) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut extensions = HashSet::new();
        for ext in supported_extensions {
            extensions.insert(normalize_extension(ext));
        }

        let base_input_dir = input_dir.as_ref().to_path_buf();
        let effective_dir = if let Some(ws) = workspace.as_deref() {
            base_input_dir.join(ws)
        } else {
            base_input_dir.clone()
        };

        file_repo
            .create_dir_all(&effective_dir)
            .await
            .with_context(|| {
                format!(
                    "failed to create input directory at {}",
                    effective_dir.display()
                )
            })?;

        Ok(Self {
            base_input_dir,
            workspace,
            supported_extensions: extensions,
            file_repo,
        })
    }

    pub fn input_dir(&self) -> PathBuf {
        if let Some(ws) = self.workspace.as_deref() {
            self.base_input_dir.join(ws)
        } else {
            self.base_input_dir.clone()
        }
    }

    pub fn is_supported_file(&self, filename: &str) -> bool {
        let ext = Path::new(filename)
            .extension()
            .and_then(|os| os.to_str())
            .map(normalize_extension);
        match ext {
            Some(ext) => self.supported_extensions.contains(&ext),
            None => false,
        }
    }

    pub fn sanitize_filename(&self, raw: &str) -> Result<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("filename cannot be empty"));
        }

        if trimmed.contains("..") || trimmed.contains('/') || trimmed.contains('\\') {
            return Err(anyhow!("invalid filename"));
        }

        Ok(trimmed.to_string())
    }

    pub async fn path_is_duplicate(&self, filename: &str) -> bool {
        let candidate = self.input_dir().join(filename);
        self.file_repo.exists(&candidate)
    }

    pub async fn move_to_enqueued(&self, file_path: &Path) -> Result<PathBuf> {
        let parent = file_path
            .parent()
            .ok_or_else(|| anyhow!("file has no parent directory"))?;
        let enqueued_dir = parent.join("__enqueued__");
        self.file_repo
            .create_dir_all(&enqueued_dir)
            .await
            .with_context(|| {
                format!(
                    "failed to create enqueued dir at {}",
                    enqueued_dir.display()
                )
            })?;

        let unique_name = self.unique_filename(&enqueued_dir, file_path)?;
        let target = enqueued_dir.join(&unique_name);
        self.file_repo.rename(file_path, &target).await?;
        Ok(target)
    }

    pub fn file_repo(&self) -> Arc<dyn FileRepository> {
        self.file_repo.clone()
    }

    fn unique_filename(&self, dir: &Path, file_path: &Path) -> Result<String> {
        let original = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("file name is missing"))?;

        let mut candidate_path = dir.join(original);
        if !self.file_repo.exists(&candidate_path) {
            return Ok(original.to_owned());
        }

        let (stem, ext) = match Path::new(original).file_stem().and_then(|s| s.to_str()) {
            Some(stem) => {
                let ext = Path::new(original)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                (stem.to_string(), ext.to_string())
            }
            None => (original.to_string(), String::new()),
        };

        let mut counter = 1usize;
        loop {
            let candidate_name = if ext.is_empty() {
                format!("{}_{}", stem, counter)
            } else {
                format!("{}_{}.{}", stem, counter, ext)
            };

            candidate_path = dir.join(&candidate_name);
            if !self.file_repo.exists(&candidate_path) {
                return Ok(candidate_name);
            }

            counter += 1;
        }
    }
}

pub fn normalize_extension(ext: &str) -> String {
    if let Some(stripped) = ext.strip_prefix('.') {
        stripped.to_ascii_lowercase()
    } else {
        ext.to_ascii_lowercase()
    }
}
