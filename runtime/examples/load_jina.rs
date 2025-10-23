use anyhow::Result;
use embed_anything::{
    embed_query,
    embeddings::{
        embed::{Embedder, TextEmbedder},
        local::jina::JinaEmbedder,
    },
};

fn dot(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[tokio::main]
async fn main() -> Result<()> {
    let jina_embedder_impl = JinaEmbedder::default();
    println!("Using device: {:?}", jina_embedder_impl.model.device);
    let jina_embedder = Embedder::Text(TextEmbedder::Jina(Box::new(jina_embedder_impl)));
    let embeddings = embed_query(
        &["hello", "my", "name", "is", "khan", "and"],
        &jina_embedder,
        None,
    )
    .await?;
    for embedding in embeddings {
        let _em = embedding.embedding.to_dense()?;
        println!("Converted");
    }
    let embs = jina_embedder.embed(&["Hello", "hello"], None, None).await?;
    let vec1 = embs[0].to_dense()?.to_vec();
    let vec2 = embs[1].to_dense()?.to_vec();
    let similarity = dot(&vec1, &vec2);
    println!("Similarity: {similarity}");
    Ok(())
}
