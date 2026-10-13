# Pendiente — web de Puerto

**Esta es la lista.** Si algo está pendiente y no está aquí, no existe.

**Última revisión:** 2026-10-13
**Estado:** construida y en verde (216 E2E, `astro check` limpio). **Sin desplegar**: sustituye a
la landing de abril en `puerto-rs.vercel.app` en cuanto se haga commit y push.

---

## 🔴 Bloquea el lanzamiento

- [ ] **Commit y despliegue.** Nada de esto está commiteado todavía. Al desplegar, confirmar en
      Vercel que el proyecto tiene *Root Directory* = `landing` y que «Include files outside the
      root directory» está activo: el build lee `../Cargo.toml`, `../docs/adr` y `../CHANGELOG.md`.
- [ ] **Auditar producción** tras el deploy: `squirrel audit https://puerto-rs.vercel.app
      --coverage surface` y comparar con `docs/auditoria-inicial.md`.

## 🟡 Decide Sergio

- [x] ~~Dominio propio~~ — decidido el 2026-10-13: **se queda en `puerto-rs.vercel.app`**.
- [ ] **Search Console** para `puerto-rs.vercel.app` (verificación por meta tag o fichero HTML,
      que en un subdominio de Vercel no hay DNS que tocar).
- [ ] ¿Una página «About» / autor? squirrel la pide (E-E-A-T 52). Para un proyecto open source
      basta con el autor en el grafo y en el pie; se añadirá si el posicionamiento lo pide.

## 🔵 Técnico

- [x] Marca del README rehecha (2026-10-13): `pnpm brand:repo` genera `../assets/brand/`.
- [ ] **Subir `assets/brand/github-social.png`** en GitHub → *Settings → Social preview*. Es manual:
      GitHub no la lee del repositorio.
- [ ] El banner ASCII de arranque (`puerto-web`) ya dibuja tres pilas de contenedores: encaja con
      la marca nueva, no hay que tocarlo.
- [ ] Servir las docs en Markdown (`Accept: text/markdown` o `.md`) si se quiere subir «Agents».
- [ ] La terminal podría enlazarse desde las guías con su estado en la URL
      (`/?cargo=order&events=db#try`). Ya funciona; falta usarlo en las docs.
