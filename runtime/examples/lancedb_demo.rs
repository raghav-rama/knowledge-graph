use std::{env::var, iter::once, path::PathBuf, sync::Arc};

use arrow_array::{Int32Array, RecordBatch, RecordBatchIterator, RecordBatchReader, StringArray};
use arrow_schema::{DataType, Field, Schema};
use futures::StreamExt;

use dotenvy::dotenv;
use lancedb::{
    connection::Connection,
    embeddings::{EmbeddingDefinition, EmbeddingFunction, openai::OpenAIEmbeddingFunction},
    index::{Index, scalar::FtsIndexBuilder},
    query::{ExecutableQuery, QueryBase},
    {Result, Table, connect},
};
use rand::random;

use runtime::{
    pipeline::utils::get_entities_as_arr,
    storage::{JsonKvStorage, JsonKvStorageConfig, KvStorage},
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    if std::path::Path::new("data").exists() {
        std::fs::remove_dir_all("data").unwrap();
    }

    let api_key = var("OPENAI_API_KEY").expect("openai api key not set in .env");
    let embedding = Arc::new(OpenAIEmbeddingFunction::new_with_model(
        api_key,
        "text-embedding-3-small",
    )?);

    let uri = "data/sample-lancedb";
    let db = connect(uri).execute().await?;

    db.embedding_registry()
        .register("openai", embedding.clone())?;

    let tbl = create_table(&db).await?;
    create_index(&tbl).await?;
    search_index(&tbl, &embedding).await?;
    Ok(())
}

async fn create_some_records() -> Result<Box<dyn RecordBatchReader + Send>> {
    const TOTAL: usize = 1000;

    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("doc", DataType::Utf8, true),
    ]));

    let working_dir = PathBuf::from("/Users/ritz/develop/ai/enhanced-kg/runtime/pgv-data");
    let full_entities = Arc::new(JsonKvStorage::new(JsonKvStorageConfig {
        working_dir,
        namespace: "full_entities".into(),
        workspace: None,
    }));
    full_entities.initialize().await.unwrap();
    let entities = get_entities_as_arr(&full_entities).await.unwrap();
    let entities = entities
        .iter()
        // .take(10)
        .map(|e| e.as_str())
        .collect::<Vec<&str>>();
    println!("Total entities: {}", entities.len());
    // println!("Entities: {:?}", entities);

    let n_terms = 3;
    let batches = RecordBatchIterator::new(
        vec![
            RecordBatch::try_new(
                schema.clone(),
                vec![
                    Arc::new(Int32Array::from_iter_values(0..TOTAL as i32)),
                    Arc::new(StringArray::from_iter_values((0..TOTAL).map(|_| {
                        (0..n_terms)
                            .map(|_| entities[random::<u32>() as usize % entities.len()])
                            .collect::<Vec<_>>()
                            .join(" ")
                    }))),
                ],
            )
            .unwrap(),
        ]
        .into_iter()
        .map(Ok),
        schema.clone(),
    );
    Ok(Box::new(batches))
}

async fn create_table(db: &Connection) -> Result<Table> {
    let initial_data: Box<dyn RecordBatchReader + Send> = create_some_records().await?;
    let tbl = db
        .create_table("my_table", initial_data)
        .add_embedding(EmbeddingDefinition::new("doc", "openai", Some("embedding")))?
        .execute()
        .await?;
    Ok(tbl)
}

async fn create_index(table: &Table) -> Result<()> {
    table
        .create_index(&["doc"], Index::FTS(FtsIndexBuilder::default()))
        .execute()
        .await?;
    Ok(())
}

async fn search_index(table: &Table, embedding: &OpenAIEmbeddingFunction) -> Result<()> {
    let query = Arc::new(StringArray::from_iter_values(once("anandamide")));
    let query_vector = embedding.compute_query_embeddings(query)?;
    let mut results = table
        .vector_search(query_vector)?
        .limit(1)
        .execute()
        .await?;

    let rb = results.next().await.unwrap()?;
    let out = rb
        .column_by_name("doc")
        .unwrap()
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let text = out.iter().next().unwrap().unwrap();
    println!("Closest match: {}", text);
    Ok(())
}
