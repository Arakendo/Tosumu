# Mac M4 Runner Requirements

## Intended Lanes

The Mac mini is initially admitted only for Apple-silicon construction and iOS
simulator experiments. Swift packages, XCFrameworks, signing, and physical
devices are later gates.

## Host Preparation

- [ ] Record Mac model, CPU/RAM/storage, serial-reference alias, macOS build,
      firmware, FileVault state, and physical location. Do not publish serial
      numbers in CI logs.
- [ ] Create a dedicated standard local account for the runner. Do not sign it
      into iCloud or add personal development certificates.
- [ ] Disable automatic login, remote GUI access, personal file sharing, and
      unnecessary inbound services for the runner account.
- [ ] Choose a fixed work volume with enough headroom for Xcode, simulator
      runtimes, Cargo outputs, and cold rebuilds; define low-space thresholds.
- [ ] Exclude runner credentials and transient workspaces from Time Machine.
- [ ] Decide whether the runner is persistent or restored from a known APFS
      snapshot/reinstall procedure between trust epochs.
- [ ] Enable bounded system and runner logs without collecting database keys,
      workflow secrets, or unrelated user data.

## Toolchain Baseline

- [ ] Install Xcode from a retained, identified source and record exact Xcode
      build plus installed iOS SDK and simulator runtime identifiers.
- [ ] Accept licenses and install command-line components administratively
      before the runner service is enabled.
- [ ] Install the admitted Rust toolchain and Apple targets independently of the
      moving compatibility toolchain.
- [ ] Record Swift, Clang, linker, `codesign`, `simctl`, and `llvm-nm` identities.
- [ ] Confirm the selected iOS runtime and device type can be created, cold
      booted, invoked, shut down, and deleted under a hard timeout.
- [ ] Confirm no signing identity is visible to the runner account during the
      unsigned/ad-hoc simulator phase.

## Runner Installation

- [ ] Create a dedicated GitHub runner group and labels that describe facts,
      for example `self-hosted`, `tosumu-lan`, `macos`, `arm64`, and
      `ios-simulator`. Do not use a label such as `qualified`.
- [ ] Restrict the group to the Tosumu repository and trusted workflows.
- [ ] Install the service under the dedicated account, not root.
- [ ] Set job and simulator timeouts and reserve enough time for cold boot.
- [ ] Verify the service recovers after host restart without an interactive
      login and does not inherit a personal shell environment.

## First Admission Probe

- [ ] Manually dispatch a reviewed commit.
- [ ] Print the commit, clean-worktree state, host alias, macOS/Xcode/SDK/Rust
      identities, available disk, and selected simulator runtime.
- [ ] Compile the existing arm64 iOS device and simulator artifacts.
- [ ] Compare production-prefixed symbols with the retained allowlist.
- [ ] Compile the independent C loader and invoke the ABI-version export in a
      newly created simulator.
- [ ] Always shut down and delete the simulator; verify cleanup after a forced
      loader failure and timeout.
- [ ] Record elapsed cold/warm boot time and peak workspace/disk consumption.
- [ ] Confirm a fork or untrusted pull request cannot select the runner.

## Deferred Until Separately Admitted

- Apple developer login, distribution certificates, notarization, App Store
  credentials, or release signing;
- tethered iPhone/iPad access and device-unlock automation;
- Keychain/Secure Enclave claims;
- app lifecycle, backgrounding, file-protection, low-storage, thermal, or power
  qualification;
- XCFramework publication or support statements.
