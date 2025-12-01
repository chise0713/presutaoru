use std::{
    fmt::{Debug, Display},
    path::Path,
};

/// PsiEntry types, it's the path to `/proc/pressure/type` files.
#[derive(Clone, Copy)]
pub enum PsiEntry {
    Cpu,
    Io,
    Irq,
    Memory,
}

impl PsiEntry {
    const CPU: &str = "/proc/pressure/cpu";
    const IO: &str = "/proc/pressure/io";
    const IRQ: &str = "/proc/pressure/irq";
    const MEMORY: &str = "/proc/pressure/memory";
    /// Returns `true` if the PsiEntry exists in the system.
    pub fn exists(&self) -> bool {
        self.as_ref().exists()
    }
}

impl Display for PsiEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl Debug for PsiEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl AsRef<Path> for PsiEntry {
    fn as_ref(&self) -> &Path {
        let path = match self {
            Self::Cpu => Self::CPU,
            Self::Io => Self::IO,
            Self::Irq => Self::IRQ,
            Self::Memory => Self::MEMORY,
        };
        Path::new(path)
    }
}
