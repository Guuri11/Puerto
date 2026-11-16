use crate::errores::ErrorSaludo;

/// Un mensaje de saludo para un nombre dado
#[derive(Debug, Clone)]
pub struct Saludo {
    pub nombre: String,
    pub mensaje: String,
}

impl Saludo {
    pub fn new(nombre: &str) -> Result<Self, ErrorSaludo> {
        if nombre.trim().is_empty() {
            return Err(ErrorSaludo::Validacion("nombre_vacio".into()));
        }

        Ok(Self {
            nombre: nombre.to_string(),
            mensaje: format!("¡Hola, {}! Saludos desde Puerto.", nombre),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deberia_crear_el_saludo_cuando_el_nombre_es_valido() {
        let resultado = Saludo::new("Mundo");
        assert!(resultado.is_ok());
        let saludo = resultado.unwrap();
        assert_eq!(saludo.nombre, "Mundo");
        assert!(saludo.mensaje.contains("Mundo"));
    }

    #[test]
    fn deberia_fallar_cuando_el_nombre_esta_vacio() {
        let resultado = Saludo::new("");
        assert!(resultado.is_err());
        assert_eq!(
            resultado.unwrap_err().to_string(),
            "saludo.validacion.nombre_vacio"
        );
    }

    #[test]
    fn deberia_fallar_cuando_el_nombre_solo_tiene_espacios() {
        let resultado = Saludo::new("   ");
        assert!(resultado.is_err());
    }
}
