#![allow(dead_code)]
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::Arc,
};

use anyhow::{Ok, Result};
use petgraph::{
    Direction,
    stable_graph::{NodeIndex, StableDiGraph},
};
use runtime::storage::{JsonKvStorage, JsonKvStorageConfig, KvStorage};
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
struct RelationEdge {
    chunk_id: String,
    doc_id: String,
    relationship_description: String,
    relationship_keywords: Vec<String>,
    source_entity_id: String,
    target_entity_id: String,
}

#[derive(Default, Clone, Debug, Deserialize)]
struct Chunk {
    content: String,
    file_path: String,
    full_doc_id: String,
    tokens: u32,
    chunk_order_index: u32,
    create_time: u32,
    update_time: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let working_dir = PathBuf::from("/Users/ritz/develop/ai/enhanced-kg/runtime/pgv-data");
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
    let text_chunks = Arc::new(JsonKvStorage::new(JsonKvStorageConfig {
        working_dir: working_dir.clone(),
        namespace: "text_chunks".into(),
        workspace: None,
    }));

    full_entities.initialize().await?;
    full_relations.initialize().await?;
    text_chunks.initialize().await?;

    let all_entities = full_entities.get_all().await?;
    let all_relations = full_relations.get_all().await?;
    let all_chunks = text_chunks.get_all().await?;

    let mut graph = StableDiGraph::<EntityNode, RelationEdge>::with_capacity(
        all_entities.len(),
        all_relations.len(),
    );
    let mut entities_index: HashMap<String, NodeIndex> = HashMap::new();
    let mut nodes_by_doc: HashMap<String, Vec<NodeIndex>> = HashMap::new();

    for (entity_id, value) in all_entities.iter() {
        let entity: EntityNode = serde_json::from_value(value.clone())?;
        let node_index = graph.add_node(entity.clone());
        entities_index.insert(entity_id.clone(), node_index);
        nodes_by_doc
            .entry(entity.doc_id.clone())
            .or_default()
            .push(node_index);
    }

    for value in all_relations.values() {
        let relation: RelationEdge = serde_json::from_value(value.clone())?;
        let source_idx = match entities_index.get(&relation.source_entity_id) {
            Some(idx) => *idx,
            None => continue,
        };
        let target_idx = match entities_index.get(&relation.target_entity_id) {
            Some(idx) => *idx,
            None => continue,
        };
        graph.add_edge(source_idx, target_idx, relation);
    }

    for value in all_chunks.values() {
        let _chunk: Chunk = serde_json::from_value(value.clone())?;
    }

    println!(
        "Knowledge graph created ({} nodes, {} edges)",
        graph.node_count(),
        graph.edge_count()
    );

    traverse_symptom_to_disease(&graph, Some("Cognitive impairment"), 6, 3, 50);
    Ok(())
}

fn is_symptom(n: &EntityNode) -> bool {
    n.entity_type.eq_ignore_ascii_case("Symptom / Phenotype")
}
fn is_disease(n: &EntityNode) -> bool {
    n.entity_type.eq_ignore_ascii_case("Disease / Disorder")
}
fn matches_query(name: &str, q: &str) -> bool {
    name.to_ascii_lowercase().contains(&q.to_ascii_lowercase())
}

fn find_symptom_nodes(
    graph: &StableDiGraph<EntityNode, RelationEdge>,
    query: Option<&str>,
) -> Vec<NodeIndex> {
    graph
        .node_indices()
        .filter(|&idx| {
            let n = &graph[idx];
            is_symptom(n)
                && match query {
                    Some(q) if !q.is_empty() => matches_query(&n.entity_name, q),
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

    while let Some((u, d)) = queue.pop_front() {
        if d > max_depth {
            continue;
        }

        if d > 0 && is_disease(&graph[u]) {
            let mut path = vec![u];
            let mut cur = u;
            while let Some(&p) = parent.get(&cur) {
                path.push(p);
                cur = p;
                if cur == start {
                    break;
                }
            }
            path.reverse();
            paths.push(path);
            if paths.len() >= max_paths {
                break;
            }
        }

        // for v in graph.neighbors_directed(u, Direction::Outgoing) {
        //     if !visited.contains(&v) {
        //         visited.insert(v);
        //         parent.insert(v, u);
        //         queue.push_back((v, d + 1));
        //     }
        // }
        for v in neighbors(graph, u, &walk_direction) {
            if visited.insert(v) {
                parent.insert(v, u);
                queue.push_back((v, d + 1));
            }
        }
    }
    paths
}

fn print_path(graph: &StableDiGraph<EntityNode, RelationEdge>, path: &[NodeIndex]) {
    for win in path.windows(2) {
        let a = win[0];
        let b = win[1];
        // first matching edge for now
        let edge_desc = graph
            .edges_connecting(a, b)
            .next()
            .map(|e| e.weight().relationship_description.clone())
            .unwrap_or_else(|| "related_to".to_string());
        println!(
            "{} --[{}]--> {}",
            graph[a].entity_name, edge_desc, graph[b].entity_name
        );
    }
    if let Some(last) = path.last() {
        println!(
            "=> Reached disease: {} ({})",
            graph[*last].entity_name, graph[*last].entity_type
        );
    }
}

fn traverse_symptom_to_disease(
    graph: &StableDiGraph<EntityNode, RelationEdge>,
    symptom_query: Option<&str>,
    max_depth: usize,
    max_paths_per_symptom: usize,
    max_symptoms: usize,
) {
    let starts = find_symptom_nodes(graph, symptom_query);
    println!(
        "Found {} symptom start node(s){}",
        starts.len(),
        symptom_query
            .map(|q| format!(" matching query '{q}'"))
            .unwrap_or_default()
    );

    for (i, s) in starts.into_iter().take(max_symptoms).enumerate() {
        let sn = &graph[s];
        println!(
            "\n[{}] Starting from Symptom: {} ({})",
            i + 1,
            sn.entity_name,
            sn.entity_type
        );
        let paths =
            bfs_symptom_to_diseases(graph, s, max_depth, max_paths_per_symptom, WalkDir::Both);
        if paths.is_empty() {
            println!("  No disease reached within depth {}", max_depth);
        } else {
            for (k, p) in paths.iter().enumerate() {
                println!("  Path {} (len={}):", k + 1, p.len());
                print_path(graph, p);
            }
        }
    }
}

fn count_by_type(g: &StableDiGraph<EntityNode, RelationEdge>) -> HashMap<String, usize> {
    let mut m = HashMap::new();
    for i in g.node_indices() {
        *m.entry(g[i].entity_type.to_ascii_lowercase()).or_default() += 1;
    }
    m
}

fn peek_symptom_edges(g: &StableDiGraph<EntityNode, RelationEdge>, name_substr: &str) {
    for n in g.node_indices() {
        let en = &g[n];
        if en.entity_type.eq_ignore_ascii_case("Symptom / Phenotype")
            && en
                .entity_name
                .to_ascii_lowercase()
                .contains(&name_substr.to_ascii_lowercase())
        {
            let out = g.neighbors_directed(n, Direction::Outgoing).count();
            let inn = g.neighbors_directed(n, Direction::Incoming).count();
            println!("Symptom '{}' -> out:{} in:{}", en.entity_name, out, inn);
        }
    }
}

enum WalkDir {
    Outgoing,
    Incoming,
    Both,
}

fn neighbors<'a>(
    g: &'a StableDiGraph<EntityNode, RelationEdge>,
    u: NodeIndex,
    wd: &WalkDir,
) -> Box<dyn Iterator<Item = NodeIndex> + 'a> {
    match wd {
        WalkDir::Outgoing => Box::new(g.neighbors_directed(u, Direction::Outgoing)),
        WalkDir::Incoming => Box::new(g.neighbors_directed(u, Direction::Incoming)),
        WalkDir::Both => Box::new(
            g.neighbors_directed(u, Direction::Outgoing)
                .chain(g.neighbors_directed(u, Direction::Incoming)),
        ),
    }
}
