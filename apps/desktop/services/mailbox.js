const draftsKey = "elyra-drafts";

export function filterMessages(messages, { category = "inbox", quickFilter = "all", query = "" } = {}) {
  const normalizedQuery = query.trim().toLowerCase();
  return messages.filter((message) => {
    const matchesCategory = category === "inbox"
      ? !["spam", "trash"].includes(message.category)
      : category === "starred"
        ? message.starred
        : message.category === category;
    const matchesQuickFilter = quickFilter === "all"
      || quickFilter === "unread" && message.unread
      || quickFilter === "starred" && message.starred;
    const searchable = `${message.sender} ${message.subject} ${message.preview}`.toLowerCase();
    return matchesCategory && matchesQuickFilter && (!normalizedQuery || searchable.includes(normalizedQuery));
  });
}

export function loadDrafts() {
  try {
    return JSON.parse(localStorage.getItem(draftsKey) || "[]");
  } catch {
    return [];
  }
}

export function saveDraft(draft) {
  const drafts = loadDrafts().filter((item) => item.id !== draft.id);
  localStorage.setItem(draftsKey, JSON.stringify([draft, ...drafts]));
}

export function removeDraft(id) {
  localStorage.setItem(draftsKey, JSON.stringify(loadDrafts().filter((draft) => draft.id !== id)));
}
