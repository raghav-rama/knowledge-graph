use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{Map, Number, Value, map::Entry};
use tokio::sync::RwLock;

use super::KvStorage;
use super::io::{ensure_parent_dir, load_or_default, write_json_file};

#[derive(Clone, Debug)]
pub struct JsonKvStorageConfig {
    pub working_dir: PathBuf,
    pub namespace: String,
    pub workspace: Option<String>,
}

pub struct JsonKvStorage {
    namespace: String,
    final_namespace: String,
    file_path: PathBuf,
    data: Arc<RwLock<HashMap<String, Value>>>,
    dirty: AtomicBool,
}

impl JsonKvStorage {
    pub fn new(config: JsonKvStorageConfig) -> Self {
        let JsonKvStorageConfig {
            working_dir,
            namespace,
            workspace,
        } = config;

        let (workspace_prefix, workspace_dir) = match workspace.as_deref() {
            Some(ws) if !ws.is_empty() => (ws.to_string(), working_dir.join(ws)),
            _ => ("_".to_string(), working_dir.clone()),
        };

        let final_namespace = format!("{}_{}", workspace_prefix, namespace);
        let file_path = workspace_dir.join(format!("kv_store_{}.json", namespace));

        Self {
            namespace,
            final_namespace,
            file_path,
            data: Arc::new(RwLock::new(HashMap::new())),
            dirty: AtomicBool::new(false),
        }
    }

    fn namespace_requires_cache_list(&self) -> bool {
        self.namespace.ends_with("text_chunks")
    }

    fn current_unix_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    fn normalize_record(key: &str, value: &Value) -> Value {
        let mut obj = match value {
            Value::Object(map) => map.clone(),
            other => {
                let mut map = Map::new();
                map.insert("value".to_string(), other.clone());
                map
            }
        };

        obj.entry("create_time".to_string())
            .or_insert_with(|| Value::Number(Number::from(0)));
        obj.entry("update_time".to_string())
            .or_insert_with(|| Value::Number(Number::from(0)));
        obj.insert("_id".to_string(), Value::String(key.to_string()));

        Value::Object(obj)
    }

    fn decorate_upsert_record(&self, key: &str, value: Value) -> Result<Value> {
        let mut map = match value {
            Value::Object(map) => map,
            other => {
                let mut map = Map::new();
                map.insert("value".into(), other);
                map
            }
        };

        let now = Self::current_unix_timestamp();

        if self.namespace_requires_cache_list() {
            map.entry("llm_cache_list".to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
        }

        match map.entry("create_time".to_string()) {
            Entry::Occupied(_) => {
                map.insert("update_time".to_string(), Value::Number(Number::from(now)));
            }
            Entry::Vacant(entry) => {
                entry.insert(Value::Number(Number::from(now)));
                map.insert("update_time".to_string(), Value::Number(Number::from(now)));
            }
        }

        map.insert("_id".to_string(), Value::String(key.to_string()));

        Ok(Value::Object(map))
    }

    async fn migrate_legacy_cache_structure(
        &self,
        data: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>> {
        if data.is_empty() {
            return Ok(data);
        }

        let looks_flat = data
            .keys()
            .next()
            .map(|key| key.split(':').count() == 3)
            .unwrap_or(false);

        if looks_flat {
            return Ok(data);
        }

        let mut migrated = HashMap::with_capacity(data.len());
        let mut migration_count = 0usize;

        for (key, value) in data {
            match value {
                Value::Object(inner) if is_legacy_cache_structure(&inner) => {
                    let mode = key;
                    for (cache_hash, cache_entry) in inner {
                        if let Value::Object(entry_obj) = cache_entry {
                            let cache_type = entry_obj
                                .get("cache_type")
                                .and_then(Value::as_str)
                                .unwrap_or("extract");
                            let flattened_key = generate_cache_key(&mode, cache_type, &cache_hash);
                            migrated.insert(flattened_key, Value::Object(entry_obj));
                            migration_count += 1;
                        } else {
                            migrated.insert(cache_hash, cache_entry);
                        }
                    }
                }
                other => {
                    migrated.insert(key, other);
                }
            }
        }

        if migration_count > 0 {
            write_json_file(&self.file_path, &migrated)
                .await
                .with_context(|| {
                    format!("failed to persist migrated cache {}", self.final_namespace)
                })?;
        }

        Ok(migrated)
    }
}

#[async_trait]
impl KvStorage for JsonKvStorage {
    async fn initialize(&self) -> Result<()> {
        ensure_parent_dir(&self.file_path).await?;
        let data: HashMap<String, Value> = load_or_default(&self.file_path).await?;
        let migrated = self.migrate_legacy_cache_structure(data).await?;
        *self.data.write().await = migrated;
        self.dirty.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn finalize(&self) -> Result<()> {
        self.sync_if_dirty().await
    }

    async fn upsert(&self, records: HashMap<String, Value>) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        let mut guard = self.data.write().await;
        for (key, value) in records {
            let decorated = self
                .decorate_upsert_record(&key, value)
                .with_context(|| format!("invalid record for key {key}"))?;
            guard.insert(key, decorated);
        }
        self.dirty.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn delete(&self, ids: &[String]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let mut guard = self.data.write().await;
        let mut removed_any = false;
        for id in ids {
            if guard.remove(id).is_some() {
                removed_any = true;
            }
        }
        if removed_any {
            self.dirty.store(true, Ordering::SeqCst);
        }
        Ok(())
    }

    async fn drop_all(&self) -> Result<()> {
        {
            let mut guard = self.data.write().await;
            if guard.is_empty() {
                return Ok(());
            }
            guard.clear();
        }
        self.dirty.store(true, Ordering::SeqCst);
        self.sync_if_dirty().await
    }

    async fn get_all(&self) -> Result<HashMap<String, Value>> {
        let guard = self.data.read().await;
        Ok(guard
            .iter()
            .map(|(k, v)| (k.clone(), Self::normalize_record(k, v)))
            .collect())
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<Value>> {
        let guard = self.data.read().await;
        Ok(guard.get(id).map(|value| Self::normalize_record(id, value)))
    }

    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<Option<Value>>> {
        let guard = self.data.read().await;
        Ok(ids
            .iter()
            .map(|id| guard.get(id).map(|v| Self::normalize_record(id, v)))
            .collect())
    }

    async fn filter_keys(&self, keys: &HashSet<String>) -> Result<HashSet<String>> {
        let guard = self.data.read().await;
        let existing: HashSet<String> = guard.keys().cloned().collect();
        Ok(keys.difference(&existing).cloned().collect::<HashSet<_>>())
    }

    async fn sync_if_dirty(&self) -> Result<()> {
        if !self.dirty.swap(false, Ordering::SeqCst) {
            return Ok(());
        }

        let snapshot = {
            let guard = self.data.read().await;
            guard.clone()
        };

        write_json_file(&self.file_path, &snapshot)
            .await
            .with_context(|| format!("failed to write kv store {}", self.final_namespace))?;
        Ok(())
    }
}

fn is_legacy_cache_structure(inner: &Map<String, Value>) -> bool {
    inner
        .values()
        .all(|value| matches!(value, Value::Object(obj) if obj.contains_key("return")))
}

fn generate_cache_key(mode: &str, cache_type: &str, cache_hash: &str) -> String {
    format!("{mode}:{cache_type}:{cache_hash}")
}
