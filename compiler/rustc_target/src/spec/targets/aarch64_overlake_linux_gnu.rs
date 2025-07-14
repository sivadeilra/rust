use std::borrow::Cow;

use crate::spec::{base, FramePointer, SanitizerSet, StackProbeType, StackProtector, Target, TargetOptions};

pub(crate) fn target() -> Target {
    Target {
        llvm_target: "aarch64-unknown-linux-gnu".into(),
        metadata: crate::spec::TargetMetadata {
            description: Some("ARM64 Linux (Overlake)".into()),
            tier: Some(1),
            host_tools: Some(true),
            std: Some(true),
        },
        pointer_width: 64,
        data_layout: "e-m:e-p270:32:32-p271:32:32-p272:64:64-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128-Fn32".into(),
        arch: "aarch64".into(),
        options: TargetOptions {
            features: "+v8a,+outline-atomics,+harden-sls-blr,+harden-sls-retbr".into(),
            llvm_args: Cow::Owned(vec!["--aarch64-slh-loads".into()]),
            mcount: "\u{1}_mcount".into(),
            max_atomic_width: Some(128),
            stack_probes: StackProbeType::Inline,
            supported_sanitizers: SanitizerSet::ADDRESS
                | SanitizerSet::CFI
                | SanitizerSet::KCFI
                | SanitizerSet::LEAK
                | SanitizerSet::MEMORY
                | SanitizerSet::MEMTAG
                | SanitizerSet::THREAD
                | SanitizerSet::HWADDRESS,
            default_sanitizers: SanitizerSet::CFI,
            default_branch_protection: Some("bti,pac-ret"),
            stack_protector: StackProtector::All,
            supports_xray: true,
            frame_pointer: FramePointer::NonLeaf,
            
            ..base::linux_gnu::opts()
        },
    }
}
