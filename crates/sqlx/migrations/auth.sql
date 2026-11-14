-- Cuentas locales (ADR-0021 § 2): lo escribe puerto, después de `usuarios`. El hash de la
-- contraseña vive aparte del usuario, así que `usuarios` no guarda ningún secreto. Una sesión
-- guarda el SHA-256 de su token, nunca el token: una copia de esta tabla no abre ninguna sesión.
-- Las dos se van con su usuario. Las sesiones caducadas se rechazan, no se borran: limpiarlas
-- (`DELETE FROM sesiones WHERE expira_en < now()`) es cosa tuya.

CREATE TABLE credenciales (
    id_usuario      UUID        PRIMARY KEY REFERENCES usuarios (id) ON DELETE CASCADE,
    hash_contrasena TEXT        NOT NULL,
    actualizado_en  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sesiones (
    hash_token BYTEA       PRIMARY KEY,
    id_usuario UUID        NOT NULL REFERENCES usuarios (id) ON DELETE CASCADE,
    creado_en  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expira_en  TIMESTAMPTZ NOT NULL
);

CREATE INDEX sesiones_id_usuario_idx ON sesiones (id_usuario);
