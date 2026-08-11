# Plan 029b — Docker capability status

Phase 3 remains open. Docker stays disabled in the relay execution policy.

The current backend executes commands inside bubblewrap, but it does not provide
an isolated Docker daemon, restricted Docker broker, or equivalent VM boundary.
Exposing the host Docker socket would therefore turn the coding capability into
host-control access, and removing `docker` from the execution prohibition would
violate the plan's critical invariant.

Accepted release decision: do not wire Docker endpoint configuration, Docker
credentials, or runtime sockets into the MCP sandbox. Keep the explicit Docker
prohibition until an operator-owned isolated remote worker, restricted broker,
or equivalent isolated backend is designed, implemented, and acceptance-tested.

Required follow-up acceptance remains: isolated image build/run, logs/inspect,
workspace-only filesystem mapping, and rejection of privileged, host namespace,
device, host-root bind-mount, and runtime-socket escape paths.
