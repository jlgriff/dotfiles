---
name: commit-message
description: >
  Write a concise commit message: one sentence, under 25 words, starting with a past-tense verb.
  Use whenever writing, proposing, or revising a commit message, and when handing back a message
  for work just finished.
---

One sentence. Under 25 words total. The first word is a past-tense verb, with nothing in front of
it: no ticket id, no scope prefix, no type tag.

Say what changed, and why when the why is not obvious from the what.

No body paragraphs, no bullet lists, no second sentence.

Good:

```
Fixed the auth middleware token expiry check to compare inclusively.
Removed the unused feature-flag server wiring and its dependency.
Redacted query params from client error reports.
```

Bad:

```
Fix auth bug                                   (imperative, and says nothing)
This commit fixes the token expiry check       (narrates itself)
Fixed the expiry check. Also renamed the ...   (two sentences)
ABC-1234: Redacted query params ...            (prefix before the verb)
feat(auth): Added inclusive expiry check       (prefix before the verb)
```

Hand the message back rather than committing it, unless asked to commit.
