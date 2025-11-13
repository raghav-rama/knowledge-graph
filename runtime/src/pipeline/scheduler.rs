use super::{chunker::Chunk, utils::compute_mdhash_id};
use crate::ai::schemas::EntitiesRelationships;
use anyhow::{Ok, Result, anyhow};
use chrono::{DateTime, Utc};
use std::{
    collections::{HashMap, HashSet, VecDeque},
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
use tracing::debug;

use crate::AppState;

pub struct Scheduler {
    pub queue: Arc<Mutex<Queue>>,
    dispatcher: Dispatcher,
    workers: Worker,
    result_rx: Receiver<JobResult>,
}

impl Scheduler {
    pub fn new(
        max_inflight: u8,
        capacity: u32,
        work_tx: Sender<JobDispatch>,
        result_rx: Receiver<JobResult>,
    ) -> Self {
        let queue = Arc::new(Mutex::new(Queue::new(capacity)));
        Scheduler {
            queue,
            dispatcher: Dispatcher::new(work_tx, max_inflight),
            workers: Worker::new(),
            result_rx,
        }
    }
    pub async fn run(&self) {
        loop {
            let now = Instant::now();
            let guard = self.queue.lock().await;
            if let Some(job) = guard.peek() {
                debug!("executing job {}", job.job_id);
            } else {
                debug!("no job found")
            }
            drop(guard);

            sleep(Duration::new(10, 0)).await;
        }
    }
}

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

struct WorkerHandles {
    work_rx: Receiver<JobDispatch>,
    result_tx: Sender<JobResult>,
}

struct Worker {
    handler: Vec<WorkerHandles>,
}

impl Worker {
    pub fn new() -> Self {
        Worker {
            handler: Vec::new(),
        }
    }
}

pub struct JobDispatch {
    job_id: String,
    chunk: Chunk,
}

pub struct JobResult {
    entity_relationships: EntitiesRelationships,
    chunk_id: String,
    job_id: String,
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
            self.jobs_map.remove(&job_id)
        } else {
            None
        }
    }

    pub fn requeue(&mut self, mut job: Job) -> Result<String> {
        job.next_run_at = Instant::now(); // update next_run_at
        if job.current_retry > job.max_retries {
            return Err(anyhow!("Max retries reachd"));
        }
        self.enqueue(job.job_id.to_owned(), job)
    }

    pub fn peek(&self) -> Option<&Job> {
        let maybe_job_id = self.jobs.front();
        if let Some(job_id) = maybe_job_id {
            let maybe_job = self.jobs_map.get(job_id);
            if let Some(job) = maybe_job {
                if job.next_run_at <= Instant::now() {
                    Some(job)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

enum JobStatus {
    Pending,
    Processing,
    Done,
    Failed,
    PartiallyFailed,
}

pub struct Job {
    pub job_id: String,
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
