# Project Guidelines

## Layer Boundaries
Do not introduce dependencies that violate the intended layer direction. Domain modules must remain independent of infrastructure and UI concerns. If a change requires crossing layers, introduce an explicit boundary (interface, adapter, or mapping) rather than bypassing the structure.

## Architecture
Keep modules small and role-focused. Prefer extracting adjacent logic into named modules over growing large entrypoint or orchestration files. When behavior can live in one canonical owner, remove forwarding wrappers instead of preserving duplicate public surfaces.

## Reuse
Before adding a new variant-specific helper or command path, look for an existing abstraction seam and extend it. Avoid combinatorial APIs such as separate methods for each unlock or protector combination when one data-driven path can express the same behavior.

## Maintainability
Favor code that is easy to navigate over clever local shortcuts. If a file is becoming a mixed container for parsing, dispatch, IO, formatting, and tests, split it by responsibility. Keep tests near the module that owns the behavior they validate.

## Readability
Write code that can be quickly understood by a new contributor. Prefer clear naming, simple control flow, and explicit behavior over clever or dense implementations. Avoid unnecessary abstraction, indirection, or generic complexity unless it provides clear and immediate value. Favor code that can be understood by reading the file in isolation. Minimize the need to jump across multiple modules to follow basic behavior.

## Validation
After each non-trivial edit, run the narrowest test or build command that covers the touched slice before widening scope. Prefer focused `cargo test -p ...` or `dotnet build` checks over whole-repo runs unless the change is cross-cutting.

## Documentation
When design or organization decisions change, update the nearest durable document instead of leaving the rationale only in code or chat. Link to existing docs such as `DESIGN.md`, `README.md`, and `INSPECT_API.md` rather than duplicating them.

## Dependencies
Do not add new dependencies unless the existing code cannot reasonably solve the problem. Prefer standard library and existing project dependencies first. Document why any new dependency is needed.

## Local Consistency Check
When modifying a file, briefly scan surrounding code for violations of the above guidelines. Prefer small, localized improvements (naming, structure, duplication) when they are low-risk and directly adjacent to the change. Avoid large or unrelated refactors. If the file is already large or has organizational issues, note them in the audit tracker and move on to the intended change.
