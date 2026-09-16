export function openableHref(value: string): string | null {
  const trimmed = value.trim();
  if (/^https?:\/\//i.test(trimmed) || /^file:/i.test(trimmed)) return trimmed;
  return null;
}
