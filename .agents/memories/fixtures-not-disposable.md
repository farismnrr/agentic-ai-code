# Fixtures are not all disposable

When removing dummy data and fixtures after migrating to a real backend, remember to check what's actually inside the `fixtures/` directory before deleting it wholesale.

In this project, `shared/utils/fixtures/models.ts` contained the **real**, curated list of 9Router models. It was mislabeled by living in `fixtures/` rather than being a disposable stub. It was moved to `shared/utils/models.ts` instead of being deleted.

Always verify the consumers of "dummy" data. If it's imported by core config or settings UI (rather than just debug views or seed scripts), it might be real data that drifted into the wrong folder.
