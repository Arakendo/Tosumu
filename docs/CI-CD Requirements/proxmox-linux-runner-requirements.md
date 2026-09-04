# Proxmox Linux Runner Requirements

## Intended Lanes

The Proxmox cluster initially supplies disposable x86-64 Linux build workers.
Later pools may host K3s recovery, storage, network, and replication fault
experiments. A cluster of machines is not automatically a set of independent
failure domains; shared power, switching, storage, quorum, and administration
must be recorded.

## Cluster Inventory

- [ ] Record each NUC alias, CPU/RAM/local storage, Proxmox and kernel versions,
      management address zone, and physical power/network dependencies.
- [ ] Record whether VM disks use node-local storage, shared storage, or a
      replicated provider, including controller and filesystem details.
- [ ] Identify correlated failures: one UPS, switch, router, storage enclosure,
      room, administrator, or Proxmox quorum.
- [ ] Reserve a dedicated Tosumu resource pool, VM-ID range, bridge/VLAN, and
      storage allocation with quotas.
- [ ] Keep the Proxmox management plane unreachable from ordinary test guests.

## Disposable Worker Template

- [ ] Build a versioned Linux cloud-image template from an identified source.
- [ ] Apply updates, CA roots, time sync, runner prerequisites, compiler tools,
      PowerShell, and the admitted Rust toolchain before sealing the template.
- [ ] Remove machine IDs, SSH host keys, cloud-init instance state, logs,
      package caches that are not intentionally retained, and all credentials.
- [ ] Configure a non-root runner account, read-only base intent where
      practical, bounded scratch disk, and no passwordless general-purpose
      privilege escalation.
- [ ] Generate unique machine identity and host keys on first boot.
- [ ] Prove clone, boot, one-job registration, evidence upload, shutdown, and
      destruction after success, failure, cancellation, and controller crash.

## Proxmox Controller

- [ ] Use a dedicated API identity limited to the Tosumu pool, VM-ID range,
      selected templates, network, and storage; prohibit host-shell and global
      administration.
- [ ] Keep the API credential on a small controller boundary, not inside the
      worker VM and not in arbitrary repository steps.
- [ ] Validate requested template, node, bridge, storage, CPU/RAM/disk bounds,
      and VM-ID before mutation.
- [ ] Tag every VM with commit/job/expiry identity and run a bounded orphan
      reaper that reports before deletion.
- [ ] Prevent concurrent jobs from reusing the same VM, disk, K3s namespace,
      port range, or fault-control identity.

## First Linux Admission Probe

- [ ] Manually dispatch a reviewed commit to a fresh clone.
- [ ] Record template hash/version, Proxmox node alias, VM boot ID, OS/kernel,
      CPU flags, memory, filesystem/mount options, clock source, Rust/Cargo, and
      native compiler/linker identities.
- [ ] Run format, strict Clippy, tests, docs, dependency-provenance check, C ABI
      harness, and a clean rebuild with bounded time/output.
- [ ] Hash retained artifacts and upload through a write-only or narrowly
      scoped evidence identity.
- [ ] Destroy the VM and prove that its registration is offline and disk is no
      longer attached.
- [ ] Confirm untrusted pull requests cannot allocate a worker or call the
      Proxmox controller.

## K3s And Fault-Laboratory Gate

Do not install K3s into the ordinary build template. Create a separate,
versioned topology whose requirements include:

- exact K3s/container-runtime/CNI/CSI versions and manifests;
- one database authority per exclusive writable volume;
- explicit node, storage, control-plane, network, clock, and power fault
  controls;
- a control path outside the failure being injected;
- bounded recovery/RPO/RTO observations and independent state verification;
- cleanup/reseed semantics after every partition or node-loss experiment; and
- no claim that Proxmox node separation proves independent production failure
  domains.

## Deferred Until Separately Admitted

- privileged or host-networked repository workloads;
- direct guest access to Proxmox, router, switch, NAS, UPS, or Ceph credentials;
- release signing or production secrets;
- long-lived mutable K3s clusters used as qualification evidence;
- automatic destructive fault injection without exact target validation and a
  recovery controller.
