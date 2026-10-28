# listent

A Sysinternals fast command-line tool to discover and list code signing entitlements for macOS executable binaries. Supports static scanning, real-time process monitoring, and background daemon operation.

## Overview

`listent` recursively scans directories to find executable binaries and extracts their code signing entitlements. It's designed for security researchers, developers, and system administrators who need to audit or understand the permissions requested by macOS applications.

## Features

### Core Capabilities
- **Fast scanning**: Efficiently traverses directory trees with smart filtering and progress indicators
- **Entitlement extraction**: Uses macOS `codesign` to extract entitlements from binaries
- **Flexible filtering**: Filter by paths and specific entitlement keys with glob pattern support
- **Multiple output formats**: Human-readable and structured JSON output
- **Multiple paths**: Scan multiple directories in a single command
- **Graceful interrupts**: Clean cancellation with Ctrl+C

## Install
Please see installation instructions [here](INSTALL.md).

## Development
Please see development instructions [here](DEVELOPMENT.md).

## Operating Modes

### 1. Static Scan Mode (Default)
Scan files and directories for entitlements:
```bash
# Scan default locations (/usr/bin and /usr/sbin)
listent

# Scan specific paths
listent /usr/bin /usr/sbin

# Filter by entitlement patterns
listent -e "com.apple.security.*"
listent -e "*network*" -e "*debug*"

# JSON output for automation
listent /usr/bin -e "*security*" --json
```

### 2. Real-time Monitor Mode
Monitor new processes for entitlements:
```bash
# Monitor all new processes
listent monitor

# Monitor with custom polling interval
listent monitor --interval 0.5

# Monitor specific entitlements only
listent monitor -e "com.apple.security.network.*"
```

### 3. Daemon Mode
Run monitoring continuously in the foreground (useful for testing or manual daemon operation):
```bash
# Run as daemon in foreground
listent daemon run

# Daemon with custom config file
listent daemon run --config /etc/listent/custom.toml
```

Custom configuration file template (`daemon.toml`):
```toml
[daemon]
# How often to poll for new processes, in seconds (0.1 - 300.0)
polling_interval = 1.0
# Start automatically when loaded by launchd (RunAtLoad)
auto_start = true

[monitoring]
# Filesystem paths to scan for running process binaries.
# Empty list = monitor processes from all paths.
path_filters = ["/usr/bin", "/usr/sbin"]
# Entitlement patterns to match (glob syntax). Empty list = all entitlements.
# Examples: "com.apple.security.*", "*network*"
entitlement_filters = []
```

Query logs with:
```bash
# View listent logs in real-time
log stream --predicate 'subsystem == "com.microsoft.sysinternals.listent"' --level info

# View recent logs
log show --predicate 'subsystem == "com.microsoft.sysinternals.listent"' --last 1h

# Filter for errors only
log show --predicate 'subsystem == "com.microsoft.sysinternals.listent" AND messageType == error' --last 24h
```

### 4. Background Daemon Service
Run monitoring as a persistent root system service managed by launchd. First copy
the trusted binary out of your build/Homebrew directory into a root-owned location:
```bash
sudo install -d -o root -g wheel -m 0755 /Library/Listent
sudo install -o root -g wheel -m 0755 "$(command -v listent)" /Library/Listent/listent

# Install and start daemon
sudo /Library/Listent/listent daemon install

# Check daemon status
listent daemon status

# View daemon logs
listent daemon logs
listent daemon logs --since 1h
listent daemon logs --since 30m
listent daemon logs --since "2025-01-15 10:00"
listent daemon logs --format json
listent daemon logs -f                    # Follow logs in real-time

# Stop daemon process
listent daemon stop

# Uninstall service
sudo listent daemon uninstall
```

The executable and all directories in its resolved path must be root-owned and
not writable by group/other users. The installer also rejects ACL allow entries
(even read-only grants); standard deny-only ACLs are accepted. A root-owned binary
inside a user-writable directory is **not** safe. Do not recursively change
permissions on your home or Homebrew tree to satisfy these checks.

After upgrading listent, uninstall the service, replace this protected binary
with the trusted new version, and install again using the desired configuration.
Uninstall removes the service definition, not the protected binary/configuration.
Standalone `listent daemon run --config FILE` is unchanged.

## Examples

### Static Scanning
```bash
# Basic scan with progress (uses default /usr/bin and /usr/sbin)
listent

# Multi-directory scan with filtering
listent /usr/bin /usr/sbin -e "*security*"

# Find all network-related entitlements
listent -e "*network*" --json | jq '.results[].entitlements'

# Scan quietly (suppress warnings)
listent /usr/bin --quiet
```

### Process Monitoring
```bash
# Monitor all processes with 2-second intervals
listent monitor --interval 2.0

# Monitor only security-related entitlements
listent monitor -e "com.apple.security.*"

# Run as daemon with custom config
listent daemon run --config ./daemon.toml
```

### Daemon Management
```bash
# Install daemon with default monitoring (requires sudo)
sudo /Library/Listent/listent daemon install

# Import a custom configuration into the protected system configuration
sudo /Library/Listent/listent daemon install --config /path/to/config.toml

# View recent daemon activity
listent daemon logs --since 1h

# Check if daemon is running
listent daemon status

# Stop and remove daemon
listent daemon stop
sudo listent daemon uninstall
```

## Configuration

### Command Line Options
- **Paths**: Multiple paths can be specified as positional arguments: `listent /path1 /path2`
- **Entitlement filtering**: `-e "pattern"` supports exact matches and globs (`*`, `?`, `[]`)
- **Output format**: `--json` or `-j` for structured output, default is human-readable
- **Quiet mode**: `--quiet` or `-q` suppresses warnings about unreadable files
- **Monitoring**: `listent monitor` subcommand enables real-time process monitoring
- **Monitor interval**: `--interval SECONDS` sets polling frequency (0.1-300.0, default: 1.0)
- **Daemon mode**: `listent daemon run` runs as background daemon process
- **Daemon management**: `listent daemon install|uninstall|status|stop|logs`
- **Config file**: `--config FILE` or `-c FILE` specifies daemon configuration path

### Entitlement Patterns
```bash
# Exact match
-e "com.apple.security.network.client"

# Wildcard patterns
-e "com.apple.security.*"        # All Apple security entitlements
-e "*network*"                   # Any entitlement containing "network"
-e "*.debug.*"                   # Debug-related entitlements

# Multiple patterns (OR logic)
-e "com.apple.private.*" -e "*.debug.*"
```

### Daemon Configuration
Daemon settings are configured via a TOML configuration file:
- **Installed service configuration**: `/etc/listent/daemon.toml`, root-owned mode
  `0600`. The plist is root-owned mode `0644`; installed paths are canonicalized
  (for example, `/etc` is recorded as `/private/etc` on macOS).
- **Custom input**: `daemon install --config FILE` validates and imports a snapshot.
  Editing the original input afterward does not affect the root service. An install
  without `--config` writes defaults.
- **Existing installations**: Uninstall and reinstall with a protected executable.
  To retain an old home-directory configuration, pass it explicitly with `--config`.

To change an installed configuration and reinstall without resetting it:
```bash
# Edit config
sudo nano /etc/listent/daemon.toml

# Reload service with the edited configuration
sudo /Library/Listent/listent daemon uninstall
sudo /Library/Listent/listent daemon install --config /etc/listent/daemon.toml
```

The installer rejects untrusted existing configuration, working, log, or service
directories instead of silently changing their ownership. Inspect any reported
path before an administrator repairs it. Config/plist replacement is atomic and
does not follow existing destination links; existing log links are rejected.

Example daemon configuration:
```toml
[daemon]
polling_interval = 1.0
auto_start = true

[monitoring]
path_filters = []
entitlement_filters = ["com.apple.security.*", "*network*"]

[logging]
level = "info"
subsystem = "com.microsoft.sysinternals.listent"
category = "daemon"
```

## Troubleshooting

### Ctrl+C Not Working in External Terminals

If Ctrl+C doesn't interrupt the scan in Terminal.app or iTerm2, this is due to a macOS terminal signal handling issue.

**Workaround**: Before running `listent`, execute:
```bash
trap - INT
```

This removes any existing interrupt trap and restores the default SIGINT behavior. After this, Ctrl+C should work normally.

Note: This issue doesn't affect VS Code's integrated terminal.

## Output Formats

### Human-Readable (Default)
```
Found 2 binaries with 5 total entitlements:

/usr/bin/security:
  com.apple.private.platformsso.security: true

/usr/bin/nc:
  com.apple.security.network.client: true
  com.apple.security.network.server: true

Scan Summary:
  Scanned: 156 files
  Matched: 2 files
  Duration: 2.34s
```

### JSON Format
```json
{
  "results": [
    {
      "path": "/usr/bin/security",
      "entitlements": {
        "com.apple.private.platformsso.security": true
      },
      "entitlement_count": 1
    }
  ],
  "summary": {
    "scanned": 156,
    "matched": 2,
    "duration_ms": 2340,
    "skipped_unreadable": 0
  }
}
```

## Security

If you believe you have found a security issue, please report it via the project's GitHub repository rather than opening a public issue.

## License

MIT

## Support

For issues, feature requests, or contributions, please use the project's GitHub repository.
