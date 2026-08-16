# Execution-plan dataflow

Plan inputs may reference prior node JSON output using `${step-id.path}`, including array selection such as `${step-1.applications[0].application_id}`. Validation requires the referenced node to exist and be a declared dependency.

The kernel maintains completed-node outputs while scheduling the graph. The gateway resolves references immediately before constructing a capability request. A missing application selection returns `APPLICATION_NOT_FOUND`; malformed or non-scalar references return `PLAN_INVALID`. Providers never receive a null application ID.
