# Workspace Routing Trade-off

**Context**: In plan 007, we introduced a "workspace" concept (a group of conversations).

**Decision**: The active workspace is scoped client-side (via a `workspace-id` cookie and `useState`) rather than through a nested URL segment (like `/w/:id/chat/:id`).

**Why**: To avoid a costly routing rewrite where every `to="/chat/..."` link, breadcrumb, and navigation item would need to be updated. A nested route tree was out of scope.

**Future considerations**: If workspaces require deep-linking (so users can share a link directly to a workspace), we will need to rewrite the routing to include the workspace ID in the URL.
