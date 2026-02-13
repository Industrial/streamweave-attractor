# Workflow DOT files

| Workflow | Status | Notes |
|----------|--------|-------|
| **pre-push.dot** | ✅ Supported | Uses `outcome=success` / `outcome=fail`; maps to out/error ports. |
| **ai-feature.dot** | ✅ Supported | Same as above. |
| **test_out_error.dot** | ✅ Supported | Minimal test for out/error and fix→exit. |
| **test_success_only.dot** | ✅ Supported | Success path only. |
| **beads-worker-loop.dot** | ✅ Supported | Uses `outcome=success` / `outcome=fail` for check_ready (has tasks → success → claim; no tasks → fail → exit). |
