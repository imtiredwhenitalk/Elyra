export function usePersistentState(key, initialValue) {
  let value = initialValue;
  try {
    const stored = localStorage.getItem(key);
    if (stored !== null) value = JSON.parse(stored);
  } catch {
    value = initialValue;
  }

  return {
    get value() {
      return value;
    },
    set(nextValue) {
      value = typeof nextValue === "function" ? nextValue(value) : nextValue;
      localStorage.setItem(key, JSON.stringify(value));
      return value;
    }
  };
}
