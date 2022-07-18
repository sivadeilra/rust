use crate::spec::{PanicStrategy, SanitizerSet, Target, TargetOptions};

pub(crate) fn make_onecore(windows_target: Target) -> Target {
    Target {
        options: TargetOptions {
            // OneCore only supports static libs.
            dynamic_linking: false,
            executables: false,
            no_default_libraries: true,

            // OneCore is expected to be used in the kernel, which doesn't support unwinding.
            panic_strategy: PanicStrategy::Abort,

            // Enable KASAN.
            supported_sanitizers: windows_target.options.supported_sanitizers | SanitizerSet::ADDRESS,

            ..windows_target.options
        },
        ..windows_target
    }
}
