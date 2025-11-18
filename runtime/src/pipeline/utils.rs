use std::collections::HashMap;

use crate::{
    pipeline::{
        chunker::Chunk,
        scheduler::{ChunkState, ChunkStatus},
        types::{EntityNode, RelationEdge},
    },
    storage::{JsonKvStorage, KvStorage},
};
use anyhow::{Result, anyhow};
use chrono::Utc;
use tiktoken_rs::{CoreBPE, o200k_base};

pub trait Tokenizer: Send + Sync {
    fn encode(&self, text: &str) -> Vec<u32>;
    fn decode(&self, tokens: &[u32]) -> Result<String>;
}

#[derive(Clone)]
pub struct TiktokenTokenizer {
    bpe: CoreBPE,
}

impl TiktokenTokenizer {
    pub fn new() -> Result<Self> {
        let bpe = o200k_base()?;
        Ok(Self { bpe })
    }
}

impl Tokenizer for TiktokenTokenizer {
    fn encode(&self, text: &str) -> Vec<u32> {
        self.bpe.encode_with_special_tokens(text)
    }

    fn decode(&self, tokens: &[u32]) -> Result<String> {
        self.bpe.decode(tokens.to_vec())
    }
}

#[derive(Debug, Clone)]
pub struct TokenChunk {
    pub tokens: usize,
    pub content: String,
    pub chunk_order_index: usize,
}

pub fn chunking_by_token_size<T: Tokenizer + ?Sized>(
    tokenizer: &T,
    content: &str,
    split_by_character: Option<&str>,
    split_by_character_only: bool,
    overlap_token_size: usize,
    max_token_size: usize,
) -> Result<Vec<TokenChunk>> {
    if overlap_token_size >= max_token_size {
        return Err(anyhow!(
            "overlap_token_size ({overlap_token_size}) must be smaller than max_token_size ({max_token_size})"
        ));
    }

    let mut results = Vec::new();

    if let Some(delimiter) = split_by_character {
        let raw_chunks = content.split(delimiter);
        let mut new_chunks: Vec<(usize, String)> = Vec::new();

        if split_by_character_only {
            for chunk in raw_chunks {
                let tokenized = tokenizer.encode(chunk);
                new_chunks.push((tokenized.len(), chunk.to_string()));
            }
        } else {
            let step = max_token_size - overlap_token_size;
            for chunk in raw_chunks {
                let tokenized = tokenizer.encode(chunk);
                if tokenized.len() > max_token_size {
                    let total_len = tokenized.len();
                    let mut start = 0usize;
                    while start < total_len {
                        let end = (start + max_token_size).min(total_len);
                        let chunk_content = tokenizer.decode(&tokenized[start..end])?;
                        new_chunks.push((end - start, chunk_content));
                        if end == total_len {
                            break;
                        }
                        start += step;
                    }
                } else {
                    new_chunks.push((tokenized.len(), chunk.to_string()));
                }
            }
        }
        for (index, (token_len, chunk_text)) in new_chunks.into_iter().enumerate() {
            results.push(TokenChunk {
                tokens: token_len,
                content: chunk_text.trim().to_string(),
                chunk_order_index: index,
            });
        }
    } else {
        let tokens = tokenizer.encode(content);
        if tokens.is_empty() {
            return Ok(results);
        }

        let step = max_token_size - overlap_token_size;
        let mut start = 0usize;
        while start < tokens.len() {
            let end = (start + max_token_size).min(tokens.len());
            let chunk_content = tokenizer.decode(&tokens[start..end])?;
            results.push(TokenChunk {
                tokens: end - start,
                content: chunk_content.trim().to_string(),
                chunk_order_index: results.len(),
            });

            if end == tokens.len() {
                break;
            }

            start += step;
        }
    }

    Ok(results)
}

pub fn compute_mdhash_id(content: &str, prefix: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let digest = hasher.finalize();
    format!("{}{:x}", prefix, digest)
}

pub async fn get_entities_as_arr(entities: &JsonKvStorage) -> Result<Vec<String>> {
    let all_entities = entities.get_all().await?;
    let mut entities = Vec::new();
    for value in all_entities.values() {
        if let Some(entity_name) = value.get("entity_name").and_then(|e| e.as_str()) {
            entities.push(entity_name.to_owned());
        };
    }
    Ok(entities)
}

pub async fn get_all_entities(s: &JsonKvStorage) -> Result<HashMap<String, EntityNode>> {
    let mut entities = HashMap::new();

    let raw_map = s.get_all().await?;
    for (entity_id, value) in raw_map {
        let entity: EntityNode = serde_json::from_value(value)?;
        entities.insert(entity_id, entity);
    }

    Ok(entities)
}

pub async fn get_all_relationships(s: &JsonKvStorage) -> Result<HashMap<String, RelationEdge>> {
    let mut relations = HashMap::new();

    let raw_map = s.get_all().await?;
    for (relation_id, value) in raw_map {
        let relation_edge: RelationEdge = serde_json::from_value(value)?;
        relations.insert(relation_id, relation_edge);
    }

    Ok(relations)
}

pub fn chunk_to_chunk_state(chunks: Vec<Chunk>, doc_id: String) -> Vec<ChunkState> {
    chunks
        .iter()
        .map(|chunk| ChunkState {
            chunk_id: chunk.id.clone(),
            doc_id: doc_id.clone(),
            chunk_status: ChunkStatus::Pending,
            chunk_order_index: chunk.order,
            content: chunk.content.clone(),
            error: None,
            output: None,
            max_retries: 10,
            current_retry: 0,
            created_at: Utc::now(),
            oai_resp_id: None,
        })
        .collect()
}
