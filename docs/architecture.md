Bowbend is made of 3 distinct layers.  The base layer is a pure rust, async crate called bowbend_core.  The vast 
majority of the crates logic is here.  It exposes an idiomatic, high level rust API.  This API should not be 
considered stable and this crate is not published on crates.io.  

It's only consumer is the ffi crate, which of the project's 3 layers.  This crate has only one purpose.  It consumes 
our unstable, internal rust API and exposes a C interface.  We use the [safer_ffi](https://github.com/getditto/safer_ffi) 
crate to remove some drudgery, boilerplate and risk.  Some being the key word.  This crate 
contains a lot of unsafe code and does very little more than shuffling data of one type to a related structure of 
another type.  The goal is to keep as much business logic out of this crate as possible.  `safer-ffi` automated the 
process of producing header files this C API.  

This is consumed by the 3rd and final layer, the language specific SDKs.  This is the only part of the project that is 
planned to have a stable API.  The most common route here will be to wrap the FFI crate with native, idiomatic, stable 
bindings that users can consume.  The one exception that exists today is the Rusk SDK.  It would end up re-exporting a
large amount of code from bowbend_core. This separation still exists in so lower layers can be free to break APIs in 
and add compatibility shims to the Rust SDKs as needed.  A similar approach over the ffi crate might be used if a C 
SDK is ever added.