# Contributing

Start with the [build instructions](../README.md#build) and [known limitations](../README.md#status). Keep changes focused and state which checks you ran; compilation, package verification, and hardware testing establish different things.

## Repository layout

| Path | Contents |
| --- | --- |
| `src/` | Shared Rust engine and platform integration |
| `platforms/android/` | Android Gradle project and native wrapper |
| `platforms/switch/` | Inherited Nintendo Switch port; outside this fork's current delivery matrix |
| `vendor/` | Vendored dependencies and their notices |
| `res/` | Application assets, locale schemas, distribution text, and README artwork |
| `tools/` | Resource conversion, build, packaging, verification, tests, and diagnostic probes |
| `docs/` | Public project guides, contributor documentation, and release notes |
| `.github/` | Build workflow, issue templates, and disabled legacy CI configuration |

Run commands from the repository root unless a guide says otherwise. The controller diagnostic remains available as `cargo run --example controller_feedback_probe`; its source is in `tools/probes/`. Build outputs and downloaded resources belong outside the source tree, in the sibling `CaveStory-rs-runs` directory used by the build scripts.

The root keeps the project README, license, and Cargo manifest/lockfile. The Cargo build script is `tools/build.rs`, contributor attribution is in [AUTHORS.md](AUTHORS.md), and Rust formatting uses `.rustfmt.toml`. Reusable ARM64 tools live in `../CaveStory-rs-runs/cache/toolchains/`. Keep local notes and temporary investigation files outside the repository.

## GitHub Actions builds

[Build release packages](../.github/workflows/build.yml) runs on pushes to `main` that change build inputs, or through **Actions → Build release packages → Run workflow**. A hosted Windows runner builds three Windows architectures and two Android ABIs, each with base and bundled-game packages. The batch uses one date in the Asia/Taipei time zone. [setup_ci.ps1](../tools/setup_ci.ps1) prepares the hosted toolchains; local deliveries still use [build_all.ps1](../tools/build_all.ps1).

Configure repository Actions secrets `ANDROID_KEYSTORE_BASE64` (the base64-encoded JKS) and `ANDROID_KEYSTORE_PASSWORD` (its store/key password). The keystore must have one private-key entry and match the certificate required by the app and delivery verifier. Do not put signing materials in source files, command logs, caches, or artifacts. Independent forks must supply their own identity and update their signing policy and verifier together.

The Windows artifact contains six ZIPs. The complete `CaveStory-rs_delivery_<date>` artifact appears only after all ten packages pass the existing signature, architecture, version, resource, and package-content checks; it contains ten packages, `SHA256SUMS`, and `verified-delivery.json`. Artifacts are retained for 14 days. Building and uploading these artifacts does not create or overwrite a GitHub Release, and package checks do not replace hardware testing. The historical upstream workflow remains disabled.

## Public content

Commit source code, reproducible maintenance tools, license notices, and documentation intended for players or contributors. Keep local investigation reports, test-session transcripts, device inventories, implementation plans, handoff notes, credentials, saves, and downloaded game data out of the repository. Redacting identifiers does not turn an internal report into public documentation.

The `docs/` ignore rules use an explicit allowlist. When adding a public guide or release note, review its audience and content, then add that exact path to `.gitignore`. Do not use `git add -f` to bypass this boundary. Links in public documents must resolve to public files or websites, never a maintainer's local evidence archive.

Before committing, inspect `git status --short`, stage named paths, and review `git diff --cached --name-status` and `git diff --cached --check`. Git ignore rules do not remove already tracked files or old commits. Check branches, tags, release source archives, and release attachments separately when correcting a publication mistake.

Preserve upstream and third-party attribution. The engine license does not grant rights to game data, translations, or fonts; keep their notices and distribution terms distinct. Do not include personal signing keys in independently built Android packages or source submissions.
