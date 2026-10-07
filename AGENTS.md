# AGENTS.md

## Planning vs implementing

- Permission to write planning documents is not permission to implement the plan. Writing or editing `planning/` (or `docs/`) allows only those documents to be written.
- Implementing the plan - creating `DDS/`, `contracts/`, `rust/`, `cpp/`, `scripts/`, `build/`, or any source, build, or generated file - is a separate step and must not be combined with a planning step.
- Only permission to implement the plan is permission to implement the plan.

## Version control

- The agent never commits, amends, pushes, or creates pull requests.
- The agent never asks whether to commit; the user decides when to commit.
- The agent may run read-only git commands (`git status`, `git diff`, `git log`) and must leave all changes in the working tree for the user to review and commit.

## Planning

- Planning lives in `planning/`: `architecture.md` (design), `backlog.md` (all work, prioritized), and `current_story.md` (the detailed plan for the one active story). Read them before writing code.
- `backlog.md` is continually groomed; story order reflects the current best understanding of priority, and the active story's status is tracked there.
- A story's title is a short, unique name; do not restate it as a longer name. The body is a description followed by `Depends on`, `Minimal test`, and `Status`.
- IDs, topic names, and file paths are specified in `planning/backlog.md` and `planning/architecture.md`; use them rather than inventing your own.

## Documentation

- Do not duplicate `planning/architecture.md` in this file. It is the single source for the technology stack, design, folder structure, and file/path conventions; refer to it instead of restating its content here.
- Never put generated or build files in `planning/` or `docs/`.
