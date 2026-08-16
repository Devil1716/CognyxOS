# Phase 10: Advanced intelligence + long-term memory

**Status:** IMPLEMENTED (extends Phase 3 WorkingMemory; local vector store)  
**Last Updated:** 2026-08-14

## Overview

Long-term memory sits beside the existing `ContextEngine` / `WorkingMemory`.
Phase 3 types are unchanged.

Memory kinds: ShortTerm, Working, Episodic, Semantic, Preference, Task,
Artifact.

## Storage

`VectorStoreProvider` is a trait. The first implementation is
`LocalVectorStore` (in-process). No Qdrant/Pinecone/Milvus hardcoded.

Embeddings are a local hash embedder, not a hosted vendor.

## Privacy

Every record has owner, scope, retention, visibility, classification,
consent. Secrets/credentials are refused. Deletion removes the record
from the map and the vector store (real deletion, not a hide flag).

Disabled mode skips indexing and retrieval.

Retrieval is ranked and capped (`max_inject`) so planners cannot dump
unlimited memory into model context.

## Planner / routing / reflection

- `retrieve(query)` is the pre-plan context hook
- `ModelRouter` selects small/large local, remote, vision, specialized
  without hardcoding a vendor SDK
- `Reflection` records what worked/failed; it does not modify system code

## What this phase does not claim

- Not a production vector database
- Not persistent across process restart (in-memory local store)
- Not an autonomous self-modifying agent

## Next

Phase 10.5: developer SDK + plugins.
