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

# Calendar
tool-get-calendar = Get items with dates in a date range
tool-get-today = Get all items for today, including overdue

# List features
tool-enable-list-feature = Enable a feature on a list. For 'deadlines', optionally configure which date fields are available. For 'quantity', optionally set a default unit. Call only after confirming with the user (unless mcp_auto_enable_features is set).
tool-disable-list-feature = Disable a feature on a list. Item data (quantities, dates) is preserved — data is hidden in UI but not deleted.
