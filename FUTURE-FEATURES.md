# Future Enterprise Features

Features below are intentionally deferred. Each was considered during architecture
design and cut for scope — not forgotten. The "Why deferred" notes explain the
reasoning so future sessions don't re-derive the same conclusions.

## Performance and Storage

### Background Compactor Process
- Long-lived helper (`fsp-compactor`) re-encodes hot PNG spool files to a denser
  archival format during idle windows.
- Durable job queue, atomic write-then-rename, background CPU/IO priority.
- Pause during active capture bursts.

**Why deferred:** Designed in detail (March 2026 session) then cut. A 4K PNG is
~10-20 MB; you'd need thousands of uncleaned captures before disk pressure matters.
The 24-hour auto-expiry handles retention for now. Design the compactor when there
are real users generating real storage pressure — their usage patterns should
inform format choice, queue depth, and throttling policy, not speculation.

### Tiered Storage (Hot Spool → Archive)
- RAM ring buffer → fast-format disk spool → dense archival format.
- QOI was evaluated as a hot spool candidate (fast encode, ~20% larger than PNG).

**Why deferred:** With async persistence off the hot path, PNG encode time doesn't
matter — it's a background operation. QOI/raw BGRA would only help if the
background thread can't keep up with sustained 5+ captures/second, which hasn't
been measured yet. Don't optimize what isn't the bottleneck.

### Disk Usage Indicator (Tray/Menu)
- Show capture storage: total bytes, capture count, warning state (normal/warn/critical).
- Configurable warning thresholds, optional toast on crossing.
- "Delete all captures" action.

**Why deferred:** Good feature, but requires the capture pipeline to exist first.
Tracked as Phase 4 in `IMPLEMENTATION_PLAN.md`.

### Crash-Safe Persistence
- Atomic write-then-rename for all persisted captures.
- Recovery scan on startup for partial writes.

## Security & Compliance
- Digital signature validation on startup
- Process integrity checks
- No external dependencies or DLLs
- Logging capability (optional, configurable)
- Screenshot metadata stripping
- Configurable output paths (network shares)

## Deployment & Management
- Registry-based configuration (IT can pre-configure)
- Command-line switches for silent deployment
- Group Policy template (.admx file)
- Unattended installation mode
- Exit codes for deployment scripts

## Monitoring & Control
- Optional audit trail (who, when, what was captured)
- Bandwidth usage: zero (no network calls)
- Process isolation (runs in user context only)
- Memory usage reporting in tray tooltip
- Capture pipeline telemetry (queue depth, pool occupancy, p95 overlay latency)
- Explicit backpressure UX when RAM/disk limits are reached
- Configurable hotkey (in case of conflicts)

## Validation Features
- Built-in hash verification
- Reproducible build documentation
- Dependency bill of materials (SBOM)
- Static analysis reports
- Code signing certificate info display

## Implementation Notes
- Keep in mind during MVP development
- Add as low-hanging fruit when encountered
- Focus on MVP first, enhance later
