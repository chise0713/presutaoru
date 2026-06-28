use std::{borrow::Cow, fmt::Display, path::Path};

#[derive(Debug, Clone, Copy)]
pub enum GlobalEntryType {
    Cpu,
    Io,
    Irq,
    Memory,
}

#[derive(Debug, Clone, Copy)]
pub enum CgroupEntryType {
    Cpu,
    Io,
    Memory,
}

/// PsiEntry types
#[derive(Debug, Clone, Copy)]
pub enum PsiEntry<'a> {
    /// path to `/proc/pressure/[type]`
    Global(GlobalEntryType),
    /// use the given cgroup directory, path to `[dir]/[type]`
    Cgroup(CgroupEntryType, &'a Path),
}

impl<'a> PsiEntry<'a> {
    const CPU: &'static str = "/proc/pressure/cpu";
    const IO: &'static str = "/proc/pressure/io";
    const IRQ: &'static str = "/proc/pressure/irq";
    const MEMORY: &'static str = "/proc/pressure/memory";

    const CG_CPU: &'static str = "cpu.pressure";
    const CG_IO: &'static str = "io.pressure";
    const CG_MEMORY: &'static str = "memory.pressure";

    /// Returns the kernel PSI file path for this entry.
    pub fn path(&self) -> Cow<'_, Path> {
        match self {
            Self::Global(entry_type) => {
                let p: &'static str = match entry_type {
                    GlobalEntryType::Cpu => Self::CPU,
                    GlobalEntryType::Io => Self::IO,
                    GlobalEntryType::Irq => Self::IRQ,
                    GlobalEntryType::Memory => Self::MEMORY,
                };
                Cow::Borrowed(Path::new(p))
            }

            Self::Cgroup(entry_type, base) => {
                let file = match entry_type {
                    CgroupEntryType::Cpu => Self::CG_CPU,
                    CgroupEntryType::Io => Self::CG_IO,
                    CgroupEntryType::Memory => Self::CG_MEMORY,
                };
                Cow::Owned(base.join(file))
            }
        }
    }
}

impl<'a> Display for PsiEntry<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.path().display().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_path() {
        let cases = [
            (GlobalEntryType::Cpu, "/proc/pressure/cpu"),
            (GlobalEntryType::Io, "/proc/pressure/io"),
            (GlobalEntryType::Irq, "/proc/pressure/irq"),
            (GlobalEntryType::Memory, "/proc/pressure/memory"),
        ];

        for (kind, expected) in cases {
            assert_eq!(PsiEntry::Global(kind).path(), Path::new(expected));
        }
    }

    #[test]
    fn cgroup_path() {
        let base = Path::new("/sys/fs/cgroup/test");

        let cases = [
            (CgroupEntryType::Cpu, "cpu.pressure"),
            (CgroupEntryType::Io, "io.pressure"),
            (CgroupEntryType::Memory, "memory.pressure"),
        ];

        for (kind, file) in cases {
            assert_eq!(PsiEntry::Cgroup(kind, base).path(), base.join(file));
        }
    }

    #[test]
    fn display_matches_path() {
        let cgroup_path = Path::new("/tmp");

        let entries = [
            PsiEntry::Global(GlobalEntryType::Cpu),
            PsiEntry::Global(GlobalEntryType::Io),
            PsiEntry::Global(GlobalEntryType::Irq),
            PsiEntry::Global(GlobalEntryType::Memory),
            PsiEntry::Cgroup(CgroupEntryType::Cpu, cgroup_path),
            PsiEntry::Cgroup(CgroupEntryType::Io, cgroup_path),
            PsiEntry::Cgroup(CgroupEntryType::Memory, cgroup_path),
        ];

        for entry in entries {
            assert_eq!(entry.to_string(), entry.path().display().to_string());
        }
    }
}
