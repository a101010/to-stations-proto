# Current story: topic-field-metadata

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Consume the fork's `DdsFieldMetadata` implementation by re-pinning `cyclonedds-rust` from `acef684` to `952906b`, regenerate the hello_world types, and add a `stations-dds` acceptance test that enumerates a generated topic's fields and round-trips a value through the typed sample. Do not change `dds-echo` in this story.

- **Depends on:** none; the fork implementation is available at `952906b`.
- **Minimal test:** `stations-dds/tests/field_metadata.rs` enumerates the fields of `HelloWorldTopic`/`HelloWorldViewTopic` from `DdsFieldMetadata::field_schema` and round-trips typed values through `to_dynamic_value`/`from_dynamic_value`.

## Context

The to-stations tool `dds-echo` (and the future `tentacles-injector`) needs generic field views for any topic registered in `contracts/topics.toml`. Rust has no runtime reflection, so the field knowledge must be generated at compile time.

The fork already implements this work on branch `multifile` at rev `952906b`:

- `cyclonedds/src/field_metadata.rs` defines `DdsFieldMetadata`.
- `cyclonedds-build` emits `field_schema`, `to_dynamic_value`, `from_dynamic_value`, and the field get/set helpers for generated structs, enums, bitmasks, and unions.
- The fork's test suite covers schema enumeration and round-tripping for primitives, strings, bounds, typedefs, nested structs, sequences, arrays, optionals, enums, bitmasks, and unions.

`to-stations-proto` still pins `acef684`, which predates the trait, so the generated hello_world types have no field metadata. This story is therefore an integration and verification story, not a second implementation of the fork feature.

The fork plan explicitly leaves `dds-echo` UI work to the later `dds-echo-send` story. The detail pane will continue to use its interim pretty-print for now.

## Decisions

1. **Consume the fork; do not duplicate it.** Re-pin to `952906b5018831683c0863cd11001bdac427ee10`. Do not copy the fork's source into this repository.
2. **Update both pins.** `rust/Cargo.toml` and `scripts/build.ps1` carry independent revision strings; both must use the new rev. Regenerate `rust/Cargo.lock` for `cyclonedds`, `cyclonedds-derive`, `cyclonedds-rust-sys`, and `cyclonedds-src`. Key the codegen install target directory by rev so Cargo cannot reuse the previous `cyclonedds-idlc` binary when the package version is unchanged.
3. **No `dds-echo` changes.** This story exposes and verifies the generated metadata. `dds-echo`'s detail pane and send form remain for `dds-echo-send`/the detail-pane follow-up.
4. **No new dependencies.** The fork feature uses `std::sync::OnceLock`, `DynamicTypeSchema`, and `DynamicValue`; to-stations gains only the pinned revision.
5. **Test the actual generated types.** The fork tests cover the full type matrix. The to-stations test uses the hello_world IDL types that this repository actually builds and consumes.

## Interfaces

`cyclonedds::DdsFieldMetadata` (native feature) provides:

```rust
pub trait DdsFieldMetadata: Sized + 'static {
    fn field_schema() -> &'static DynamicTypeSchema;
    fn to_dynamic_value(&self) -> DynamicValue;
    fn from_dynamic_value(value: &DynamicValue) -> DdsResult<Self>;
    fn field_names() -> Vec<String>;
    fn get_field(&self, name: &str) -> DdsResult<DynamicValue>;
    fn set_field(&mut self, name: &str, value: DynamicValue) -> DdsResult<()>;
}
```

For the current IDL, codegen should emit impls for:

- `math::Quaternion`
- `hello_world::HelloWorldTopic`
- `hello_world::HelloWorldViewTopic`

The generated `HelloWorldTopic` schema is a struct with `greeting: string` and `attitude: math::Quaternion`. The view schema adds `valid: boolean`.

## Implementation steps

1. **Planning bookkeeping.** The completed `hello-service` story has been moved to `completed_stories.md` with completion time `2026-10-27 10:08:01` and removed from `backlog.md`; `topic-field-metadata` is now the active story.
2. **Re-pin.** Change the fork revision in:
   - `rust/Cargo.toml` (`cyclonedds`, `cyclonedds-rust-sys`)
   - `scripts/build.ps1` (`$forkRev`)
3. **Update the lock file.**
   ```powershell
   cargo update -p cyclonedds -p cyclonedds-derive -p cyclonedds-rust-sys -p cyclonedds-src
   ```
   Confirm all four packages resolve to `952906b5018831683c0863cd11001bdac427ee10`.
4. **Regenerate.** Run `.\scripts\build.ps1 -Test`; it reinstalls the codegen tool from the new rev and regenerates `build/gen/rust/hello_world.rs`. Confirm the generated file contains `impl cyclonedds::DdsFieldMetadata` for all three types and nested `Quaternion` schema references.
5. **Add the acceptance test** at `rust/lib/stations-dds/tests/field_metadata.rs`.
6. **Update living documentation**:
   - `docs/versions.md`: new fork revision and the field-metadata feature.
   - `README.md`: new revision and feature list.
   - `planning/backlog.md`: external-component revision.
   - `planning/cyclonedds-build-plan.md`: record the metadata increment.
7. **Verify once** with `.\scripts\build.ps1 -Test`; if it passes, run `.\scripts\build.ps1` once to refresh deploy artifacts. Do not take screenshots; there is no UI change.
8. **Record completion** in `current_story.md` while leaving the backlog status `in progress` until the user asks to plan the next story.

## Acceptance test

`rust/lib/stations-dds/tests/field_metadata.rs`:

- Import `DdsFieldMetadata`, `DynamicPrimitiveKind`, `DynamicTypeSchema`, and `DynamicValue` through `stations_dds::cyclonedds`.
- Build a `HelloWorldTopic` with a greeting and a non-default `Quaternion`.
- Assert `HelloWorldTopic::field_names() == ["greeting", "attitude"]`.
- Assert the schema name is `hello_world::HelloWorldTopic`.
- Assert `greeting` is `DynamicTypeSchema::String { bound: None }`.
- Assert `attitude` is the same schema as `math::Quaternion`.
- Round-trip `HelloWorldTopic` through `to_dynamic_value`/`from_dynamic_value`.
- Use `get_field`/`set_field` and assert the typed value changes.
- Assert a wrong-type field value is rejected.
- Assert a missing required field is rejected.
- Repeat schema/round-trip coverage for `HelloWorldViewTopic`, including `valid: Boolean`.

## Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
```

- `build.ps1 -Test` must pass once; the new test is included in the workspace test run.
- `build.ps1` must deploy successfully once.
- No screenshots and no repeated UI runs.
- If the new fork revision fails to compile or the generated metadata is incorrect, stop after the allowed iterations and report the failure.

## Risks

- **Revision drift:** two independent pins plus `Cargo.lock` must agree.
- **Fork compatibility:** generated metadata is emitted unconditionally; a compile failure means the fork integration needs attention, not a local copy of the fork.
- **Generated code visibility:** the generated file is temporary and not committed; verify it after regeneration.
- **Scope:** `dds-echo` remains unchanged by design; its detail pane/send form are follow-up work.

## Result

Implementation complete and verified:

- The fork is pinned to `952906b` in `rust/Cargo.toml` and `scripts/build.ps1`; `rust/Cargo.lock` resolves all fork packages at that rev.
- Code generation emits `DdsFieldMetadata` for `math::Quaternion`, `hello_world::HelloWorldTopic`, and `hello_world::HelloWorldViewTopic`.
- `stations-dds/tests/field_metadata.rs` passes five tests covering field enumeration, nested schemas, round-trips, get/set, invalid values, missing fields, and the view topic.
- `.\scripts\build.ps1 -Test` passed and `.\scripts\build.ps1` deployed successfully.
- The codegen install target is rev-keyed; without that, Cargo reused the old generator binary.
- No `dds-echo` UI changes were made.

## Bookkeeping

- `hello-service` was completed at `2026-10-27 10:08:01` and moved to `completed_stories.md`.
- `topic-field-metadata` is `in progress` in `backlog.md`.
- Do not move `topic-field-metadata` to `completed_stories.md` until the user asks to plan the next story.
