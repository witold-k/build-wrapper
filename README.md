# build-wrapper

A deterministic **configure → build → lint → test** framework for Rust projects and other build systems, extracted from [aiagents](https://github.com/witold-k/aiagents).

`build-wrapper` provides both a **command-line executable** and a **Rust library**. It runs build tools, captures diagnostics and returns structured results. It does **not** depend on an LLM or attempt to repair source code.

## Features

- Detects **Cargo, CMake, Meson, Maven, Just, Make and Lean 4 (Lake)** projects.
- Runs optional setup/configuration, required build, optional lint and optional tests.
- Stops at the first failed stage; records skipped and unexecuted stages.
- Supports **automatic**, **forced** and **disabled** configuration.
- Supports **manual configure-only execution**, forwarding arguments to the underlying build system.
- Exposes the same command profiles and execution results through the CLI and Rust API.
- Captures exit codes, stdout, stderr, execution time and process errors.
- Allows custom commands through JSON, including per-command environment variables.
- Executes commands directly, without an implicit shell.

## Installation

Build from the repository:

```sh
cargo build --release
./target/release/build-wrapper --help
```

Alternatively, use the included `Justfile` and `build.py` development helpers (see [Development](#development)).

## Quick start

Run the detected build system in the current directory:

```sh
build-wrapper --project .
```

Choose a build system explicitly:

```sh
build-wrapper --project /path/to/project --system cmake
build-wrapper --project /path/to/project --system meson
build-wrapper --project /path/to/project --system cargo
build-wrapper --project /path/to/lean-project --system lean
```

Request structured output:

```sh
build-wrapper --project . --json
```

The default pipeline is:

```text
SETUP (optional) → BUILD (required) → LINT (optional) → TEST (optional)
```

A failure stops subsequent stages. Missing optional commands are reported as `skipped`; stages not reached because of a failure are `not_run`.

## Build-system support

| System | Detection file | Configure/setup | Build | Lint | Test |
| --- | --- | --- | --- | --- | --- |
| Cargo | `Cargo.toml` | — | `cargo build` | `cargo clippy` | `cargo test` |
| CMake | `CMakeLists.txt` | `cmake -S … -B … -G Ninja` | `cmake --build` | — | `ctest` |
| Meson | `meson.build` | `meson setup` | `meson compile` | — | `meson test` |
| Lean 4 (Lake) | `lakefile.lean` or `lakefile.toml` | — | `lake build` | — | — |
| Maven | `pom.xml` | — | `mvn -DskipTests package` | — | `mvn test` |
| Just | `Justfile` | — | `just build` | — | — |
| Make | `Makefile` | — | `make` | — | — |

When multiple project files exist, detection prefers **Cargo → Lake → Meson → CMake → Maven → Just → Make**. Use `--system` to override this order.

The default output directory is `build` for CMake, Meson and Make, and `target` for Cargo, Just and Maven; Lean uses `.lake/build`. Override it with `--target DIR`. Relative target paths are resolved under the project directory.

## Lean 4 projects

Lean 4 projects use [Lake](https://github.com/leanprover/lean4/tree/master/src/lake) as their build system. Both `lakefile.lean` and `lakefile.toml` are detected automatically. Select `--system lean` or `--system lake` explicitly when needed:

```sh
build-wrapper --project /path/to/lean-project
build-wrapper --project /path/to/lean-project --system lean
```

The default profile runs `lake build`. No generic test or lint command is assumed because Lake projects can define different test and lint drivers. Supply those commands through a custom JSON profile if your project defines them. The Lean toolchain and `lake` executable must be installed separately. The `--target` option does not relocate Lake's output directory.

## Configuration

### Automatic configuration

The CLI defaults to `--setup auto`:

- **CMake:** configure when `<build-dir>/CMakeCache.txt` does not exist.
- **Meson:** configure when `<build-dir>/meson-private/coredata.dat` does not exist.
- **Other systems:** no built-in setup command is defined.

```sh
build-wrapper --system cmake --setup auto
build-wrapper --system meson --setup auto
```

For an already configured build directory, the setup stage is skipped. Ordinary build-system regeneration is handled by CMake or Meson when their build commands run.

### Force or skip configuration

```sh
# Run the setup command even if the build directory is configured
build-wrapper --system cmake --setup always

# Never run the setup command
build-wrapper --system meson --setup never

# Equivalent to --setup never
build-wrapper --system cmake --no-setup
```

### Configure arguments during a normal build

Use repeated `--configure-arg` options to append arguments to the CMake or Meson setup command:

```sh
build-wrapper --system cmake --setup always \
  --configure-arg '-DCMAKE_BUILD_TYPE=Debug' \
  --configure-arg '-DBUILD_TESTING=ON'

build-wrapper --system meson --setup always \
  --configure-arg '--buildtype=debug'
```

Use `--setup always` when changing options in an existing build directory: `auto` skips setup when the configuration marker is present.

### Manual configuration only

Use `--configure -- ARG...` to execute the underlying **setup command only**, with additional arguments forwarded unchanged. Build, lint and test are **not** run.

```sh
# CMake
build-wrapper --project . --system cmake --configure -- \
  -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=ON

# Meson
build-wrapper --project . --system meson --configure -- \
  --buildtype=debug -Db_lto=true

# Configure a nondefault build directory
build-wrapper --project . --system cmake --target build-debug \
  --configure -- -DCMAKE_BUILD_TYPE=Debug
```

The `--` separates wrapper options from the underlying build-system arguments. `--configure` also works with a custom JSON setup command, but additional `--configure-arg` or passthrough arguments cannot be combined with `--commands`.

> **Note:** The wrapper appends arguments to the built-in setup command; it does not replace that command. For fully custom configure invocations, use a JSON command profile.

## CLI reference

```text
build-wrapper [--project DIR] [--system SYSTEM] [--target DIR]
              [--commands FILE.json] [--setup auto|always|never]
              [--configure-arg ARG] [--configure -- ARGS...]
              [--no-setup] [--json]
```

| Option | Description |
| --- | --- |
| `--project DIR` | Project root; defaults to `.` |
| `--system SYSTEM` | Select `cargo`, `cmake`, `meson`, `maven`, `just`, `make`, `lake` or `lean` |
| `--target DIR` | Override the build/output directory |
| `--setup MODE` | `auto` (CLI default), `always` or `never` |
| `--no-setup` | Alias for `--setup never` |
| `--configure-arg ARG` | Append one setup argument; repeat as needed |
| `--configure` | Run setup only; use `-- ARGS...` for passthrough |
| `--commands FILE.json` | Replace the detected command profile with JSON |
| `--json` | Emit machine-readable results |
| `--help`, `-h` | Display usage |

Exit codes: **0** success, **1** a build/configure stage returned failure, **2** a wrapper configuration or launch error.

## Custom command profiles

Use `--commands FILE.json` to replace the built-in profile:

```json
{
  "setup": {
    "program": "cmake",
    "args": ["-S", ".", "-B", "build"],
    "env": {}
  },
  "build": {
    "program": "cmake",
    "args": ["--build", "build"],
    "env": {}
  },
  "lint": null,
  "test": {
    "program": "ctest",
    "args": ["--test-dir", "build", "--output-on-failure"],
    "env": {}
  }
}
```

```sh
build-wrapper --project . --commands commands.json --json
build-wrapper --project . --commands commands.json --configure
```

`build` is required for a normal BLT run; `setup`, `lint` and `test` are optional. With a custom profile, automatic setup has no configuration marker and therefore executes the configured setup command when present. `--system` and `--target` do not affect custom command profiles.

Commands are executed directly as `program` plus `args`. The process inherits the caller's environment; `env` overrides individual variables. A shell is used only when explicitly requested in the command profile.

## Rust library

Add the repository as a dependency (SSH credentials may be required if it is private):

```toml
[dependencies]
build-wrapper = { git = "ssh://git@github.com/witold-k/build-wrapper.git", branch = "experiment/extract-blt-framework" }
```

Example using automatic configuration:

```rust
use build_wrapper::{BltRunner, BuildSystem, SetupMode};
use std::path::Path;

fn validate(project: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let system = BuildSystem::detect(project).ok_or("unsupported build system")?;
    let target = system.default_target_dir(project);
    let commands = system.commands(project, &target);

    let mut runner = BltRunner::new(project, commands);
    runner.setup_mode = SetupMode::Auto;
    runner.setup_marker = system.setup_marker(project, &target);

    let report = runner.run()?;
    if let Some(failure) = report.first_failure() {
        eprintln!(
            "{:?} failed (exit {:?}): {}",
            failure.stage, failure.exit_code, failure.stderr
        );
    }

    Ok(report.succeeded())
}
```

To pass configure options from the library:

```rust
let options = vec!["-DCMAKE_BUILD_TYPE=Release".to_owned()];
let commands = BuildSystem::Cmake.commands_with_config_args(
    project,
    &BuildSystem::Cmake.default_target_dir(project),
    &options,
);
```

Important API details:

- `BltRunner::new` defaults to **`SetupMode::Always`**, preserving compatibility with existing library callers. Set `setup_mode` and `setup_marker` for auto configuration.
- `run()` returns a `RunReport` with per-stage status, command, output, exit code and duration.
- `run_with_observer(...)` reports stages just before execution.
- `run_setup = false` remains available for callers that handle setup separately, including the existing `aiagents` integration.
- `process::execute` is available for direct command execution.

## Integration with aiagents

`build-wrapper` owns the deterministic BLT functionality. The LLM-driven diagnosis, repair and retry logic remains in `aiagents`.

| Previous aiagents component | build-wrapper API |
| --- | --- |
| `Buildsystem::from_dir` | `BuildSystem::detect` |
| `Buildsystem::build_cmd` | `BuildSystem::commands` |
| `BLTWorkflow::build` | `BltRunner::run` |
| `generic_work_step::run_cmd` | `process::execute` |
| `Buildresult` | `RunReport` / `StageResult` |

The existing `aiagents` adapter still uses a separate setup workflow and disables setup in its BLT runner. The CLI's automatic configuration policy does not silently change that integration.

## Development

The repository includes a `Justfile` and `build.py`, migrated and adapted from `aiagents`.

```sh
just build
just test
just lint
just build-release
just install
```

Other available tasks include `just pull` and `just rbuild`, which use the Git remote named `github` and may rebase the current branch. Review your working tree before running them. A normal `just build` does not pull or change Git history.

Run checks directly with Cargo:

```sh
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

**Validation status:** The implementation has not yet been compiled or tested in this assistant environment. Run the commands above before relying on it in production.

### End-to-end build-system tests

Five Rust integration test files under `tests/`, one per build system, are modeled on
the standalone project scenarios in `aiagents/runtests`. They use version-controlled source projects under `tests/fixtures/`, copy them into Cargo's resolved target directory for execution, and run real build tools through `BltRunner` to verify
successful builds and diagnostics from deliberately broken source files.

| Test | External tools |
| --- | --- |
| C++ / CMake | `cmake`, `ninja`, `c++`, `ctest` |
| C++ / Meson | `meson`, `ninja`, `c++` |
| Java / Maven | `mvn`, `java`, `javac` |
| Lean 4 / Lake | `lake`, `lean` |
| Rust / Cargo | `cargo`, `rustc`, `cargo clippy` |

Run all integration tests or select one build system:

```sh
cargo test --tests -- --nocapture
cargo test --test cmake_e2e -- --nocapture
cargo test --test meson_e2e -- --nocapture
cargo test --test maven_e2e -- --nocapture
cargo test --test lake_e2e -- --nocapture
cargo test --test cargo_e2e -- --nocapture
```

Shared fixture helpers are in `tests/common/mod.rs`. Source fixtures (including intentionally broken variants in `broken/` subdirectories) are committed under `tests/fixtures/{cargo,cmake,meson,maven,lake}/`; tests do not generate source code. Only working copies and build artifacts are placed under `<cargo target_directory>/build-wrapper-e2e/` for inspection. The directory is resolved through `cargo metadata --no-deps --format-version 1`, honoring workspace and configured external Cargo targets. Nothing is generated in the source checkout or the OS temporary directory. Remove the `build-wrapper-e2e/` subtree of your Cargo target directory to clean up.

When a required external tool is missing, its test prints a `SKIP` message and
returns successfully. For a strict environment where missing toolchains should
fail the test suite, set:

```sh
BUILD_WRAPPER_E2E_REQUIRED=1 cargo test --tests -- --nocapture
```

Maven may need to download standard build plugins on its first run. The
end-to-end suite is not a substitute for the smaller, dependency-free tests in
`tests/blt_test.rs`.

## Security and limitations

- Running a build may execute arbitrary code from the target project. **This tool is not a sandbox.**
- Output is captured rather than streamed.
- There is currently no command timeout or automatic retry.
- Automatic configuration checks for setup markers; it does not independently compare configuration options or file timestamps.
- The framework has no LLM dependency and does not modify project source files to fix failures.

## License

Apache-2.0. Copyright © 2026 Witold Kaminski.
