use std::sync::Arc;

use super::{DocStatusStorage, KvStorage, StorageResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoragesStatus {
    Created,
    Initialized,
}

impl Default for StoragesStatus {
    fn default() -> Self {
        StoragesStatus::Created
    }
}

enum ManagedStorage {
    Kv(Arc<dyn KvStorage>),
    DocStatus(Arc<dyn DocStatusStorage>),
}

impl ManagedStorage {
    async fn initialize(&self) -> StorageResult<()> {
        match self {
            ManagedStorage::Kv(storage) => storage.initialize().await,
            ManagedStorage::DocStatus(storage) => storage.initialize().await,
        }
    }

    async fn finalize(&self) -> StorageResult<()> {
        match self {
            ManagedStorage::Kv(storage) => storage.finalize().await,
            ManagedStorage::DocStatus(storage) => storage.finalize().await,
        }
    }
}

/// sequentially initializes registered backends to avoid deadlocks
pub struct StorageManager {
    status: StoragesStatus,
    storages: Vec<ManagedStorage>,
}

impl StorageManager {
    pub fn new() -> Self {
        Self {
            status: StoragesStatus::Created,
            storages: Vec::new(),
        }
    }

    pub fn status(&self) -> StoragesStatus {
        self.status
    }

    pub fn register_kv<T>(&mut self, storage: Arc<T>)
    where
        T: KvStorage + 'static,
    {
        let storage: Arc<dyn KvStorage> = storage;
        self.storages.push(ManagedStorage::Kv(storage));
    }

    pub fn register_doc_status<T>(&mut self, storage: Arc<T>)
    where
        T: DocStatusStorage + 'static,
    {
        let storage: Arc<dyn DocStatusStorage> = storage;
        self.storages.push(ManagedStorage::DocStatus(storage));
    }

    pub fn is_empty(&self) -> bool {
        self.storages.is_empty()
    }

    pub async fn initialize_all(&mut self) -> StorageResult<()> {
        if self.status == StoragesStatus::Initialized {
            return Ok(());
        }

        for storage in &self.storages {
            storage.initialize().await?;
        }

        self.status = StoragesStatus::Initialized;
        Ok(())
    }

    pub async fn finalize_all(&self) -> StorageResult<()> {
        for storage in &self.storages {
            storage.finalize().await?;
        }
        Ok(())
    }
}
