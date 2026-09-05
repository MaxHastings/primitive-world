# Direction: small beginnings, evolving organisms

Chosen direction, September 5, 2026. The V6 foundation implements this commitment.

## The commitment

Build a living, watchable world where organisms start with small brains and
inherit structural changes that can make those brains larger, smaller, or simply
different. Structure has a cost. Organisms survive and reproduce through what
they do in the world. We do not assign meanings or reward intelligence.

We are choosing this direction because it fits the project and offers compelling
possibilities. We do not need to establish that it is the optimal architecture
before building it. Research informs our judgment; it does not set a sequence of
permission gates. Ordinary engineering checks should keep the implementation
honest without turning every design decision into a research campaign.

The guiding principle is: **Give evolution room to change the organism. Do not
write the behavior we hope to see.**

This is a continuation of Primitive World, not a restart from nothing. Its local
world, resource economy, recurrent memory, physical interactions, and inspector
are already a strong foundation. V6 replaces the permanently fixed brain structure.

## Keep

Keep local sensing, generic recurrent memory, movement, collection, automatic
digestion, transfer, force, and scalar signals. None needs a built-in social
meaning. Communication can remain unused until organisms find a use for it.

Keep paid reproduction, local offspring, inherited variation, empty memory at
birth, finite bodies, and extinction. Keep the current body rules while changing
the brain; there is no reason to redesign physiology at the same time.

Keep the ecology and the enjoyable continuous viewer. Food depletion, changing
patches, weather, and other organisms already give behavior consequences. Keep
inspection and ancestry so we can watch what develops and investigate surprises.

Keep the optional survivor restart loop. It is useful for watching a lineage
continue across worlds. Describe it plainly as external carryover: it rejuvenates
bodies and favors late survivors, so it is different from an uninterrupted
population reproducing in one world. That distinction needs clear presentation,
not removal of a useful play mode.

## Change

### Make brain structure inherited and mutable

Start fresh organisms with four generic recurrent units. Allow node duplication,
node deletion, connection insertion/removal, and parameter mutation at birth.
Children inherit their own structure; parents do not grow extra units during life.

Use one generic gated unit type, preserving the useful idea of controller-owned
memory retention. Keep the existing sensory and physical actuator interface.
Four units are a starting size, not a floor: initially allow 1–64 expressed units.
The upper bound is an engineering budget, not an evolutionary destination.

Use explicit sparse connections. Count the sensory, recurrent, gate, and output
connections as part of the brain. Four units must not conceal a second large,
free controller around them.

Prefer duplication that initially preserves the old computation. Copy incoming
structure and split outgoing influence so the extra copy does not simply double
its effect. Handle recurrent loops and gates consistently. Copies can then
change independently. Start with individual units; arbitrary module duplication
can wait until the basic representation is working.

### Give structure a straightforward resource cost

Charge modest upkeep for expressed units and connections. Charge construction
and copying at reproduction for inherited structure. A node doing little useful
work still occupies structure; zero activation does not make it free.

Unused GPU allocation is not part of the organism. An inherited disconnected
node is. Keep that distinction explicit so padding is free but genome bloat is
not. Extra structure must be affordable before a child is created.

Choose simple, visible constants and move forward. We can adjust the world's
balance through experience without pretending those constants are natural laws.
Do not add intelligence rewards, complexity bonuses, or success-dependent growth.

### Simplify mutation

The new model removes the two brain outputs that requested offspring mutation
probability and magnitude. Use declared world-level mutation settings,
including nonzero ordinary variation and structural changes in both directions.

Having the controller also discover how to keep evolution going is an unnecessary
extra problem right now. Inherited mutation machinery can be a later extension.
Do not silently reinterpret the old output slots or convert old genomes into
supposedly equivalent new ones.

### Make observation serve the world

Show brain size, connection count, structural upkeep, and inherited changes in
the inspector. Show whether descendants are living and reproducing. Keep existing
journey and signal information as observations, not achievement requirements.

Do not display an intelligence score. Bigger brains are not automatically better.
A successful tiny lineage is just as legitimate as a complex one. Interesting
behavior should invite investigation, without requiring a formal campaign before
we can enjoy it or make another design decision.

## Remove and defer

The sparse brain is the sole production architecture. Preserve the old executable and saves for their existing worlds;
do not maintain several competing production brain systems indefinitely.

Remove brain-controlled mutation from the new foundation. Remove any expectation
that every feature must be used, populations must become social, or brain sizes
must increase. Do not add rescue rules to make those outcomes happen.

Defer lifetime brain growth, evolvable body shapes, new sensory organs, explicit
speciation protection, novelty rewards, and additional learning systems. They are
possible future directions, not prerequisites for this one.

Do not make a fixed-small/fixed-large comparison campaign, a shrinkage result,
or proof of random-origin establishment a prerequisite for building the new
model. Useful diagnostics remain available when they answer an actual question.

## Implemented foundation

The same sparse graph and mutation law run in the live GPU engine and the native
survivor transfer. Genomes hold 1–64 units and up to 512 explicit connections.
The four-unit founders use 144 connections. The maximum allocation uses about
105.4 MiB at 16,384 body slots; absent allocation is neither expressed nor copied
as genetic material.

The inspector shows units, connections, birth changes, and energy costs. World
settings expose upkeep, copying, and paired structural mutation rates.
Banks use format 7, checkpoints format 18, and both identify
`primitive-v6-variable-brain`. Earlier files remain for the earlier engine.

Engineering verification focuses on broken inheritance, invalid graph references,
CPU/GPU disagreement, incorrect energy charges, and corrupt saves. It does not
gate the creative direction on a comparative research campaign.

## Longer term

After this foundation, favor richer consequences in the shared world over
continually adding specialist brain features. Organisms already deplete food,
affect soil, and leave inventory behind. Persistent material changes could later
let one organism's activity create opportunities for another, without us naming
houses, tools, tribes, or cooperation in the rules.

Development and morphology can follow when they feel like a coherent extension.
We do not need to promise endless complexity. The commitment is to a world with
room for inherited structure and behavior to surprise us, and enough clarity
that we can understand what we built.
