use anyhow::{Context, Result};
use dotenvy::dotenv;
use embed_anything::embeddings::cloud::openai::OpenAIEmbedder;
use std::env;

fn dot(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv().with_context(|| ".env not found");
    let api_key = env::var("OPENAI_API_KEY").with_context(|| "OPENAI_API_KEY not set in env")?;
    let oai_embedder = OpenAIEmbedder::new("text-embedding-3-small".to_string(), Some(api_key));
    let embed_result = oai_embedder.embed(&["alzheimer's", "AD"]).await?;
    // for embed in embed_result {
    //     let embeddings = embed.to_dense()?;
    //     let only_ten = embeddings.iter().take(10).collect::<Vec<&f32>>();
    //     println!("Embeddings: {only_ten:?}")
    // }
    let vec1 = embed_result[0].to_dense()?;
    let vec2 = embed_result[1].to_dense()?;
    let similarity = dot(&vec1, &vec2);
    println!("Similarity: {similarity}");
    Ok(())
}
