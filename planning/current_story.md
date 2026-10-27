# Current story: hello-service

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the hello_world model service (`hello-service`): publish `HelloWorldTopic` at 10 Hz with a greeting that cycles through Latin-script languages every 15-60 seconds, and with the earth attitude moved from the view service (23.5 degree tilt, 12-minute spin). The view service subscribes to `HelloWorldTopic` and relays `HelloWorldViewTopic` at 100 Hz. Add a validity flag to the view topic; after 2 seconds without model data, mark it invalid and show `Disconnected hh:mm:ss` with the last received earth position. Before the service first connects, show the disconnected greeting after 2 seconds but no earth. Add the service to build/deploy, launch, and cleanup; update the view service contract.

- **Depends on:** none.
- **Minimal test:** `launch-rust.ps1` starts the model service, view service, and display; `dds-echo` shows `HelloWorldTopic` and `HelloWorldViewTopic`; the display renders the model-driven world; stopping `hello-service` shows the last earth and a counting `Disconnected hh:mm:ss`; restarting resumes valid data; the greeting changes within 15-60 seconds; `cleanup.ps1` stops all three.

## Context

`hello-view-service` currently owns the greeting and the complete attitude (initial 23.5-degree tilt and existing 12-minute spin) and publishes `HelloWorldViewTopic` at 100 Hz. The display library subscribes to that topic and renders the earth and greeting; it stays black until the first view sample.

The model/view split moves the model data to `hello-service`:

- `hello-service` publishes `HelloWorldTopic` (greeting + `math::Quaternion attitude`).
- `hello-view-service` subscribes to `HelloWorldTopic` and relays `HelloWorldViewTopic`.
- The display library continues to subscribe to `HelloWorldViewTopic`; it does not know about the model service.

The view payload gains a `valid` flag so consumers can distinguish live data from stale data. The model payload does not need a validity flag.

## Decisions

1. **Move the attitude.** The initial tilt and 12-minute spin move from `hello-view-service/src/view.rs` to `hello-service/src/attitude.rs`. Behavior is preserved.
2. **Rates.** Model publishes at 10 Hz (`--rate-ms 100`). The view service relays the latest model sample at 100 Hz (`--rate-ms 10`).
3. **Greeting cycle.** The model cycles sequentially through a fixed Latin-script list, starting with English. It advances after a random interval uniformly in 15-60 seconds. Use a small in-crate PRNG; introduce no new dependency.
4. **View validity.** Add `boolean valid` to `HelloWorldViewTopic`. The view service sets it true only while the latest model sample is younger than 2 seconds.
5. **Timeout baseline.** The timeout reference is the receive time of the last model sample, or the view-service start time when no sample has ever arrived.
6. **View output.**
   - Before 2 seconds and before any model sample: publish nothing (display stays black).
   - After 2 seconds with no model sample: publish `valid=false`, greeting `Disconnected hh:mm:ss`, identity attitude.
   - While fresh: publish `valid=true`, latest model greeting/attitude.
   - After a disconnect: publish `valid=false`, `Disconnected hh:mm:ss`, last model attitude.
   - The disconnected greeting counts from the last model receive (or view start if none) and updates every second.
7. **Display behavior.**
   - No view sample: black.
   - View with no valid attitude yet: background plus the disconnected greeting, no earth.
   - Valid view: earth plus greeting.
   - Invalid view after a connection: last valid earth plus the disconnected greeting.
   - The display remembers the last valid attitude locally so a later invalid/identity sample cannot erase the last received position.
8. **Greeting source.** The greeting constant moves out of the view service. The view service only relays.
9. **Contracts/build.** `hello-service` already publishes `HelloWorldTopic`; add the subscription to `hello-view-service.toml`. Add the model service to build, deploy, launch, and cleanup.

## Greeting cycle

Sequential order, wrapping:

```text
Hello, world!          English
Hola, mundo!           Spanish
Bonjour le monde!      French
Hallo, Welt!           German
Ciao, mondo!           Italian
Olá, mundo!            Portuguese
Hallo, wereld!         Dutch
Hej, världen!          Swedish
Hei, maailma!          Finnish
Witaj, świecie!        Polish
Merhaba dünya!         Turkish
Salut, lume!           Romanian
Salve, munde!          Latin
toki, ma o!            Toki Pona
Halo, wol!             Tok Pisin
Habari, dunia!         Swahili
Pẹlẹ o, ayé!           Yoruba
```

All are written in the Latin alphabet; Toki Pona is lowercase by convention. Verify that FiraSans/HarfBuzz renders the accented Yoruba text; if a glyph is missing, use a supported Latin transliteration (for example `Pele o, aye!`) and record the choice.

## Interfaces

### IDL

`DDS/hello_world.idl`:

```idl
module hello_world {
  struct HelloWorldTopic {
    string greeting;
    math::Quaternion attitude;
  };
  struct HelloWorldViewTopic {
    string greeting;
    math::Quaternion attitude;
    boolean valid;
  };
};
```

`HelloWorldTopic` and its contract entry are unchanged. The type name of the view topic is unchanged, so the registry needs no new type.

### Contracts

`contracts/services/hello-view-service.toml`:

```toml
[service]
name = "hello-view-service"

[[publish]]
topic = "HelloWorldViewTopic"

[[subscribe]]
topic = "HelloWorldTopic"
```

`contracts/services/hello-service.toml` remains publish-only.

### `rust/service/hello-service`

New crate `rust/service/hello-service`:

- `main.rs`
  - Args: `--domain` (0), `--rate-ms` (100), `--config`, `--topics`.
  - Load/validate the registry and `services/hello-service.toml`.
  - `Node::new`, `writer::<HelloWorldTopic>("HelloWorldTopic")`.
  - Loop: advance the greeting cycle using `Instant`, build `HelloWorldTopic { greeting, attitude: attitude::attitude_at(start.elapsed()) }`, write it, sleep for the period.
- `greeting.rs`
  - `GREETINGS` ordered list.
  - `GreetingCycle` with current index, next change time, and a small SplitMix64/xorshift PRNG.
  - Random interval uniformly in 15-60 seconds. Advance sequentially and wrap.
  - Unit tests for interval bounds, advancement, wrap, and deterministic test seeding.
- `attitude.rs`
  - Move `hello-view-service/src/view.rs` here; update module docs; keep the existing tests.
- Add the crate to the workspace and its dependencies (`stations-dds`, `clap`).

### `hello-view-service`

- Remove `mod view`, `src/view.rs`, the greeting constant, and attitude generation.
- Keep `--rate-ms` as the relay period.
- Create a `HelloWorldTopic` reader and a `HelloWorldViewTopic` writer.
- Add `relay.rs` with a testable state machine:

```rust
struct ViewRelay {
    started: Instant,
    latest: Option<HelloWorldTopic>,
    last_received: Option<Instant>,
}

impl ViewRelay {
    fn update(&mut self, sample: HelloWorldTopic, now: Instant);
    fn view(&self, now: Instant) -> Option<HelloWorldViewTopic>;
}
```

- `view()`:
  - If no sample has arrived and `now - started < 2s`: `None`.
  - If no sample and `now - started >= 2s`: invalid view with `Disconnected hh:mm:ss` and identity attitude.
  - If a sample arrived less than 2s ago: valid view with the model greeting/attitude.
  - If a sample arrived 2s or more ago: invalid view with the last attitude and `Disconnected hh:mm:ss`.
- Format the elapsed time as `Disconnected {:02}:{:02}:{:02}` (hours, minutes, seconds).
- Main loop drains `reader.take()`, keeps the last sample, publishes `relay.view(now)` if present, and sleeps for the relay period.
- Unit tests for no-data startup, first valid sample, timeout at 2 seconds, elapsed-time formatting, last-attitude retention, and recovery.

### Display library

- `display-lib/src/view.rs`:
  - `View { greeting: String, attitude: Option<[f32; 4]>, valid: bool }`.
  - `ViewSource` keeps a `Cell<Option<[f32; 4]>>` of the last valid attitude.
  - On a valid sample, update the last valid attitude.
  - Every published view carries `valid` from the sample and attitude `last_valid` (`None` until the first valid sample).
- `display-lib/src/formats/hello_world.rs`:
  - No view: clear black.
  - View: clear background, render the earth only when `attitude` is `Some`, and always draw the greeting.
  - This yields no earth before the first connection, a frozen last earth after a disconnect, and live earth while valid.
- Keep the `valid` flag available for future format styling.

### `dds-echo`

- Update the `HelloWorldViewTopic` decode test for the new `valid` field.
- No registry change; decode already reads the generated struct.

## Build and launch

- `scripts/build.ps1`:
  - Deploy `build\rust\release\hello-service.exe` to `build\deploy\bin`.
  - Add `rust\service\hello-service\src` to the DDS bypass-scan roots.
- `scripts/launch-rust.ps1`:
  - Require `hello-service.exe`.
  - Start `hello-service` first, then `hello-view-service`, then `lil-d`.
  - Update the description and final message.
- `scripts/cleanup.ps1`:
  - Add `hello-service` to the process list.
- `build/topics.md` will list `hello-service` as publisher of `HelloWorldTopic` and `hello-view-service` as subscriber.

## Tests

- Greeting cycle: intervals in [15, 60] seconds, sequential advancement, wrapping, deterministic seeded behavior.
- Attitude tests moved from the view service.
- View relay: no data before 2s, disconnected at 2s, elapsed time increments, last attitude retained, recovery on a new sample.
- Display mapping: no earth before the first valid sample; last valid attitude retained across invalid samples.
- `dds-echo` view decode includes `valid`.
- Existing workspace tests continue to pass.

## Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\scripts\cleanup.ps1
.\scripts\launch-rust.ps1
```

1. `dds-echo` shows `HelloWorldTopic` from `hello-service` and `HelloWorldViewTopic` from the relay.
2. The display renders the model-driven tilted earth and greeting.
3. Stop `hello-service`: after 2 seconds the display keeps the last earth and shows `Disconnected 00:00:02`, then counts upward.
4. Restart `hello-service`: the display resumes valid data.
5. Start only the view service/display without `hello-service`: after 2 seconds the disconnected greeting appears on the background, with no earth.
6. Observe at least one greeting change and confirm the new Latin-script text renders correctly.
7. `cleanup.ps1` stops `hello-service`, `hello-view-service`, and `lil-d`.

## Assumptions / open checks

- The interpretation of "don't show it unless the service connects" is: **no earth before the first connection**, while the disconnected greeting still appears after the 2-second timeout.
- The Yoruba wording is `Pẹlẹ o, ayé!`; verify font coverage and fall back to a plain-Latin transliteration only if necessary.
- Model greeting changes are sequential with random intervals; no language is skipped.

## Bookkeeping

- `dds-echo-discovery-attribution` has been moved to `completed_stories.md` with its completion timestamp.
- This story is `in progress`; do not move it to `completed_stories.md` until the user asks to plan the next story.
