# API standards

All future APIs are contract-first and documented before implementation. Methods use `verb_noun` names, resources use reverse-domain IDs, and a service exposes explicit `vN` major namespaces. Minor versions are negotiated through the registry. Unary lists use cursor pagination (`page_size`, `page_token`, `next_page_token`) and stable ordering. Streaming uses ordered chunks, flow control, cancellation, and terminal status.

All calls require authenticated process/session identity and capability-based authorization. Responses use typed success bodies or the common error model—never ad hoc string errors. APIs publish deadline, idempotency, retry, classification, and audit behavior. Deprecation is announced in schema/documentation with a removal date that satisfies the platform compatibility window.

Every API requires contract tests (positive, invalid, unauthorized, timeout, cancellation, version compatibility), generated documentation, schema validation fixtures, and observability fields. Changes must update ADRs where they affect transport, persistence, security, or compatibility.
