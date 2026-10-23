# UNX Project Roadmap

**Project:** UNX — an open-source, readable, English-like programming language and universal app project format  
**Repository:** https://github.com/ItsJeffTheDev/UNX  
**Updated:** 2026-10-23  
**Initial target:** Windows desktop, with cross-platform architecture planned from the beginning

This is an ambitious, dependency-ordered roadmap, not a calendar promise. Each phase is complete only when its acceptance criteria are met and automated checks pass.

## Product direction

UNX aims to let developers write an application in a readable project format and generate distributable builds for supported platforms. The first priority is a dependable language, predictable package format, strong diagnostics, and a useful Windows workflow—not immediate support for every operating system.

Principles:
- **Open source:** MIT-licensed source, contributions reviewed through pull requests.
- **Readable syntax:** approachable, consistent, and unambiguous.
- **Portable projects:** source and assets separated from platform-specific build outputs.
- **Safe by default:** a .unx extension does not make a package trustworthy.
- **Incremental delivery:** stabilize the CLI/runtime before investing in a graphical IDE.
- **Honest compatibility:** advertise only targets and features that are tested.

## Current baseline

The repository contains a Rust CLI/interpreter prototype, draft language specification, sample `.unxc` source program, ZIP-based `.unx` package implementation, an experimental VS Code extension with syntax highlighting/snippets/run/package commands, a beginner tutorial, MIT license, and GitHub Actions workflow. As checked on 2026-10-23, GitHub Actions completed successfully on Windows, macOS, and Ubuntu, including Clippy, `cargo build`, and `cargo test` (latest verified run: [#158](https://github.com/ItsJeffTheDev/UNX/actions/runs/39976371144)). A non-blocking `cargo fmt --all -- --check` audit is also present; formatting enforcement is not yet blocking. Dependabot is configured to propose weekly Cargo dependency updates and monthly GitHub Actions updates.

Implemented today: full-line and trailing comments, string variables and variable copying, `say` output, single-line exact-string `if` conditions, nested multi-line exact-string `if` / optional `else` / required `end` blocks, experimental GUI windows/buttons, and the `run`, `check`, `pack`, and `inspect` commands.

Not yet implemented: committed Cargo.lock, locked builds, full CI formatting/lint gates, mature parser/value model, arithmetic and control flow, stable public package specification, Windows GUI/installer, dependency management, a hardened security boundary, signatures, and release automation. CLI integration tests cover source validation, direct execution, pack → inspect → run, invalid source rejection, manifest traversal, malformed ZIPs, duplicate ZIP entry names, missing/invalid manifests, unsupported package versions, missing entry files, invalid UTF-8, and unsafe ZIP entry paths. Duplicate-entry rejection is tested using a handcrafted ZIP fixture because the standard test ZIP writer does not allow duplicate names. The current interpreter is not a hardened sandbox.

---

## Phase 0 — Stabilize the foundation

**Goal:** Make the prototype reproducible and contributor-friendly.

Tasks:
- [ ] Generate and commit Cargo.lock.
- [ ] After committing the lockfile, use cargo build --locked and cargo test --locked in CI.
- [x] Add cargo clippy --all-targets --all-features -- -D warnings to CI and keep it passing on Windows, macOS, and Ubuntu.
- [ ] Format the existing Rust source and tests, then make cargo fmt --all -- --check a blocking CI gate (currently an informational, non-blocking audit).
- [ ] Verify documented commands from a clean checkout.
- [x] Update project baseline documentation to reflect passing cross-platform CI.
- [x] Add contribution instructions, bug/feature issue forms, and a pull-request template.
- [x] Document the current stable-channel policy and the outstanding lockfile/CI limitations in `docs/RUST-TOOLCHAIN.md`.

**Acceptance:** Clean locked builds and tests pass; CI checks pushes and pull requests; docs match actual behavior; new contributors can build and test.

## Phase 1 — Reliable CLI and package validation

**Goal:** Make current commands predictable before expanding the language.

Tasks:
- [x] Add integration tests for run, check, pack, and inspect.
- [x] Test pack → inspect → run end to end.
- [x] Test malformed ZIPs, missing manifests, invalid JSON, missing entry files, invalid UTF-8, and unsupported manifest versions.
- [x] Validate package paths by components; reject absolute paths and parent traversal without rejecting harmless filenames just because they contain two dots.
- [x] Set limits for manifest (1 MiB), source (10 MiB), package file size (20 MiB), and package entry count (exactly two).
- [x] Reject duplicate ZIP entry names before loading the package; cover the case with a handcrafted regression fixture.
- [x] Make package output deterministic where practical by fixing ZIP timestamps and file permissions; integration tests compare repeated package bytes.
- [x] Return clear diagnostics and non-zero exit codes for tested invalid inputs.
- [x] Add and test help and version output; document success, command-failure, and usage-error exit codes.
- [x] Test that inspect reports package contents without starting the application.

**Acceptance:** Happy-path and malformed-package tests pass; invalid input produces useful errors instead of panics; inspection is non-executing; exit codes are documented.

## Phase 2 — Define the language core (UNX 0.2)

**Goal:** Replace ad-hoc parsing with a specified, testable language.

Tasks:
- [ ] Publish lexical rules and a versioned grammar.
- [ ] Define identifiers, case sensitivity, whitespace, comments, escapes, and line endings.
- [ ] Implement a tokenizer and parser with source spans (line and column).
- [ ] Build a small syntax tree instead of parsing statements ad hoc.
- [ ] Define runtime values, conversions, assignment, and variable scope.
- [ ] Add numeric literals and arithmetic.
- [ ] Add Boolean values and comparisons.
- [ ] Add if/else and bounded loops once expression semantics are stable.
- [ ] Add functions and return values after control flow works.
- [ ] Make runtime errors identify the source location.
- [ ] Add positive and negative tests for every feature.

Illustrative future syntax (not finalized):

    # A comment
    set name to "UNX"
    say "Hello"
    say name
    set count to 2 + 3
    if count > 4
        say "Count is greater than four"
    else
        say "Count is four or less"
    end

**Acceptance:** Grammar and implementation agree; parse errors identify locations; unsupported syntax is rejected clearly.

## Phase 3 — Stable .unx package specification (UNX 0.2)

**Goal:** Make packages portable, inspectable, and versionable.

Tasks:
- [x] Document the current ZIP layout and required manifest fields in `docs/UNX-PACKAGE-FORMAT.md` (prototype reference; stable compatibility policy remains open).
- [ ] Version the package format independently from the language.
- [ ] Define application ID, display name, package version, entry point, assets, permissions, and minimum runtime version.
- [ ] Define path normalization, duplicate entries, case collisions, and allowed file types.
- [ ] Specify behavior for unknown manifest fields and newer package versions.
- [ ] Decide how dependencies work; prefer no third-party runtime dependencies in the first MVP.
- [ ] Add a validator that inspects packages without running them.
- [ ] Add valid, invalid, old-version, and unsupported-version fixtures.
- [ ] Publish compatibility rules.

**Acceptance:** A third party can create a compatible package from the written spec; validation is actionable; compatibility is tested; path behavior is OS-independent.

## Phase 4 — Runtime correctness and security design

**Goal:** Establish a threat model before adding machine-affecting capabilities.

Tasks:
- [ ] Document risks: malicious packages, traversal, denial of service, untrusted dependencies, filesystem/network access, and privilege escalation.
- [ ] Define execution limits for steps, recursion, memory, and output.
- [ ] Design explicit capabilities for filesystem, network, process execution, and other sensitive operations.
- [ ] Deny capabilities by default unless explicitly granted.
- [ ] Keep parsing/inspection separate from execution.
- [ ] Add fuzz or property-based tests for parser, tokenizer, manifest, and package reader.
- [ ] Turn every discovered bug into a regression test.
- [ ] Evaluate separate-process and OS-level isolation for each platform.
- [ ] Document that application-level checks alone are not a secure sandbox.
- [ ] Do not claim safe execution of untrusted code until isolation has been independently reviewed and tested.

**Acceptance:** Threat and capability models are documented; resource exhaustion has defined behavior; sensitive features deny by default; security claims match implementation.

## Phase 5 — Developer experience and project tooling

**Goal:** Make UNX easy to create, debug, and maintain.

Tasks:
- [ ] Finalize commands: new, check, run, pack, inspect, and --version.
- [ ] Add unx new to create a project from a template.
- [ ] Document project layout for source, assets, tests, and manifest.
- [ ] Add source diagnostics with line/column and helpful suggestions where practical.
- [ ] Add a test harness for UNX programs.
- [ ] Add a formatter after syntax/style rules stabilize.
- [ ] Add VS Code syntax highlighting and snippets.
- [ ] Add examples for variables, calculations, conditions, functions, and assets.
- [ ] Keep generated artifacts out of source control and make builds reproducible.

**Acceptance:** A new user can scaffold, validate, run, and package a sample; errors are understandable; examples run in CI.

## Phase 6 — Windows desktop MVP

**Goal:** Deliver the first end-user workflow without confusing a project package with an installer.

Tasks:
- [ ] Decide the initial app model: console first, then a minimal windowed API.
- [ ] Build and test a Windows release on a clean Windows environment.
- [ ] Design a small UI abstraction for windows, text, buttons, and events before promising a full UI toolkit.
- [ ] Separate shared app logic from Windows-specific implementations.
- [ ] Define asset loading and app data directories.
- [ ] Build a graphical launcher only after runtime behavior is stable.
- [ ] Generate a Windows installer/distributable as a separate artifact; .unx itself is not a native installer.
- [ ] Test install, launch, upgrade, uninstall, and data-retention behavior.
- [ ] Document supported Windows versions.

**Acceptance:** A sample runs on a clean supported Windows system; installer lifecycle is tested; platform requirements are explicit; experimental UI APIs are labeled.

## Phase 7 — Cross-platform backend architecture

**Goal:** Add targets without destabilizing the language or package format.

Tasks:
- [ ] Define a backend interface between runtime and OS services.
- [ ] Keep language semantics and package metadata platform-neutral where practical.
- [ ] Choose the next target based on contributor capacity and CI support; Linux is a useful candidate for automated testing.
- [ ] Put platform-specific behavior behind shared interfaces.
- [ ] Add CI jobs for every officially supported target.
- [ ] Define platform-specific assets and capability declarations.
- [ ] Generate per-platform artifacts from one project where supported.
- [ ] Publish a matrix of supported, experimental, and planned targets.

**Acceptance:** Shared language tests pass on all supported targets; platform differences are documented; a target is not called supported before CI and release builds work.

## Phase 8 — Dependencies, signing, updates, and distribution

**Goal:** Distribute apps responsibly.

Tasks:
- [ ] Define dependency manifest and lockfile before enabling third-party packages.
- [ ] Specify version constraints, integrity hashes, and reproducible resolution.
- [ ] Do not execute install scripts by default.
- [ ] Design signing and publisher identity, key rotation, and revocation.
- [ ] Verify signatures and integrity before installation/update where applicable.
- [ ] Design updates with user consent, rollback, and clear version information.
- [ ] Publish artifacts with checksums and release notes.
- [ ] Establish vulnerability reporting and dependency-update policies.
- [ ] Consider a package registry only after format, dependency, and security designs mature.

**Acceptance:** Dependencies are reproducible and integrity-checked; signatures can be verified; update failures have a recovery path; security claims are reviewed.

## Phase 9 — Public beta and ecosystem

**Goal:** Validate with real developers before promising 1.0 stability.

Tasks:
- [ ] Publish install instructions and a quick-start tutorial.
- [ ] Ask external developers to build small sample apps and report issues.
- [ ] Track usability, build failures, runtime bugs, compatibility, and security reports.
- [ ] Document public APIs and deprecation policy.
- [ ] Add a code of conduct and security reporting instructions as community participation grows.
- [ ] Publish release notes and an upgrade path for beta releases.
- [ ] Prioritize reliability and feedback over adding many features.

**Acceptance:** External users can follow setup docs; limitations are documented; no known release-blocking security or data-loss bugs remain; compatibility policy exists.

## Phase 10 — UNX 1.0

**Goal:** Declare stability only when the core contracts can be maintained.

Release gates:
- [ ] Language grammar and runtime behavior are versioned and documented.
- [ ] Package format and compatibility policy are stable.
- [ ] Clean builds and tests pass on every supported platform.
- [ ] Windows installation lifecycle is tested; every other target has its own tested distribution path.
- [ ] Security review and threat model are complete for the promised capability set.
- [ ] Diagnostics, examples, onboarding, and troubleshooting docs are complete.
- [ ] Release artifacts include checksums and meet the project's reproducibility goals.
- [ ] Breaking-change, deprecation, and support policies are published.

Do not schedule 1.0 by date alone. Release when the gates are met.

---

## Recommended immediate task order

1. Commit Cargo.lock and switch CI to locked builds.
2. Refresh README and link to this roadmap.
3. Add integration tests for pack → inspect → run and malformed packages.
4. Harden path and manifest validation with regression tests.
5. Add formatting and lint checks to CI.
6. Implement tokenizer/parser with source locations before adding many keywords.
7. Finalize v0.2 language and package specifications.
8. Add numbers, expressions, comparisons, and conditionals in tested increments.
9. Add project scaffolding and better diagnostics.
10. Start Windows packaging/UI prototyping once core CLI/package behavior is dependable.

## Scope boundaries for the first usable release

Include a documented language subset, dependable CLI, project templates, package validation, a tested Windows runtime/distribution path, examples, and honest security limitations.

Do not block that release on mobile support, a full visual IDE, a public package registry, automatic native UI generation for every platform, or a universal installer. Each is a separate project with substantial engineering and security costs.

## Progress tracking

- Track individual tasks and bugs in GitHub Issues.
- Use milestones for phases and keep issues small enough to review.
- Link completed work to pull requests or commits.
- Require CI to pass before merging.
- Label features implemented, experimental, planned, or unsupported.
- Revisit this roadmap after each milestone using test results and contributor feedback.

The roadmap is intentionally ambitious. The practical strategy is to make each layer dependable before building the next.
