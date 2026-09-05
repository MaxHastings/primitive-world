# Independent foundation review — September 4, 2026

The user requested an independent read-only review of the project. The reviewer
examined main, the physiology research branch, historical preparation, raw
development results, physics and observer/UI code. No simulations, GUI actions,
edits or process stops were delegated. This is an audit, not acceptance evidence.

## Findings and decisions

1. **Simulation integrity is not inherited competence.** Earlier passing tests
   established bounded wiring/accounting properties. The rotating-sensor prior
   mismatch and hungry-empty ingest behavior were real starting-policy obstacles
   ([frame audit](FOUNDATION_FRAME_AUDIT.md), [departure audit](DEPARTURE_ATTRITION.md)).
   The v2 alternative removes these obstacles but retains authored bootstrap
   coefficients, including a reproduction preference that can ask before the
   required food stock is available. Failed requests are not missing births or
   an accounting fault. Do not add automatic reproduction or action masks merely
   to improve the counters.
2. **Relevant preparation was limited.** Historical `training/prepare.py` DID
   carry the first exported bank into the second world; it was not universally
   fresh-weight training. Its default two12k worlds individually ended before
   the first major16,384–24,576 resource transition. Later independent diagnostic
   worlds do not themselves add cross-world training. The current registered
   cumulative campaign is the appropriate next test; finish it before changing
   the controller again. Endpoint survivor sampling and fresh-world reserves are
   authored offline selection, not an unbroken natural history.
3. **Actual movement matters more than the speed ceiling.** In the v2 seed808
   failed-crossing sample, actual pace was far below the allowed maximum. The
   observer's optimistic reach calculation often found sufficient range at
   maximum speed, but not at observed speed. That does not give agents knowledge
   of the destination or prove it is discoverable. Do not infer that bigger
   brains, cheaper metabolism or higher maximum speed are necessary from this
   sample alone. Review raw evidence in the research branch's
   `reports/PHYSIOLOGY_DEVELOPMENT.md`.
4. **The migration observer cannot yet certify the full contract.** Schema2
   samples last-step feedback, so it can miss intervening feeding or movement.
   Spatial footprints are not persistent resource-patch identities, and completed
   journeys are not yet attributed to separate major relocations. Close these
   evidence gaps before final holdouts. The contract does not require the same
   individual, or silently add a requirement that one connected family perform
   every relocation; it requires descendant migration/feeding/reproduction
   evidence across at least three separate major relocations per qualifying world.
5. **Inspection and saving undermine play/debugging.** The original GUI stores
   a click-time snapshot indefinitely and highlights by storage slot alone;
   newborn slot reuse can make that misleading. Saving repeatedly to one fixed
   filename correctly refuses overwrite but requires external file management.
   Fix live identity-aware inspection and separately named editable-path saves
   without changing the frozen experimental executable.

## Follow-through and limits

The research branch is implementing the inspection/persistence changes with
CPU and headless GPU checks. The cumulative experiment retains its frozen
executable, banks, settings, seed orders, failures and final baseline repeats.
No fresh final holdouts have been used. More training is a testable hypothesis,
not a guarantee, and this review does not establish that the project is done.

The reviewer proposed a small follow-up if needed after the registered campaign:
repeat a matched prepared/unprepared comparison on development seed1001 to test
whether its apparent preparation benefit repeats. This is not yet registered
or started, and is not a substitute for final fresh-seed validation.

See [current status](RESEARCH_STATUS.md) and [the unchanged contract](../GOAL_CONTRACT.md).
