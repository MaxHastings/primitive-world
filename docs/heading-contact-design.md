# Heading, sampling, and contact design

This is the replacement contract for the fixed-compass and abstract-target
interface. It is intentionally one model change: heading without body-relative
sampling, or sampling that still selects a hidden body slot, would retain the
special-purpose semantics this change removes.

## Body state

Each body has a heading angle, linear and angular velocity, and equal fixed unit inertial mass. Heading is
initialized from a uniform angle, reset for a newborn, and is never a cognitive
input as an absolute world angle. Controller vectors are transformed between the
body frame and world frame only at the physics boundary.

Voluntary locomotion supplies bounded forward thrust and turning torque. Angular
velocity becomes `0.85*angular_velocity + 0.0375*turn_effort` before heading
integration. Applied turning effort costs `0.02*abs(turn_effort)` energy;
coasting is free. Angular velocity starts at zero for founders and newborns. Contact
impulses change actual velocity independently. Damping and integration then
advance position on the torus. This makes observed relative motion, recoil, and
resistance consequences of the same state rather than separate feedback rules.

## Perception

Sixteen identical samples occupy eight body-relative bearings at two ranges.
They are limited-area kernels, not named regions or identity slots. Each sample
contains the same physical measurements:

- resource density;
- body occupancy;
- mean relative velocity in body coordinates;
- signed local signal activity; and
- contact/proximity pressure when applicable.

The controller receives no global compass, nearest-body record, body identity,
lineage, inventory, or target selector. Empty samples read zero. The recurrent
state must integrate ambiguous samples over time.

## Contact and actions

Finite-radius bodies establish contact from their wrapped positions and a fixed six-unit contact range. Transfer
selects an available contact by physical arbitration, never a controller body
slot. A force actuator produces a bounded body-relative impulse at contact;
the equal-and-opposite impulse changes both bodies. Energy is charged for
generated impulse/mechanical energy, not recipient displacement.

Removing the eight target logits frees controller outputs for turn/thrust,
body-relative impulse, and bounded body-relative packet placement. Placement
is applied only by packet manufacture and remains toroidally wrapped.

## Required invariants

- Rotating an entire world and every heading rotates behavior without changing
  body-frame controller inputs.
- Translating bodies across the torus seam changes no local measurement or
  contact result.
- Contact never occurs remotely, and total impulse is equal and opposite.
- No controller-visible field identifies another body or classifies its social
  relationship.
- Newborns reset heading/velocity/runtime state while inheriting only allowed
  hereditary traits.


The implementation uses the area-kernel version of this interface: sixteen
body-relative wedges/bands with identical measurements. Square-grid food
aliasing limits exact global rotation tests to quarter turns, while body-only
contact/placement tests also use arbitrary angles. See agents.md and world.md for
current numeric mechanics, lifecycle assumptions and the reserve-energy ledger.
