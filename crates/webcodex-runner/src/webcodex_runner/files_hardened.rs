use super::config::RunnerPolicy;
use super::files_impl;
use super::output::CommandResult;
use crate::runner_protocol::RunnerRequest;
use std::path::{Path, PathBuf};
use std::time::Instant;
use webcodex_workspace::file_read_range::ReadFileReason;

#[cfg(test)]
pub(crate) use files_impl::sha256_hex_bytes;
pub(crate) use files_impl::{is_basic_file_request_kind, resolve_requested_path};

fn read_file_reason_message(reason: ReadFileReason) -> String {
    format!("read_file failed: {}", reason.as_str())
}

fn canonical_secret_checked_target(
    project_root: &Path,
    resolved: &Path,
) -> Result<PathBuf, ReadFileReason> {
    let project_root = project_root
        .canonicalize()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => ReadFileReason::NotFound,
            std::io::ErrorKind::PermissionDenied => ReadFileReason::PermissionDenied,
            _ => ReadFileReason::IoError,
        })?;
    let target = resolved
        .canonicalize()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => ReadFileReason::NotFound,
            std::io::ErrorKind::PermissionDenied => ReadFileReason::PermissionDenied,
            _ => ReadFileReason::IoError,
        })?;

    if !webcodex_runner_config::paths::path_is_within(&target, &project_root) {
        return Err(ReadFileReason::InvalidPath);
    }

    let relative = target
        .strip_prefix(&project_root)
        .map_err(|_| ReadFileReason::InvalidPath)?;
    let relative = relative.to_string_lossy().replace('\\', "/");
    if webcodex_core::sensitive_paths::is_secret_path(&relative) {
        return Err(ReadFileReason::SensitivePath);
    }
    if !target.is_file() {
        return Err(ReadFileReason::NotFile);
    }
    Ok(target)
}

pub(crate) fn handle_basic_file_request(
    policy: &RunnerPolicy,
    request: &RunnerRequest,
    resolved: &Path,
    start: Instant,
) -> CommandResult {
    if request.kind != "file_read" {
        return files_impl::handle_basic_file_request(policy, request, resolved, start);
    }

    let Some(project_root) = request.cwd.as_deref() else {
        return CommandResult {
            exit_code: None,
            stdout: None,
            stderr: None,
            duration_ms: Some(start.elapsed().as_millis() as u64),
            error: Some(read_file_reason_message(ReadFileReason::InvalidPath)),
        };
    };

    let canonical = match canonical_secret_checked_target(Path::new(project_root), resolved) {
        Ok(path) => path,
        Err(reason) => {
            return CommandResult {
                exit_code: None,
                stdout: None,
                stderr: None,
                duration_ms: Some(start.elapsed().as_millis() as u64),
                error: Some(read_file_reason_message(reason)),
            };
        }
    };

    // Delegate the existing, well-tested range/size/UTF-8 behavior, but pass the
    // canonical target rather than the attacker-controlled alias. The original
    // handler therefore opens the already-resolved real path after this module
    // has re-applied the secret policy to that real project-relative target.
    files_impl::handle_basic_file_request(policy, request, &canonical, start)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "webcodex-hardened-read-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn symlink_alias_to_dotenv_is_rejected() {
        let root = temp_root();
        std::fs::create_dir_all(&root).expect("mkdir");
        std::fs::write(root.join(".env"), b"FAKE_SECRET=not-real\n").expect("write secret");
        symlink(".env", root.join("settings.txt")).expect("symlink");
        let result = canonical_secret_checked_target(&root, &root.join("settings.txt"));
        assert!(matches!(result, Err(ReadFileReason::SensitivePath)));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn symlink_alias_to_normal_file_is_allowed() {
        let root = temp_root();
        std::fs::create_dir_all(&root).expect("mkdir");
        std::fs::write(root.join("normal.txt"), b"ok\n").expect("write normal");
        symlink("normal.txt", root.join("settings.txt")).expect("symlink");
        let result = canonical_secret_checked_target(&root, &root.join("settings.txt"))
            .expect("normal alias should resolve");
        assert_eq!(result, root.join("normal.txt").canonicalize().expect("canonical"));
        let _ = std::fs::remove_dir_all(root);
    }
}
