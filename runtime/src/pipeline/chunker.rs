use std::sync::Arc;

use anyhow::{Result, anyhow};

use crate::pipeline::utils::{Tokenizer, chunking_by_token_size, compute_mdhash_id};

#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: String,
    pub content: String,
    pub order: usize,
    pub token_count: i64,
}

#[derive(Debug, Clone)]
pub struct ChunkConfig {
    pub max_tokens: usize,
    pub overlap_tokens: usize,
    pub split_by_character: Option<String>,
    pub split_by_character_only: bool,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_tokens: 500,
            overlap_tokens: 50,
            split_by_character: None,
            split_by_character_only: false,
        }
    }
}

pub trait Chunker: Send + Sync {
    fn chunk(&self, content: &str, config: &ChunkConfig) -> Result<Vec<Chunk>>;
}

#[derive(Clone)]
pub struct TokenizerChunker {
    tokenizer: Arc<dyn Tokenizer>,
}

impl TokenizerChunker {
    pub fn new(tokenizer: Arc<dyn Tokenizer>) -> Self {
        Self { tokenizer }
    }
}

impl Chunker for TokenizerChunker {
    fn chunk(&self, content: &str, config: &ChunkConfig) -> Result<Vec<Chunk>> {
        if config.overlap_tokens >= config.max_tokens {
            return Err(anyhow!(
                "overlap_token_size ({}) must be smaller than max_token_size ({})",
                config.overlap_tokens,
                config.max_tokens
            ));
        }

        let token_chunks = chunking_by_token_size(
            self.tokenizer.as_ref(),
            content,
            config.split_by_character.as_deref(),
            config.split_by_character_only,
            config.overlap_tokens,
            config.max_tokens,
        )?;

        let chunks = token_chunks
            .into_iter()
            .map(|chunk| Chunk {
                id: compute_mdhash_id(&chunk.content, "chunk-"),
                content: chunk.content,
                order: chunk.chunk_order_index,
                token_count: chunk.tokens as i64,
            })
            .collect();

        Ok(chunks)
    }
}
