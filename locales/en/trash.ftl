# /trash — the review page for pending deletions (#226)

trash-title = Trash
trash-subtitle = Entries marked for deletion. Deleting is permanent and cannot be undone.
trash-loading-error = Could not load the trash: { $detail }
trash-empty = The trash is empty — nothing is waiting to be deleted.
trash-section-containers = Containers
trash-section-lists = Lists
trash-section-items = Items
trash-section-comments = Comments
trash-select-all = Select all
trash-selected = Selected: { $count }
trash-action-unmark = Unmark
trash-action-delete = Delete
trash-action-unmark-selected = Unmark selected
trash-action-delete-selected = Delete selected
trash-action-delete-all = Empty trash
trash-marked-just-now = just now
trash-marked-unknown = —
trash-marked-minutes = { $count ->
        [one] { $count } minute ago
        *[other] { $count } minutes ago
    }
trash-marked-hours = { $count ->
        [one] { $count } hour ago
        *[other] { $count } hours ago
    }
trash-marked-days = { $count ->
        [one] { $count } day ago
        *[other] { $count } days ago
    }
trash-cascade-lists = { $count ->
        [one] { $count } sublist
        *[other] { $count } sublists
    }
trash-cascade-containers = { $count ->
        [one] { $count } subcontainer
        *[other] { $count } subcontainers
    }
trash-cascade-items = { $count ->
        [one] { $count } item
        *[other] { $count } items
    }
trash-cascade-badge = +{ $details }
trash-confirm-delete-title = Delete permanently
trash-confirm-delete-message = Permanently delete { $count ->
        [one] { $count } entry
        *[other] { $count } entries
    } from the trash? This cannot be undone.
trash-confirm-delete-cascade = This also removes: { $details }.
trash-confirm-delete-button = Delete permanently
