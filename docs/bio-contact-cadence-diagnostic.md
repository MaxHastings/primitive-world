# Reproductive timing at larger biological steps

2026-09-23. This is a short diagnostic, not a decision to change the live
wallpaper. The production build and saves were untouched. The three variants
started from the same copied v45-to-v46 imported checkpoint with 542 living
organisms and 38 packets, then ran one world for 3,000 biological units.
`BIO_DT=2` and `BIO_DT=4` updated ecology every eight biological units;
`BIO_DT=3` updated it every nine. A temporary, read-only GPU observer recorded
the production, proximity and packet-fusion funnel. The compact data is in
[the evidence JSON](evidence/bio-contact-cadence-20260923.json). The old
unobserved 2/4 probes showed the same direction (35 versus five births); GPU
scheduling makes exact trajectories vary.
The temporary observer is preserved as an
[reusable patch](evidence/bio-contact-cadence-harness.patch) against commit
`59dd28c`; it only adds diagnostic counters and an ignored test. For the
DT=3/4 runs, the disposable worktree changed `BIO_DT` to 3/4 and scheduled
ecology every 3/2 macro-steps with integration intervals of 9/8 biological
units. The checkpoint was opened read-only.

| Measure at 3,000 biological units | DT=2 | DT=3 | DT=4 |
| --- | ---: | ---: | ---: |
| Living organisms | 425 | 386 | 376 |
| Force selections | 659,909 | 432,035 | 320,873 |
| Successful force contacts | 4,320 | 3,091 | 2,671 |
| Packet-production selections | 357 | 176 | 105 |
| Productions with a near sensed entity at decision time | 320 | 151 | 78 |
| Productions with another organism within 6 units after movement | 223 | 99 | 32 |
| Packets actually made | 770 | 540 | 332 |
| Compatible packet proposal observations | 157 | 33 | 8 |
| Viable packet proposal observations | 157 | 33 | 8 |
| Viable births | 41 | 12 | 3 |

The near field means half the 24-unit sensor radius, approximately 12 units,
and can contain an organism or packet. Only 4.4%, 4.4%, and 4.8% of all
organism decisions respectively had a near sensed entity, while 90%, 86%,
and 74% of production decisions did. Production is strongly associated with
proximity in these trajectories. This is an association, not proof that an
impact triggers the controller. There is no explicit impact input.

At DT=4, half as many decisions per biological unit and a lower probability
of choosing production while sensing a near entity combine to leave roughly
one quarter as many near-field production decisions as DT=2 (78 versus 320).
Production with another organism still within interaction range after the
longer movement falls about sevenfold (32 versus 223). The paid five-packet
burst partially compensates: manufactured packets fall 2.3-fold (332 versus
770), less than production selections do. Force contacts per force selection
rise from 0.65% at DT=2 to 0.83% at DT=4, so the data do not show a failing
organism force-contact detector.

The larger loss is before packet-packet proposal: observations fall about
20-fold (157 to eight), while viable births fall about 14-fold (41 to three).
All observed proposals had enough combined packet energy to survive fusion
loss. Roughly a quarter to a third of proposal observations became births;
the post-proposal conversion did not collapse. Compatible packets coexisted
somewhere in the world for all 3,000 biological units at DT=2 and 2,452 at
DT=4, so global coexistence alone did not make local pairs meet. Packet
availability measured in packet-biological-units also fell from 88,668 to
42,176. Proposal observations count packet-macro-steps, may include both
members of a pair, and are not independent fusion attempts.

The code explains what the probe can and cannot attribute. An organism
selects one action before all biological substeps. Force and transfer are
resolved once after movement, and packet production is a mutually exclusive
action. Packet fusion proposals are then selected before newly produced
packets enter the spatial index. Existing packets move along a straight line
with common scalar damping during a macro-step, so the endpoint-to-endpoint
same-time sweep should cover their intervening packet-packet crossings.
Organism motion can bend because turning and thrust are integrated in each
substep, but its force/transfer sweep approximates that path by one chord.
This geometry limitation remains real; these aggregate results do not show
that it dominates reproduction loss.

**Interpretation:** The current inherited behavior appears tuned to a short
proximity-and-production timing window. Larger biological steps reduce the
chances to execute it and leave fewer packets in locally compatible paths.
The probe does not establish whether adaptation at DT=4 can discover a new
reproductive habit before the chain collapses. The earlier 200,000-unit ramp
recovered population through world restarts but did not show sustained
late-four-unit closed-chain retention. For a controlled next experiment,
compare a copied checkpoint with a counterfactual that
preserves the old 2-unit social decision/contact cadence while allowing four
units of physiology, or selectively adds an encounter-triggered decision
using the existing 107 inputs. Measure closed descendants and births per
packet, not just population or macro throughput.

**Owner-selected follow-up:** After reviewing this diagnostic, the owner chose
to run the existing wallpaper experiment at DT=3 for a long adaptation trial.
The short diagnostic remains a warning about early reproductive closure; it
does not set an early stop rule for the live trial. The last DT=2 checkpoint
and executable were archived separately before the handoff.
After observing the three-unit wallpaper continuing with hundreds of agents,
the owner chose to move the live wallpaper to DT=4. A complete DT=3 handoff
checkpoint and executable were also archived. Sustained closed-chain fitness
at DT=4 remains unmeasured.
