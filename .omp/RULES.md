# Permanent Rules — Seej

- Never read, print, or edit secret files (`.env`, `.env.*`, `*.pem`, `*.key`, credentials, or private keys).
- No commit, push, merge, rebase, or tag without explicit user request. Never overwrite or discard uncommitted local changes.
- No new Cargo dependency or installed tool without explicit agreement.
- Never hand-edit `seej/server/Cargo.lock`; regenerate it with Cargo when dependencies change.
- `AGENTS.md` is the single root agent entry point: never create `CLAUDE.md`, `.github/copilot-instructions.md`, or `.omp/AGENTS.md`.
- Never report a check as passing without executing it; cite the exact command and observed result.
- Never commit anything under `work/` or store temporary build artifacts inside tracked source paths.
