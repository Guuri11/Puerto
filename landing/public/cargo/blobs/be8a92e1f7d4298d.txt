/**
 * El mensaje bajo un input, cuando el error de la API es de ese campo (`problema.campo`). Va
 * detrás del `<label>` del input, no dentro: dentro pasaría a formar parte del nombre del input.
 */
export function ErrorCampo({
  problema,
  campo,
}: {
  problema: { campo: string | null; texto: string } | null;
  campo: string;
}) {
  if (problema?.campo !== campo) return null;
  return <span className="text-error text-sm">{problema.texto}</span>;
}
