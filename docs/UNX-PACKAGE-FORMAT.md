# UNX Package Format v1 (Prototype)

**Status:** Implementation reference for the current prototype, not a finalized long-term compatibility promise.  
**Package extension:** `.unx`  
**Container:** ZIP archive  
**Current package format version:** `1`

A `.unx` file is a ZIP archive containing a JSON manifest and exactly one UTF-8 UNX source file. It is not a native executable, installer, or security boundary. Do not execute packages from untrusted sources.

## Required archive layout

A valid package contains exactly these two file entries:

- `manifest.json`
- The source file named by the manifest's `entry` field (for example, `main.unx`)

The entry may be in a safe relative subdirectory, such as `src/main.unx`, but directory entries are not allowed. The entry name must not be `manifest.json`. ZIP entry names must be unique.

## Manifest

The current Rust implementation deserializes the following JSON fields:

```json
{
  "format": "UNX",
  "format_version": 1,
  "id": "org.example.hello",
  "name": "Hello",
  "version": "0.1.0",
  "entry": "main.unx",
  "permissions": []
}
```

| Field | Current requirement |
| --- | --- |
| `format` | Must be the exact string `UNX`. |
| `format_version` | Must be the integer `1`. |
| `id` | Required string; must not be empty or whitespace-only. |
| `name` | Required string; must not be empty or whitespace-only. |
| `version` | Required string; must not be empty or whitespace-only. |
| `entry` | Safe relative ZIP path to the one source file; cannot be `manifest.json`. |
| `permissions` | Required JSON array of strings by virtue of the current Rust struct. No permission enforcement is implied. |

The current implementation rejects unsupported format/version values and unsafe entry paths. It does not yet enforce a formal identifier syntax or semantic-version grammar for `id` and `version`. Unknown JSON fields follow Serde's current default behavior and are not explicitly rejected.

## Path rules

Entry names must be relative and use forward slashes. The current validator rejects:

- Empty paths or path components
- Absolute paths beginning with `/`
- Components equal to `.` or `..`
- Backslashes, colons, and control characters
- The reserved manifest name as the entry source

These rules are intended to prevent traversal and platform-dependent path interpretation. They are validation rules, not proof that arbitrary packages are safe.

## Current validation and size limits

| Item | Current limit or rule |
| --- | --- |
| Whole package file | 20 MiB maximum |
| Manifest data | 1 MiB maximum |
| Entry source data | 10 MiB maximum |
| ZIP entries | Exactly 2 |
| Directories in ZIP | Rejected |
| Duplicate ZIP entry names | Rejected |
| Symbolic-link ZIP entries | Rejected when identified by Unix mode metadata |
| Manifest/source encoding | UTF-8 |
| Source validity | Checked by the current UNX parser/validator when packing; package execution validates before running |

The size limits on manifest and source are checked against the ZIP entry's reported uncompressed size. The package file limit applies to the archive file on disk.

## Package creation and reproducibility

Use the CLI from the repository root:

```powershell
cargo run -- pack .\\examples\\hello .\\Hello.unx
cargo run -- inspect .\\Hello.unx
cargo run -- run .\\Hello.unx
```

The pack command writes the manifest and source text into a ZIP archive using Deflate compression. It sets fixed ZIP modification timestamps and file permissions so repeated packaging of the same project with the same build produces byte-identical output; this behavior has an integration regression test. Byte-for-byte identity across different ZIP/compression-library versions is not yet a compatibility guarantee.

## Compatibility and security limitations

- Format version 1 describes the implementation that exists today; the format is still a prototype and may change before a stable release.
- Package format versioning is separate from the language version, but a formal compatibility policy is not yet established.
- `permissions` is metadata only; the runtime does not currently provide an enforceable capability sandbox.
- Inspecting a package parses its manifest and displays source, but is not a security scanner.
- The runtime is not hardened against all denial-of-service or malicious-input attacks.
- The duplicate-entry rejection path is implemented; a reliable duplicate-entry regression fixture is still outstanding.

## Open design work

Before declaring the format stable, specify application ID syntax, version grammar, unknown-field policy, case-collision behavior across filesystems, future assets and dependencies, minimum runtime version, compatibility guarantees, and a robust cross-platform regression fixture for duplicate ZIP entries.
