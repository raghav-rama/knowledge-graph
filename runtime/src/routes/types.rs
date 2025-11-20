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

#[derive(Default, Clone, Debug, Deserialize, TS, Serialize)]
#[ts(export)]
pub struct GraphSearchEntity {
    pub id: String,
    pub entity_name: String,
    pub entity_description: String,
    pub entity_type: String,
}

#[derive(Default, Clone, Debug, Deserialize, TS, Serialize)]
#[ts(export)]
pub struct GraphSearchEdge {
    pub relation_description: String,
    pub relationship_keywords: Vec<String>,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub is_forward: bool,
}

#[derive(Default, Clone, Debug, Deserialize, TS, Serialize)]
#[ts(export)]
pub struct GraphSearchPath {
    pub nodes: Vec<GraphSearchEntity>,
    pub edges: Vec<GraphSearchEdge>,
}

#[derive(Default, Clone, Debug, Deserialize, TS, Serialize)]
#[ts(export)]
pub struct GraphSearchResult {
    pub symptom: GraphSearchEntity,
    pub paths: Vec<GraphSearchPath>,
}

#[derive(Default, Clone, Debug, Deserialize, TS, Serialize)]
#[ts(export)]
pub struct GraphSearchResponse {
    pub query: Option<String>,
    pub results: Option<Vec<GraphSearchResult>>,
    pub message: Option<String>,
    pub paths: Option<Vec<String>>,
}
