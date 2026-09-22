# Contributing

Start with the [build instructions](../README.md#build) and [known limitations](../README.md#status). Keep changes focused and state which checks you ran; compilation, package verification, and hardware testing establish different things.

## Repository layout

| Path | Contents |
| --- | --- |
| `src/` | Shared Rust engine and platform integration |
| `drsandroid/` | Android Gradle project and native wrapper |
| `drshorizon/` | Inherited Nintendo Switch port; outside this fork's current delivery matrix |
| `vendor/` | Vendored dependencies and their notices |
| `res/` | Application assets, locale schemas, distribution text, and README artwork |
| `tools/` | Resource conversion, build, packaging, verification, tests, and diagnostic probes |
| `docs/` | Public project guides, contributor documentation, and release notes |
| `.github/` | Issue templates and disabled legacy CI configuration |

Run commands from the repository root unless a guide says otherwise. The controller diagnostic remains available as `cargo run --example controller_feedback_probe`; its source is in `tools/probes/`. Build outputs and downloaded resources belong outside the source tree, in the sibling `CaveStory-rs-runs` directory used by the build scripts.

## Public content

Commit source code, reproducible maintenance tools, license notices, and documentation intended for players or contributors. Keep local investigation reports, test-session transcripts, device inventories, implementation plans, handoff notes, credentials, saves, and downloaded game data out of the repository. Redacting identifiers does not turn an internal report into public documentation.

The `docs/` ignore rules use an explicit allowlist. When adding a public guide or release note, review its audience and content, then add that exact path to `.gitignore`. Do not use `git add -f` to bypass this boundary. Links in public documents must resolve to public files or websites, never a maintainer's local evidence archive.

Before committing, inspect `git status --short`, stage named paths, and review `git diff --cached --name-status` and `git diff --cached --check`. Git ignore rules do not remove already tracked files or old commits. Check branches, tags, release source archives, and release attachments separately when correcting a publication mistake.

Preserve upstream and third-party attribution. The engine license does not grant rights to game data, translations, or fonts; keep their notices and distribution terms distinct. Do not include personal signing keys in independently built Android packages or source submissions.
