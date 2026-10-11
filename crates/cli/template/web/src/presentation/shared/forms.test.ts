import { describe, expect, test } from "bun:test";
import {
  booleanOrNull,
  dateTimeOrNull,
  formatEntries,
  fromDateTimeLocal,
  numberOrNull,
  parseEntries,
  splitList,
  splitNumbers,
  textOf,
  textOrNull,
  toDateTimeLocal,
} from "./forms";

describe("forms", () => {
  test("should leave an optional input empty when the value is null", () => {
    expect(textOf(null)).toBe("");
    expect(textOf(42)).toBe("42");
    expect(textOf(false)).toBe("false");
  });

  test("should read an empty optional input as null", () => {
    expect(textOrNull("")).toBeNull();
    expect(textOrNull("Alice")).toBe("Alice");
    expect(numberOrNull("")).toBeNull();
    expect(numberOrNull("1.5")).toBe(1.5);
    expect(booleanOrNull("")).toBeNull();
    expect(booleanOrNull("true")).toBe(true);
    expect(booleanOrNull("false")).toBe(false);
  });

  test("should split a comma-separated list and drop its empty items", () => {
    expect(splitList(" red, green ,, blue ")).toEqual(["red", "green", "blue"]);
    expect(splitList("")).toEqual([]);
    expect(splitNumbers("1, 2")).toEqual([1, 2]);
  });

  test("should write a map one entry per line and read it back", () => {
    const entries = { color: "red", query: "a=b" };

    const text = formatEntries(entries);

    expect(text).toBe("color=red\nquery=a=b");
    expect(parseEntries(`${text}\n\n`)).toEqual(entries);
    expect(formatEntries(entries, ", ")).toBe("color=red, query=a=b");
  });

  test("should round-trip an instant through a datetime-local input", () => {
    const iso = "2026-01-01T10:00:00.000Z";

    const text = toDateTimeLocal(iso);

    expect(text).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/);
    expect(fromDateTimeLocal(text)).toBe(iso);
    expect(toDateTimeLocal(null)).toBe("");
    expect(dateTimeOrNull("")).toBeNull();
  });
});
