# ADR 0002: `_reversa_sdd` Is Reference Material, Not Parity Scope

## Status

Accepted for MVP planning.

## Context

The `_reversa_sdd/` directory contains generated specifications derived from Bevy analysis. Those documents include useful architectural patterns, but also describe systems far beyond the `seishin2d` MVP.

## Decision

Use `_reversa_sdd/` as a reference corpus for patterns, risks, and vocabulary. Do not treat it as a requirement to reproduce Bevy behavior or public API surface.

## Consequences

- Bevy-scale plugin graphs, exact scheduler parity, advanced render nodes, UI stacks, remote protocols, hot reload, and editor-like tooling are out of MVP scope.
- Conflicting `_reversa_sdd` signals are resolved in favor of the explicit `seishin2d` MVP contract.
- Architecture lessons can be adopted selectively when they serve the small vertical slice.
