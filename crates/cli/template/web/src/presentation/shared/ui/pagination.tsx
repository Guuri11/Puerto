/** "1–20 of 45" with Previous / Next. Renders nothing when the whole list fits on one page. */
export function Pagination({
  total,
  limit,
  offset,
  onOffsetChange,
}: {
  total: number;
  limit: number;
  offset: number;
  onOffsetChange: (offset: number) => void;
}) {
  if (offset === 0 && total <= limit) return null;
  const last = Math.min(offset + limit, total);

  return (
    <nav className="flex items-center justify-between" aria-label="Pagination">
      <span className="text-sm">
        {offset + 1}–{last} of {total}
      </span>
      <div className="join">
        <button
          type="button"
          className="join-item btn btn-sm"
          disabled={offset === 0}
          onClick={() => onOffsetChange(Math.max(offset - limit, 0))}
        >
          Previous
        </button>
        <button
          type="button"
          className="join-item btn btn-sm"
          disabled={last >= total}
          onClick={() => onOffsetChange(offset + limit)}
        >
          Next
        </button>
      </div>
    </nav>
  );
}

/** The offset the URL's `?page=` names (1-based); the first page when there is none. */
export function offsetFromSearch(search: string, limit: number): number {
  const page = Number(new URLSearchParams(search).get("page"));
  return Number.isInteger(page) && page > 1 ? (page - 1) * limit : 0;
}

/** The query string for the page at `offset`: none for the first. */
export function pageSearch(offset: number, limit: number): string {
  const page = Math.floor(offset / limit) + 1;
  return page > 1 ? `?page=${page}` : "";
}
