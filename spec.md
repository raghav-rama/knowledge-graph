# Knowledge Graph Technical Spec

## Intelligence

- Chunking; Write the advanced chunking logic
- Entity Extraction Logic
- Search Logic - short terms
- Full-blown agentic retrieval - atomic question creation

## API

- /graph-search
- /qa

## Low Level Impl Details

- Production grade job queue system
- Has a multi-threaded paper processing pipeline
- Primitives used
  - ARC
  - MPSC
  - Mutex (for safely accessing the queue)
- Scheduler scans for avaiable jobs
  - If no jobs, or only one job, sleep until that job needs to be run, or every 10 secs
  - If job found, dispatcher sends it to the worker thread pool
  - Once a chunk is processed by worker result (success/failure) is communicated over the channel
  - If a chunk exhausts max retries job is marked partially failed and marked for requeing
  - Process repeats until all chunks are processed or job fails entirely after exhausting its max retries
