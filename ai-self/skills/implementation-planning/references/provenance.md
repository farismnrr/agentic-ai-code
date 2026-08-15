# Planning Skill Provenance

The local `implementation-planning` skill synthesizes compatible ideas from reviewed public sources while keeping local policy authoritative.

## Reviewed sources

- GitHub `github/awesome-copilot` — `create-implementation-plan`
  - deterministic, machine-readable phases/tasks
  - explicit requirements, dependencies, files, tests, risks, and acceptance criteria
  - https://github.com/github/awesome-copilot/tree/main/skills/create-implementation-plan

- Jesse Vincent / obra `superpowers` — `writing-plans`
  - inspect file structure before decomposition
  - right-size tasks around independently testable/reviewable deliverables
  - exact file responsibility and step-by-step validation
  - https://github.com/obra/superpowers/tree/main/skills/writing-plans

## Local deviations

- Planning is plan-only by default.
- Repository conventions override hard-coded plan locations.
- Exact paths/line numbers are never invented when not verified.
- TDD is not mandatory when the repository/task does not warrant it.
- No arbitrary wall-clock estimates.
- Git/team/security approval rules come from `ai-self/CONSTITUTION.md` and `ai-self/policies/default.yaml`.
