use crate::spec::{StackProtector, TargetOptions, base};

pub(crate) fn opts() -> TargetOptions {
    TargetOptions {
        env: "gnu".into(),
        stack_protector: StackProtector::Strong,
        ..base::linux::opts()
    }
}
