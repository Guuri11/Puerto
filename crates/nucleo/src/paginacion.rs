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
}
