use anyhow::Result;
use embed_anything::{
    embed_query,
    embeddings::{
        embed::{Embedder, TextEmbedder},
        local::jina::JinaEmbedder,
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    let jina_embedder = Embedder::Text(TextEmbedder::Jina(Box::new(JinaEmbedder::default())));
    let embeddings = embed_query(&["hello"], &jina_embedder, None).await?;
    for embedding in embeddings {
        let em = embedding.embedding.to_dense()?;
        println!("{:?}", &em);
    }
    Ok(())
}
