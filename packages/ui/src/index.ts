export interface ElyraButtonOptions {
  label: string;
  variant?: "primary" | "secondary" | "danger";
  disabled?: boolean;
  onClick?: () => void;
}

export function createButton(options: ElyraButtonOptions): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = options.label;
  button.dataset.variant = options.variant ?? "primary";
  button.disabled = options.disabled ?? false;
  if (options.onClick) button.addEventListener("click", options.onClick);
  return button;
}
