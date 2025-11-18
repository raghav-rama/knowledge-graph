use super::{
    chunker::{Chunk, ChunkConfig},
    pipeline::{AppStorages, Pipeline},
    utils::compute_mdhash_id,
};
use crate::{
    ai::schemas::EntitiesRelationships, pipeline::utils::chunk_to_chunk_state, storage::KvStorage,
};
use anyhow::{Ok, Result, anyhow};
use chrono::{DateTime, Utc};
use serde_json::{self as serde_json, Value, json};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    result::Result::{Err as StdErr, Ok as StdOk},
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{
        Mutex,
        mpsc::{Receiver, Sender},
    },
    time::{Instant, sleep},
};
use tracing::{debug, error};

use crate::AppState;

#[derive(Clone)]
pub struct Scheduler {
    pub queue: Arc<Mutex<Queue>>,
    dispatcher: Dispatcher,
    // workers: Worker,
    result_rx: Arc<Mutex<Receiver<JobResult>>>,
    pipeline: Arc<Pipeline>,
    storage: Arc<AppStorages>,
}

impl Scheduler {
    pub fn new(
        max_inflight: u8,
        capacity: u32,
        work_tx: Sender<JobDispatch>,
        result_rx: Arc<Mutex<Receiver<JobResult>>>,
        pipeline: Arc<Pipeline>,
        storage: Arc<AppStorages>,
        work_rx: Arc<Mutex<Receiver<JobDispatch>>>,
        result_tx: Sender<JobResult>,
    ) -> Self {
        let queue = Arc::new(Mutex::new(Queue::new(capacity)));

        let scheduler = Scheduler {
            queue,
            dispatcher: Dispatcher::new(work_tx, max_inflight),
            result_rx,
            pipeline: pipeline.clone(),
            storage,
        };
        Worker::spawn_pool(
            pipeline.clone(),
            Arc::new(scheduler.clone()),
            work_rx,
            result_tx,
            10,
        ); // spawn 10 concurrent ER extraction workers
        // tokio::spawn(async move { worker.handle().await });
        scheduler
    }
    pub async fn run(self: Arc<Self>) -> Result<()> {
        loop {
            let result_rx = self.result_rx.clone();
            let mut guard = result_rx.lock().await;
            tokio::select! {
                _ = self.schedule_tick() => {},
                maybe_result = guard.recv() => {
                    self.process_chunk_result(maybe_result).await?
                }
            };
            // let now = Instant::now();
            // let job = {
            //     let mut guard = self.queue.lock().await;
            //     guard.peek().cloned()
            // };
            // if let Some(job) = job {
            //     debug!("executing job {}", job.job_id);
            //     {
            //         let mut guard = self.queue.lock().await;
            //         guard.mark_processing(&job.job_id)?;
            //     }
            //     let chunks = self.make_chunks(&job).await?;
            //     debug!("Made {} chunk(s)", chunks.len());
            //     for chunk in chunks.iter().cloned() {
            //         self.dispatcher
            //             .work_tx
            //             .send(JobDispatch {
            //                 job_id: job.job_id.clone(),
            //                 chunk,
            //             })
            //             .await?;
            //     }

            //     // if let Err(_) = self.dispatcher.work_tx.send(job).await {}
            // } else {
            //     debug!("no job found")
            // }

            // sleep(Duration::new(10, 0)).await;
        }
    }

    async fn process_chunk_result(&self, maybe_result: Option<JobResult>) -> Result<()> {
        if let Some(job_result) = maybe_result {
            debug!("Chunk processed {}", job_result.chunk_id);
            let er = job_result.entity_relationships.clone();
            {
                let mut guard = self.queue.lock().await;
                if let Some(job) = guard.jobs_map.get_mut(&job_result.job_id) {
                    if let Some(chunk) = job
                        .chunks
                        .iter_mut()
                        .find(|chunk| &chunk.chunk_id == &job_result.chunk_id)
                    {
                        chunk.chunk_status = ChunkStatus::Success;
                        chunk.output = Some(job_result.entity_relationships);
                    }
                }
            };
            let entities = er.entities;
            let relationships = er.relationships;
            let mut entities_to_upsert: HashMap<String, Value> = HashMap::new();
            let mut relationships_to_upsert: HashMap<String, Value> = HashMap::new();
            let mut entity_name_to_entity_id_mapping: HashMap<String, String> = HashMap::new();
            for entity in entities {
                let entity_id = compute_mdhash_id(
                    &format!(
                        "{}:{}:{}",
                        &job_result.doc_id,
                        &entity.entity_name,
                        &entity.entity_type.as_str()
                    ),
                    "entity-",
                );

                entities_to_upsert.insert(
                    entity_id.clone(),
                    json!({
                        "entity_name": entity.entity_name,
                        "entity_type": entity.entity_type,
                        "entity_description": entity.entity_description,
                        "doc_id": job_result.doc_id,
                        "chunk_id": job_result.chunk_id,
                        "chunk_order_index": job_result.chunk_order_index
                    }),
                );
                entity_name_to_entity_id_mapping.insert(entity.entity_name, entity_id);
            }

            for relationship in relationships {
                let relation_id = compute_mdhash_id(
                    &format!(
                        "{}:{}:{}",
                        &job_result.doc_id,
                        &relationship.source_entity,
                        &relationship.target_entity
                    ),
                    "rel-",
                );

                relationships_to_upsert.insert(relation_id, json!({
                    "source_entity_id": entity_name_to_entity_id_mapping.get(&relationship.source_entity),
                    "target_entity_id": entity_name_to_entity_id_mapping.get(&relationship.target_entity),
                    "keywords": relationship.relationship_keywords,
                    "description": relationship.relationship_description,
                    "doc_id": job_result.doc_id,
                    "chunk_id": job_result.chunk_id,
                }));
            }

            if !entities_to_upsert.is_empty() {
                self.pipeline
                    .storages
                    .full_entities
                    .upsert(entities_to_upsert)
                    .await?;
            }

            if !relationships_to_upsert.is_empty() {
                self.pipeline
                    .storages
                    .full_relations
                    .upsert(relationships_to_upsert)
                    .await?;
            }

            self.pipeline.persist_all().await?;
        }
        Ok(())
    }
    async fn schedule_tick(&self) -> Result<()> {
        let now = Instant::now();
        let job = {
            let mut guard = self.queue.lock().await;
            if let Some(job) = guard.peek() {
                let chunks = self.get_pending_chunks_for_doc(&job.doc_id).await?;
                let chunks_state = chunk_to_chunk_state(chunks, job.doc_id.clone());
                job.chunks = chunks_state;
            }
            guard.peek().cloned()
        };
        if let Some(job) = job {
            debug!("executing job {}", job.job_id);
            let chunks = self.get_pending_chunks_for_doc(&job.doc_id).await?;
            let chunk_ids = chunks
                .iter()
                .map(|chunk| chunk.id.clone())
                .collect::<Vec<String>>();
            let chunks_state = chunk_to_chunk_state(chunks, job.doc_id.clone());
            debug!("Made {} chunk(s)", chunks_state.len());
            {
                let mut guard = self.queue.lock().await;
                guard.mark_processing(&job.job_id)?;
                if let Some(doc) = self
                    .pipeline
                    .storages
                    .doc_status
                    .get_by_id(&job.doc_id)
                    .await?
                {
                    self.pipeline
                        .status_service
                        .mark_processing(&job.doc_id, &doc, &chunk_ids)
                        .await?;
                }
            }
            for chunk in job.chunks.iter().cloned() {
                self.dispatcher
                    .work_tx
                    .send(JobDispatch {
                        job_id: job.job_id.clone(),
                        chunk,
                    })
                    .await?;
            }

            // if let Err(_) = self.dispatcher.work_tx.send(job).await {}
        } else {
            debug!("no job found")
        }

        sleep(Duration::new(10, 0)).await;
        Ok(())
    }

    async fn get_pending_chunks_for_doc(&self, doc_id: &str) -> Result<Vec<Chunk>> {
        let all = self.pipeline.storages.text_chunks.get_all().await?;
        let pending_chunks: HashMap<String, Value> = all
            .iter()
            .filter_map(|(chunk_id, value)| {
                if value.get("status").and_then(Value::as_str) == Some("Pending")
                    || value.get("status").and_then(Value::as_str) == Some("Failed")
                        && value.get("full_doc_id").and_then(Value::as_str) == Some(doc_id)
                {
                    Some((chunk_id.clone(), value.clone()))
                } else {
                    None
                }
            })
            .collect();
        let vec = pending_chunks
            .into_iter()
            .filter_map(|(chunk_id, value)| {
                let content = value.get("content")?.as_str()?.to_owned();
                let order = value.get("chunk_order_index")?.as_u64()? as usize;
                let token_count_field = value.get("token").or_else(|| value.get("tokens"))?;
                let token_count = token_count_field.as_i64()?;
                Some(Chunk {
                    id: chunk_id,
                    content,
                    order,
                    token_count,
                })
            })
            .collect::<Vec<_>>();

        Ok(vec)
    }

    async fn make_chunks(&self, job: &Job) -> Result<Vec<Chunk>> {
        debug!("Making chunks for {}", job.doc_id);
        let content_value = self
            .pipeline
            .storages
            .full_docs
            .get_by_id(&job.doc_id)
            .await?
            .ok_or_else(|| anyhow!("document missing"))?;

        debug!("Got content value {}", job.job_id);
        let content = content_value
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("document content field missing"))?;

        debug!("Got content {}", job.job_id);
        let chunk_config = ChunkConfig {
            max_tokens: self.pipeline.config.chunk_size,
            overlap_tokens: self.pipeline.config.chunk_overlap,
            split_by_character: self.pipeline.config.split_by_character.clone(),
            split_by_character_only: self.pipeline.config.split_by_character_only,
        };
        let chunks = self.pipeline.chunker.chunk(content, &chunk_config)?;
        debug!("Exiting make_chunks {}", job.job_id);
        Ok(chunks)
    }
}

#[derive(Clone)]
struct Dispatcher {
    work_tx: Sender<JobDispatch>,
    max_inflight: u8,
    inflight: HashSet<String>,
}

impl Dispatcher {
    pub fn new(work_tx: Sender<JobDispatch>, max_inflight: u8) -> Self {
        Dispatcher {
            work_tx,
            max_inflight,
            inflight: HashSet::new(),
        }
    }
}

struct Worker {
    work_rx: Arc<Mutex<Receiver<JobDispatch>>>,
    result_tx: Sender<JobResult>,
}

impl Worker {
    pub fn new(work_rx: Arc<Mutex<Receiver<JobDispatch>>>, result_tx: Sender<JobResult>) -> Self {
        Worker { work_rx, result_tx }
    }

    pub fn spawn_pool(
        pipeline: Arc<Pipeline>,
        scheduler: Arc<Scheduler>,
        work_rx: Arc<Mutex<Receiver<JobDispatch>>>,
        result_tx: Sender<JobResult>,
        size: usize,
    ) {
        for _ in 0..size {
            let pipeline = pipeline.clone();
            let scheduler = scheduler.clone();
            let work_rx = work_rx.clone();
            let result_tx = result_tx.clone();
            tokio::spawn(async move {
                loop {
                    let maybe_job_dispatch = {
                        let mut guard = work_rx.lock().await;
                        guard.recv().await
                    };
                    match maybe_job_dispatch {
                        Some(job_dispatch) => {
                            debug!("Processing Chunk {}", job_dispatch.chunk.chunk_id);

                            // let job = {
                            //     let mut guard = scheduler.queue.lock().await;
                            //     guard.jobs_map.get_mut(&job_dispatch.job_id)
                            // };

                            {
                                let mut guard = scheduler.queue.lock().await;
                                if let Some(job) = guard.jobs_map.get_mut(&job_dispatch.job_id) {
                                    if let Some(chunk) = job.chunks.iter_mut().find(|chunk| {
                                        &chunk.chunk_id == &job_dispatch.chunk.chunk_id
                                    }) {
                                        chunk.chunk_status = ChunkStatus::Running;
                                    }
                                }
                            };

                            let result = pipeline
                                .entity_relationship_extractor
                                .extract_entities_and_relationships(&Chunk {
                                    id: job_dispatch.chunk.chunk_id.clone(),
                                    content: job_dispatch.chunk.content.clone(),
                                    order: 0,
                                    token_count: 0,
                                })
                                .await;
                            match result {
                                StdOk(entity_relationships) => {
                                    debug!(
                                        "Extracted {} entities and {} relationships",
                                        entity_relationships.entities.len(),
                                        entity_relationships.relationships.len()
                                    );
                                    if let StdOk(Some(mut chunk_record)) = pipeline
                                        .storages
                                        .text_chunks
                                        .get_by_id(&job_dispatch.chunk.chunk_id)
                                        .await
                                    {
                                        chunk_record["status"] = Value::String("Success".into());
                                        let mut update = HashMap::new();
                                        update.insert(
                                            job_dispatch.chunk.chunk_id.clone(),
                                            chunk_record,
                                        );
                                        if let Err(store_err) =
                                            pipeline.storages.text_chunks.upsert(update).await
                                        {
                                            error!(error = %store_err, "failed to persist chunk success");
                                        } else if let Err(sync_err) =
                                            pipeline.storages.text_chunks.sync_if_dirty().await
                                        {
                                            error!(error=%sync_err, "failed to flush chunk failure");
                                        }
                                    }
                                    if let StdErr(err) = result_tx
                                        .send(JobResult {
                                            entity_relationships,
                                            chunk_id: job_dispatch.chunk.chunk_id,
                                            job_id: job_dispatch.job_id,
                                            doc_id: job_dispatch.chunk.doc_id,
                                            chunk_order_index: job_dispatch.chunk.chunk_order_index,
                                        })
                                        .await
                                    {
                                        error!(err=%err, "Error");
                                    }
                                }
                                StdErr(err) => {
                                    error!(error=%err, "Got error in extracting entity relationship");
                                    for (depth, err) in err.chain().skip(1).enumerate() {
                                        error!(depth=%depth, error=%err, chunk_id=%job_dispatch.chunk.chunk_id, "caused by");
                                    }

                                    if let StdOk(Some(mut chunk_record)) = pipeline
                                        .storages
                                        .text_chunks
                                        .get_by_id(&job_dispatch.chunk.chunk_id)
                                        .await
                                    {
                                        chunk_record["status"] = Value::String("Failed".into());
                                        chunk_record["error"] = Value::String(err.to_string());
                                        let mut update = HashMap::new();
                                        update.insert(
                                            job_dispatch.chunk.chunk_id.clone(),
                                            chunk_record,
                                        );
                                        if let Err(store_err) =
                                            pipeline.storages.text_chunks.upsert(update).await
                                        {
                                            error!(error=%store_err, "failed to persist chunk failure");
                                        } else if let Err(sync_err) =
                                            pipeline.storages.text_chunks.sync_if_dirty().await
                                        {
                                            error!(error=%sync_err, "failed to flush chunk failure");
                                        }
                                        if let StdErr(err) =
                                            scheduler.dispatcher.work_tx.send(job_dispatch).await
                                        {
                                            error!(error=%err, "Error occurred while sending failed chunk for retry");
                                        };
                                    }
                                }
                            }
                        }
                        None => break,
                    }
                }
            });
        }
    }

    pub async fn handle(&mut self) {
        let work_rx = self.work_rx.clone();
        let next_job = {
            let mut guard = work_rx.lock().await;
            guard.recv().await
        };
        while let Some(ref job_dispatch) = next_job {
            debug!("Processing Chunk {}", job_dispatch.chunk.chunk_id);
            sleep(Duration::new(5, 0)).await;
        }
    }
}

pub struct JobDispatch {
    job_id: String,
    chunk: ChunkState,
}

pub struct JobResult {
    entity_relationships: EntitiesRelationships,
    chunk_id: String,
    job_id: String,
    doc_id: String,
    chunk_order_index: usize,
}

pub struct Queue {
    jobs: VecDeque<String>,         // stores job ids
    jobs_map: HashMap<String, Job>, // for O(1) look up =)
    capacity: u32,
}

impl Queue {
    pub fn new(capacity: u32) -> Self {
        Queue {
            jobs: VecDeque::new(),
            jobs_map: HashMap::new(),
            capacity,
        }
    }

    pub fn enqueue(&mut self, job_id: String, job: Job) -> Result<String> {
        debug!("Enqueing {}", job_id);
        if self.jobs.contains(&job_id) {
            let already_maybe_job = self.jobs_map.get(&job_id);
            if already_maybe_job.is_none() {
                return Err(anyhow!("Job already exists"));
            }
        }
        if self.jobs.len() >= self.capacity as usize {
            return Err(anyhow!("Capacity reached"));
        }
        self.jobs.push_back(job_id.clone());
        self.jobs_map.insert(job_id.clone(), job);
        Ok(job_id)
    }

    pub fn dequeue(&mut self) -> Option<Job> {
        let maybe_job_id = self.jobs.pop_front();
        if let Some(job_id) = maybe_job_id {
            debug!("Dequeing {}", job_id);
            self.jobs_map.remove(&job_id)
        } else {
            None
        }
    }

    pub fn requeue(&mut self, mut job: Job) -> Result<String> {
        debug!("Requeing {}", job.job_id);
        job.next_run_at = Instant::now(); // update next_run_at
        if job.current_retry > job.max_retries {
            return Err(anyhow!("Max retries reachd"));
        }
        self.enqueue(job.job_id.to_owned(), job)
    }

    pub fn peek(&mut self) -> Option<&mut Job> {
        let now = Instant::now();
        let eligible_id = self.jobs.iter().find(|job_id| {
            if let Some(job) = self.jobs_map.get(*job_id) {
                job.next_run_at <= now && job.job_status == JobStatus::Pending
            } else {
                false
            }
        })?;
        self.jobs_map.get_mut(eligible_id)
    }

    pub fn mark_processing(&mut self, job_id: &String) -> Result<()> {
        if let Some(job) = self.jobs_map.get_mut(job_id) {
            job.job_status = JobStatus::Processing;
            Ok(())
        } else {
            Err(anyhow!("Job {job_id} doesn't exist"))
        }
    }

    pub fn mark_done(&mut self, job_id: &String) -> Result<()> {
        if let Some(job) = self.jobs_map.get_mut(job_id) {
            job.job_status = JobStatus::Done;
            Ok(())
        } else {
            Err(anyhow!("Job {job_id} doesn't exist"))
        }
    }

    pub fn mark_failed(&mut self, job_id: &String) -> Result<()> {
        if let Some(job) = self.jobs_map.get_mut(job_id) {
            job.job_status = JobStatus::Failed;
            Ok(())
        } else {
            Err(anyhow!("Job {job_id} doesn't exist"))
        }
    }
}

#[derive(Clone, PartialEq)]
enum JobStatus {
    Pending,
    Processing,
    Done,
    Failed,
    PartiallyFailed,
}

#[derive(Clone)]
pub struct Job {
    pub job_id: String,
    pub doc_id: String,
    max_retries: u8,
    current_retry: u8,
    job_status: JobStatus,
    chunks: Vec<ChunkState>,
    created_at: DateTime<Utc>,
    next_run_at: Instant,
    last_error: Option<String>,
}

impl Job {
    pub fn new(doc_id: String) -> Self {
        let now = Utc::now();
        let job_id = compute_mdhash_id(&format!("{}:{}", doc_id, now.timestamp()), "job-");
        Job {
            job_id,
            doc_id,
            max_retries: 5,
            current_retry: 0,
            job_status: JobStatus::Pending,
            chunks: vec![],
            created_at: now,
            next_run_at: Instant::now(),
            last_error: None,
        }
    }
}

#[derive(Clone)]
pub struct ChunkState {
    pub chunk_id: String,
    pub doc_id: String,
    pub chunk_status: ChunkStatus,
    pub chunk_order_index: usize,
    pub content: String,
    pub error: Option<String>,
    pub output: Option<EntitiesRelationships>,
    pub max_retries: u8,
    pub current_retry: u8,
    pub created_at: DateTime<Utc>,
    pub oai_resp_id: Option<String>,
}

#[derive(Clone)]
pub enum ChunkStatus {
    Success,
    Failed,
    Pending,
    Running,
}
