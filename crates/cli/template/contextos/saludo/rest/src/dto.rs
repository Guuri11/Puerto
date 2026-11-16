use dominio_saludo::modelo::Saludo;
use poem_openapi::Object;

#[derive(Object, Debug)]
#[oai(rename_all = "camelCase")]
pub struct DtoSaludo {
    pub nombre: String,
    pub mensaje: String,
}

impl DtoSaludo {
    pub fn from_dominio(saludo: &Saludo) -> Self {
        Self {
            nombre: saludo.nombre.clone(),
            mensaje: saludo.mensaje.clone(),
        }
    }
}
