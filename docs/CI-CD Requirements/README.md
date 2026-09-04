# Tosumu CI/CD Laboratory Requirements

| Field | Value |
| --- | --- |
| Status | Proposed infrastructure requirements; no LAN runner is admitted yet |
| Owner | Build, release, and qualification infrastructure |
| Scope | One Mac M4 mini and a Proxmox NUC cluster on a private LAN |
| Related reviews | AR-0007, AR-0010, AR-0015, AR-0017 |
| Related plans | MVP+11 mobile embedding, cluster fault tolerance and replication, high-assurance evidence export |

## Purpose

This folder defines the minimum conditions for using private LAN machines as
Tosumu build and qualification workers. It is a setup and admission record, not
an assertion that a machine is trustworthy merely because it is self-hosted.

The proposed laboratory has two complementary roles:

- the Mac M4 mini provides controlled Apple-silicon, Xcode, iOS simulator,
  Swift, packaging, and later physical-device observations;
- the Proxmox NUC cluster provides disposable Linux workers, K3s topologies,
  storage and recovery exercises, network partitions, process/node failure,
  and later replication experiments.

GitHub-hosted CI remains the independent portability baseline. LAN jobs add
named observations and longer-running fault evidence; they do not silently
replace hosted checks or upgrade a build into a support claim.

## Trust Boundary

```text
untrusted pull request
        |
        v
GitHub-hosted checks only
        |
        | reviewed commit on protected branch or explicit trusted dispatch
        v
LAN runner control boundary
        |
        +-- Mac M4: Apple build/simulator lane
        |
        +-- Proxmox API: create disposable Linux worker
                         |
                         +-- ordinary build/test lane
                         +-- K3s/fault laboratory lane
```

No workflow triggered by a fork, an untrusted pull request, or an unreviewed
workflow change may execute on a LAN runner. A GitHub Actions job can run
arbitrary commands with the runner account's authority; repository write
access, LAN reachability, cached credentials, and hypervisor control therefore
belong inside the threat model.

## Documents

- [Shared Security And Evidence Requirements](shared-security-and-evidence-requirements.md)
  defines triggering, isolation, credentials, provenance, retention, and
  incident rules common to every worker.
- [Mac M4 Runner Requirements](mac-m4-runner-requirements.md) is the weekend
  checklist for the Apple lane.
- [Proxmox Linux Runner Requirements](proxmox-linux-runner-requirements.md) is
  the weekend checklist for disposable Linux and later K3s workers.
- [Admission Worksheet](admission-worksheet.md) records the concrete topology,
  versions, validation results, and unresolved decisions before workflows gain
  self-hosted labels.

## Staged Admission

1. Record the intended machines, network segment, administrators, and failure
   domains without installing a runner.
2. Establish dedicated identities, least-privilege network rules, backup
   exclusions, logging, and a clean removal procedure.
3. Register one runner in an isolated runner group with no production secrets.
4. run a harmless trusted-branch probe that reports identity and tool versions;
5. prove cleanup, credential absence, and rejection of an untrusted trigger;
6. add a Tosumu build/test workload with retained subject and provenance;
7. add platform or fault experiments one at a time, each with a named claim and
   explicit non-claims.

An infrastructure architecture review should reconcile runner trust,
ephemeral-worker control, artifact custody, and evidence admission before a LAN
result becomes release or qualification evidence. Setup experiments may begin
before that review closes, but must remain labelled experimental.

## Weekend Exit Condition

The weekend setup is successful when at least one isolated worker can accept a
manually dispatched trusted commit, run a harmless probe, export bounded logs,
and be returned to a known state. Installing both platforms is optional. No
release signing, public-PR execution, physical-device claim, K3s resilience
claim, or artifact publication is required for this checkpoint.
