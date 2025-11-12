use super::{chunker::Chunk, utils::compute_mdhash_id};
use crate::ai::schemas::EntitiesRelationships;
use chrono::{DateTime, Utc};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};
use tokio::{
    sync::{
        Mutex,
        mpsc::{Receiver, Sender},
    },
    time::Instant,
};

use crate::AppState;

struct Scheduler {
    queue: Arc<Mutex<Queue>>,
    dispatcher: Dispatcher,
    workers: Worker,
    result_rx: Receiver<JobResult>,
}

impl Scheduler {
    pub async fn run(self) {
        loop {
            let maybe_job = self.queue.as_ref();
        }
    }
}

struct Dispatcher {
    work_tx: Sender<JobDispatch>,
    max_inflight: u8,
    inflight: HashSet<String>,
}

struct WorkerHandles {
    work_rx: Receiver<JobDispatch>,
    result_tx: Sender<JobResult>,
}

struct Worker {
    handler: Vec<WorkerHandles>,
}

struct JobDispatch {
    job_id: String,
    chunk: Chunk,
}

struct JobResult {
    entity_relationships: EntitiesRelationships,
    chunk_id: String,
    job_id: String,
}

struct Queue {
    jobs: VecDeque<String>,
    jobs_map: HashMap<String, Job>, // for O(1) look up =)
    capacity: u32,
}

enum JobStatus {
    Pending,
    Processing,
    Done,
    Failed,
    PartiallyFailed,
}

struct Job {
    job_id: String,
    doc_id: String,
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

struct ChunkState {
    chunk_id: String,
    chunk_status: ChunkStatus,
    error: Option<String>,
    output: Option<String>,
    max_retries: u8,
    current_retry: u8,
    created_at: DateTime<Utc>,
}

enum ChunkStatus {
    Success,
    Failed,
    Pending,
    Running,
}
