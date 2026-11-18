# 0026 — Los casos de uso de escritura: la entidad emite, la factoría guarda, el span registra

**Estado:** Aceptada (2026-11-18). Especificación: `PLAN.md` § Fase 24. Amplía la ADR-0018 § 2
(la unidad de trabajo) y sustituye en la ADR-0006 la regla «un caso de uso registra `inicio` y
`fin`».

## Contexto

Un `crear_<e>` generado en un proyecto con eventos tenía unas 14 líneas en su cuerpo y 2 de negocio.
Las demás eran fontanería:

- **Logs a mano:** `inicio`, `fin`, un cierre `fallido` encadenado con `inspect_err` a cada llamada
  y un `ok_or_else` de 5 líneas solo para registrar un `warn`. Ya divergían (`crear` registraba
  `fin`, `actualizar` y `borrar` no) y el demo registraba un dato de la petición. Además eran
  redundantes: los adaptadores ya registran cada fallo con la causa del driver.
- **La unidad de trabajo a mano:** `begin`, `guardar`, `registrar`, `commit` en cada escritura.
- **El evento lo decidía la aplicación:** el caso de uso construía `EventoProducto::Creado(…)`, y
  `borrar_<e>` ponía `borrado = true` él mismo — lógica de dominio fuera del dominio.

Referencias consultadas:

- **eShop** (la referencia de Microsoft): la entidad acumula sus eventos (`AddDomainEvent`) y
  `UnitOfWork.SaveEntitiesAsync()` los despacha y guarda; los logs y la transacción van en
  *pipeline behaviors* de MediatR.
- **Wolverine**: el handler *devuelve* los eventos (el patrón Decider) y un middleware carga el
  agregado, los añade y confirma.
- **`cqrs-es`** (Rust): `Aggregate::handle` emite eventos a un sumidero y el framework persiste.

Wolverine y `cqrs-es` hacen event sourcing; Puerto guarda el estado y un outbox (ADR-0018), como
eShop.

## Decisión

- **La entidad emite sus eventos y los devuelve.** `Producto::crear(props) -> Result<(Producto,
  EventoProducto), _>`, `actualizar(&mut self, props) -> Result<EventoProducto, _>`,
  `borrar(&mut self) -> EventoProducto`. Es el Decider de Wolverine adaptado a un agregado con
  estado: el hecho de negocio lo decide el dominio, donde se prueba sin dobles. No una lista
  interna como en eShop: un campo privado rompería los literales de `from_repositorio`, de
  `TryFrom<Fila…>` y de la madre, y sería estado oculto que vaciar.
- **`#[must_use]` en `Evento<E>`.** Un evento emitido que nadie guarda es un aviso, y el
  `make lint` del proyecto lo rechaza. Es la garantía que eShop no tiene, y se suma a la de la
  ADR-0018 (el repositorio no escribe, así que una escritura sin outbox no compila).
- **`UnitOfWorkFactory<E>::guardar(&entidad, evento)`**, un método por defecto: `begin`,
  `guardar`, `registrar`, `commit`. Es el `save` del repositorio de Vernon y el `SaveEntitiesAsync`
  de eShop; el evento va en la firma, así que el nombre no lo repite. `begin`/`commit` siguen para
  escribir más de una vez.
- **Los logs, con `#[tracing::instrument(name = "<caso>", skip_all, err(level = "warn"))]`.** El
  span da el nombre del caso de uso a los logs del adaptador; `err` registra una vez el código del
  error; `skip_all` no registra la petición. Un caso de uso que acaba bien no escribe nada: poem ya
  registra la respuesta con su estado y su duración.
- **Sin bus ni middleware.** Un bus genérico necesita borrar tipos o un `CasoDeUso<P, R, E>` con
  tipos asociados, y cambiaría `Arc<dyn CrearProducto>` y `MockCrearProducto` en las rutas, el
  cableado y los tests. La transacción tampoco se puede envolver de forma genérica: la unidad de
  trabajo tiene un tipo por entidad. Y Puerto ya es el generador: lo que el middleware de Wolverine
  genera en tiempo de ejecución, aquí queda escrito en el repositorio, a la vista.

## Alternativas rechazadas

- **La lista de eventos en la entidad (eShop).** Ver arriba.
- **Un bus de comandos con *behaviors* (MediatR).** Ver arriba.
- **Despachar eventos en el mismo proceso antes del commit** (la opción A de eShop). Deja que un
  handler modifique otro agregado en la misma transacción, contra la regla de un agregado por
  transacción que ya impone la unidad de trabajo por entidad. Entre agregados, consistencia
  eventual por el outbox.
- **`FmtSpan::CLOSE` en `puerto-web`**, para una línea de `fin` con la duración. Duplicaría la
  línea de respuesta de poem.

## Consecuencias

- Los casos de uso de escritura generados se quedan en el negocio: cargar, decidir, guardar.
- Cambia lo que genera `scaffold` (el dominio, la aplicación, los snippets y los `AGENTS.md`);
  no cambia ninguna ruta, crate ni puerto público.
- El usuario que escribe un evento propio añade un método a la entidad que lo devuelve, y lo
  guarda con `uow_factory.guardar`.
- Las proyecciones y las sagas se apoyan en esto sin cambiarlo: son consumidores.
