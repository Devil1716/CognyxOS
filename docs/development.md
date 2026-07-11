# Development setup

Prerequisites: Windows 11, Node 22+, pnpm 10+, Python 3.13+, Rust stable, and Git.

```powershell
./scripts/bootstrap.ps1
pnpm build
pnpm test
pnpm lint
pnpm docs
pnpm dev
```

`COGNYX_ENV`, `COGNYX_LOG_LEVEL`, and `COGNYX_PLUGIN_DIR` override defaults. Copy `.env.example` to `.env` for local configuration; never commit secrets. Development secrets belong in Windows Credential Manager, not configuration files.
