use chrono::{DateTime, Utc};

struct Scheduler {
    queue: Queue,
    dispatcher: Dispatch,
    worker: Worker,
}

struct Queue {
    jobs: Vec<Job>,
    capacity: u32,
}

enum JobStatus {
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
    create_at: DateTime<Utc>,
    next_run_at: Instant,
    last_error: Option<String>,
}

struct ChunkState {
    chunk_id: String,
    chunk_status: ChunkStatus,
    error: Option<String>,
    output: Option<String>,
    max_retries: u8,
    current_retry: u8,
}

enum ChunkStatus {
    Success,
    Failed,
}
