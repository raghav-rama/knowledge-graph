// use std::{cmp::Ordering, collections::HashMap, env::var, path::PathBuf, sync::Arc};

// use arrow_array::{
//     Float32Array, Int32Array, RecordBatch, RecordBatchIterator, RecordBatchReader, StringArray,
// };
// use arrow_schema::{DataType, Field, Schema};
// use futures::StreamExt;

// use dotenvy::dotenv;
// use lancedb::{
//     connection::Connection,
//     embeddings::{EmbeddingDefinition, EmbeddingFunction, openai::OpenAIEmbeddingFunction},
//     index::{Index, scalar::FtsIndexBuilder},
//     // query::Select,
//     query::{ExecutableQuery, QueryBase},
//     {Result, Table, connect},
// };

// use runtime::{
//     pipeline::utils::get_entities_as_arr,
//     storage::{JsonKvStorage, JsonKvStorageConfig, KvStorage},
// };

// #[tokio::main]
// async fn main() -> Result<()> {
//     dotenv().ok();

//     if std::path::Path::new("data").exists() {
//         std::fs::remove_dir_all("data").unwrap();
//     }

//     let api_key = var("OPENAI_API_KEY").expect("openai api key not set in .env");
//     let embedding = Arc::new(OpenAIEmbeddingFunction::new_with_model(
//         api_key,
//         "text-embedding-3-small",
//     )?);

//     let uri = "data/sample-lancedb";
//     let db = connect(uri).execute().await?;

//     db.embedding_registry()
//         .register("openai", embedding.clone())?;
//     let working_dir = PathBuf::from("/Users/ritz/develop/ai/enhanced-kg/runtime/pgv-data");
//     let full_entities = Arc::new(JsonKvStorage::new(JsonKvStorageConfig {
//         working_dir,
//         namespace: "full_entities".into(),
//         workspace: None,
//     }));
//     full_entities.initialize().await.unwrap();
//     let entities = get_entities_as_arr(&full_entities).await.unwrap();
//     let entities = entities
//         .iter()
//         .take(2000)
//         .map(|e| e.as_str())
//         .collect::<Vec<&str>>();
//     let tbl = create_table(&db, &entities).await?;
//     create_index(&tbl).await?;
//     search_index(&tbl, &embedding, &entities).await?;
//     Ok(())
// }

// async fn create_some_records(entities: &Vec<&str>) -> Result<Box<dyn RecordBatchReader + Send>> {
//     let total = entities.len() as usize;

//     let schema = Arc::new(Schema::new(vec![
//         Field::new("id", DataType::Int32, false),
//         Field::new("doc", DataType::Utf8, true),
//     ]));

//     println!("Total entities: {}", entities.len());
//     // println!("Entities: {:?}", entities);

//     let batches = RecordBatchIterator::new(
//         vec![
//             RecordBatch::try_new(
//                 schema.clone(),
//                 vec![
//                     Arc::new(Int32Array::from_iter_values(0..total as i32)),
//                     Arc::new(StringArray::from_iter_values(entities.iter().cloned())),
//                 ],
//             )
//             .unwrap(),
//         ]
//         .into_iter()
//         .map(Ok),
//         schema.clone(),
//     );
//     Ok(Box::new(batches))
// }

// async fn create_table(db: &Connection, entities: &Vec<&str>) -> Result<Table> {
//     let initial_data: Box<dyn RecordBatchReader + Send> = create_some_records(entities).await?;
//     let tbl = db
//         .create_table("my_table", initial_data)
//         .add_embedding(EmbeddingDefinition::new("doc", "openai", Some("embedding")))?
//         .execute()
//         .await?;
//     Ok(tbl)
// }

// async fn create_index(table: &Table) -> Result<()> {
//     table
//         .create_index(&["doc"], Index::FTS(FtsIndexBuilder::default()))
//         .execute()
//         .await?;
//     Ok(())
// }

// async fn search_index(
//     table: &Table,
//     embedding: &OpenAIEmbeddingFunction,
//     entities: &Vec<&str>,
// ) -> Result<()> {
//     const THRESHOLD: f32 = 0.5;

//     let mut closest_matches: HashMap<&str, Vec<(String, f32)>> = HashMap::new();

//     for &entity in entities {
//         println!("Finding duplicates for: {:?}", entity);

//         let query = Arc::new(StringArray::from_iter_values(std::iter::once(entity)));
//         let query_vector = embedding.compute_query_embeddings(query)?;

//         let mut stream = table
//             .vector_search(query_vector)?
//             .limit(5)
//             .execute()
//             .await?;

//         while let Some(batch) = stream.next().await {
//             let rb = batch?;
//             let docs = rb
//                 .column_by_name("doc")
//                 .unwrap()
//                 .as_any()
//                 .downcast_ref::<StringArray>()
//                 .unwrap();

//             let distances = rb
//                 .column_by_name("_distance")
//                 .unwrap()
//                 .as_any()
//                 .downcast_ref::<Float32Array>()
//                 .unwrap();

//             for (doc, distance) in docs.iter().zip(distances.iter()) {
//                 println!("Closest match: {:?}, Distance: {:?}", doc, distance);
//             }

//             for (doc_opt, dist_opt) in docs.iter().zip(distances.iter()) {
//                 if let (Some(doc), Some(dist)) = (doc_opt, dist_opt) {
//                     if dist == 0.0 || doc == entity {
//                         continue;
//                     }
//                     if dist < THRESHOLD {
//                         closest_matches
//                             .entry(entity)
//                             .or_default()
//                             .push((doc.to_string(), dist));
//                     }
//                 }
//             }
//         }
//     }

//     println!("\nCosine similarity (< {:.3} distance)", THRESHOLD);
//     if closest_matches.is_empty() {
//         println!("No close matches found (< {:.3} distance).", THRESHOLD);
//     } else {
//         for (query_entity, mut hits) in closest_matches {
//             hits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

//             println!("{: <15} \"{}\"", "Query Entity:", query_entity);

//             if hits.is_empty() {
//                 println!("{: <15} {}", "  -> Matches:", "None");
//             } else {
//                 for (doc, dist) in hits {
//                     println!("{: <15} \"{}\"", "  -> Match:", doc);
//                     println!("{: <15} {:.4}", "  -> Distance:", dist);
//                 }
//             }
//         }
//     }

//     Ok(())
// }
fn main() {}
