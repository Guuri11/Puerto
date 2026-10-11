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
