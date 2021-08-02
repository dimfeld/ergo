use clru::CLruCache;
use deno_core::ModuleLoader;

pub struct MemoryModuleCache {
    cache: CLruCache<String, String>,
    fallback: Box<dyn ModuleLoader>,
}

impl ModuleLoader for MemoryModuleCache {
    fn resolve(
        &self,
        op_state: std::rc::Rc<std::cell::RefCell<deno_core::OpState>>,
        specifier: &str,
        referrer: &str,
        _is_main: bool,
    ) -> Result<deno_core::ModuleSpecifier, deno_core::error::AnyError> {
        todo!()
    }

    fn load(
        &self,
        op_state: std::rc::Rc<std::cell::RefCell<deno_core::OpState>>,
        module_specifier: &deno_core::ModuleSpecifier,
        maybe_referrer: Option<deno_core::ModuleSpecifier>,
        is_dyn_import: bool,
    ) -> std::pin::Pin<Box<deno_core::ModuleSourceFuture>> {
        todo!()
    }
}
