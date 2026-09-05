---
name: demo-video
description: >
  Route a demo video request to the repo's own recording skill. Use when asked to demo, record,
  screen-capture, or show a change working as a video, before doing any recording work.
---

Repos own their recording harness. Find the repo's skill and follow it rather than building a
recording setup.

## 1. UI or API

- Visible in the app (component, page, interaction, styling): **UI**.
- Server contract (GraphQL field, resolver, endpoint, response shape): **API**.

If both changed, record whichever proves the feature. If that is genuinely both, ask which.

## 2. Find the repo's skill

```sh
find . -maxdepth 4 -path '*skills/*demo*' -name SKILL.md -not -path './node_modules/*'
```

Covers `.claude/skills/`, `.agents/skills/`, and `.codex/skills/`. Read the matching skill and
follow it exactly; its harness, template, and output format are repo-specific.

## 3. Nothing matches

Say so and ask before improvising. A hand-rolled recording will not match how the repo produces
its demos, and it will not prove the feature the way a reviewer expects.
