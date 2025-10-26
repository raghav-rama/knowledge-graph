#![allow(dead_code)]
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::{Ok, Result};
use petgraph::{
    dot::{Config, Dot},
    // graph::UnGraph,
    stable_graph::{EdgeIndex, NodeIndex, StableUnGraph},
};
use runtime::{
    // pipeline::utils::get_entities_as_arr,
    storage::{JsonKvStorage, JsonKvStorageConfig, KvStorage},
};
use serde::Deserialize;

#[derive(Default, Clone, Debug, Deserialize)]
struct EntityNode {
    chunk_id: String,
    chunk_order_index: u32,
    doc_id: String,
    entity_description: String,
    entity_name: String,
    entity_type: String,
}

#[derive(Default, Clone, Debug, Deserialize)]
struct EdgeLabel {
    chunk_id: String,
    doc_id: String,
    relationship_description: String,
    relationship_keywords: Vec<String>,
    source_entity_id: String,
    target_entity_id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let working_dir = PathBuf::from("/Users/ritz/develop/ai/enhanced-kg/runtime/pgv-data");
    let full_entities = Arc::new(JsonKvStorage::new(JsonKvStorageConfig {
        working_dir: working_dir.clone(),
        namespace: "full_entities".into(),
        workspace: None,
    }));
    let mut entities_node_index: HashMap<String, NodeIndex> = HashMap::new();
    let mut relation_edge_index: HashMap<String, EdgeIndex> = HashMap::new();
    let full_relations = Arc::new(JsonKvStorage::new(JsonKvStorageConfig {
        working_dir: working_dir.clone(),
        namespace: "full_relations".into(),
        workspace: None,
    }));
    full_entities.initialize().await?;
    full_relations.initialize().await?;
    let all_entities = full_entities.get_all().await?;
    for (k, v) in all_entities.iter().take(10) {
        let entity: EntityNode = serde_json::from_value(v.clone())?;
        println!("{:?}: {:?}", k, entity);
    }
    let all_relations = full_relations.get_all().await?;
    for (k, v) in all_relations.iter().take(10) {
        let relation: EdgeLabel = serde_json::from_value(v.clone())?;
        println!("{:?}: {:?}", k, relation);
    }
    let mut g = StableUnGraph::<EntityNode, EdgeLabel>::with_capacity(
        all_entities.len(),
        all_relations.len(),
    );
    for (k, v) in all_entities.iter() {
        let entity: EntityNode = serde_json::from_value(v.clone())?;
        let node_index = g.add_node(entity);
        entities_node_index.insert(k.clone(), node_index);
    }

    for (k, v) in all_relations.iter() {
        let relation: EdgeLabel = serde_json::from_value(v.clone())?;
        let source_entity_id = &relation.source_entity_id;
        let target_entity_id = &relation.target_entity_id;
        let source_entity_node_index = entities_node_index.get(source_entity_id);
        let target_entity_node_index = entities_node_index.get(target_entity_id);
        if source_entity_node_index.is_some() && target_entity_node_index.is_some() {
            let edge_index = g.add_edge(
                source_entity_node_index.unwrap().clone(),
                target_entity_node_index.unwrap().clone(),
                relation,
            );
            relation_edge_index.insert(k.clone(), edge_index);
        }
    }
    println!("KG created!");
    let dot_repr = Dot::with_config(&g, &[Config::EdgeNoLabel]);
    tokio::fs::write("graph.dot", format!("{:?}", dot_repr)).await?;
    Ok(())
}
