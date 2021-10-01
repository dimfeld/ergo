use anyhow::anyhow;
use deno_core::ModuleLoader;
use futures::FutureExt;

// pub mod memory;
// pub mod network;
// pub mod redis;

// pub use memory::*;

/// A module loader that doesn't actually load anything that wasn't already provided.
pub struct TrivialModuleLoader {}

impl ModuleLoader for TrivialModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _is_main: bool,
    ) -> Result<deno_core::ModuleSpecifier, deno_core::error::AnyError> {
        Ok(deno_core::resolve_import(specifier, referrer)?)
    }

    /// This just returns an error because the loader is built to assume that the source is always
    /// provided up-front to the runtime, which won't be true in the future but is for right now.
    fn load(
        &self,
        _module_specifier: &deno_core::ModuleSpecifier,
        _maybe_referrer: Option<deno_core::ModuleSpecifier>,
        _is_dyn_import: bool,
    ) -> std::pin::Pin<Box<deno_core::ModuleSourceFuture>> {
        async { Err(anyhow!("Module loading is not supported")) }.boxed_local()
    }
}
