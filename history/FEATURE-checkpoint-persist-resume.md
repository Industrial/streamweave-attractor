# Feature: Checkpoint persistence and resume

## Summary

Add **checkpoint save at pipeline exit** and **resume from a saved checkpoint** so that long or multi-step runs can be restarted from the last successful state (e.g. after a manual stop or tooling restart). This aligns with Attractor spec §3.1 (checkpoint and resume) and uses the existing `Checkpoint` type and `CreateCheckpointNode` plumbing; the runner currently does not persist or resume.

**Branch name:** `feature/checkpoint-persist-resume`

---

## Scope (this iteration)

- **Save checkpoint at exit**: When the compiled graph run completes, write a serializable checkpoint (context, `current_node_id`, `completed_nodes`) to a run directory.
- **Resume from checkpoint**: Allow `run_compiled_graph` and `run_dot` to accept an optional checkpoint (e.g. loaded from disk); when present, start the graph from `checkpoint.current_node_id` with `checkpoint.context` instead of from the start node.
- **No per-node checkpointing in this PR**: Persist only at successful exit. Crash recovery (persist after every node) can be a follow-up.

---

## Implementation steps

### 1. Checkpoint serialization

- Add **serde** (Serialize/Deserialize) to:
  - `Checkpoint` (context, current_node_id, completed_nodes)
  - `RunContext` (alias of `HashMap<String, String>` — ensure it’s serializable; may need a newtype or `#[serde(transparent)]` if needed)
- Add small **checkpoint I/O** helpers in a new module or under `types`:
  - `save_checkpoint(path: &Path, cp: &Checkpoint) -> Result<(), E>`
  - `load_checkpoint(path: &Path) -> Result<Checkpoint, E>` (or `Option<Checkpoint>` if file missing)
- Use JSON via `serde_json` (already a dependency). Decide on a single canonical filename (e.g. `checkpoint.json`) under the run directory.

### 2. Track current node and completed nodes in the compiled graph

- Extend **`GraphPayload`** with:
  - `current_node_id: String`
  - `completed_nodes: Vec<String>`
- Update **`GraphPayload::initial(…)`** to set `current_node_id` to the start node and `completed_nodes: vec![]` (or take optional resume state).
- In every node that forwards or produces payloads (**IdentityNode**, **ExecNode**, **CodergenNode**, **OutcomeRouterNode**):
  - When emitting a payload, set `current_node_id` to this node’s id and append this node’s id to `completed_nodes` (preserve incoming `completed_nodes` and append).
- Ensure the **runner**’s final `AttractorResult` uses `completed_nodes` (and current node) from the received payload instead of `vec![]`.

### 3. Compiler: optional entry point for resume

- In **`compile_attractor_graph`**, add an optional **entry point** (e.g. `entry_node_id: Option<&str>`).
  - When `None`, keep current behavior: graph input is connected to the start node.
  - When `Some(id)`, connect the graph’s single input to `id` instead of start (so the first message goes to the resume node).
- Validate that `entry_node_id` is a valid node id in the graph when provided.

### 4. Runner: run directory and checkpoint at exit

- **`run_compiled_graph`** (and any helper it uses) should:
  - Accept an optional **`run_dir: Option<&Path>`** (or `Option<PathBuf>`). If present, after a successful run, build a `Checkpoint` from the final payload (context, `current_node_id`, `completed_nodes`) and call `save_checkpoint(run_dir.join("checkpoint.json"), &cp)`.
  - Accept an optional **`resume_checkpoint: Option<Checkpoint>`**. If `Some(cp)`:
    - Call `compile_attractor_graph(ast, Some(&cp.current_node_id))` (or equivalent).
    - Build initial `GraphPayload` from `cp.context` and optionally `cp.completed_nodes` / `current_node_id`; send that as the single input to the graph.
- Keep backward compatibility: when `run_dir` is `None` and `resume_checkpoint` is `None`, behavior equals current implementation.

### 5. run_dot CLI

- Add **`--run-dir DIR`**: pass `run_dir` to the runner; checkpoint is written at exit to `DIR/checkpoint.json`.
- Add **`--resume DIR`** (or **`--resume-from DIR`**): load checkpoint from `DIR/checkpoint.json`; if successful, call runner with `resume_checkpoint: Some(cp)` and same graph (from the same .dot file). User can run `run_dot foo.dot --run-dir .attractor_run` then later `run_dot foo.dot --resume .attractor_run`.
- Document in help and README that checkpoint is written only at successful exit (no mid-run crash recovery in this iteration).

### 6. Tests and docs

- **Unit tests**: (a) Checkpoint round-trip (serialize then deserialize). (b) `load_checkpoint` on missing file returns error or None as designed.
- **Integration**: (optional) Run a small .dot pipeline with `--run-dir`, assert `checkpoint.json` exists and contains expected keys; run with `--resume` and assert completion (e.g. exit node runs).
- Update **README** or **CONTRIBUTING** to mention `--run-dir` and `--resume` and that checkpoints are saved at exit.

---

## Out of scope (follow-up)

- Persisting a checkpoint after **every** node (crash recovery).
- Checkpoint format versioning or migration.

---

## Files to touch (checklist)

| Area              | Files |
|-------------------|--------|
| Types             | `src/types/checkpoint.rs`, `src/types/graph_payload.rs`, `src/types/mod.rs` (if RunContext newtype) |
| Checkpoint I/O    | New: `src/checkpoint_io.rs` or under `src/types/` |
| Nodes             | `src/nodes/identity_node.rs`, `src/nodes/exec_node.rs`, `src/nodes/codergen_node.rs`, `src/nodes/outcome_router_node.rs` — update payload handling for current_node_id / completed_nodes |
| Compiler          | `src/compiler.rs` — optional entry_node_id, wire input to entry node |
| Runner            | `src/runner.rs` — run_dir, resume_checkpoint, save at exit, pass entry to compiler |
| CLI               | `src/bin/run_dot.rs` — --run-dir, --resume, load checkpoint and pass to runner |
| Tests             | New or existing tests for checkpoint round-trip, runner resume, run_dot flags |
| Docs              | README or similar |

---

## Branch name

```text
feature/checkpoint-persist-resume
```
