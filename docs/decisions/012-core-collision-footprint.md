# ADR-012 — Retired Core collision proposal

Retired as an active behavioral requirement by maintainer instruction,
2026-10-23. The proposal's invisible-effect exclusion contradicted pinterf's
transparent border reservation and must not block compatibility corrections.

Historical context: the owner approved a conservative geometric effect hull
for the earlier Core implementation on2026-10-19. That work established separate
local and placement bounds, explicit-position exclusions and stateless placement.
Those useful engineering properties remain. The later pinterf investigation
found first/top and last/bottom line borders are reserved independently of alpha.

Current implementation findings, fixtures and remaining discrepancies live in
[compatibility notes](../compatibility-notes.md#layout-positioning-and-collision).
These notes are descriptive and can be corrected without another behavioral ADR.
