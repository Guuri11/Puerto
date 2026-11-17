import { useEffect, useRef } from "react";

/** Un `<dialog>` modal nativo: Escape y «Cancelar» llaman a `alCancelar`; `abierto` es del padre. */
export function DialogoConfirmar({
  abierto,
  mensaje,
  textoConfirmar = "Borrar",
  alConfirmar,
  alCancelar,
}: {
  abierto: boolean;
  mensaje: string;
  textoConfirmar?: string;
  alConfirmar: () => void;
  alCancelar: () => void;
}) {
  const dialogo = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const elemento = dialogo.current;
    if (elemento === null) return;
    if (abierto && !elemento.open) elemento.showModal();
    if (!abierto && elemento.open) elemento.close();
  }, [abierto]);

  return (
    <dialog
      ref={dialogo}
      className="modal"
      onCancel={(event) => {
        event.preventDefault();
        alCancelar();
      }}
    >
      <div className="modal-box">
        <p>{mensaje}</p>
        <div className="modal-action">
          <button type="button" className="btn" onClick={alCancelar}>
            Cancelar
          </button>
          <button type="button" className="btn btn-error" onClick={alConfirmar}>
            {textoConfirmar}
          </button>
        </div>
      </div>
    </dialog>
  );
}
