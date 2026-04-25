# Lists
tool-list-lists = List all lists for the current user
tool-create-list = Create a new list, optionally inside a container or as a sublist
tool-update-list = Update a list's name, description, type, or archive status
tool-move-list = Move a list into a container or remove from container
tool-get-list-sublists = Get direct sublists of a list
tool-set-list-placement = Move one or more lists between root, containers, and parent lists

# Items
tool-get-items = Get items in a specific list, optionally filtered by completion, deadline presence, or date range
tool-search-items = Search items globally by title, description, and tags
tool-search-entities = Search items, lists, and containers globally by name/title and description
tool-next-cursor-page = Fetch the next page for a previously returned paginated cursor
tool-add-item = Add one or more items to a list
tool-update-item = Update an existing item
tool-toggle-item = Set the completed state for one or more items
tool-move-item = Move one or more items to a different list

# Containers
tool-list-containers = List all containers for the current user
tool-create-container = Create a new container (folder or project)
tool-get-container = Get a container with progress metrics
tool-get-container-children = Get sub-containers and lists inside a container
tool-get-home = Get home dashboard: pinned, recent, root containers and lists

# Tags
tool-list-tags = List all tags for the current user
tool-create-tag = Create a new tag
tool-assign-tag = Assign a tag to an item or list
tool-remove-tag = Remove a tag from an item or list
tool-set-tag-links = Assign or remove many tag links for items or lists in one call
tool-get-tagged-items = Get all items tagged with a specific tag
tool-get-tag-entities = Get items and lists linked to a tag, optionally filtered by entity type

# Calendar
tool-get-calendar = Get items with dates in a date range
tool-get-today = Get all items for today, including overdue

# List features
tool-enable-list-feature = Enable a feature on a list. For 'deadlines', optionally configure which date fields are available. For 'quantity', optionally set a default unit. Call only after confirming with the user (unless mcp_auto_enable_features is set).
tool-disable-list-feature = Disable a feature on a list. Item data (quantities, dates) is preserved — data is hidden in UI but not deleted.

# OAuth consent page
oauth-consent-title = Authorize access
oauth-consent-client-requests = { $client } wants to access your Kartoteka account.
oauth-consent-scope-label = Permissions requested:
oauth-consent-warning = Only approve if you trust this application. You can revoke access later in Settings.
oauth-consent-approve = Approve
oauth-consent-deny = Deny
oauth-consent-scope-mcp = Read and modify your lists, items, tags, comments, and time tracking.

# New MCP tool descriptions
mcp-tool-create_item-desc = Create a new item in a list.
mcp-tool-update_item-desc = Update fields of an existing item. Use the "clear" array to explicitly set fields to null (e.g. clear: ["deadline", "description"]).
mcp-tool-search_items-desc = Full-text search across items and their comments.
mcp-tool-add_comment-desc = Add a comment to an item, list, or container. Omit author_name when writing on the user's behalf (their voice). Set it to your name (e.g. "Claude") when the comment is your own observation, suggestion, or analysis.
mcp-tool-add_relation-desc = Create a blocks or relates_to relation between two items.
mcp-tool-remove_relation-desc = Remove an existing relation between two items.
mcp-tool-start_timer-desc = Start time tracking on an item (auto-stops any currently-running timer).
mcp-tool-stop_timer-desc = Stop the currently-running timer.
mcp-tool-log_time-desc = Log a retrospective time entry with start time and duration.
mcp-tool-create_list_from_template-desc = Create a new list from one or more templates.
mcp-tool-save_as_template-desc = Snapshot an existing list as a reusable template.

# MCP resource descriptions
mcp-res-lists-desc = Minimal list projections for discovery (id, name, container, pinned, archived, item_count).
mcp-res-containers-desc = Minimal container projections (id, name, parent_id, status, pinned).
mcp-res-tags-desc = Tag projections paginated (id, name, tag_type, parent_id).
mcp-res-today-desc = Items due today, resolved in the user's timezone.
mcp-res-time-summary-desc = Aggregated time tracking: today, week, top-10 per list.
mcp-res-list-detail-desc = Full list object with features and item count.
mcp-res-list-items-desc = Items in a list, paginated with opaque cursors.
mcp-res-container-detail-desc = Container with its children projections.

# MCP error messages
mcp-err-unauthorized = Unauthorized: missing or invalid bearer token.
mcp-err-not-found = { $entity } not found.
mcp-err-validation = Validation failed: { $reason }.
mcp-err-feature-required = List does not have feature: { $feature }.
mcp-err-forbidden = Access forbidden.
mcp-err-internal = Internal error.
mcp-err-bad-uri = Invalid resource URI: { $uri }.
mcp-tool-list_lists-desc = List all your lists.
mcp-tool-get_list-desc = Get details of a specific list by ID.
mcp-tool-list_items-desc = List items in a list, with optional cursor-based pagination.
mcp-tool-list_containers-desc = List all your containers.
mcp-tool-get_container-desc = Get details of a specific container by ID.
mcp-tool-list_tags-desc = List all your tags.
mcp-tool-get_today-desc = Get items due today.
mcp-tool-get_time_summary-desc = Get all time entries.
mcp-tool-create_list-desc = Create a new list. list_type: checklist (default), shopping, habit, or custom.
mcp-tool-get_item-desc = Get details of a specific item by ID.
mcp-tool-list_templates-desc = List all saved list templates.
mcp-tool-list_overdue-desc = Get all items past their deadline that are not yet completed.
mcp-tool-get_active_timer-desc = Get the currently running timer, if any.
