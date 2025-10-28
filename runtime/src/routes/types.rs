use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Default, Clone, Debug, Deserialize, TS, Serialize)]
#[ts(export)]
pub struct EntityResponse {
    pub id: String,
    pub entity_name: String,
    pub entity_description: String,
    pub entity_type: String,
}

#[derive(Default, Clone, Debug, Deserialize, TS, Serialize)]
#[ts(export)]
pub struct RelationshipEdgeResponse {
    pub id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub relation_description: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct GraphResponse {
    pub entities: Vec<EntityResponse>,
    pub relations: Vec<RelationshipEdgeResponse>,
}
