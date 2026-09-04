# LAN CI/CD Admission Worksheet

Copy this worksheet into a dated supporting record when setup begins. Do not
put secrets, serial numbers, registration tokens, public IPs, or recovery codes
in the repository.

## Ownership And Scope

| Question | Recorded value |
| --- | --- |
| Primary administrator | TBD |
| Backup administrator | TBD |
| GitHub organization/repository | `Arakendo/Tosumu` |
| Runner group | TBD |
| Allowed trigger classes | Manual trusted commit initially |
| Evidence retention location and period | TBD |
| Emergency disable/revoke procedure | TBD |

## Network

| Question | Recorded value |
| --- | --- |
| Runner VLAN/firewall zone | TBD |
| Allowed outbound destinations/classes | TBD |
| Denied LAN destinations/classes | TBD |
| DNS/NTP sources | TBD |
| Proxmox controller location | TBD |
| Artifact/evidence endpoint | TBD |

## Mac M4 Subject

| Question | Recorded value |
| --- | --- |
| Non-sensitive host alias | TBD |
| Hardware class/RAM/storage | Mac mini M4 / TBD |
| macOS version and build | TBD |
| Xcode version and build | Candidate 16.4 (`16F6`); verify installed subject |
| iOS SDK/runtime/device type | Candidate 18.5 / iPhone 16; verify |
| Rust construction toolchain | Candidate 1.95.0; verify |
| Runner persistence/reset model | TBD |
| FileVault and backup-exclusion observation | TBD |
| First probe run/job/commit | TBD |

## Proxmox Subject

| Question | Recorded value |
| --- | --- |
| Cluster/node aliases | TBD |
| Proxmox/kernel versions | TBD |
| Shared failure domains | TBD |
| Tosumu pool/VM-ID range/bridge/storage | TBD |
| Controller API role summary | TBD |
| Linux template source/version/hash | TBD |
| Guest OS/kernel/filesystem | TBD |
| Ephemeral registration/destruction mechanism | TBD |
| First probe run/job/commit | TBD |

## Admission Tests

| Test | Mac | Proxmox Linux | Evidence |
| --- | --- | --- | --- |
| trusted manual dispatch runs | Not run | Not run | TBD |
| untrusted PR cannot select worker | Not run | Not run | TBD |
| identity/toolchain report is complete | Not run | Not run | TBD |
| Tosumu focused workload passes | Not run | Not run | TBD |
| timeout/failure cleanup succeeds | Not run | Not run | TBD |
| credentials absent after job | Not run | Not run | TBD |
| rebuild/recreate returns known baseline | Not run | Not run | TBD |
| runner can be revoked independently | Not run | Not run | TBD |

## Admission Result

- State: `not_evaluated`
- Permitted claims: none
- Explicit non-claims: no release, qualification, device, durability,
  fault-tolerance, replication, or support evidence
- Unresolved risks: TBD
- Reviewer/date: TBD
- Related AR decision: TBD
