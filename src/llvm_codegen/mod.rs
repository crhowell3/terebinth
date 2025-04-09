use std::ffi::CStr;
use std::mem::ManuallyDrop;

#[derive(Clone)]
pub struct LlvmCodegenBackend(());

impl ExtraBackendMethods for LlvmCodegenBackend {
    fn codegen_allocator<'tcx>(
        &self,
        tcx: TyCtx<'tcx>,
        mod_name: &str,
        kind: AllocatorKind,
        alloc_err_handler_kind: AllocatorKind,
    ) -> ModuleLlvm {
        let mod_llvm = ModuleLlvm::new_metadata(tcx, mod_name);
        let cx = SimpleCx::new(
            mod_llvm.llmod(),
            &mod_llvm.llcx,
            tcx.data_layout.pointer_size,
        );
        unsafe {
            allocator::codegen(tcx, cx, mod_name, kind, alloc_err_handler_kind);
        }
        mod_llvm
    }

    fn compile_codegen_unit(
        &self,
        tcx: TyCtx<'_>,
        cgu_name: Symbol,
    ) -> (ModuleCodegen<ModuleLlvm>, u64) {
        base::compile_codegen_unit(tcx, cgu_name)
    }

    fn target_machine_factory(
        &self,
        sess: &Session,
        optlvl: OptLevel,
        target_features: &[String],
    ) -> TargetMachineFactoryFn<Self> {
        back::write::target_machine_factory(sess, optlvl, target_features)
    }

    fn spawn_named_thread<F, T>(
        time_trace: bool,
        name: String,
        f: F,
    ) -> std::io::Result<std::thread::JoinHandle<T>>
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        std::thread::Builder::new().name(name).spawn(move || {
            let _profiler = TimeTraceProfiler::new(time_trace);
            f()
        })
    }
}

impl WriteBackendMethods for LlvmCodegenBackend {
    type Module = ModuleLlvm;
    type ModuleBuffer = back::lto::ModuleBuffer;
    type TargetMachine = OwnedTargetMachine;
    type TargetMachineError = crate::errors::LlvmError<'static>;
    type ThinData = back::lto::ThinData;
    type ThinBuffer = back::lto::ThinBuffer;

    fn print_pass_timings(&self) {
        let timings = llvm::build_string(|s| unsafe { llvm::LLVMRustPrintPassTimings(s) }).unwrap();
        print!("{timings}");
    }

    fn print_statistics(&self) {
        let stats = llvm::build_string(|s| unsafe { llvm::LLVMRustPrintStastistics(s) }).unwrap();
        print!("{stats}");
    }

    fn run_link(
        cgcx: &CodegenContext<Self>,
        dcx: DiagCtxHandle<'_>,
        modules: Vec<ModuleCodegen<Self::Module>>,
    ) -> Result<ModuleCodegen<Self::Module>, FatalError> {
        back::write::link(cgcx, dcx, modules)
    }
}

impl LlvmCodegenBackend {
    pub fn new() -> Box<dyn CodegenBackend> {
        Box::new(LlvmCodegenBackend(()))
    }
}

impl CodegenBackend for LlvmCodegenBackend {
    fn locale_resource(&self) -> &'static str {
        crate::DEFAULT_LOCALE_RESOURCE
    }

    fn init(&self, sess: &Session) {
        llvm_util::init(sess);
    }

    fn provide(&self, providers: &mut Providers) {
        providers.global_backend_features =
            |tcx, ()| llvm_util::global_llvm_features(tcx.sess, true, false)
    }

    fn print(&self, req: &PrintRequest, out: &mut String, sess: &Session) {
        use std::fmt::Write;
        match req.kind {
            PrintKind::RelocationModels => {
                writeln!(out, "Available relocation models:").unwrap();
                for name in &[
                    "static",
                    "pic",
                    "pie",
                    "dynamic_no_pic",
                    "ropi",
                    "rwpi",
                    "ropi-rwpi",
                    "default",
                ] {
                    writeln!(out, "    {name}").unwrap();
                }
                writeln!(out).unwrap();
            }
            PrintKind::CodeModels => {
                writeln!(out, "Available code models:").unwrap();
                for name in &["tiny", "small", "kernel", "medium", "large"] {
                    writeln!(out, "    {name}").unwrap();
                }
                writeln!(out).unwrap();
            }
            PrintKind::TlsModels => {
                writeln!(out, "Available TLS models:").unwrap();
                for name in &[
                    "global-dynamic",
                    "local-dynamic",
                    "initial-exec",
                    "local-exec",
                    "emulated",
                ] {
                    writeln!(out, "    {name}").unwrap();
                }
                writeln!(out).unwrap();
            }
            PrintKind::StackProtectorStrategies => {
                writeln!(
                    out,
                    r#"Available stack protector strategies:
                        all
        Generate stack canaries in all functions.

    strong
        Generate stack canaries in a function if it either:
        - has a local variable of `[T; N]` type, regardless of `T` and `N`
        - takes the address of a local variable.

          (Note that a local variable being borrowed is not equivalent to its
          address being taken: e.g. some borrows may be removed by optimization,
          while by-value argument passing may be implemented with reference to a
          local stack variable in the ABI.)

    basic
        Generate stack canaries in functions with local variables of `[T; N]`
        type, where `T` is byte-sized and `N` >= 8.

    none
        Do not generate stack canaries.
        "#
                )
                .unwrap();
            }
            _other => llvm_util::print(req, out, sess),
        }
    }
}

pub struct ModuleLlvm {
    llcx: &'static mut llvm::Context,
    llmod_raw: *const llvm::Module,
    tm: ManuallyDrop<OwnedTargetMachine>,
}

unsafe impl Send for ModuleLlvm {}
unsafe impl Sync for ModuleLlvm {}

impl ModuleLlvm {
    fn new(tcx: TyCtx<'_>, mod_name: &str) -> Self {
        unsafe {
            let llcx = llvm::LLVMRustContextCreate(tcx.sess.fewer_names());
            let llmod_raw = context::create_module(tcx, llcx, mod_name) as *const _;
            ModuleLlvm {
                llmod_raw,
                llcx,
                tm: ManuallyDrop::new(create_target_machine(tcx, mod_name)),
            }
        }
    }

    fn new_metadata(tcx: TxCtx<'_>, mod_name: &str) -> Self {
        unsafe {
            let llcx = llvm::LLVMRustContextCreate(tcx.sess.fewer_names());
            let llmod_raw = context::create_module(tcx, llcx, mod_name) as *const _;
            ModuleLlvm {
                llmod_raw,
                llcx,
                tm: ManuallyDrop::new(create_informational_target_machine(tcx.sess, false)),
            }
        }
    }

    fn parse(
        cgcx: &CodegenContext<LlvmCodegenBackend>,
        name: &CStr,
        buffer: &[u8],
        dcx: DiagCtxHandle<'_>,
    ) -> Result<Self, FatalError> {
        unsafe {
            let llcx = llvm::LLVMRustContextCreate(cgcx.fewer_names);
            let llmod_raw = back::lto::parse_module(llcx, name, buffer, dcx)?;
            let tm_factory_config = TargetMachineFactoryConfig::new(cgcx, name.to_str().unwrap());
            let tm = match (cgcx.tm_factory)(tm_factory_config) {
                Ok(m) => m,
                Err(e) => {
                    return Err(dcx.emit_almost_fatal(ParseTargetMachineConfig(e)));
                }
            };

            Ok(ModuleLlvm {
                llmod_raw,
                llcx,
                tm: ManuallyDrop::new(tm),
            })
        }
    }

    fn llmod(&self) -> &llvm::Module {
        unsafe { &*self.llmod_raw }
    }
}

impl Drop for ModuleLlvm {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.tm);
            llvm::LLVMContextDispose(&mut *(self.llcx as *mut _));
        }
    }
}
