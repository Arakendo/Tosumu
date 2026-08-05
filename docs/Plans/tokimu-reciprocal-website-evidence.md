# Tokimu Reciprocal Website Evidence

| Field | Value |
| --- | --- |
| Status | Proposed |
| Opened | 2026-08-04 |
| Last updated | 2026-08-04 |
| Owner | Tosumu and Tokimu maintainers |
| Tosumu target | MkDocs documentation and bounded storage labs |
| Peer plan | Tokimu `docs/Plans/tokimu-tosumu-reciprocal-website-evidence.md` |
| Related reviews | AR-0001, AR-0002, AR-0005 |

## Purpose

Use Tokimu as an optional presentation provider for Tosumu's public,
versioned observations while letting Tosumu provide durable storage evidence
for Tokimu's Resource Space and asset consumers.

The full reciprocal scope, lab matrix, artifact contract, slices, risks, and
completion criteria live in the peer Tokimu plan. This Tosumu record exists so
the work is visible from Tosumu's own governance and documentation lifecycle.

## Tosumu Ownership

Tosumu owns:

- storage, integrity, recovery, and public inspection facts;
- TQL command/result meaning and JSON schema;
- disclosure policy and protected-data boundaries;
- generation of Tosumu-originated evidence artifacts.

Tokimu may present those facts, but it must not parse pages, infer storage
truth, or redefine command outcomes.

## Initial Tosumu Deliverables

- [ ] Select one bounded public TQL or inspection fixture.
- [ ] Define and emit `reciprocal-site-evidence-v1` metadata.
- [ ] Add a static Tosumu lab page with accessible textual evidence.
- [ ] Add an optional Tokimu-rendered view of the same artifact.
- [ ] Validate that unknown schema versions and stale artifacts fail
      explicitly.
- [ ] Keep the Tosumu MkDocs build independent of Tokimu-generated output.

## Acceptance Criteria

- [ ] Tosumu remains useful and buildable without Tokimu WASM or JavaScript.
- [ ] Visual evidence names its fixture, producer revision, schema, and limits.
- [ ] No secret values, physical host paths, protector material, or unreviewed
      provider errors cross the evidence boundary.
- [ ] Tokimu presentation and Tosumu source facts remain independently
      testable.
- [ ] Any reusable adapter is admitted only after evidence from both sites.

## Reopening Trigger

Begin implementation when one reviewed Tosumu JSON fixture and one Tokimu
presentation island can be pinned without adding a runtime dependency between
the sites.
