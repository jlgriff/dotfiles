---
name: failing-test-first
description: >
  Show a test failing before adding it. Use whenever adding or changing a test, writing an ad-hoc
  verification script or harness, or reporting that a check passed. A test never seen to fail is
  not evidence.
---

Run the new test against the unfixed code first. Watch it fail, and read the failure: it must
fail for the reason the test exists, not for a typo, a missing stub or an unbuilt fixture.

Then apply the change and watch it pass. Report both, with the failure output quoted.

A test that has never been red proves nothing. It may assert something already true, exercise a
path the change does not touch, or pass because a helper swallowed the error.

## Getting to red

Keep the test, remove the change:

```sh
git stash push -q <source files>   # test stays, fix goes
<run the test>                     # expect red, read why
git stash pop -q
```

Reverting the one line by hand is equally fine. What matters is seeing it, not how.

## Ad-hoc scripts are tests

A script that checks behaviour is a test and the rule still applies. Most of the ways these lie
are avoidable:

- **Assert preconditions, never print them.** A run whose baseline already satisfies the
  post-condition has no signal. Abort when setup is not what the check needs.
- **Abort on failure, never continue to a verdict.** A helper that returns null or a sentinel on
  error will hand you a confident wrong answer.
- **Prove the mechanism fired**, not just that the outcome looks right. A hash that changed, a
  version that moved, a log line naming the entity.
- **Confirm the mechanism exists where you are testing.** Passing locally says nothing about an
  environment whose auth, permissions or data-visibility differ.

## Ask what would have made it green wrongly

Before trusting a pass, name the thing that would have made it red. If nothing would have, it is
not a check. Recurring causes:

- an error sent to `/dev/null`
- an unsupported flag, so the command never ran at all
- empty output counted as a result (`wc -l` reports 1 for a single blank line)
- a mutation that was rejected while the script carried on

Good:

```
Removed the fix, ran the test: "expected [] to have a length of 1". Restored it, 30 pass.
Baseline asserted: record active, present in the response and in its parent list. Then deactivated.
```

Bad:

```
Added a test and it passes.
Baseline: {present: false} ... after: {present: false} ... CONFIRMED.
Verified locally, so it will work in production.
```
