-- Correo (ADR-0020): lo escribe puerto. Una fila por correo que un consumidor del worker envió por
-- un evento, para que un evento reentregado no lo envíe otra vez. Crece una fila por correo: borrar
-- las antiguas es cosa tuya.

CREATE TABLE correos_enviados (
    id_evento  UUID        NOT NULL,
    correo     TEXT        NOT NULL,
    enviado_en TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id_evento, correo)
);
