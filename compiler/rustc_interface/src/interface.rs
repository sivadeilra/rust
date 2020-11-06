pub use crate::passes::BoxedResolver;
use crate::util;

use rustc_ast as ast;
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_data_structures::fx::FxHashMap;
use rustc_data_structures::sync::Lrc;
use rustc_data_structures::OnDrop;
use rustc_errors::registry::Registry;
use rustc_errors::{ErrorReported, Handler};
use rustc_lint::LintStore;
use rustc_middle::ty;
use rustc_session::config::{self, Input, OutputFilenames};
use rustc_session::lint;
use rustc_session::{DiagnosticOutput, Session};
use rustc_span::source_map::FileLoader;
use std::path::PathBuf;
use std::result;
use std::sync::{Arc, Mutex};

pub type Result<T> = result::Result<T, ErrorReported>;
pub use crate::cfg::parse_cfgspecs;

/// Represents a compiler session.
///
/// Can be used to run `rustc_interface` queries.
/// Created by passing [`Config`] to [`run_compiler`].
pub struct Compiler {
    pub(crate) sess: Lrc<Session>,
    codegen_backend: Lrc<Box<dyn CodegenBackend>>,
    pub(crate) input: Input,
    pub(crate) input_path: Option<PathBuf>,
    pub(crate) output_dir: Option<PathBuf>,
    pub(crate) output_file: Option<PathBuf>,
    pub(crate) register_lints: Option<Box<dyn Fn(&Session, &mut LintStore) + Send + Sync>>,
    pub(crate) override_queries:
        Option<fn(&Session, &mut ty::query::Providers, &mut ty::query::Providers)>,
}

impl Compiler {
    pub fn session(&self) -> &Lrc<Session> {
        &self.sess
    }
    pub fn codegen_backend(&self) -> &Lrc<Box<dyn CodegenBackend>> {
        &self.codegen_backend
    }
    pub fn input(&self) -> &Input {
        &self.input
    }
    pub fn output_dir(&self) -> &Option<PathBuf> {
        &self.output_dir
    }
    pub fn output_file(&self) -> &Option<PathBuf> {
        &self.output_file
    }
    pub fn register_lints(&self) -> &Option<Box<dyn Fn(&Session, &mut LintStore) + Send + Sync>> {
        &self.register_lints
    }
    pub fn build_output_filenames(
        &self,
        sess: &Session,
        attrs: &[ast::Attribute],
    ) -> OutputFilenames {
        util::build_output_filenames(
            &self.input,
            &self.output_dir,
            &self.output_file,
            &attrs,
            &sess,
        )
    }
}

/// The compiler configuration
pub struct Config {
    /// Command line options
    pub opts: config::Options,

    /// cfg! configuration in addition to the default ones
    pub crate_cfg: crate::cfg::CrateCfg,

    pub input: Input,
    pub input_path: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub output_file: Option<PathBuf>,
    pub file_loader: Option<Box<dyn FileLoader + Send + Sync>>,
    pub diagnostic_output: DiagnosticOutput,

    /// Set to capture stderr output during compiler execution
    pub stderr: Option<Arc<Mutex<Vec<u8>>>>,

    pub lint_caps: FxHashMap<lint::LintId, lint::Level>,

    /// This is a callback from the driver that is called when we're registering lints;
    /// it is called during plugin registration when we have the LintStore in a non-shared state.
    ///
    /// Note that if you find a Some here you probably want to call that function in the new
    /// function being registered.
    pub register_lints: Option<Box<dyn Fn(&Session, &mut LintStore) + Send + Sync>>,

    /// This is a callback from the driver that is called just after we have populated
    /// the list of queries.
    ///
    /// The second parameter is local providers and the third parameter is external providers.
    pub override_queries:
        Option<fn(&Session, &mut ty::query::Providers, &mut ty::query::Providers)>,

    /// This is a callback from the driver that is called to create a codegen backend.
    pub make_codegen_backend:
        Option<Box<dyn FnOnce(&config::Options) -> Box<dyn CodegenBackend> + Send>>,

    /// Registry of diagnostics codes.
    pub registry: Registry,
}

pub fn create_compiler_and_run<R>(config: Config, f: impl FnOnce(&Compiler) -> R) -> R {
    let registry = &config.registry;
    let (sess, codegen_backend) = util::create_session(
        config.opts,
        config.crate_cfg,
        config.diagnostic_output,
        config.file_loader,
        config.input_path.clone(),
        config.lint_caps,
        config.make_codegen_backend,
        registry.clone(),
    );

    let compiler = Compiler {
        sess,
        codegen_backend,
        input: config.input,
        input_path: config.input_path,
        output_dir: config.output_dir,
        output_file: config.output_file,
        register_lints: config.register_lints,
        override_queries: config.override_queries,
    };

    rustc_span::with_source_map(compiler.sess.parse_sess.clone_source_map(), move || {
        let r = {
            let _sess_abort_error = OnDrop(|| {
                compiler.sess.finish_diagnostics(registry);
            });

            f(&compiler)
        };

        let prof = compiler.sess.prof.clone();
        prof.generic_activity("drop_compiler").run(move || drop(compiler));
        r
    })
}

pub fn run_compiler<R: Send>(mut config: Config, f: impl FnOnce(&Compiler) -> R + Send) -> R {
    tracing::trace!("run_compiler");
    let stderr = config.stderr.take();
    util::setup_callbacks_and_run_in_thread_pool_with_globals(
        config.opts.edition,
        config.opts.debugging_opts.threads,
        &stderr,
        || create_compiler_and_run(config, f),
    )
}

pub fn try_print_query_stack(handler: &Handler, num_frames: Option<usize>) {
    eprintln!("query stack during panic:");

    // Be careful relying on global state here: this code is called from
    // a panic hook, which means that the global `Handler` may be in a weird
    // state if it was responsible for triggering the panic.
    let i = ty::tls::with_context_opt(|icx| {
        if let Some(icx) = icx {
            icx.tcx.queries.try_print_query_stack(icx.tcx, icx.query, handler, num_frames)
        } else {
            0
        }
    });

    if num_frames == None || num_frames >= Some(i) {
        eprintln!("end of query stack");
    } else {
        eprintln!("we're just showing a limited slice of the query stack");
    }
}
