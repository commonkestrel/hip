use proc_macro::TokenStream;
use quote::quote;
use syn::{ parse::Parse, parse_macro_input, Token, LitCStr, LitInt };

struct FixedInput {
    cstr: LitCStr,
    length: usize
}

impl Parse for FixedInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let cstr: LitCStr = input.parse()?;
        let _: Token![,] = input.parse()?;
        let len: LitInt = input.parse()?;

        let length: usize = len.base10_parse()?;

        Ok(FixedInput {cstr, length})
    }
}

#[proc_macro]
pub fn fixed_cstr(input: TokenStream) -> TokenStream {
    let FixedInput { cstr, length } = parse_macro_input!(input as FixedInput);

    let value = cstr.value();
    if value.count_bytes() != length {
        return syn::Error::new_spanned(cstr, "Input string length must be equal to length.")
            .into_compile_error()
            .into();
    }

    let bytes = value.as_bytes().into_iter();

    quote!{
        [#(#bytes,)*]
    }.into()
}
