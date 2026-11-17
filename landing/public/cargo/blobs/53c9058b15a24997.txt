import type { ReactNode } from "react";

export function SinElementos({ mensaje, children }: { mensaje: string; children?: ReactNode }) {
  return (
    <div className="card bg-base-100">
      <div className="card-body items-center text-center">
        <p>{mensaje}</p>
        {children}
      </div>
    </div>
  );
}
