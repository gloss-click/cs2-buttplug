use std::{
    env::current_exe,
    fs::{metadata, write},
    path::PathBuf,
    time::SystemTime,
};

use anyhow::{Context, Error};
use csgo_gsi::update::{Update, CSGOPackage};
use fehler::throws;
use rhai::{AST, Engine, packages::Package, Scope, RegisterFn};
use tokio::sync::broadcast;
use crate::timer_thread::ScriptCommand;

pub struct ScriptHost {
    engine: Engine,
    scope: Scope<'static>,
    ast: AST,
    script_path: PathBuf,
    last_modified: SystemTime,
}

impl ScriptHost {
    #[throws]
    pub fn new(send: broadcast::Sender<ScriptCommand>) -> Self {
        let vsend = send.clone();
        let ssend = send.clone();
        let mut engine = Engine::new();
        
        engine.set_max_call_levels(200);
        engine.set_max_expr_depths(200, 200);
        engine.set_max_operations(200);

        engine.load_package(CSGOPackage::new().get());
        engine.register_fn("vibrate", move |speed: f64, time: f64| {
            let result = vsend.send(ScriptCommand::VibrateFor(speed, time));
            if let Err(err) = result {
                error!("Error sending command from script to buttplug: {}", err);
            }
        });
        engine.register_fn("stop", move || {
            let result = ssend.send(ScriptCommand::Stop);
            if let Err(err) = result {
                error!("Error sending command from script to buttplug: {}", err);
            }
        });
        let mut scope = Scope::new();

        let exe_path = current_exe().context("couldn't get path of Crotch-Stim: Get Off executable")?;
        let script_path = exe_path.with_extension("rhai");

        if !script_path.exists() {
            info!("Creating default script {} with default settings, go look over that script and then come back and press Enter", script_path.display());
            write(&script_path, include_str!("default_script.rhai")).context("couldn't save default config file")?;
        }

        let mtime = metadata(&script_path)
            .and_then(|x| x.modified())
            .context("couldn't check modification time of script")?;

        let ast = engine.compile_file_with_scope(&mut scope, script_path.clone())
            .map_err(|err| {
                error!("{}", err);
                anyhow::anyhow!("{}", err)
            })?;
        // if there's some global state or on-boot handling, make sure it runs
        engine.consume_ast_with_scope(&mut scope, &ast)
            .map_err(|err| anyhow::anyhow!("{}", err))?;
        Self {
            engine,
            scope,
            ast,
            script_path,
            last_modified: mtime,
        }
    }

    pub fn handle_update(&mut self, update: &Update) {
        let needs_rebuild = metadata(&self.script_path)
            .and_then(|x| x.modified())
            .map(|mtime| (mtime > self.last_modified, mtime));
        if let Ok((true, mtime)) = needs_rebuild {
            info!("Noticed live script change, rebuilding...");
            let compile_result = self.engine.compile_file_with_scope(&mut self.scope, self.script_path.clone())
                .and_then(|ast| {
                    self.engine.consume_ast_with_scope(&mut self.scope, &ast)?;
                    self.ast = ast;
                    Ok(())
                });
            if let Err(e) = compile_result {
                error!("Error when rebuilding script: {}", e);
            }
            self.last_modified = mtime;
        }
        let result = self.engine.call_fn::<(Update,), ()>(&mut self.scope, &mut self.ast, "handle_update", (update.clone(),));
        if let Err(e) = result {
            error!("Error when handling update: {}", e);
        };
    }
}
