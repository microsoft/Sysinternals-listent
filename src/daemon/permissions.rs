//! Filesystem trust checks for the root LaunchDaemon, not standalone daemon runs.

use anyhow::{bail, Context, Result};
use std::fs::{self, DirBuilder, Metadata, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

fn check_owner_and_mode(uid: u32, mode: u32, path: &Path) -> Result<()> {
    if uid != 0 || mode & 0o022 != 0 {
        bail!(
            "Permission denied: daemon resource must be root-owned and not group/other-writable: {}",
            path.display()
        );
    }
    Ok(())
}

// ACLs can grant writes independently of POSIX mode bits. Accept deny-only ACLs
// (including the standard /Library ACL); reject allow entries conservatively.
#[cfg(target_os = "macos")]
fn check_acl(path: &Path) -> Result<()> {
    let output = std::process::Command::new("/bin/ls")
        .env("LC_ALL", "C")
        // Numeric output also renders ACL identities as UUIDs, not names with spaces.
        .args(["-lden", "--"])
        .arg(path)
        .output()
        .context("Failed to inspect daemon resource ACL")?;
    if !output.status.success() {
        bail!(
            "Failed to inspect ACL for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let listing = std::str::from_utf8(&output.stdout)
        .context("Invalid UTF-8 in daemon resource ACL listing")?;
    check_acl_listing(listing, path)
}

#[cfg(target_os = "macos")]
fn check_acl_listing(listing: &str, path: &Path) -> Result<()> {
    let mut lines = listing.lines();
    let header = lines
        .next()
        .context("Missing daemon resource ACL listing")?;
    let mode = header
        .split_whitespace()
        .next()
        .context("Missing file mode")?;
    let mut entries = 0;
    for entry in lines {
        let fields: Vec<_> = entry.split_whitespace().collect();
        let deny_entry = matches!(
            fields.as_slice(),
            [_, _, "deny", _] | [_, _, "inherited", "deny", _]
        );
        if !deny_entry
            || fields[0]
                .strip_suffix(':')
                .and_then(|n| n.parse::<usize>().ok())
                != Some(entries)
        {
            bail!(
                "Permission denied: daemon resource has an unsupported or allow ACL: {}",
                path.display()
            );
        }
        entries += 1;
    }
    if mode.contains('+') && entries == 0 {
        bail!(
            "Incomplete ACL listing for daemon resource: {}",
            path.display()
        );
    }
    Ok(())
}

fn check_metadata(metadata: &Metadata, path: &Path) -> Result<()> {
    check_owner_and_mode(metadata.uid(), metadata.mode(), path)?;
    #[cfg(target_os = "macos")]
    check_acl(path)?;
    Ok(())
}

fn check_path_text(path: &Path) -> Result<()> {
    let text = path
        .to_str()
        .context("Daemon resource path contains invalid UTF-8")?;
    if text.chars().any(char::is_control) {
        bail!("Daemon resource path contains control characters");
    }
    Ok(())
}

/// Resolve aliases, then verify the target and every ancestor. Callers must use
/// the returned path so a replaceable alias cannot redirect the installed job.
pub fn protected_path(path: &Path, directory: bool) -> Result<PathBuf> {
    let resolved = fs::canonicalize(path)
        .with_context(|| format!("Failed to resolve daemon resource: {}", path.display()))?;
    check_path_text(&resolved)?;
    for (index, component) in resolved.ancestors().enumerate() {
        let metadata = fs::symlink_metadata(component).with_context(|| {
            format!("Failed to inspect daemon resource: {}", component.display())
        })?;
        let expect_directory = index != 0 || directory;
        if (expect_directory && !metadata.is_dir()) || (!expect_directory && !metadata.is_file()) {
            bail!("Unexpected daemon resource type: {}", component.display());
        }
        check_metadata(&metadata, component)?;
    }
    Ok(resolved)
}

/// Create missing directories only beneath already protected parents.
pub fn protected_directory(path: &Path) -> Result<PathBuf> {
    match fs::symlink_metadata(path) {
        Ok(_) => protected_path(path, true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent =
                protected_directory(path.parent().context("Daemon directory has no parent")?)?;
            let target = parent.join(path.file_name().context("Daemon directory has no name")?);
            DirBuilder::new()
                .mode(0o755)
                .create(&target)
                .with_context(|| {
                    format!("Failed to create daemon directory: {}", target.display())
                })?;
            protected_path(&target, true)
        }
        Err(error) => Err(error)
            .with_context(|| format!("Failed to inspect daemon directory: {}", path.display())),
    }
}

fn write_atomic(path: &Path, content: &[u8], mode: u32) -> Result<()> {
    let mut temporary =
        tempfile::NamedTempFile::new_in(path.parent().context("Daemon file has no parent")?)
            .context("Failed to create temporary daemon file")?;
    temporary
        .write_all(content)
        .context("Failed to write daemon file")?;
    temporary
        .as_file()
        .set_permissions(fs::Permissions::from_mode(mode))
        .context("Failed to set daemon file permissions")?;
    temporary
        .as_file()
        .sync_all()
        .context("Failed to sync daemon file")?;
    // Rename replaces a pre-existing symlink/hard link instead of following it.
    temporary
        .persist(path)
        .context("Failed to replace daemon file")?;
    Ok(())
}

/// Atomically replace a config/plist without trusting an existing destination.
pub fn write_protected_file(path: &Path, content: &[u8], mode: u32) -> Result<PathBuf> {
    let parent = protected_directory(path.parent().context("Daemon file has no parent")?)?;
    let target = parent.join(path.file_name().context("Daemon file has no name")?);
    write_atomic(&target, content, mode)?;
    protected_path(&target, false)
}

/// Preserve existing logs without letting launchd follow an untrusted log path.
pub fn protected_log(path: &Path) -> Result<PathBuf> {
    let parent = protected_directory(path.parent().context("Daemon log has no parent")?)?;
    let target = parent.join(path.file_name().context("Daemon log has no name")?);
    match fs::symlink_metadata(&target) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.nlink() != 1 {
                bail!(
                    "Daemon log must be a regular file with one link: {}",
                    target.display()
                );
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&target)
                .context("Failed to create daemon log")?;
        }
        Err(error) => return Err(error).context("Failed to inspect daemon log"),
    }
    protected_path(&target, false)
}
