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
- Never mark a story done or move it to `completed_stories.md` unless the user asks you to plan the next story. Until then, leave the active story in `backlog.md` as `in progress` and keep its detailed plan in `current_story.md`. When the user asks to plan the next story, move the finished story to `completed_stories.md` with its completion timestamp; entries stay grouped by increment and ordered by completion time.
- A story's title is a short, unique name; do not restate it as a longer name. The body is a description followed by `Depends on`, `Minimal test`, and `Status`.
- IDs, topic names, and file paths are specified in `planning/backlog.md` and `planning/architecture.md`; use them rather than inventing your own.
- When planning a new story, do not list already-completed stories as prerequisites; list only the not-yet-done stories it depends on.

### Epics and detailed plans

- An epic is a high-level umbrella story in `backlog.md` for a goal that spans multiple stories. A story may remain high-level until it is refined; placeholder umbrella stories are allowed, whether added by the user or at the user's instruction.
- A detailed plan for an epic is its decomposition: the set of actionable stories needed to achieve it, added to `backlog.md` using the story conventions above and the IDs, topic names, and paths from `backlog.md` and `architecture.md`, ordered by dependency and priority. The user may choose whether the epic entry remains as the umbrella or is replaced by its stories.
- A detailed plan for a single story lives in `current_story.md` and is written when the user asks to plan that story.
- Epic decomposition does not create or rewrite `current_story.md`, mark any story `in progress`, or begin implementation. Add the stories and stop; the user will separately ask for the detailed plan for the next story.

## Build and deploy artifacts

- Final build products under `build/deploy` must be self-contained and relocatable. They must not embed or require repository paths (`REPO_ROOT`, absolute compile-time paths, or paths outside the install tree).
- Executables and deployable libraries resolve assets, contracts, and configuration relative to their own location in the deploy layout (for example `<exe_dir>/../config`, `<exe_dir>/../assets`) or from explicit command-line options. They must work from any working directory.
- Deployed executables must not depend on an externally set environment variable to find their configuration. If a library requires an environment variable internally (for example CycloneDDS's `CYCLONEDDS_URI`), the executable sets it itself from its deploy-relative configuration before initialization. An explicit command-line option may override the deploy-relative default.
- `REPO_ROOT` is permitted only at build/test time (code generation, tests, tooling) and must not be a runtime default in a deployed crate. Runtime code composes parent directories rather than using `..` strings.

## Verification and iteration

- Avoid unnecessary iteration on screenshots or test builds; they are expensive and time-consuming.
- Run the minimum verification needed.
- Never delete, overwrite, or use as scratch the real `%LOCALAPPDATA%\to-stations` runtime state. GUI smoke tests must launch processes with a temporary `LOCALAPPDATA` and remove only that temporary directory.
- Up to three iterations are allowed before user input is required.
- If a problem remains after three iterations, or user input is needed, ask and stop.

## Documentation

- Do not duplicate `planning/architecture.md` in this file. It is the single source for the technology stack, design, folder structure, and file/path conventions; refer to it instead of restating its content here.
- Never put generated or build files in `planning/` or `docs/`.

## Writing

- Do not use "vendor" to mean copying a dependency's source into the repository (for example "copy a dependency's source into the repository"). It is jargon from a few ecosystems, not general usage. Say what is meant: "copy the dependency's source into the repository", "install the dependency to the external prefix", or "check the dependency out to an external location".
- "Vendor" meaning a supplier is fine (for example "vendor root namespace", "source from a vendor").
- Do not navigate a path with "..". Address in-repository locations relative to the repository root, and external components by an absolute path under the external prefix (`docs/versions.md`).
