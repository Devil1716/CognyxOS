# Filesystem capabilities

The contract defines read, write/create, copy, move, delete, list, metadata, permissions, search, and watch. The scoped local provider implements read, write/create, copy, move, delete, list, metadata, and permissions. It rejects absolute paths and traversal outside its configured root; search/watch await dedicated providers.
