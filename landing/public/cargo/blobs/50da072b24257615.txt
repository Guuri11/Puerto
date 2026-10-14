// Conversions between what a form input holds (text) and what the model holds. The generated
// form views call them; an input that holds text always holds a `string`, never `null`.

/** The text of a value, `""` for `null`. */
export function textOf(value: string | number | boolean | null): string {
  return value === null ? "" : String(value);
}

/** `null` for an empty input. */
export function textOrNull(text: string): string | null {
  return text === "" ? null : text;
}

export function numberOrNull(text: string): number | null {
  return text === "" ? null : Number(text);
}

/** A `<select>` of `""` / `"true"` / `"false"`. */
export function booleanOrNull(text: string): boolean | null {
  return text === "" ? null : text === "true";
}

/** Comma-separated items, trimmed; empty ones dropped. */
export function splitList(text: string): string[] {
  return text
    .split(",")
    .map((item) => item.trim())
    .filter((item) => item !== "");
}

export function splitNumbers(text: string): number[] {
  return splitList(text).map(Number);
}

/** One `key=value` per line (or per `separator`). */
export function formatEntries(entries: Record<string, string>, separator = "\n"): string {
  return Object.entries(entries)
    .map(([key, value]) => `${key}=${value}`)
    .join(separator);
}

/** Lines of `key=value`; blank lines dropped, a line without `=` is a key with an empty value. */
export function parseEntries(text: string): Record<string, string> {
  const entries: Record<string, string> = {};
  for (const line of text.split("\n")) {
    if (line.trim() === "") continue;
    const [key = "", ...value] = line.split("=");
    entries[key.trim()] = value.join("=").trim();
  }
  return entries;
}

/** An ISO 8601 instant as a `datetime-local` input shows it: local time, to the minute. */
export function toDateTimeLocal(iso: string | null): string {
  if (iso === null) return "";
  const date = new Date(iso);
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}

/** A `datetime-local` value (local time) as an ISO 8601 instant. */
export function fromDateTimeLocal(text: string): string {
  return new Date(text).toISOString();
}

export function dateTimeOrNull(text: string): string | null {
  return text === "" ? null : fromDateTimeLocal(text);
}

/** An ISO 8601 instant in the reader's locale and time zone. */
export function formatDateTime(iso: string): string {
  return new Date(iso).toLocaleString();
}

/** A choice of a reference's `<select>`: the id it sends and what it shows. */
export interface ReferenceOption {
  id: string;
  label: string;
}

/** The choices of a reference's `<select>`: none until its targets have loaded. */
export function referenceOptions<T extends { id: string }>(
  loaded: { status: "loading" } | { status: "error" } | { status: "ok"; data: { items: T[] } },
  label: (item: T) => string,
): ReferenceOption[] {
  return loaded.status === "ok"
    ? loaded.data.items.map((item) => ({ id: item.id, label: label(item) }))
    : [];
}
