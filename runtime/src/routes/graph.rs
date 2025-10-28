use std::sync::Arc;

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};

use serde::Serialize;
use ts_rs::TS;

use super::types::{EntityResponse, GraphResponse, RelationshipEdgeResponse};
use crate::{
    AppState,
    pipeline::utils::{get_all_entities, get_all_relationships},
    storage::KvStorage,
};

pub fn graph_routes() -> Router<Arc<AppState>> {
    Router::new().route("/graph", get(get_graph))
}

async fn get_graph(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GraphResponse>, (StatusCode, String)> {
    let all_entities = get_all_entities(state.storages.full_entities.as_ref())
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error getting entities {}", err),
            )
        })?;
    let all_relationships = get_all_relationships(state.storages.full_relations.as_ref())
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error getting relationships {}", err),
            )
        })?;
    let mut entities_vec = Vec::new();
    let mut relations_vec = Vec::new();
    for (entity_id, entity_node) in all_entities {
        entities_vec.push(EntityResponse {
            id: entity_id,
            entity_name: entity_node.entity_name,
            entity_description: entity_node.entity_description,
            entity_type: entity_node.entity_type,
        });
    }
    for (relationship_id, relationship_edge) in all_relationships {
        relations_vec.push(RelationshipEdgeResponse {
            id: relationship_id,
            source_node_id: relationship_edge.source_entity_id,
            target_node_id: relationship_edge.target_entity_id,
            relation_description: relationship_edge.relationship_description,
        });
    }
    Ok(Json(GraphResponse {
        entities: entities_vec,
        relations: relations_vec,
    }))
}
