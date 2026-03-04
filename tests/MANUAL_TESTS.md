# Manual Test Plan — listent

**Purpose:** Validates end-to-end behavior that automated tests cannot fully cover (interactive output, system integration, real-world data).
**Prerequisites:** Build the release binary with `cargo build --release`. All commands below assume you are in the project root directory.

---

A great tool to validate entitlements in Monitor mode is:

```
/usr/bin/ssh localhost
```

## 1. Help & Version

| # | Test | Command | Expected Result |
|---|------|---------|-----------------|
| 1.1 | Show help | `./target/release/listent --help` | Displays usage, options, examples, and all subcommands (monitor, daemon) |
| 1.2 | Show version | `./target/release/listent --version` | Prints `listent <version>` on a single line |
| 1.3 | Monitor help | `./target/release/listent monitor --help` | Displays monitor-specific options (--interval, -e, --json, --quiet) |
| 1.4 | Daemon help | `./target/release/listent daemon --help` | Displays daemon subcommands (run, install, uninstall, status, stop, logs) |
| 1.5 | Invalid flag | `./target/release/listent --bogus` | Exits with error and shows usage hint |

---

## 2. Static Scan Mode

### 2.1 Default Scan

| # | Test | Command | Expected Result |
|---|------|---------|-----------------|
| 2.1.1 | Scan defaults | `./target/release/listent` | Scans /usr/bin and /usr/sbin. Progress shows `Processed X/Y files (scanned: N, skipped: M)`. Prints binaries with entitlements and a scan summary. |
| 2.1.2 | Progress indicator | (observe stderr during 2.1.1) | Real-time progress updates with directory name in brackets, e.g., `[sbin]`. Final line shows `✓ Processed X/Y files ... - completed` |
| 2.1.3 | Summary stats | (observe stdout after 2.1.1) | `Scan Summary:` block with Scanned, Matched, and Duration fields. Duration is reasonable (< 30s for default paths). |

### 2.2 Custom Paths

| # | Test | Command | Expected Result |
|---|------|---------|-----------------|
| 2.2.1 | Scan /usr/bin | `./target/release/listent /usr/bin` | Finds binaries with entitlements. |
| 2.2.2 | Scan single file | `./target/release/listent /usr/bin/true` | Progress shows `Processed 1/1 files (scanned: 1, skipped: 0)`. Likely reports no entitlements. |
| 2.2.3 | Multiple files | `./target/release/listent /usr/bin/true /usr/bin/false /usr/bin/env` | Progress shows `Processed 3/3 files (scanned: 3, skipped: 0)` |
| 2.2.4 | Shell glob expansion | `./target/release/listent /usr/bin/tr*` | Shell expands to matching files (true, tr, traceroute, etc.). Each is checked individually. |
| 2.2.5 | Multiple directories | `./target/release/listent /usr/bin /usr/sbin` | Both directories scanned. Summary shows combined totals. |
| 2.2.6 | Nonexistent path | `./target/release/listent /nonexistent/path` | Exits with error: `Path does not exist: /nonexistent/path` |
| 2.2.7 | Empty directory | `mkdir /tmp/empty_test && ./target/release/listent /tmp/empty_test` | Completes successfully. Reports `No binaries found with entitlements.` |

### 2.3 Entitlement Filtering

| # | Test | Command | Expected Result |
|---|------|---------|-----------------|
| 2.3.1 | Exact match | `./target/release/listent /usr/bin -e "com.apple.security.network.client"` | Only shows binaries with that exact entitlement |
| 2.3.2 | Glob wildcard | `./target/release/listent /usr/bin -e "*network*"` | Shows binaries with any entitlement containing "network" |
| 2.3.3 | Prefix glob | `./target/release/listent /usr/bin -e "com.apple.security.*"` | Matches all entitlements under com.apple.security |
| 2.3.4 | Multiple filters | `./target/release/listent /usr/bin -e "*network*" -e "*debug*"` | Shows binaries matching either pattern (OR logic) |
| 2.3.5 | Comma-separated | `./target/release/listent /usr/bin -e "*network*,*debug*"` | Same as 2.3.4 — comma-delimited filters are split |
| 2.3.6 | No matches | `./target/release/listent /usr/bin -e "nonexistent.entitlement.xyz"` | Completes successfully. Reports `No binaries found with entitlements.` |

### 2.4 Output Formats

| # | Test | Command | Expected Result |
|---|------|---------|-----------------|
| 2.4.1 | Human-readable (default) | `./target/release/listent /usr/bin` | Formatted text with binary paths and entitlements listed clearly |
| 2.4.2 | JSON output | `./target/release/listent /usr/bin --json` | Valid JSON object with `results` array. Each entry has `path`, `entitlements`, `entitlement_count`. Pipe to `jq .` to validate. |
| 2.4.3 | JSON + filter | `./target/release/listent /usr/bin -e "*network*" --json \| jq .` | Valid JSON. Only entries matching the filter are included. |
| 2.4.4 | Quiet mode | `./target/release/listent /usr/bin --quiet` | No warning messages on stderr about unreadable files |
| 2.4.5 | JSON + quiet | `./target/release/listent /usr/bin --json --quiet` | Clean JSON on stdout, no progress or warnings on stderr |

### 2.5 Interrupt Handling

| # | Test | Command | Expected Result |
|---|------|---------|-----------------|
| 2.5.1 | Ctrl+C during scan | `./target/release/listent /usr/bin` then press Ctrl+C | Stops cleanly. Partial results may be shown. No error messages. Exit code 0. |
| 2.5.2 | Ctrl+C during counting | Start scan on large directory, press Ctrl+C immediately during "Counting files..." phase | Stops cleanly without error |

---

## 3. Monitor Mode

### 3.1 Basic Monitoring

| # | Test | Command | Expected Result |
|---|------|---------|-----------------|
| 3.1.1 | Start monitor | `./target/release/listent monitor` | Begins polling. Shows header indicating monitoring is active. Detects new processes. |
| 3.1.2 | Detect new process | While monitor runs, open a new app (e.g., Calculator) | Monitor detects and displays the new process with its entitlements |
| 3.1.3 | Stop with Ctrl+C | Press Ctrl+C during monitoring | Exits cleanly with exit code 0. No error messages. |

### 3.2 Monitor Configuration

| # | Test | Command | Expected Result |
|---|------|---------|-----------------|
| 3.2.1 | Custom interval | `./target/release/listent monitor --interval 0.5` | Polls every 0.5 seconds (faster detection) |
| 3.2.2 | Slow interval | `./target/release/listent monitor --interval 10.0` | Polls every 10 seconds |
| 3.2.3 | Invalid interval (too low) | `./target/release/listent monitor --interval 0.01` | Error: interval must be between 0.1 and 300.0 |
| 3.2.4 | Invalid interval (too high) | `./target/release/listent monitor --interval 999` | Error: interval must be between 0.1 and 300.0 |
| 3.2.5 | Entitlement filter | `./target/release/listent monitor -e "com.apple.security.*"` | Only reports processes with matching entitlements |
| 3.2.6 | JSON output | `./target/release/listent monitor --json` | Each detection event is a valid JSON line |
| 3.2.7 | Quiet mode | `./target/release/listent monitor --quiet` | Suppresses non-essential output |

---

## 4. Daemon Mode

### 4.1 Foreground Daemon

| # | Test | Command | Expected Result |
|---|------|---------|-----------------|
| 4.1.1 | Run foreground | `./target/release/listent daemon run` | Starts daemon. Prints startup message. Runs until Ctrl+C. |
| 4.1.2 | Run with config | Create a config.toml (see below), then: `./target/release/listent daemon run --config config.toml` | Starts with custom config settings |
| 4.1.3 | Stop with Ctrl+C | Press Ctrl+C during daemon run | Exits cleanly |
| 4.1.4 | Duplicate daemon | In one terminal run `listent daemon run`, in another run `listent daemon run` | Second instance errors: `Daemon already running, please stop it first.` |

**Sample config.toml for test 4.1.2:**
```toml
[daemon]
polling_interval = 2.0
auto_start = false

[monitoring]
path_filters = ["/usr/bin"]
entitlement_filters = []

[logging]
level = "info"
```

### 4.2 Daemon Management

| # | Test | Command | Expected Result |
|---|------|---------|-----------------|
| 4.2.1 | Status (not running) | `./target/release/listent daemon status` | Shows daemon is not running |
| 4.2.2 | Start then status | Start daemon in one terminal, then: `./target/release/listent daemon status` | Shows daemon is running with PID info |
| 4.2.3 | Stop daemon | `./target/release/listent daemon stop` | Daemon process terminates. Prints success message. |
| 4.2.4 | Stop (none running) | `./target/release/listent daemon stop` | Reports no daemon processes found |
| 4.2.5 | View logs | `./target/release/listent daemon logs` | Displays recent daemon log entries from macOS Unified Logging |
| 4.2.6 | Logs with time filter | `./target/release/listent daemon logs --since 1h` | Shows only logs from last hour |
| 4.2.7 | Follow logs | `./target/release/listent daemon logs --follow` | Streams logs continuously. Ctrl+C to stop. |

### 4.3 LaunchD Service (requires sudo)

| # | Test | Command | Expected Result |
|---|------|---------|-----------------|
| 4.3.1 | Install service | `sudo ./target/release/listent daemon install` | Creates plist in /Library/LaunchDaemons, loads service via launchctl |
| 4.3.2 | Verify plist | `cat /Library/LaunchDaemons/com.microsoft.sysinternals.listent.plist` | Valid XML plist with correct binary path and arguments |
| 4.3.3 | Service running | `launchctl list \| grep listent` | Service appears in launchctl output |
| 4.3.4 | Uninstall service | `sudo ./target/release/listent daemon uninstall` | Unloads service and removes plist file |
| 4.3.5 | Install without sudo | `./target/release/listent daemon install` | Fails with permission denied error |

---

## 5. Edge Cases & Error Handling

| # | Test | Command | Expected Result |
|---|------|---------|-----------------|
| 5.1 | Permission denied files | `./target/release/listent /System/Library` | Completes scan. May show warnings about unreadable files (unless --quiet). Does not crash. |
| 5.2 | Symlink handling | `ln -s /usr/bin/true /tmp/test_link && ./target/release/listent /tmp/test_link` | Follows symlink, scans the target binary |
| 5.3 | Non-binary file | `./target/release/listent /etc/hosts` | Scans file, skips it (not Mach-O). Reports no entitlements. Progress shows skipped: 1. |
| 5.4 | Large directory | `./target/release/listent /` | Handles large traversal gracefully. Progress updates continuously. Can be interrupted with Ctrl+C. |
| 5.5 | Mixed files + dirs | `./target/release/listent /usr/bin/true /usr/sbin` | Handles mix of individual files and directories correctly |
| 5.6 | Unicode paths | Create a directory with Unicode name, copy a binary into it, scan | Handles correctly without crashes |

---

## 6. Performance Validation

| # | Test | How to Verify | Expected Result |
|---|------|--------------|-----------------|
| 6.1 | Default scan speed | `time ./target/release/listent --quiet` | Completes in under 10 seconds for /usr/bin + /usr/sbin |
| 6.2 | Memory usage | Run `./target/release/listent /usr/bin` and monitor with Activity Monitor or `top -pid $(pgrep listent)` | Memory stays under 50MB even for large scans |
| 6.3 | Monitor CPU usage | Run `./target/release/listent monitor` for 60+ seconds, check CPU | CPU usage < 5% at default 1s polling interval |
| 6.4 | Progress accuracy | Compare final `Processed X/Y` line — X should equal Y at completion | Processed count matches total count |

---

## 7. Code Signing Verification (for release binaries)

| # | Test | Command | Expected Result |
|---|------|---------|-----------------|
| 7.1 | Verify signature | `codesign -vvv --deep --strict ./target/release/listent` | `valid on disk`, `satisfies its Designated Requirement` |
| 7.2 | Notarization (quarantine test) | `cp ./listent /tmp/listent_test && xattr -w com.apple.quarantine "0083;66a54e35;Safari;" /tmp/listent_test && /tmp/listent_test --version` | Prints version successfully (Gatekeeper allows it). If blocked, notarization failed. |
| 7.3 | Clean up quarantine test | `rm /tmp/listent_test` | File removed |

> **Note:** Test 7.2 will hang in SSH sessions if the binary is NOT notarized because Gatekeeper tries to show a GUI dialog. Run from a local terminal or use `timeout 5 /tmp/listent_test --version`.

---

## Test Completion Checklist

- [ ] Section 1: Help & Version (5 tests)
- [ ] Section 2: Static Scan Mode (17 tests)
- [ ] Section 3: Monitor Mode (10 tests)
- [ ] Section 4: Daemon Mode (12 tests)
- [ ] Section 5: Edge Cases (6 tests)
- [ ] Section 6: Performance (4 tests)
- [ ] Section 7: Code Signing (3 tests)

**Total: 57 manual tests**
