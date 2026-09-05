---
name: build-feature
description: >
  Plan and build a feature: research first, drive it with tests, implement the smallest thing that
  works, then keep only the tests that earn their place. Use when asked to build, add, implement,
  or plan a feature or capability, when sequencing that work, and when delegating parts of it to
  sub-agents.
---

## 1. Research

Get a complete picture before writing code: existing code, callers, constraints, prior art in the
repo.

Report what you verified separately from what you assumed. If the research shows the feature is the
wrong fix, say so before building it.

## 2. Write tests to drive the build

Write them before the implementation, and be as extensive as helps you build. A test written to
drive the work is scaffolding, not a commitment to keep it.

Aim lean and purpose-driven from the first one anyway: each test asserts a behavior that matters.
Tests written that way rarely need throwing away, while broad speculative ones become noise you
delete later.

Run them before implementing and confirm **every one fails**. A test that passes before the feature
exists does not test the feature.

## 3. Implement small

The smallest change that works. Fewest lines, not the most defensive.

Handle the edge cases the feature actually meets, and list the ones you left out so the user can ask
for them. Speculative edge-case handling needs approval first.

## 4. Verify

Run the whole suite, not only the new tests.

## 5. Keep only the tests that earn it

Once the suite is green, cull. Keep a test when its failure would break other systems or be
catastrophic. Delete the scaffolding that only helped you build, and say what you deleted.

## Never change a failing test to make it pass

Changing a failing test and deleting a passing test are different acts. Deleting a passing
scaffolding test in step 5 is fine. Changing a failing one hides the failure.

So if a test does not fail when you expected it to, or does not pass once the feature is in, stop
and ask: what you expected, what happened, and why the test now looks wrong.

## Sub-agents

On Fable, keep planning and orchestration on Fable and give sub-agents the highest-level Opus
available.
