#![allow(dead_code)]
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    path::PathBuf,
    sync::Arc,
};

use anyhow::{Ok, Result};
use petgraph::{
    stable_graph::{NodeIndex, StableDiGraph},
    visit::{EdgeRef, IntoEdgeReferences},
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

    full_entities.initialize().await?;
    full_relations.initialize().await?;

    let all_entities = full_entities.get_all().await?;
    let all_relations = full_relations.get_all().await?;

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

    println!(
        "Knowledge graph created ({} nodes, {} edges)",
        graph.node_count(),
        graph.edge_count()
    );

    let dot_repr = render_graph(&graph, &nodes_by_doc);
    tokio::fs::write("graph.dot", dot_repr).await?;
    Ok(())
}

fn render_graph(
    graph: &StableDiGraph<EntityNode, RelationEdge>,
    nodes_by_doc: &HashMap<String, Vec<NodeIndex>>,
) -> String {
    let mut output = String::new();
    writeln!(
        &mut output,
        "digraph KnowledgeGraph {{\n    graph [bgcolor=\"#0d1117\", fontname=\"Inter\", rankdir=LR, splines=true, overlap=false, pad=0.4];\n    node [style=filled, fontname=\"Inter\", fontsize=10, shape=rect, color=\"#1f6feb\", fillcolor=\"#161b22\", fontcolor=\"#e6edf3\"];\n    edge [color=\"#8b949e\", arrowsize=0.7, penwidth=1.1];"
    )
    .unwrap();

    let mut emitted: HashSet<NodeIndex> = HashSet::new();

    for (doc_id, nodes) in nodes_by_doc.iter() {
        let cluster_name = format!("cluster_{}", sanitize_identifier(doc_id));
        writeln!(
            &mut output,
            "    subgraph {} {{\n        label=\"{}\";\n        color=\"#30363d\";\n        style=rounded;",
            cluster_name,
            sanitize_text(doc_id)
        )
        .unwrap();

        for node_idx in nodes {
            if let Some(node) = graph.node_weight(*node_idx) {
                write_node_line(&mut output, *node_idx, node);
                emitted.insert(*node_idx);
            }
        }
        writeln!(&mut output, "    }}").unwrap();
    }

    for node_idx in graph.node_indices() {
        if emitted.contains(&node_idx) {
            continue;
        }
        if let Some(node) = graph.node_weight(node_idx) {
            write_node_line(&mut output, node_idx, node);
        }
    }

    for edge in graph.edge_references() {
        let relation = edge.weight();
        let tooltip = truncate(&relation.relationship_description, 180);
        writeln!(
            &mut output,
            "    {} -> {} [label=\"\", tooltip=\"{}\", color=\"#58a6ff\"];",
            edge.source().index(),
            edge.target().index(),
            sanitize_text(&tooltip)
        )
        .unwrap();
    }

    output.push_str("}\n");
    output
}

fn write_node_line(output: &mut String, idx: NodeIndex, node: &EntityNode) {
    let label = format!(
        "{}\\n[{}]",
        truncate(&node.entity_name, 40),
        truncate(&node.entity_type, 28)
    );
    let tooltip = truncate(&node.entity_description, 200);
    let (fill, shape) = style_for_entity_type(&node.entity_type);
    writeln!(
        output,
        "        {} [label=\"{}\", tooltip=\"{}\", shape={}, fillcolor=\"{}\"];",
        idx.index(),
        sanitize_text(&label),
        sanitize_text(&tooltip),
        shape,
        fill
    )
    .unwrap();
}

fn style_for_entity_type(entity_type: &str) -> (&'static str, &'static str) {
    match entity_type {
        "Protein" => ("#ea6045", "ellipse"),
        "Drug / Compound / Chemical Substance" => ("#f69d50", "oval"),
        "Disease / Disorder" => ("#ff7b72", "hexagon"),
        "Publication / Reference" => ("#7dc4fa", "note"),
        "Method / Technique / Assay / Protocol" => ("#c297ff", "parallelogram"),
        "Result / Observation / Finding" => ("#9ecbff", "rect"),
        "Organ" => ("#56d364", "oval"),
        "Model (computational, statistical, or biological)" => ("#91cbff", "diamond"),
        _ => ("#1f6feb", "rect"),
    }
}

fn sanitize_identifier(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn sanitize_text(value: &str) -> String {
    let mut sanitized = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => sanitized.push_str("\\\\"),
            '"' => sanitized.push_str("\\\""),
            '\n' => sanitized.push_str("\\n"),
            '\t' => sanitized.push_str("\\t"),
            '\r' => continue,
            c if c.is_control() => continue,
            _ => sanitized.push(ch),
        }
    }
    sanitized
}

fn truncate(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    let mut result = String::new();
    for ch in value.chars().take(limit.saturating_sub(1)) {
        result.push(ch);
    }
    result.push('…');
    result
}
