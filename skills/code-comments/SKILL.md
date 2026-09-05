---
name: code-comments
description: >
  Rules for code comments: a one-line doc on every function, and almost nothing else. Use when
  writing or reviewing code that carries comments, when tempted to explain a line inline, when
  deciding whether a comment should exist at all, or when tempted to reference a ticket in code.
---

Every function gets a one-line doc. Beyond that, wanting an inline comment is a signal that the
code is not clear enough yet.

## The urge to comment is a smell

If you are about to explain a line, the code is not self-documenting. Rename, extract, or
restructure until the explanation is unnecessary. That is the default response, not a last resort.

A comment restating what the code does earns nothing. Delete it and fix the code.

## Function docs

One line above every function, even trivial ones, so editor hover surfaces it. State what the
function is **for**, in words its name does not already supply. Lead with purpose and append
rationale only if it still fits on the line.

"The visited set guards a parent cycle" fails: it explains one detail and never says the function
collects a leaf's inherited topics.

## When a comment survives

Only for something genuinely unclear that the code cannot express, such as an external constraint.
One line, terse, plain human language.

## Never a ticket number in the code

Not in comments, identifiers, test or describe names, file names, string literals, error messages,
or schema descriptions. A tracker's ids are a third party's system; wiring them into the codebase
couples it to something that gets renamed, migrated, and eventually retired.

State the fact the ticket carried: "payloads written before scope stamping", not "pre-ABC-1234
payloads". Ticket ids belong in branch names, commit messages (never as a leading prefix), and PR
descriptions.

## Never

- **Dated or transient notes.** No TODO handoffs, no project phases, no "as of <date>". If a
  purpose cannot be stated evergreen, question whether the function belongs.
- **Counts of surrounding things.** "both layers", "the three callers" go silently false as
  populations change. State the invariant, which holds at any count.
- **Navigation comments.** No section banners, dividers, or step signposts. If a block needs a
  signpost to be followable, split it into named functions or tests.
