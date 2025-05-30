use crate::spec::{LinkerFlavor, Lld, TargetOptions, base, cvs};

pub(crate) fn opts() -> TargetOptions {
    let mut base = base::msvc::opts();

    base.add_pre_link_args(
        LinkerFlavor::Msvc(Lld::No),
        &[
            // Vulcan generally requires fixups in the debug info. By prepending this arg to link.exe
            // command line, we're effectively making /DEBUGTYPE:CV,FIXUP the default for this target.
            "/DEBUGTYPE:CV,FIXUP",
        ],
    );

    TargetOptions {
        os: "windows".into(),
        env: "msvc".into(),
        vendor: "pc".into(),
        dynamic_linking: true,
        dll_prefix: "".into(),
        dll_suffix: ".dll".into(),
        exe_suffix: ".exe".into(),
        staticlib_prefix: "".into(),
        staticlib_suffix: ".lib".into(),
        families: cvs!["windows"],
        crt_static_allows_dylibs: true,
        crt_static_respected: true,
        requires_uwtable: true,
        // We don't pass the /NODEFAULTLIB flag to the linker on MSVC
        // as that prevents linker directives embedded in object files from
        // including other necessary libraries.
        //
        // For example, msvcrt.lib embeds a linker directive like:
        //    /DEFAULTLIB:vcruntime.lib /DEFAULTLIB:ucrt.lib
        // So that vcruntime.lib and ucrt.lib are included when the entry point
        // in msvcrt.lib is used. Using /NODEFAULTLIB would mean having to
        // manually add those two libraries and potentially further dependencies
        // they bring in.
        //
        // See also https://learn.microsoft.com/en-us/cpp/preprocessor/comment-c-cpp?view=msvc-170#lib
        // for documentation on including library dependencies in C/C++ code.
        no_default_libraries: false,
        has_thread_local: true,

        ..base
    }
}
