# Server-rendered HTML is not evidence that the app runs

Plan 001 shipped a chat UI that rendered perfectly and was completely unusable. `Comark` never resolved — `@comark/nuxt` registers `Markdown`, and the prop is `value`, not `markdown`. An unresolved component SSRs as an empty node and then fails to match on the client, so hydration broke for the **whole page** and nothing was clickable.

Every check up to that point read server-rendered HTML over a socket. It all passed, because **SSR does not care that a component is missing**. The dev log had been saying `Failed to resolve component: Comark` 34 times, followed by `Hydration completed but contains mismatches`, for hours.

**What to do instead:**

- A socket fetch proves markup exists. It does not execute JS, so it proves nothing about hydration, event handlers, or client state. Don't report it as verification of behaviour.
- Nuxt pipes the browser console into the dev-server log. After a real browser loads a page, grep that log for `Failed to resolve component`, `Hydration`, `missing template`, and `[console.error]`. Zero of those is the actual pass condition.
- Better still, add a headless browser. It was declined for this project (2026-08-07), so the log check is the fallback — but it only produces signal once someone opens the page.

**Related trap:** the `nuxt-ui` skill's `references/layouts/chat.md` is where the wrong component name came from. Skill references drift from installed packages. Read the installed `.d.ts` or use the `nuxt-ui` MCP for anything you're about to depend on — see [[ai-sdk-native-features]] for the same lesson from the other direction.
