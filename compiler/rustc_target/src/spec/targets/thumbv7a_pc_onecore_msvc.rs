use crate::spec::{base, Target};

pub(crate) fn target() -> Target {
    base::onecore_msvc::make_onecore(
        super::thumbv7a_pc_windows_msvc::target())
}
