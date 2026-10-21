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

- Planning lives in `planning/`: `architecture.md` (design), `backlog.md` (remaining work, prioritized), `completed_stories.md` (done stories), and `current_story.md` (the detailed plan for the one active story). Read them before writing code.
- `backlog.md` is continually groomed; story order reflects the current best understanding of priority, and the active story's status is tracked there.
- When a story is done, move its entry from `backlog.md` to `completed_stories.md`, recording its completion timestamp; entries stay grouped by increment and ordered by completion time.
- A story's title is a short, unique name; do not restate it as a longer name. The body is a description followed by `Depends on`, `Minimal test`, and `Status`.
- IDs, topic names, and file paths are specified in `planning/backlog.md` and `planning/architecture.md`; use them rather than inventing your own.
- When planning a new story, do not list already-completed stories as prerequisites; list only the not-yet-done stories it depends on.

## Documentation

- Do not duplicate `planning/architecture.md` in this file. It is the single source for the technology stack, design, folder structure, and file/path conventions; refer to it instead of restating its content here.
- Never put generated or build files in `planning/` or `docs/`.

## Writing

- Do not use "vendor" to mean copying a dependency's source into the repository (for example "vendor a dependency", "vendored source" in that sense). It is jargon from a few ecosystems, not general usage. Say what is meant: "copy the dependency's source into the repository", "install the dependency to the external prefix", or "check the dependency out to an external location".
- "Vendor" meaning a supplier is fine (for example "vendor root namespace", "source from a vendor").
- Do not navigate a path with "..". Address in-repository locations relative to the repository root, and external components by an absolute path under the external prefix (`docs/versions.md`).
