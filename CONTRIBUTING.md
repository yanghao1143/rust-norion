# Contributing

rust-norion accepts public collaboration through issues and pull requests.

## Ground Rules

- All changes must be reviewed and approved by the repository maintainer before
  merge.
- Do not push directly to protected branches.
- Keep contributions compatible with the repository license: non-commercial
  research, education, evaluation, and experimental deployment only.
- Do not copy GPL, AGPL, commercial, or otherwise incompatible code into this
  repository.
- `fortunto2/rust-code` may be used only as an MIT-licensed reference or port
  with attribution.
- `Kuberwastaken/claurst` may be used only for high-level architecture
  inspiration unless the project explicitly accepts GPL-3.0 obligations.
- Do not commit local state, memory databases, `.ndkv` files, model weights,
  credentials, logs, generated `target` directories, or private datasets.

## Pull Requests

Every pull request should include:

- A short description of the behavioral change.
- The validation commands that passed.
- Any rollback plan for self-evolving or memory-admission changes.
- A note confirming that no commercial-use permission is being requested by the
  pull request itself.

The maintainer may ask for focused tests, benchmark evidence, or a smaller
scope before approving a merge.
