import { type ReactNode, useEffect, useRef } from "react";

/**
 * Un `<dialog>` modal nativo: Escape y «Cancelar» (`textoCancelar`) llaman a `alCancelar`;
 * `abierto` es del padre. `children` va entre el mensaje y los botones (por qué falló, p. ej.).
 */
export function DialogoConfirmar({
  abierto,
  mensaje,
  textoConfirmar = "Borrar",
  textoCancelar = "Cancelar",
  alConfirmar,
  alCancelar,
  children,
}: {
  abierto: boolean;
  mensaje: string;
  textoConfirmar?: string;
  textoCancelar?: string;
  alConfirmar: () => void;
  alCancelar: () => void;
  children?: ReactNode;
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
        {children}
        <div className="modal-action">
          <button type="button" className="btn" onClick={alCancelar}>
            {textoCancelar}
          </button>
          <button type="button" className="btn btn-error" onClick={alConfirmar}>
            {textoConfirmar}
          </button>
        </div>
      </div>
    </dialog>
  );
}
