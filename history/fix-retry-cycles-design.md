# Fix-and-retry cycles in compiled model (bb7.6)

**Date:** 2026-02-13  
**Status:** Implemented (hybrid approach)

## Problem

StreamWeave graphs are DAG-only. Attractor workflows with fix-and-retry (e.g. `exec → fail → fix → exec`) have a cycle in the DOT. Compiling such an edge would create a cycle in the StreamWeave graph.

## Design choice: hybrid

We keep the compiled graph as a **DAG** by omitting back-edges from fix nodes to the exec node:

- **Compiler:** When adding edges, skip any edge `fix → exec` where `fix` is the target of an `outcome=fail` edge and `exec` is a node that has an outcome router. The graph then has: `exec → router → success|fail → fix → (no edge to exec)`.
- **Retry semantics:** Retry is handled by the **runner** (e.g. `run_compiled_graph`): run the graph once per attempt; if the result indicates the fail path was taken and retries remain, re-invoke the graph with updated context. No cycles in the graph.

## Alternatives not chosen

- **Unroll:** Compile N copies of the chain. More complex and fixed N.
- **Streamweave cycle support:** Would require library changes; out of scope.

## Implementation

- `src/compiler.rs`: `fix_node_ids` set; when connecting edges, skip edge if `from ∈ fix_node_ids` and `to ∈ routed_sources`.
