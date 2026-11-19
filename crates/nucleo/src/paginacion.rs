/// The page size when the caller does not choose one.
pub const LIMITE_PAGINA_POR_DEFECTO: u32 = 20;
/// The largest page anyone can ask for: no list reads an unbounded table.
pub const LIMITE_PAGINA_MAXIMO: u32 = 100;

/// Which slice of a list to read. The fields are private, so every request goes through `new`,
/// which applies the cap — no route, use case or adapter can ask for more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeticionPagina {
    limite: u32,
    desplazamiento: u32,
}

impl PeticionPagina {
    /// `limite` defaults to [`LIMITE_PAGINA_POR_DEFECTO`] and is clamped to `1..=LIMITE_PAGINA_MAXIMO`;
    /// `desplazamiento` defaults to 0.
    pub fn new(limite: Option<u32>, desplazamiento: Option<u32>) -> Self {
        Self {
            limite: limite
                .unwrap_or(LIMITE_PAGINA_POR_DEFECTO)
                .clamp(1, LIMITE_PAGINA_MAXIMO),
            desplazamiento: desplazamiento.unwrap_or(0),
        }
    }

    pub fn limite(&self) -> u32 {
        self.limite
    }

    pub fn desplazamiento(&self) -> u32 {
        self.desplazamiento
    }
}

impl Default for PeticionPagina {
    fn default() -> Self {
        Self::new(None, None)
    }
}

/// One page of a list, and how many items the whole list has.
#[derive(Debug, Clone, PartialEq)]
pub struct Pagina<T> {
    pub elementos: Vec<T>,
    pub total: u64,
    /// The limit actually applied — after the cap.
    pub limite: u32,
    pub desplazamiento: u32,
}

impl<T> Pagina<T> {
    pub fn new(elementos: Vec<T>, total: u64, request: PeticionPagina) -> Self {
        Self {
            elementos,
            total,
            limite: request.limite(),
            desplazamiento: request.desplazamiento(),
        }
    }

    /// The same page, with every item converted (a DTO, a view model).
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Pagina<U> {
        Pagina {
            elementos: self.elementos.into_iter().map(f).collect(),
            total: self.total,
            limite: self.limite,
            desplazamiento: self.desplazamiento,
        }
    }
}

/// Every item of a list: `leer` once per page of [`LIMITE_PAGINA_MAXIMO`], from the start, until a
/// page comes back short or `total` items are read; the first error stops it. For code no caller
/// pages for — a consumer of the worker, a job:
///
/// `todas_las_paginas(|pagina| self.listar.ejecutar(ParamsListar… { pagina, … }))`.
///
/// A plain closure returning the future, not an `async` closure: rustc cannot yet prove the latter
/// `Send` inside an `async_trait` method such as `ManejadorEvento::manejar` (higher-ranked
/// lifetimes), and this one is `Send` when its future is.
pub async fn todas_las_paginas<T, E, F, Fut>(mut leer: F) -> Result<Vec<T>, E>
where
    F: FnMut(PeticionPagina) -> Fut,
    Fut: std::future::Future<Output = Result<Pagina<T>, E>>,
{
    let mut elementos = Vec::new();
    loop {
        // shortcut: a list past u32::MAX items stays on its last offset; none fits in memory anyway.
        let desplazamiento = u32::try_from(elementos.len()).unwrap_or(u32::MAX);
        let pagina = leer(PeticionPagina::new(
            Some(LIMITE_PAGINA_MAXIMO),
            Some(desplazamiento),
        ))
        .await?;
        let corta = pagina.elementos.len() < pagina.limite as usize;
        elementos.extend(pagina.elementos);
        if corta || elementos.len() as u64 >= pagina.total {
            return Ok(elementos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_take_the_defaults_when_nothing_is_asked() {
        let page = PeticionPagina::new(None, None);
        assert_eq!(page.limite(), LIMITE_PAGINA_POR_DEFECTO);
        assert_eq!(page.desplazamiento(), 0);
    }

    #[test]
    fn should_cap_the_limit_at_the_maximum() {
        let page = PeticionPagina::new(Some(LIMITE_PAGINA_MAXIMO + 1), Some(40));
        assert_eq!(page.limite(), LIMITE_PAGINA_MAXIMO);
        assert_eq!(page.desplazamiento(), 40);
    }

    #[test]
    fn should_raise_a_zero_limit_to_one() {
        assert_eq!(PeticionPagina::new(Some(0), None).limite(), 1);
    }

    #[test]
    fn should_keep_the_applied_limit_and_offset_in_the_page() {
        let page = Pagina::new(vec!["a", "b"], 7, PeticionPagina::new(Some(2), Some(4)));
        let mapped = page.map(str::len);
        assert_eq!(
            mapped,
            Pagina {
                elementos: vec![1, 1],
                total: 7,
                limite: 2,
                desplazamiento: 4
            }
        );
    }

    /// A list of `total` numbers, read through `todas_las_paginas`: the requests it got, and what
    /// it answered.
    async fn leer_todo(total: u32) -> (Vec<PeticionPagina>, Result<Vec<u32>, String>) {
        let mut pedidas = vec![];
        let leido = todas_las_paginas(|pagina: PeticionPagina| {
            pedidas.push(pagina);
            let desde = pagina.desplazamiento().min(total);
            let hasta = (desde + pagina.limite()).min(total);
            let elementos = (desde..hasta).collect();
            std::future::ready(Ok::<_, String>(Pagina::new(
                elementos,
                u64::from(total),
                pagina,
            )))
        })
        .await;
        (pedidas, leido)
    }

    fn desplazamientos(pedidas: &[PeticionPagina]) -> Vec<u32> {
        pedidas.iter().map(PeticionPagina::desplazamiento).collect()
    }

    #[tokio::test]
    async fn should_ask_once_when_the_list_is_empty() {
        let (pedidas, leido) = leer_todo(0).await;

        assert_eq!(leido, Ok(vec![]));
        assert_eq!(desplazamientos(&pedidas), [0]);
    }

    #[tokio::test]
    async fn should_ask_once_for_a_list_that_fits_in_one_page() {
        let (pedidas, leido) = leer_todo(7).await;

        assert_eq!(leido, Ok((0..7).collect()));
        assert_eq!(desplazamientos(&pedidas), [0]);
        assert!(pedidas.iter().all(|p| p.limite() == LIMITE_PAGINA_MAXIMO));
    }

    #[tokio::test]
    async fn should_read_every_page_in_order_when_the_list_is_longer_than_one() {
        let (pedidas, leido) = leer_todo(250).await;

        assert_eq!(leido, Ok((0..250).collect()));
        assert_eq!(desplazamientos(&pedidas), [0, 100, 200]);
    }

    #[tokio::test]
    async fn should_stop_at_the_total_when_the_last_page_is_full() {
        let (pedidas, leido) = leer_todo(200).await;

        assert_eq!(leido.map(|e| e.len()), Ok(200));
        assert_eq!(desplazamientos(&pedidas), [0, 100]);
    }

    #[tokio::test]
    async fn should_stop_at_a_short_page_when_the_list_shrinks_while_it_is_read() {
        // The total says 300, but the second page comes back short: someone deleted rows.
        let mut pedidas = 0;
        let leido = todas_las_paginas(|pagina: PeticionPagina| {
            pedidas += 1;
            let elementos = if pagina.desplazamiento() == 0 {
                vec![0; 100]
            } else {
                vec![1; 40]
            };
            std::future::ready(Ok::<_, String>(Pagina::new(elementos, 300, pagina)))
        })
        .await;

        assert_eq!(leido.map(|e| e.len()), Ok(140));
        assert_eq!(pedidas, 2);
    }

    #[tokio::test]
    async fn should_answer_the_first_error_and_ask_no_more() {
        let mut pedidas = 0;
        let leido: Result<Vec<u32>, &str> = todas_las_paginas(|pagina: PeticionPagina| {
            pedidas += 1;
            std::future::ready(match pagina.desplazamiento() {
                0 => Ok(Pagina::new(vec![7; 100], 300, pagina)),
                _ => Err("repositorio"),
            })
        })
        .await;

        assert_eq!(leido, Err("repositorio"));
        assert_eq!(pedidas, 2);
    }

    /// What a consumer of the worker does: an `async_trait` method (its future must be `Send`)
    /// reading through a port it holds.
    #[tokio::test]
    async fn should_be_send_when_it_reads_through_a_port_from_an_async_trait() {
        #[async_trait::async_trait]
        trait Listar: Send + Sync {
            async fn ejecutar(&self, pagina: PeticionPagina) -> Result<Pagina<u32>, String>;
        }
        struct Doce;
        #[async_trait::async_trait]
        impl Listar for Doce {
            async fn ejecutar(&self, pagina: PeticionPagina) -> Result<Pagina<u32>, String> {
                Ok(Pagina::new((0..12).collect(), 12, pagina))
            }
        }
        struct Consumidor {
            listar: std::sync::Arc<dyn Listar>,
        }
        #[async_trait::async_trait]
        trait Manejar {
            async fn manejar(&self) -> usize;
        }
        #[async_trait::async_trait]
        impl Manejar for Consumidor {
            async fn manejar(&self) -> usize {
                todas_las_paginas(|pagina| self.listar.ejecutar(pagina))
                    .await
                    .map_or(0, |e| e.len())
            }
        }

        let consumidor = Consumidor {
            listar: std::sync::Arc::new(Doce),
        };

        assert_eq!(consumidor.manejar().await, 12);
    }
}
