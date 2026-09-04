# Shared Security And Evidence Requirements

## Required Before Registration

- [ ] Name the owner and backup administrator for the runner group.
- [ ] Put LAN workers in a dedicated network segment or equivalently enforced
      firewall zone.
- [ ] Deny unsolicited access from runners to ordinary client devices, NAS
      shares, hypervisor administration, routers, and secret-management hosts.
- [ ] Permit only the outbound endpoints required for GitHub Actions, source
      acquisition, admitted package registries, time synchronization, and
      operating-system updates. Record exceptions.
- [ ] Use dedicated non-administrator runner identities with no interactive
      personal credentials, browser profiles, SSH agent, cloud CLI login, or
      mounted personal shares.
- [ ] Decide which jobs may be persistent and which must receive a disposable
      machine. Fault injection and untrusted decoding should default to
      disposable workers.
- [ ] Ensure snapshots, backups, and Time Machine do not capture live runner
      registration credentials or job secrets.
- [ ] Define how to revoke the runner, token, machine identity, and Proxmox API
      identity independently.

## Trigger Policy

LAN labels may be selected only by workflows whose files are protected by
review. The initial policy is:

- public and fork pull requests: GitHub-hosted runners only;
- pushes to a protected main branch: eligible for bounded, non-secret LAN jobs;
- manual dispatch: eligible when the requested commit is already reviewed and
  the dispatcher is authorized;
- scheduled jobs: eligible for retained fault, recovery, and extended-test
  workloads against an immutable commit;
- release and signing jobs: prohibited until a separate admission decision.

Environment approval or runner-group restriction should guard any job that can
reach a physical device, Proxmox control API, K3s credentials, or a retained
artifact store. Repository membership alone is not sufficient authority.

## Isolation And Cleanup

- A job workspace is hostile after executing repository code.
- Persistent workers require an explicit cleanup procedure and a periodic
  rebuild test. Deleting the checkout alone is not a reset.
- Disposable Proxmox workers should boot from a versioned template, obtain a
  one-job registration, upload bounded evidence, and be destroyed even when
  the job fails.
- Caches are performance inputs and cross-job state. Each cache needs a scope,
  owner, integrity source, maximum age, and purge operation.
- Docker/Podman sockets, host devices, privileged containers, KVM, and
  hypervisor APIs must be absent unless the particular lane requires them.
- Job timeouts and output-size limits are mandatory for fuzzing, fault
  injection, simulator/emulator boot, and recovery loops.

## Credential Rules

- Prefer short-lived registration and job credentials over stored personal
  access tokens.
- Give the runner no repository write permission unless a later workflow
  explicitly requires it.
- Separate GitHub registration, artifact upload, Proxmox lifecycle, K3s, and
  code-signing identities.
- Never place a broad Proxmox administrator token on a guest worker. A separate
  controller may create/destroy VMs through a role limited to the selected
  pool, templates, networks, and storage.
- Masking a value in logs is not proof that executed code could not read it.

## Evidence Record For Every LAN Job

Retain or print, without leaking secrets:

- repository, full commit ID, dirty-state result, workflow revision, job name,
  trigger class, and actor class;
- runner pool, ephemeral/persistent state, host class, architecture, operating
  system image/template ID, and boot identity;
- Rust/Cargo and relevant compiler, SDK, Xcode, NDK, container, K3s, kernel,
  filesystem, and storage-provider versions;
- command/profile identity, selected features and targets, start/end time,
  timeout, exit status, and expected artifact set;
- hashes and sizes of admitted outputs, plus where they were retained;
- cleanup/destruction result and any known contamination or retry.

A passing LAN job is an observation about the named subject. It does not imply
reproducibility, independence, filesystem durability, device support, recovery
correctness, or release suitability unless those claims have their own
acceptance criteria.

## Incident And Maintenance Rules

- Disable the runner group first if compromise or unexpected LAN access is
  suspected; investigate before reusing caches or templates.
- Revoke credentials and rotate any secret exposed to the job, even if logs do
  not show it.
- Treat runner OS, Xcode/SDK, NDK, Rust, template, firmware, hypervisor, storage,
  and workflow changes as evidence-changing inputs.
- Patch on a documented cadence, then rerun the admission probe. Do not leave a
  worker permanently unpatched merely to preserve reproducibility.
- Keep a tested de-registration and rebuild path. A snowflake runner is not a
  qualification environment.
