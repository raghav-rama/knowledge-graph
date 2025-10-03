use std::collections::{HashMap, HashSet};

use backend::storage::{
    DocProcessingStatus, DocStatus, DocStatusStorage, JsonDocStatusConfig, JsonDocStatusStorage,
    JsonKvStorage, JsonKvStorageConfig, KvStorage,
};
use serde_json::json;
use tempfile::TempDir;

fn temp_working_dir() -> TempDir {
    TempDir::new().expect("create temp dir")
}

#[tokio::test]
async fn json_kv_roundtrip_delete_and_reload() -> anyhow::Result<()> {
    let dir = temp_working_dir();
    let config = JsonKvStorageConfig {
        working_dir: dir.path().into(),
        namespace: "kv_roundtrip".to_string(),
        workspace: None,
    };

    let storage = JsonKvStorage::new(config.clone());
    storage.initialize().await?;

    let mut records = HashMap::new();
    records.insert("doc-1".to_string(), json!({"value": 1}));
    records.insert(
        "doc-2".to_string(),
        json!({"value": 2, "create_time": 1, "update_time": 1}),
    );
    storage.upsert(records).await?;
    storage.sync_if_dirty().await?;

    let stored = storage.get_by_id("doc-1").await?;
    assert!(stored.is_some());

    let mut filter_keys = HashSet::new();
    filter_keys.insert("doc-1".to_string());
    filter_keys.insert("missing".to_string());
    let existing = storage.filter_keys(&filter_keys).await?;
    assert!(existing.contains("doc-1"));
    assert!(!existing.contains("missing"));

    let reopened = JsonKvStorage::new(config.clone());
    reopened.initialize().await?;
    let all = reopened.get_all().await?;
    assert_eq!(all.len(), 2);
    assert!(all.contains_key("doc-1"));

    reopened.delete(&["doc-1".to_string()]).await?;
    reopened.sync_if_dirty().await?;
    assert!(reopened.get_by_id("doc-1").await?.is_none());

    reopened.drop_all().await?;
    assert!(reopened.get_all().await?.is_empty());

    Ok(())
}

#[tokio::test]
async fn json_kv_migrates_legacy_cache_structure() -> anyhow::Result<()> {
    let dir = temp_working_dir();
    let config = JsonKvStorageConfig {
        working_dir: dir.path().into(),
        namespace: "legacy_cache".to_string(),
        workspace: None,
    };

    let file_path = dir
        .path()
        .join(format!("kv_store_{}.json", config.namespace));

    let legacy = json!({
        "modeA": {
            "hash1": {
                "cache_type": "extract",
                "return": "value1"
            },
            "hash2": {
                "cache_type": "embed",
                "return": "value2"
            }
        }
    });

    tokio::fs::create_dir_all(dir.path()).await?;
    tokio::fs::write(&file_path, legacy.to_string()).await?;

    let storage = JsonKvStorage::new(config);
    storage.initialize().await?;

    let all = storage.get_all().await?;
    assert!(all.contains_key("modeA:extract:hash1"));
    assert!(all.contains_key("modeA:embed:hash2"));

    Ok(())
}

#[tokio::test]
async fn json_doc_status_roundtrip_and_pagination() -> anyhow::Result<()> {
    let dir = temp_working_dir();
    let config = JsonDocStatusConfig {
        working_dir: dir.path().into(),
        namespace: "doc_status".to_string(),
        workspace: Some("workspace".to_string()),
    };

    let storage = JsonDocStatusStorage::new(config.clone());
    storage.initialize().await?;

    let docs: HashMap<_, _> = vec![
        (
            "doc-1".to_string(),
            DocProcessingStatus {
                id: Some("doc-1".to_string()),
                status: DocStatus::PROCESSED,
                content_summary: Some("summary 1".into()),
                content_length: Some(100),
                created_at: Some("2025-02-10T12:00:00Z".into()),
                updated_at: Some("2025-02-10T12:05:00Z".into()),
                file_path: Some("/tmp/doc-1.pdf".into()),
                track_id: Some("track-1".into()),
                chunks_list: Some(vec!["chunk-a".into(), "chunk-b".into()]),
                metadata: Some(json!({"score": 0.99})),
                error_msg: None,
            },
        ),
        (
            "doc-2".to_string(),
            DocProcessingStatus {
                id: Some("doc-2".to_string()),
                status: DocStatus::PROCESSING,
                content_summary: None,
                content_length: None,
                created_at: Some("2025-02-10T12:01:00Z".into()),
                updated_at: Some("2025-02-10T12:06:00Z".into()),
                file_path: Some("/tmp/doc-2.pdf".into()),
                track_id: Some("track-1".into()),
                chunks_list: Some(vec!["chunk-x".into()]),
                metadata: None,
                error_msg: Some("pending".into()),
            },
        ),
        (
            "doc-3".to_string(),
            DocProcessingStatus {
                id: Some("doc-3".to_string()),
                status: DocStatus::PROCESSED,
                content_summary: Some("summary 3".into()),
                content_length: Some(250),
                created_at: Some("2025-02-10T12:02:00Z".into()),
                updated_at: Some("2025-02-10T12:07:00Z".into()),
                file_path: Some("/tmp/doc-3.pdf".into()),
                track_id: Some("track-2".into()),
                chunks_list: Some(vec!["chunk-y".into()]),
                metadata: Some(json!({"tags": ["science"]})),
                error_msg: None,
            },
        ),
    ]
    .into_iter()
    .collect();

    storage.upsert(docs).await?;
    storage.sync_if_dirty().await?;

    let counts = storage.status_counts().await?;
    assert_eq!(counts.get(&DocStatus::PROCESSED), Some(&2));

    let counts_with_total = storage.status_counts_with_total().await?;
    assert_eq!(counts_with_total.get(&DocStatus::ALL), Some(&3));

    let by_status = storage.docs_by_status(&DocStatus::PROCESSED).await?;
    assert_eq!(by_status.len(), 2);

    let by_track = storage.docs_by_track_id("track-1").await?;
    assert_eq!(by_track.len(), 2);

    let (page, total) = storage
        .docs_paginated(Some(&DocStatus::PROCESSED), 1, 2, "updated_at", "desc")
        .await?;
    assert_eq!(total, 2);
    assert_eq!(page.len(), 2);

    storage.delete(&["doc-2".to_string()]).await?;
    storage.sync_if_dirty().await?;
    assert!(storage.get_by_id("doc-2").await?.is_none());

    let reopened = JsonDocStatusStorage::new(config);
    reopened.initialize().await?;
    assert!(reopened.get_by_id("doc-1").await?.is_some());

    reopened.drop_all().await?;
    assert!(reopened.status_counts().await?.is_empty());

    Ok(())
}
