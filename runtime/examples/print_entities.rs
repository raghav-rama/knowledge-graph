use anyhow::{Ok, Result};
use runtime::{
    pipeline::utils::get_entities_as_arr,
    storage::{JsonKvStorage, JsonKvStorageConfig, KvStorage},
};

use std::{path::PathBuf, sync::Arc};

#[tokio::main]
async fn main() -> Result<()> {
    let working_dir = PathBuf::from("/Users/ritz/develop/ai/enhanced-kg/runtime/pgv-data");
    let full_entities = Arc::new(JsonKvStorage::new(JsonKvStorageConfig {
        working_dir,
        namespace: "full_entities".into(),
        workspace: None,
    }));
    full_entities.initialize().await?;
    let entities = get_entities_as_arr(&full_entities).await?;
    println!("Total entities: {}", entities.len());
    for (index, entity) in entities.iter().take(10).enumerate() {
        println!("{:>2}: {}", index, entity);
    }
    Ok(())
}
