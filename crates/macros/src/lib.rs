//! The attribute macros of Puerto projects (ADR-0021 § 4). A project reaches them through
//! `puerto-web` — `puerto_web::require_role` — never by depending on this crate.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{FnArg, ImplItemFn, LitStr, Pat, ReturnType, Type, parse_quote};

/// `#[require_role("admin")]` on a handler that takes `auth: BearerAuth`: a caller without the role
/// is answered `403 auth.forbidden` before the body runs.
///
/// ```ignore
/// #[oai(path = "/products/:id", method = "delete")]
/// #[require_role("admin")]
/// async fn delete(&self, auth: BearerAuth, id: Path<Uuid>) -> DeleteProductResponse { … }
/// ```
///
/// The return type must have `from_status(StatusCode, Json<ErrorResponse>)`, as every generated
/// response enum does. The 403 is not documented by the macro: the enum's `Forbidden` variant is.
#[proc_macro_attribute]
pub fn require_role(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    expand(attr.into(), item.into()).into()
}

fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut handler: ImplItemFn = match syn::parse2(item.clone()) {
        Ok(handler) => handler,
        Err(err) => return err.to_compile_error(),
    };
    match role_check(attr, &handler) {
        Ok(check) => {
            let body = &handler.block;
            handler.block = parse_quote!({ #check #body });
            handler.into_token_stream()
        }
        // The method is emitted unchanged next to the error: without it, rustc also reports every
        // call to it as "no method named …", which hides the one error that matters.
        Err(err) => {
            let err = err.to_compile_error();
            quote!(#err #handler)
        }
    }
}

/// The `if` that answers 403, or why it cannot be written for this handler.
fn role_check(attr: TokenStream, handler: &ImplItemFn) -> syn::Result<TokenStream> {
    let role: LitStr = syn::parse2(attr).map_err(|err| {
        syn::Error::new(
            err.span(),
            "#[require_role] takes one role name: #[require_role(\"admin\")]",
        )
    })?;
    if !is_role_name(&role.value()) {
        return Err(syn::Error::new_spanned(
            &role,
            "#[require_role]: a role name is lowercase letters, digits, `_`, `:` or `-`, starting with a letter",
        ));
    }
    let auth = bearer_auth_param(handler)?;
    let ret = match &handler.sig.output {
        ReturnType::Type(_, ty) => ty,
        ReturnType::Default => {
            return Err(syn::Error::new_spanned(
                &handler.sig,
                "#[require_role] needs a handler returning a response enum with `from_status`",
            ));
        }
    };
    Ok(quote! {
        if !#auth.0.has_role(#role) {
            let (status, error) = ::puerto_web::auth::forbidden();
            return <#ret>::from_status(status, error);
        }
    })
}

/// The name the handler binds its `BearerAuth` parameter to.
fn bearer_auth_param(handler: &ImplItemFn) -> syn::Result<&syn::Ident> {
    for input in &handler.sig.inputs {
        let FnArg::Typed(param) = input else {
            continue;
        };
        if !is_bearer_auth(&param.ty) {
            continue;
        }
        return match &*param.pat {
            Pat::Ident(binding) => Ok(&binding.ident),
            other => Err(syn::Error::new_spanned(
                other,
                "#[require_role]: bind the `BearerAuth` parameter to a name: `auth: BearerAuth`",
            )),
        };
    }
    Err(syn::Error::new_spanned(
        &handler.sig,
        "#[require_role] needs the handler to take `auth: BearerAuth`: a role is checked on an authenticated caller",
    ))
}

fn is_bearer_auth(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "BearerAuth"),
        _ => false,
    }
}

/// `[a-z][a-z0-9_:-]*` — what `puerto validate` accepts in `roles`.
fn is_role_name(role: &str) -> bool {
    let mut chars = role.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && chars
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | ':' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expanded(attr: TokenStream, item: TokenStream) -> String {
        expand(attr, item).to_string()
    }

    #[test]
    fn should_check_the_role_before_the_body_when_the_handler_takes_bearer_auth() {
        // Act
        let output = expanded(
            quote!("admin"),
            quote! {
                async fn delete(&self, auth: BearerAuth) -> DeleteResponse {
                    DeleteResponse::NoContent
                }
            },
        );

        // Assert
        let expected = quote! {
            async fn delete(&self, auth: BearerAuth) -> DeleteResponse {
                if !auth.0.has_role("admin") {
                    let (status, error) = ::puerto_web::auth::forbidden();
                    return <DeleteResponse>::from_status(status, error);
                }
                {
                    DeleteResponse::NoContent
                }
            }
        };
        assert_eq!(output, expected.to_string());
    }

    #[test]
    fn should_find_bearer_auth_by_its_last_segment_when_the_path_is_qualified() {
        // Act
        let output = expanded(
            quote!("admin"),
            quote! {
                async fn delete(&self, id: Path<Uuid>, caller: puerto_web::auth::BearerAuth) -> R { R::Ok }
            },
        );

        // Assert
        assert!(output.contains("caller . 0 . has_role"), "{output}");
        assert!(!output.contains("compile_error"), "{output}");
    }

    #[test]
    fn should_fail_naming_the_attribute_when_the_handler_takes_no_bearer_auth() {
        // Act
        let output = expanded(
            quote!("admin"),
            quote! { async fn delete(&self, id: Path<Uuid>) -> R { R::Ok } },
        );

        // Assert
        assert!(output.contains("compile_error"), "{output}");
        assert!(
            output.contains("needs the handler to take `auth: BearerAuth`"),
            "{output}"
        );
    }

    #[test]
    fn should_still_emit_the_method_when_the_attribute_is_wrong() {
        // Act
        let output = expanded(
            quote!("admin"),
            quote! { async fn delete(&self) -> R { R::Ok } },
        );

        // Assert
        assert!(output.contains("async fn delete"), "{output}");
        assert!(!output.contains("has_role"), "{output}");
    }

    #[test]
    fn should_fail_when_the_role_is_not_a_role_name() {
        for attr in [
            quote!("Admin"),
            quote!(""),
            quote!("1admin"),
            quote!("ad min"),
        ] {
            // Act
            let output = expanded(
                attr.clone(),
                quote! { async fn delete(&self, auth: BearerAuth) -> R { R::Ok } },
            );

            // Assert
            assert!(
                output.contains("a role name is lowercase"),
                "{attr}: {output}"
            );
        }
    }

    #[test]
    fn should_fail_when_the_attribute_is_not_one_string() {
        for attr in [quote!(), quote!(admin), quote!("admin", "editor")] {
            // Act
            let output = expanded(
                attr.clone(),
                quote! { async fn delete(&self, auth: BearerAuth) -> R { R::Ok } },
            );

            // Assert
            assert!(output.contains("takes one role name"), "{attr}: {output}");
        }
    }

    #[test]
    fn should_fail_when_bearer_auth_is_destructured() {
        // Act
        let output = expanded(
            quote!("admin"),
            quote! { async fn delete(&self, BearerAuth(p): BearerAuth) -> R { R::Ok } },
        );

        // Assert
        assert!(
            output.contains("bind the `BearerAuth` parameter to a name"),
            "{output}"
        );
    }

    #[test]
    fn should_fail_when_the_handler_returns_nothing() {
        // Act
        let output = expanded(
            quote!("admin"),
            quote! { async fn delete(&self, auth: BearerAuth) {} },
        );

        // Assert
        assert!(output.contains("needs a handler returning"), "{output}");
    }

    #[test]
    fn should_accept_role_names_with_digits_and_separators() {
        for role in ["admin", "a", "billing:read", "team-lead", "level_2"] {
            assert!(is_role_name(role), "{role}");
        }
    }
}
