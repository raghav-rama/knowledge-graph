use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Default, Clone, Debug, Deserialize, TS, Serialize)]
#[ts(export)]
pub struct EntityNode {
    pub chunk_id: String,
    pub chunk_order_index: u32,
    pub doc_id: String,
    pub entity_description: String,
    pub entity_name: String,
    pub entity_type: String,
}

#[derive(Default, Clone, Debug, Deserialize, TS, Serialize)]
#[ts(export)]
pub struct RelationEdge {
    pub chunk_id: String,
    pub doc_id: String,
    pub relationship_description: String,
    pub relationship_keywords: Vec<String>,
    pub source_entity_id: String,
    pub target_entity_id: String,
}
