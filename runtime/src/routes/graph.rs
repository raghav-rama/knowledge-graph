use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::Arc,
};

use super::types::{
    EntityResponse, GraphResponse, GraphSearchEdge, GraphSearchEntity, GraphSearchPath,
    GraphSearchResponse, GraphSearchResult, RelationshipEdgeResponse,
};
use crate::{
    AppState,
    pipeline::{
        types::{EntityNode, RelationEdge},
        utils::{get_all_entities, get_all_relationships},
    },
    storage::{JsonKvStorage, JsonKvStorageConfig, KvStorage},
};
use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::get,
};
use petgraph::{
    Direction,
    stable_graph::{NodeIndex, StableDiGraph},
};
use serde::Deserialize;
use x402_axum::{IntoPriceTag, X402Middleware};
use x402_rs::address_evm;
use x402_rs::network::{Network, USDCDeployment};
use x402_rs::telemetry::Telemetry;
use x402_rs::types::{EvmAddress, MixedAddress};

const DEFAULT_MAX_DEPTH: usize = 6;
const DEFAULT_MAX_PATHS: usize = 5;
const DEFAULT_MAX_SYMPTOMS: usize = 50;

pub fn graph_routes() -> Router<Arc<AppState>> {
    let x402 = X402Middleware::try_from("https://facilitator.x402.rs").unwrap();
    let address = address_evm!("0x2C1b291B3946e06ED41FB543B18a21558eBa3d62");
    let usdc = USDCDeployment::by_network(Network::BaseSepolia).pay_to(address);

    Router::new().route("/graph", get(get_graph)).route(
        "/graph-search",
        get(graph_search), // .layer(
                           //     x402.with_description("Search for a term on the knowledge graph")
                           //         .with_price_tag(usdc.amount(0.01).unwrap()),
                           // ),
    )
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

#[derive(Default, Deserialize)]
#[serde(default)]
struct GraphSearchQuery {
    q: Option<String>,
    max_depth: Option<usize>,
    max_paths: Option<usize>,
    max_symptoms: Option<usize>,
    llm_friendly: Option<bool>,
}

async fn graph_search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<GraphSearchQuery>,
) -> Result<Json<GraphSearchResponse>, (StatusCode, String)> {
    let all_entities = get_all_entities(state.storages.full_entities.as_ref())
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error getting entities {err}"),
            )
        })?;
    let all_relationships = get_all_relationships(state.storages.full_relations.as_ref())
        .await
        .map_err(|err| {
            let full_error = err
                .chain()
                .map(|cause| cause.to_string())
                .collect::<Vec<_>>()
                .join(": ");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error getting relationships {full_error}"),
            )
        })?;

    let (graph, node_ids) = build_graph(&all_entities, &all_relationships);

    let query = params.q.as_ref().and_then(|q| {
        let trimmed = q.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let max_depth = params.max_depth.unwrap_or(DEFAULT_MAX_DEPTH);
    let max_paths = params.max_paths.unwrap_or(DEFAULT_MAX_PATHS);
    let max_symptoms = params.max_symptoms.unwrap_or(DEFAULT_MAX_SYMPTOMS);
    let llm_friendly = params.llm_friendly.unwrap_or(false);

    if !llm_friendly {
        let results = traverse_symptom_to_disease(
            &graph,
            &node_ids,
            query.as_deref(),
            max_depth,
            max_paths,
            max_symptoms,
        );
        Ok(Json(GraphSearchResponse {
            query,
            results: Some(results),
            message: None,
            paths: None,
        }))
    } else {
        let paths = traverse_symptom_to_disease_llm_friendly(
            &graph,
            &node_ids,
            query.as_deref(),
            max_depth,
            max_paths,
            max_symptoms,
        );
        Ok(Json(GraphSearchResponse {
            query,
            results: None,
            message: None,
            paths: Some(paths),
        }))
    }
}

fn build_graph(
    all_entities: &HashMap<String, EntityNode>,
    all_relationships: &HashMap<String, RelationEdge>,
) -> (
    StableDiGraph<EntityNode, RelationEdge>,
    HashMap<NodeIndex, String>,
) {
    let mut graph = StableDiGraph::<EntityNode, RelationEdge>::with_capacity(
        all_entities.len(),
        all_relationships.len(),
    );
    let mut entity_indices: HashMap<String, NodeIndex> = HashMap::new();
    let mut node_ids: HashMap<NodeIndex, String> = HashMap::new();

    for (entity_id, entity_node) in all_entities.iter() {
        let node_index = graph.add_node(entity_node.clone());
        entity_indices.insert(entity_id.clone(), node_index);
        node_ids.insert(node_index, entity_id.clone());
    }

    for relation_edge in all_relationships.values() {
        let Some(&source_idx) = entity_indices.get(&relation_edge.source_entity_id) else {
            continue;
        };
        let Some(&target_idx) = entity_indices.get(&relation_edge.target_entity_id) else {
            continue;
        };
        graph.add_edge(source_idx, target_idx, relation_edge.clone());
    }

    (graph, node_ids)
}

fn traverse_symptom_to_disease_llm_friendly(
    graph: &StableDiGraph<EntityNode, RelationEdge>,
    node_ids: &HashMap<NodeIndex, String>,
    symptom_query: Option<&str>,
    max_depth: usize,
    max_paths_per_symptom: usize,
    max_symptoms: usize,
) -> Vec<String> {
    let start_nodes = find_symptom_nodes(graph, symptom_query);

    start_nodes
        .into_iter()
        .take(max_symptoms)
        .flat_map(|start_idx| {
            bfs_symptom_to_diseases(
                graph,
                start_idx,
                max_depth,
                max_paths_per_symptom,
                WalkDir::Both,
            )
            .into_iter()
            .filter_map(|path| build_path_as_str(graph, node_ids, path))
        })
        .collect::<Vec<String>>()
}

fn traverse_symptom_to_disease(
    graph: &StableDiGraph<EntityNode, RelationEdge>,
    node_ids: &HashMap<NodeIndex, String>,
    symptom_query: Option<&str>,
    max_depth: usize,
    max_paths_per_symptom: usize,
    max_symptoms: usize,
) -> Vec<GraphSearchResult> {
    let start_nodes = find_symptom_nodes(graph, symptom_query);
    start_nodes
        .into_iter()
        .take(max_symptoms)
        .map(|start_idx| {
            let paths = bfs_symptom_to_diseases(
                graph,
                start_idx,
                max_depth,
                max_paths_per_symptom,
                WalkDir::Both,
            )
            .into_iter()
            .map(|path| build_path(graph, node_ids, path))
            .collect();

            GraphSearchResult {
                symptom: build_entity(graph, node_ids, start_idx),
                paths,
            }
        })
        .filter(|result| !result.paths.is_empty())
        .collect()
}

fn build_path_as_str(
    graph: &StableDiGraph<EntityNode, RelationEdge>,
    node_ids: &HashMap<NodeIndex, String>,
    path: Vec<NodeIndex>,
) -> Option<String> {
    let mut path_strs = Vec::new();
    for window in path.windows(2) {
        let a = window[0];
        let b = window[1];
        if let Some((relation, _is_forward)) = find_edge(graph, a, b) {
            let node_a = &graph[a];
            let node_b = &graph[b];
            path_strs.push(format!(
                "{} ---  {}  ---> {}",
                node_a.entity_name, relation.relationship_description, node_b.entity_name
            ));
        };
    }
    if path_strs.is_empty() {
        return None;
    }
    Some(path_strs.join("-----"))
}

#[tokio::test]
async fn test_build_path_as_str() -> Result<()> {
    let working_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("pgv-data-test");
    let full_entities = Arc::new(JsonKvStorage::new(JsonKvStorageConfig {
        working_dir: working_dir.clone(),
        namespace: "full_entities".into(),
        workspace: None,
    }));

    let full_relations = Arc::new(JsonKvStorage::new(JsonKvStorageConfig {
        working_dir: working_dir.clone(),
        namespace: "full_relations".into(),
        workspace: None,
    }));
    full_entities.initialize().await?;
    full_relations.initialize().await?;
    let all_entities = get_all_entities(full_entities.as_ref()).await?;
    let all_relationships = get_all_relationships(full_relations.as_ref()).await?;
    let (graph, node_ids) = build_graph(&all_entities, &all_relationships);

    let max_depth = DEFAULT_MAX_DEPTH;
    let max_paths = DEFAULT_MAX_PATHS;
    let max_symptoms = DEFAULT_MAX_SYMPTOMS;

    let start_nodes = find_symptom_nodes(&graph, Some("Progeria"));

    let paths =
        bfs_symptom_to_diseases(&graph, start_nodes[0], max_depth, max_paths, WalkDir::Both);

    build_path_as_str(&graph, &node_ids, paths[1].clone());
    Ok(())
}

fn build_path(
    graph: &StableDiGraph<EntityNode, RelationEdge>,
    node_ids: &HashMap<NodeIndex, String>,
    path: Vec<NodeIndex>,
) -> GraphSearchPath {
    let nodes = path
        .iter()
        .map(|idx| build_entity(graph, node_ids, *idx))
        .collect();

    let mut edges = Vec::new();
    for window in path.windows(2) {
        let a = window[0];
        let b = window[1];
        if let Some((relation, is_forward)) = find_edge(graph, a, b) {
            edges.push(GraphSearchEdge {
                relation_description: relation.relationship_description.clone(),
                relationship_keywords: relation.relationship_keywords.clone(),
                source_entity_id: relation.source_entity_id.clone(),
                target_entity_id: relation.target_entity_id.clone(),
                is_forward,
            });
        }
    }

    GraphSearchPath { nodes, edges }
}

fn build_entity(
    graph: &StableDiGraph<EntityNode, RelationEdge>,
    node_ids: &HashMap<NodeIndex, String>,
    idx: NodeIndex,
) -> GraphSearchEntity {
    let entity = &graph[idx];
    GraphSearchEntity {
        id: node_ids.get(&idx).cloned().unwrap_or_default(),
        entity_name: entity.entity_name.clone(),
        entity_description: entity.entity_description.clone(),
        entity_type: entity.entity_type.clone(),
    }
}

fn find_edge(
    graph: &StableDiGraph<EntityNode, RelationEdge>,
    a: NodeIndex,
    b: NodeIndex,
) -> Option<(RelationEdge, bool)> {
    if let Some(edge) = graph.edges_connecting(a, b).next() {
        return Some((edge.weight().clone(), true));
    }
    if let Some(edge) = graph.edges_connecting(b, a).next() {
        return Some((edge.weight().clone(), false));
    }
    None
}

fn is_symptom(entity: &EntityNode) -> bool {
    entity.entity_type.eq_ignore_ascii_case("Symptom")
}

fn is_disease(entity: &EntityNode) -> bool {
    entity.entity_type.eq_ignore_ascii_case("Disease")
}

fn matches_query(name: &str, query: &str) -> bool {
    name.to_ascii_lowercase()
        .contains(&query.to_ascii_lowercase())
}

fn find_symptom_nodes(
    graph: &StableDiGraph<EntityNode, RelationEdge>,
    query: Option<&str>,
) -> Vec<NodeIndex> {
    graph
        .node_indices()
        .filter(|&idx| {
            let entity = &graph[idx];
            is_symptom(entity)
                && match query {
                    Some(q) if !q.is_empty() => matches_query(&entity.entity_name, q),
                    _ => true,
                }
        })
        .collect()
}

fn bfs_symptom_to_diseases(
    graph: &StableDiGraph<EntityNode, RelationEdge>,
    start: NodeIndex,
    max_depth: usize,
    max_paths: usize,
    walk_direction: WalkDir,
) -> Vec<Vec<NodeIndex>> {
    let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();
    let mut visited: HashSet<NodeIndex> = HashSet::new();
    let mut parent: HashMap<NodeIndex, NodeIndex> = HashMap::new();
    let mut paths: Vec<Vec<NodeIndex>> = Vec::new();

    queue.push_back((start, 0));
    visited.insert(start);

    while let Some((node, depth)) = queue.pop_front() {
        if depth > max_depth {
            continue;
        }

        if depth > 0 && is_disease(&graph[node]) {
            let mut path = vec![node];
            let mut cursor = node;
            while let Some(&p) = parent.get(&cursor) {
                path.push(p);
                cursor = p;
                if cursor == start {
                    break;
                }
            }
            path.reverse();
            paths.push(path);
            if paths.len() >= max_paths {
                break;
            }
        }

        for neighbor in neighbors(graph, node, &walk_direction) {
            if visited.insert(neighbor) {
                parent.insert(neighbor, node);
                queue.push_back((neighbor, depth + 1));
            }
        }
    }

    paths
}

enum WalkDir {
    Outgoing,
    Incoming,
    Both,
}

fn neighbors<'a>(
    graph: &'a StableDiGraph<EntityNode, RelationEdge>,
    node: NodeIndex,
    direction: &WalkDir,
) -> Box<dyn Iterator<Item = NodeIndex> + 'a> {
    match direction {
        WalkDir::Outgoing => Box::new(graph.neighbors_directed(node, Direction::Outgoing)),
        WalkDir::Incoming => Box::new(graph.neighbors_directed(node, Direction::Incoming)),
        WalkDir::Both => Box::new(
            graph
                .neighbors_directed(node, Direction::Outgoing)
                .chain(graph.neighbors_directed(node, Direction::Incoming)),
        ),
    }
}
