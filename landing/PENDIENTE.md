# Pendiente — web de Puerto

**Esta es la lista.** Si algo está pendiente y no está aquí, no existe.

**Última revisión:** 2026-10-13
**Estado:** **en producción** desde el 2026-10-13 (`1904402`), en `puerto-rs.vercel.app`. 216 E2E
en verde, `astro check` limpio. Auditoría de producción: 83/100 (B), frente a 58 (F) del baseline.

---

## 🔴 Bloquea el lanzamiento

Nada. Desplegado por la integración de Git de Vercel (push a `main`), y el build lee bien `..`
(versión, ADRs y changelog salen correctos en producción). Auditoría en
`docs/auditoria-lanzamiento.md`.

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
