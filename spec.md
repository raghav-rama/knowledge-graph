# Knowledge Graph Technical Spec

## Intelligence

- **Context-Aware Sequential Chunking**

  - **Logic:** Iteratively splits a document into semantically coherent chunks by passing a summary of the previous chunk forward to inform the processing of the next.
  - **Process:**
    1.  Initialize `forward_summary` as an empty string.
    2.  For each `text_block` (e.g., 1000-token segment) in the source document:
    3.  Call an LLM with a prompt to summarize the `text_block`, including the `forward_summary` as prior context.
    4.  Store the `text_block` as a `Chunk` node and the resulting `new_summary` in its metadata.
    5.  Set `forward_summary = new_summary` and loop to the next block.

- **Entity Extraction Logic**

  - **Extractor implementation:** `EntityRelationshipExtract` (`runtime/src/pipeline/extractor.rs`) wraps `ResponsesClient::responses_structured`, calling `gpt-5-mini` and passing the chunk text (`chunk.content`) as user input. Each request sets `conversation_id = chunk.id` so retries remain stable.
  - **Schema contract:** responses must validate against `entities_relationships_schema()` (`runtime/src/ai/schemas.rs`), which enforces:
    - `entities`: array of `{ entity_name, entity_type, entity_description }`, where `entity_type` ∈ `BASE_ENTITY_TYPES` (gene, protein, compound, etc.) with optional longevity extensions if enabled. (Entity types refined by Lukas et el)
    - `relationships`: array of `{ source_entity, target_entity, relationship_keywords[], relationship_description }`, all names title-cased and kept consistent with extracted entities.
    - Descriptions must be grounded strictly in chunk text; empty or whitespace-only outputs are rejected.
  - **Post-processing rules:**
    - Deduplicate entities per chunk by `(entity_name, entity_type)` key; merge descriptions when the model emits near-duplicates.
    - Normalize casing of names (title case) and trim punctuation before persisting to KV/vector stores.
    - Enrich relationships by splitting `relationship_keywords` into individual tags for downstream search.
  - **Error handling:** extraction returns `anyhow::Result<EntitiesRelationships>`; worker catches errors, increments chunk retry counters, and schedules backoff. After `max_chunk_retries`, chunk marked failed and the parent job enters `PartiallyFailed`.
  - **Metrics/observability:** log chunk id, entity/relationship counts, and model latency; emit counters for schema validation failures vs. upstream API errors to guide prompt or schema adjustments.

- **Search Logic - short terms revealing paths**

  - Example if the search term was 'Oleoylethanolamide'
  - `Oleoylethanolamide --[Oleoylethanolamide supplementation was investigated for its effect on the abundance of Akkermansia muciniphila in people with obesity in a randomized clinical trial (Appetite. 2019).]--> Akkermansia muciniphila`
  - `Akkermansia muciniphila --[Cross-talk between Akkermansia muciniphila and the intestinal epithelium controls diet-induced obesity (PNAS. 2013).]--> Diet-Induced Obesity`

- **Agentic Rationale Retrieval (inspired from PIKE-RAG)**

  - **Logic:** A two-part system. First, an ingestion pipeline creates an index of potential questions for each chunk. Second, a query-time agent iteratively decomposes a user's question, searching this index to build a rationale before answering.

  - **1. Indexing (Ingestion Pipeline):**

    1.  For each `Chunk` created by the chunking process:
    2.  Call an LLM with a prompt: "Generate 5-10 specific, atomic questions that this text chunk can directly answer."
    3.  Generate embeddings for these new questions.
    4.  Store them as `AtomicQuestion` nodes in the graph/vector store.
    5.  Create a graph edge: (`AtomicQuestion`) --\[**ANSWERS_IN**\]--\> (`Chunk`).
    6.  PS: This `AtomicQuestion` generation module is learnable and can be improved by fine-tuning also.

  - **2. Retrieval (Query-Time Decomposer):**

    1.  Initialize `original_query` (String) and `accumulated_context` (List\<Chunk\>).
    2.  **Start Loop** (e.g., max 5 iterations):
    3.  Call "Decomposer" LLM with `original_query` + `accumulated_context`. Prompt: "Given the goal and this context, what 3 sub-questions should we find answers for next?"
    4.  Receive `proposed_questions` (List\<String\>).
    5.  Use `proposed_questions` as vector search queries against the `AtomicQuestion` index.
    6.  Select the top-matching `AtomicQuestion` node (e.g., highest similarity score).
    7.  If no match is found or similarity is too low, **break loop**.
    8.  Traverse the **`ANSWERS_IN`** edge from the matched `AtomicQuestion` to its parent `Chunk`.
    9.  Add this `Chunk` to `accumulated_context`.
    10. **End Loop**.
    11. Call final "Reasoner" LLM with `original_query` + `accumulated_context` to synthesize the final answer.

## API

- /graph-search - single term search revealing pathways
- /qa - reason and answer

## Low Level Impl Details

- Production-grade job queue system
  - Queue storage = `VecDeque<JobId>` + `HashMap<JobId, Job>` protected by `Arc<Mutex<_>>` for O(1) lookup and FIFO ordering.
  - Capacity guard rejects uploads when `len >= capacity`, preventing unbounded memory growth.
  - Job struct tracks `job_id`, `doc_id`, `status`, `retry_attempts`, `next_run_at`, and per-chunk state (`ChunkState` with attempt counters, errors, outputs).
  - Helper API on `Queue` (`enqueue`, `peek_ready`, `mark_running`, `schedule_retry`, `complete`) enforces invariants so callers never mutate internal maps directly.
- Multi-threaded paper processing pipeline
  - Scheduler runs as a Tokio task, peeking the queue without holding the mutex across awaits; releases lock before chunking or dispatch to avoid blocking uploads.
  - Chunking happens via `Pipeline::chunker`, yielding `Chunk` structs consumed by workers.
  - Workers are Tokio tasks spawned in a pool (size configurable). A single `tokio::sync::mpsc::Receiver<JobDispatch>` is wrapped in `Arc<Mutex<_>>`; each worker locks long enough to `recv().await`, drops the lock, and processes chunks concurrently.
  - Result flow uses a second `mpsc::Sender<JobResult>` so workers can report success/failure back to the scheduler loop.
- Synchronization primitives
  - `Arc` for sharing queue, pipeline, storages, and channel handles across async tasks.
  - `Mutex` around the queue and shared receiver to provide exclusive access during dequeue/recv operations.
  - `tokio::sync::mpsc` channels (`work_tx`, `result_tx`) for scheduler→worker dispatch and worker→scheduler feedback.
- Scheduler cycle
  - Poll interval defaults to 10s or sleeps until the next job’s `next_run_at`.
  - When a ready job is found, dispatcher enqueues each chunk as a `JobDispatch { job_id, chunk }`.
  - Workers process chunks, push `JobResult` with status/error data.
  - Scheduler consumes results, updates chunk states, and decides whether to mark job `Done`, `PartiallyFailed`, or schedule retry with exponential backoff.
  - If any chunk exceeds max retries, job becomes `PartiallyFailed` and is requeued or dead-lettered based on policy; otherwise loop continues until all chunks succeed or job-level retries exhausted.
