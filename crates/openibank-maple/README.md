# openibank-maple (WorldLine Adapter)

This crate now exposes a Maple WorldLine adapter used by OpenIBank demo/TUI flows.

## Maple APIs selected

- `worldline-runtime::fabric::EventFabric`
  - append immutable events to WorldLine WAL
  - subscribe/tail event stream for UI
- `worldline-runtime::gate::CommitmentGate`
  - enforce Intent -> Commitment -> Consequence boundary
  - consequence is recorded only after approved commitment
- `worldline-core::identity::IdentityManager`
  - agent worldline identity registry

## Adapter surfaces

- `MapleWorldlineRuntime`
  - action execution pipeline (mint/permit/escrow/settle)
  - receipt verification against signature + WorldLine pointer
  - export bundle (`worldline_slice.json`, `receipts.json`, receipt files)
- Traits:
  - `WorldLineWriter`
  - `WorldLineReader`
  - `CommitmentGatePort`

## Local mode

The runtime uses local file-backed Event Fabric (`~/.openibank/runs/<run_id>/fabric`) and does not require network or external DB services.

