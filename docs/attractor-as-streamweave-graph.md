# Attractor as a StreamWeave Graph

**Status:** Design / plan  
**Goal:** Move workflow logic from Rust into **graph topology**. Treat the DOT file as source that compiles to a StreamWeave graph; the graph structure is the program.

**Audience:** Implementers and reviewers of streamweave-attractor.

---

## 1. Summary

Today, streamweave-attractor runs a **single imperative loop** inside one StreamWeave node (ParseDot → Validate → Init → **ExecutionLoop**). Control flow, branching, retries, and edge selection are implemented in Rust. This document describes a **target architecture** where:

- **DOT is compiled** to a StreamWeave graph of small, dedicated nodes (exec, condition, fix, start/exit).
- **Logic lives in the graph:** algorithms are expressed as nodes and edges, not as `match` and loops in Rust.
- The compiler (DOT → Attractor AST → StreamWeave graph) becomes the only place that interprets the DSL; the runtime is generic.

---

## 2. Design Principles

### 2.1 Logic in Topology, Not in Code

StreamWeave's strength is **graph-as-program**: which nodes exist, how they are connected, and what flows on the edges defines behavior. The target design pushes all workflow logic into the graph so that:

- Adding a new workflow pattern (e.g. a new retry shape) can be done by adding node types and wiring, not by changing a central loop.
- The execution engine stays generic; it does not branch on Attractor-specific concepts.
- Debugging and visualization map directly to the graph.

### 2.2 `exec` Handler Requires `command`

For handler type `exec` (tool execution), the DSL **must** provide a `command` attribute. Without it, the node has nothing to run. The compiler should reject such nodes or treat them as invalid. This is a DSL rule, not only a runtime check.

### 2.3 Align with Attractor Spec

Behavior (edge selection, conditions, goal gates, retries) follows the [Attractor Specification](https://github.com/strongdm/attractor/blob/main/attractor-spec.md). References to "attractor-spec §X" below point to that document.

---

## 3. Current Architecture (Imperative Engine)

The pipeline is a linear StreamWeave graph:

```
graph.input → ParseDot → Validate → Init → ExecutionLoop → graph.output
                                                    ↓
                                              graph.error
```

- **ParseDotNode**  
  Consumes DOT source (string), produces `AttractorGraph` (or error). Implemented in `src/nodes/parse_dot.rs`; parsing in `src/dot_parser.rs`.

- **ValidateGraphNode**  
  Validates the graph (exactly one start, one exit) per attractor-spec §7.  
  `src/nodes/validate_graph.rs` → `validate(&AttractorGraph)`.

- **InitContextNode**  
  Builds initial `ExecutionState`: start node id, empty context, `graph.goal` in context.  
  `src/nodes/init_context.rs` → `create_initial_state(graph)`.

- **AttractorExecutionLoopNode**  
  Single node that runs the full traversal loop in Rust:
  1. Look up current node; call `execute_handler` (dispatch by `handler_type`: start, exit, codergen, or stub).
  2. Apply context updates from `NodeOutcome`; record outcome; call `select_edge` (attractor-spec §3.3).
  3. Advance `current_node_id` or terminate.

Control flow, branching, and retry semantics are thus **encoded in Rust** inside this one node, not in the graph. `execute_handler` in `src/nodes/execute_handler.rs` does not yet implement `exec` with `command`; such nodes currently fall through to a generic stub.

Supporting pieces used inside or alongside this pipeline:

- **ExecuteHandlerNode**, **SelectEdgeNode**  
  Wrappers around `execute_handler` and `select_edge`; used by the loop (or for composition).
- **ApplyContextUpdatesNode**, **CheckGoalGatesNode**, **CreateCheckpointNode**, **FindStartNode**  
  Used internally or for finer-grained composition.

Existing building blocks that are **not** wired into the main graph today:

- **ExecNode** (`src/nodes/exec_node.rs`): runs a shell command, emits `NodeOutcome` (success on exit 0, fail otherwise).
- **FixNode** (`src/nodes/fix_node.rs`): stub that forwards a trigger (for retry loops).

---

## 4. Target Architecture (Compiled Graph)

### 4.1 Compilation Model

```
DOT source  →  Parse  →  Attractor AST  →  Compile  →  StreamWeave graph
```

- **Parse:** Existing DOT parser → `AttractorGraph` (nodes, edges, attributes).
- **Attractor AST:** The current in-memory model (`AttractorNode`, `AttractorEdge`, etc.) is the AST.
- **Compile:** A new compiler step that, given `AttractorGraph`, produces a StreamWeave `Graph` of primitive nodes and edges. No single "execution loop" node; traversal is implicit in the graph.

### 4.2 Mapping Attractor Constructs to StreamWeave

| Attractor concept | StreamWeave realization |
|-------------------|---------------------------|
| `type=exec`, `command="..."` | **ExecNode** (or equivalent): one node per exec, command from attribute. |
| Start node (`shape=Mdiamond` / `id=start`) | **Identity / no-op node**: pass-through, no side effect. |
| Exit node (`shape=Msquare` / `id=exit`) | **Identity / no-op node**: pass-through; downstream can treat as "done". |
| `type=codergen` (or default box) | **Placeholder / stub node** until codergen is implemented as a node. |
| Edge `condition="outcome=success"` / `outcome=fail` | **ConditionNode or SwitchNode**: route the stream (e.g. by `NodeOutcome.status`) to the correct outgoing edge. |
| Retry on failure | **RetryNode** or a **fix-loop**: subgraph with feedback edge from fix node back to the exec node. |
| Fix-and-retry loop | Subgraph: Exec → (success → next; fail → FixNode → back to Exec). |

The compiler's job is to turn each Attractor node and edge into the right StreamWeave nodes and edges so that "run this DOT workflow" is "run this compiled graph."

### 4.3 Example: `pre-push.dot` as a Compiled Graph

`examples/workflows/pre-push.dot` defines:

- **start** → **pre_push** (exec: `devenv shell -- bin/pre-push`) → **test_coverage** (exec: `devenv shell -- bin/test-coverage`) → **exit**.
- On success: pre_push → test_coverage; test_coverage → exit.
- On failure: pre_push → fix_pre_push → back to pre_push; test_coverage → fix_test_coverage → back to test_coverage.

In the target design, this becomes a StreamWeave graph with:

- **Exec nodes** for `pre_push` and `test_coverage`, each with its `command`.
- **Condition / switch nodes** (or equivalent routing) so that `outcome=success` goes to the next stage and `outcome=fail` goes to the fix node.
- **Fix nodes** (e.g. FixNode or codergen stubs) with prompts, feeding back into the corresponding exec node.

So the **structure** of pre-push (linear chain + two fix loops) is expressed as graph topology (nodes and edges), not as logic in an execution loop.

---

## 5. Implementation Phases

A possible ordering that builds the compiled-graph design incrementally:

| Phase | Focus | Deliverable |
|-------|--------|--------------|
| **1** | Compiler skeleton | DOT → AttractorGraph → "trivial" StreamWeave graph (e.g. single path start→exit). No exec yet. |
| **2** | Handler nodes | Wire ExecNode for `type=exec` + `command`; start/exit as identity nodes; codergen as stub. |
| **3** | Conditional routing | Implement routing for `condition="outcome=success"` / `outcome=fail` (ConditionNode/SwitchNode or equivalent). |
| **4** | Retry / fix loops | Subgraphs with feedback: exec → fail → fix → back to exec; optional RetryNode if available. |
| **5** | Full pre-push.dot | Compile `examples/workflows/pre-push.dot` to a graph that runs the same workflow as the current imperative loop. |

Phases can be refined or reordered based on StreamWeave's actual node set (e.g. whether condition_node, switch_node, retry_node exist in [Industrial/streamweave](https://github.com/Industrial/streamweave) and how they are used).

---

## 6. Benefits

- **Logic in topology:** Workflow is visible and editable as a graph; no hidden control flow in a monolithic loop.
- **Dedicated nodes:** Exec, condition, fix, start/exit are separate, testable, reusable.
- **Condition-based routing:** `outcome=success` / `outcome=fail` become explicit edges from a condition/switch node.
- **Retry/fix as structure:** Fix-and-retry is a subgraph with a back-edge, not special-case code.
- **Extensibility:** New behavior = new node types and wiring; engine stays generic.
- **Any valid DOT** can in principle be compiled to a graph, with a single interpretation path (the compiler).

---

## 7. References

- **Attractor Specification**  
  https://github.com/strongdm/attractor/blob/main/attractor-spec.md  
  Sections especially relevant: §2 (DOT schema, node/edge attributes), §3 (execution engine, edge selection §3.3, goal gates §3.4, retry §3.5–3.7), §4 (handlers), §7 (validation).

- **StreamWeave**  
  https://github.com/Industrial/streamweave  
  For graph construction, node types (e.g. condition_node, switch_node, retry_node, graph_macro_*), and streaming semantics.

- **This repo**
  - Pipeline construction: `src/graph.rs` (`attractor_graph()`).
  - Execution loop: `src/nodes/execution_loop.rs` (`run_execution_loop_once`, `AttractorExecutionLoopNode`).
  - Edge selection: `src/nodes/select_edge.rs` (attractor-spec §3.3).
  - Handlers: `src/nodes/execute_handler.rs`; exec building block: `src/nodes/exec_node.rs`; fix building block: `src/nodes/fix_node.rs`.
  - Example workflow: `examples/workflows/pre-push.dot`.
