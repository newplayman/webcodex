# Security hardening for newplayman/webcodex

Audit base: `6b37efc64b81a1e09a7a4d7b83f754f4860429b2`

This fork keeps normal WebCodex capabilities (file read/write, Git, shell, Jobs,
Python/tool execution, multiple Runners) while hardening the concrete issues
identified in the audit.

## Implemented

### 1. Server authentication fails closed

`webcodex-server` refuses to start when `WEBCODEX_TOKEN` is missing or blank.
The old unauthenticated bootstrap/admin behavior is reachable only with the
explicit local-development escape hatch:

```text
WEBCODEX_ALLOW_UNAUTHENTICATED_BOOTSTRAP=true
```

Do not set this on a networked Server.

### 2. Runner file reads re-check the canonical target

The Runner now routes `file_read` through a hardening wrapper. It canonicalizes
the actual target, verifies it remains inside the registered project, applies
the shared sensitive-path policy to the canonical project-relative path, and
then delegates the existing range/size/UTF-8 read logic using the canonical
path.

This closes the audited case where a harmless-looking filename was a symlink to
`.env`, `*.key`, `*.pem`, Runner configuration, or another protected path. A
symlink to an ordinary project file remains usable.

The wrapper substantially narrows the original alias race by opening the
canonical target path, but this is not an OS-level hostile-local-writer sandbox.
A process that can concurrently replace the canonical target itself still sits
inside the same operating-system trust boundary. Hosts that treat another local
process as hostile should enforce that separation with OS permissions/namespaces.

### 3. Deployment no longer follows upstream `latest`

The default `compose.yaml` uses:

```text
webcodex-server-local:security-hardened
pull_policy: never
```

Build the audited checkout with:

```bash
docker compose -f compose.yaml -f compose.build.yaml build
docker compose -f compose.yaml -f compose.build.yaml up -d
```

Alternatively set `WEBCODEX_SERVER_IMAGE` to an image reference whose digest
has been independently recorded and approved.

### 4. Full payload tracing requires an explicit acknowledgement

`WEBCODEX_TOOL_REQUEST_TRACE=full` can retain source text, command output, and
other semantic tool payloads. The hardened Server refuses to start in that mode
unless this additional acknowledgement is set:

```text
WEBCODEX_ALLOW_FULL_PAYLOAD_TRACE=true
```

## Deliberate credential execution remains deliberate

A system cannot simultaneously promise unrestricted shell execution and promise
that a credential intentionally made available to that shell can never be used
or printed. Do not place wallet/exchange/cloud secrets in the ambient environment
of the Runner service itself.

For projects that genuinely need a credential, inject the minimum scoped
credential through the project's explicit execution configuration and treat
that execution boundary as authorized to use it. Prefer separate read-only,
paper/micro-live/live credentials and API-side withdrawal/trading restrictions
where supported.

## Validation status

The changes are committed on `security/harden-6b37efc`. This fork currently did
not execute GitHub Actions jobs automatically, so the branch must not be merged
or deployed as production solely on the basis of a green CI claim that does not
exist. Run the repository's normal Rust/CI matrix before production deployment.
