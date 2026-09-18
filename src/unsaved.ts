export function canLeave() {
  return !document.querySelector('[data-unsaved="true"]') || window.confirm("You have unsaved changes. Leave without saving?");
}
