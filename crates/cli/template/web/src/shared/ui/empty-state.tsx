import type { ReactNode } from "react";

export function EmptyState({ message, children }: { message: string; children?: ReactNode }) {
  return (
    <div className="card bg-base-100">
      <div className="card-body items-center text-center">
        <p>{message}</p>
        {children}
      </div>
    </div>
  );
}
