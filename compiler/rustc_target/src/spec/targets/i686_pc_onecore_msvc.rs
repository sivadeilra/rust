use crate::spec::{base, Target};

pub(crate) fn target() -> Target {
    let mut windows_target = super::i686_pc_windows_msvc::target();
    windows_target.options.features += ",+retpoline-indirect-branches,+retpoline-indirect-calls";
    base::onecore_msvc::make_onecore(windows_target)
}
