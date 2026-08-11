# Remote relay proxy trust is loopback-scoped and opt-in

Remote relay mode defaults to a loopback listener and does not trust forwarded HTTPS headers; a local TLS edge or secure tunnel must be explicitly enabled with `--trusted-proxy`. Keeping this trust behind a loopback bind is intentional: a global proxy boolean on a public listener would let direct peers spoof `X-Forwarded-Proto` and bypass the transport-security gate.
