> 🌐 **English** · [Українська](../ua/ARCHITECTURE.md)


# Architecture

This directory contains the Elyra documentation set. The canonical architecture
page is [`architecture.md`](architecture.md); it describes the shipped 0.5.0
system, data flows, trust boundaries, and known limitations.

## Documentation map

| Area | Page |
| --- | --- |
| Documentation index | [`README.md`](README.md) |
| Install and first run | [`getting-started.md`](getting-started.md) |
| Interface and features | [`user-guide.md`](user-guide.md) |
| System architecture | [`architecture.md`](architecture.md) |
| Server and API | [`server.md`](server.md) |
| Security model | [`security.md`](security.md) |
| Development and release | [`development.md`](development.md) |
| Deployment overview | [`deployment.md`](deployment.md) |
| Full VPS runbook | [`deploy/SERVER.md`](deploy/SERVER.md) |
| Production verification checklist | [`deploy/VERIFY.md`](deploy/VERIFY.md) |
| Troubleshooting | [`troubleshooting.md`](troubleshooting.md) |
| Configuration reference | [`reference/configuration.md`](reference/configuration.md) |
| Tauri command reference | [`reference/tauri-commands.md`](reference/tauri-commands.md) |

## Directory scope notes

These are short markers for contributors working inside a directory, not
narrative documentation:

- [`architecture/ARCHITECTURE.md`](architecture/ARCHITECTURE.md) — architecture
  folder scope.
- [`crypto/README.md`](crypto/README.md) — crypto boundary guarantees.
- [`security/ARCHITECTURE.md`](security/ARCHITECTURE.md) — security baseline.

Architecture changes require explicit approval under the rules in
[`ai.md`](../../ai.md). Documentation changes do not, but they must describe the
implementation as it is and be recorded in [`changes.md`](../../changes.md).
