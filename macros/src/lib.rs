mod enum_macro;
mod struct_macro;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn stuff(args: TokenStream, input: TokenStream) -> TokenStream {
    //applies to either a struct, or an enum
    // if enum, we generate a field that can be put in the struct.
    // for the struct, the valid field are the primitive unsigned ints
    // a bool (if 1 bit), or an enum
    if let Ok(input) = syn::parse(input.clone()) {
        struct_macro::process(args, input)
    } else if let Ok(input) = syn::parse(input) {
        enum_macro::process(args, input)
    } else {
        panic!("not enum or struct")
    }
}
