# Implementation Plan: execution.log.json as Single Source of Truth

## Goal

Replace checkpoint.json with execution.log.json as the single artifact for run state. Write to the log on every execution step (incremental), and resume from the log (partial or complete).

## Current State

- **Checkpoint**: context, current_node_id, completed_nodes. Written once at successful exit to run_dir/checkpoint.json. Loaded when user passes --resume DIR.
- **Execution log**: version, goal, started_at, finished_at, final_status, completed_nodes, steps[]. Written once at end of run when execution_log_path is set. Not used for resume.
- **Resume today**: --resume DIR loads DIR/checkpoint.json and passes resume_checkpoint into run_compiled_graph.

## Target State

- **Single file**: execution.log.json (path from execution_log_path or default under stage/run dir).
- **Incremental writes**: After each step in the sync execution loop, rewrite the log so the file on disk reflects current progress (crash recovery).
- **Resume from log**: On start, if log path exists, load it. If finished_at is set -> treat as already completed. If partial (no finished_at) -> derive resume state from last step and continue.
- **No checkpoint.json**: Stop writing/reading checkpoint for resume. CLI --resume DIR means resume from DIR/execution.log.json.

## Design Decisions

1. **Partial runs**: finished_at: null, final_status: "running". Last step in steps defines resume state.
2. **Write strategy**: Rewrite whole file after each step (simpler, atomic).
3. **Sync path only**: Incremental log and resume-from-log for sync path (execution_log_path set). Async path unchanged for now.
4. **Resume state**: From last ExecutionStepEntry: context = context_after, completed_nodes = completed_nodes_after, current_node_id = next_node_id or last of completed_nodes_after.

## Task Summary for Beads

1. ExecutionLog/ExecutionStepEntry: add Deserialize.
2. execution_log_io: load_execution_log, write_execution_log_partial (rewrite after each step).
3. resume_state_from_log: derive Checkpoint from ExecutionLog (partial only).
4. execution_loop: support persist-after-each-step (callback or one-step API).
5. runner: on start load log; if complete return already_completed, if partial resume; after each step write log.
6. runner: remove save_checkpoint for sync path.
7. run_dot: --resume DIR loads DIR/execution.log.json.
8. Tests: update to log; add partial-resume and already-completed-from-log tests.
9. Remove checkpoint from resume flow.
