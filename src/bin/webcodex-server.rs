use webcodex::{server_binary_action, ServerBinaryAction};

// The default macOS Tokio worker stack is too small for the deepest MCP request
// path exercised by the local Server. Keep this explicit instead of requiring
// callers to set RUST_MIN_STACK for `webcodex share`.
#[cfg(target_os = "macos")]
const MACOS_SERVER_RUNTIME_STACK_SIZE: usize = 8 * 1024 * 1024;

fn build_server_runtime() -> std::io::Result<tokio::runtime::Runtime> {
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.enable_all();
    #[cfg(target_os = "macos")]
    builder.thread_stack_size(MACOS_SERVER_RUNTIME_STACK_SIZE);
    builder.build()
}

fn env_truthy(name: &str) -> bool {
    std::env::var(name).ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn require_server_authentication() -> Result<(), std::io::Error> {
    let token_present = std::env::var("WEBCODEX_TOKEN")
        .ok()
        .is_some_and(|value| !value.trim().is_empty());
    if token_present || env_truthy("WEBCODEX_ALLOW_UNAUTHENTICATED_BOOTSTRAP") {
        return Ok(());
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "WEBCODEX_TOKEN is required; refusing to start with unauthenticated bootstrap/admin access. For an explicitly trusted local development environment only, set WEBCODEX_ALLOW_UNAUTHENTICATED_BOOTSTRAP=true.",
    ))
}

fn require_safe_trace_configuration() -> Result<(), std::io::Error> {
    let full_trace = std::env::var("WEBCODEX_TOOL_REQUEST_TRACE")
        .ok()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("full"));
    if !full_trace || env_truthy("WEBCODEX_ALLOW_FULL_PAYLOAD_TRACE") {
        return Ok(());
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "WEBCODEX_TOOL_REQUEST_TRACE=full may persist file contents and command/tool payloads. Refusing to start unless WEBCODEX_ALLOW_FULL_PAYLOAD_TRACE=true is explicitly set.",
    ))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match server_binary_action(std::env::args().skip(1)) {
        ServerBinaryAction::Run { stop_on_stdin_eof } => {
            webcodex::prepare_server_process_environment().map_err(std::io::Error::other)?;
            require_server_authentication()?;
            require_safe_trace_configuration()?;
            build_server_runtime()?
                .block_on(webcodex::run_server_with_parent_liveness(stop_on_stdin_eof))
        }
        ServerBinaryAction::Exit {
            code,
            stdout,
            stderr,
        } => {
            if !stdout.is_empty() {
                print!("{stdout}");
            }
            if !stderr.is_empty() {
                eprint!("{stderr}");
            }
            std::process::exit(code);
        }
    }
}

#[cfg(test)]
mod security_hardening_tests {
    use super::*;

    #[test]
    fn truthy_parser_is_explicit() {
        for value in ["1", "true", "TRUE", "yes", "on"] {
            assert!(matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            ));
        }
        for value in ["", "0", "false", "off", "anything"] {
            assert!(!matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            ));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn server_runtime_uses_large_worker_stack() {
        let runtime = build_server_runtime().expect("build WebCodex Server runtime");
        let stack_size = runtime.block_on(async {
            tokio::spawn(async {
                // SAFETY: pthread_self returns the current worker thread and
                // pthread_get_stacksize_np only queries that thread's stack metadata.
                unsafe { libc::pthread_get_stacksize_np(libc::pthread_self()) }
            })
            .await
            .expect("observe WebCodex Server worker")
        });
        assert!(
            stack_size >= MACOS_SERVER_RUNTIME_STACK_SIZE,
            "WebCodex Server worker stack was {stack_size} bytes; expected at least {MACOS_SERVER_RUNTIME_STACK_SIZE}"
        );
    }
}
