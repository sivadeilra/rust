use crate::spec::{base, Target};

pub(crate) fn target() -> Target {
    base::onecore_msvc::make_onecore(
        super::aarch64_pc_windows_msvc::target())
}
