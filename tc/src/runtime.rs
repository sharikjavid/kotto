use deno_core::JsRuntime;
use crate::error::Error;

struct Runtime {
    rt: JsRuntime
}

impl Runtime {
    async fn new() -> Result<Self, Error> {
        let main_module = deno_core::resolve_path()?;
        let mut rt = JsRuntime::new(deno_core::RuntimeOptions {
            module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
            ..Default::default()
        });

        let mod_id = rt.load_main_module(&main_module, None).await?;
        let result = rt.mod_evaluate(mod_id);
        rt.run_event_loop(false).await?;
        let _ = result.await?;

        Ok(Self { rt })
    }
}