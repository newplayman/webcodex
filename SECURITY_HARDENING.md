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

The source `compose.yaml` uses:

```text
webcodex-server-local:security-hardened
pull_policy: never
```

Build the audited checkout with:

```bash
docker compose -f compose.yaml -f compose.build.yaml build
docker compose -f compose.yaml -f compose.build.yaml up -d
```

The release-asset path remains usable, but it replaces the local source marker
with an immutable `ghcr.io/...@sha256:...` image reference before enabling a
pull. A mutable tag is not used as the release identity.

### 4. Raw Shell / Process execution does not inherit ambient credentials

Plain `run_shell`, structured process execution, Jobs, and the initial prepared
profile environment no longer inherit the Runner service process's complete
environment. Parent inheritance is limited to a small operating-system/session
baseline such as `PATH`, `HOME`, locale, terminal, temporary-directory and the
Windows variables needed to start normal native processes.

A project that deliberately needs an additional parent variable may opt in that
exact variable name with:

```text
WEBCODEX_SHELL_INHERIT_ENV_ALLOWLIST=CUDA_VISIBLE_DEVICES,LD_LIBRARY_PATH
```

Prefer the existing project/Runner `shell.env` or shell-profile `env`
configuration for credentials. Explicitly configured exchange/RPC/wallet/API
variables remain available to the project, while `WEBCODEX_TOKEN`,
`WEBCODEX_AGENT_TOKEN`, `WEBCODEX_USER_TOKEN`, and `AUTHORIZATION` are still
blocked from child execution.

An init script may create ordinary environment variables (for example
`VIRTUAL_ENV`) after the narrow parent snapshot is established. This preserves
normal virtualenv/toolchain activation without restoring ambient credential
inheritance.

### 5. Full payload tracing requires an explicit acknowledgement

`WEBCODEX_TOOL_REQUEST_TRACE=full` can retain source text, command output, and
other semantic tool payloads. The hardened Server refuses to start in that mode
unless this additional acknowledgement is set:

```text
WEBCODEX_ALLOW_FULL_PAYLOAD_TRACE=true
```

For credential-bearing production operation, leave full tracing disabled.

## Deliberate credential execution remains deliberate

A system cannot simultaneously promise unrestricted shell execution and promise
that a credential intentionally made available to that shell can never be used
or printed. The hardening above prevents *accidental ambient inheritance*; it
does not pretend that deliberately injected credentials are invisible to code
that is authorized to execute with them.

For projects that genuinely need a credential, inject the minimum scoped
credential through the project's explicit execution configuration and treat
that execution boundary as authorized to use it. Prefer separate read-only,
paper/micro-live/live credentials and API-side withdrawal/trading restrictions
where supported.

## Validation

The security branch is `security/harden-6b37efc` and PR #1 is based directly on
the audited commit above. Production deployment should pin the exact final
commit that passed the repository CI matrix; do not deploy a moving branch name
or an upstream mutable image tag.
