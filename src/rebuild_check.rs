use crate::pkg_utils;

use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use alpm::Alpm;
use anyhow::{Context, Result};

#[derive(Debug)]
pub struct BrokenPackage {
    pub name: String,
    pub version: String,
    pub missing_libs: Vec<String>,
}

struct PackageAnalysis {
    name: String,
    version: String,
    provides: Vec<String>,
    needs: HashSet<String>,
}

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const LIB32_SEP: &[u8] = b"/lib32/";
const SO_DOT: &[u8] = b".so.";
const MAX_ELF_SIZE: u64 = 100 * 1024 * 1024; // 100 MB

/// Load .so filenames from all repos configured in the given pacman.conf
/// by syncing .files databases and scanning package file lists.
fn get_arch_repo_sofiles(pacman_conf: Option<&Path>) -> Result<HashSet<String>> {
    let conf = match pacman_conf {
        Some(path) => pacmanconf::Config::from_file(path)
            .with_context(|| format!("Failed to parse {}", path.display()))?,
        None => pacmanconf::Config::new().context("Failed to parse system pacman.conf")?,
    };

    tracing::info!("Loading .files databases from {} repos", conf.repos.len());

    let tmp = tempfile::tempdir().context("Failed to create temp dir for ALPM")?;
    let db_path = tmp.path().join("db");
    std::fs::create_dir_all(&db_path)?;

    let mut alpm = Alpm::new(
        tmp.path().to_str().context("invalid temp path")?,
        db_path.to_str().context("invalid db path")?,
    )?;
    alpm.set_dbext(".files");
    alpm_utils::configure_alpm(&mut alpm, &conf)?;
    alpm.syncdbs_mut().update(false)?;

    let mut sofiles = HashSet::with_capacity(50_000);
    for db in alpm.syncdbs() {
        for pkg in db.pkgs() {
            for file in pkg.files().files() {
                let name = file.name();
                // Skip lib32, those are 32-bit multilib libraries that
                // cannot satisfy dependencies of 64-bit packages.
                if name.windows(LIB32_SEP.len()).any(|w| w == LIB32_SEP) {
                    continue;
                }
                if let Some(pos) = name.iter().rposition(|&b| b == b'/') {
                    let filename = &name[pos + 1..];
                    if filename.windows(SO_DOT.len()).any(|w| w == SO_DOT)
                        || filename.ends_with(b".so")
                    {
                        let s = std::str::from_utf8(filename).map_or_else(|_| String::from_utf8_lossy(filename).into_owned(), std::string::ToString::to_string);
                        sofiles.insert(s);
                    }
                }
            }
        }
    }

    tracing::info!("Found {} .so files from repos", sofiles.len());
    Ok(sofiles)
}

fn analyze_package(pkg_path: &Path) -> Result<PackageAnalysis> {
    let filename = pkg_path
        .file_name()
        .and_then(|f| f.to_str())
        .with_context(|| format!("Invalid package path: {}", pkg_path.display()))?;
    let name = pkg_utils::get_pkgname_from_filename(filename).to_string();
    let version = pkg_utils::get_pkgver_from_filename(filename).to_string();

    let file =
        File::open(pkg_path).with_context(|| format!("Failed to open {}", pkg_path.display()))?;
    let decoder = zstd::Decoder::new(file)
        .with_context(|| format!("Failed to decompress {}", pkg_path.display()))?;
    let mut archive = tar::Archive::new(decoder);

    let mut provides = Vec::new();
    let mut needs = HashSet::new();

    for entry_result in archive.entries()? {
        let mut entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue,
        };

        if entry.header().entry_type() != tar::EntryType::Regular {
            continue;
        }

        let size = entry.header().size().unwrap_or(0);
        if !(4..=MAX_ELF_SIZE).contains(&size) {
            continue;
        }

        let mut magic = [0u8; 4];
        if entry.read_exact(&mut magic).is_err() || magic != ELF_MAGIC {
            continue;
        }

        let mut buf = Vec::with_capacity(size as usize);
        buf.extend_from_slice(&magic);
        let entry_path = entry.path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        if let Err(e) = entry.read_to_end(&mut buf) {
            tracing::warn!("Failed to read ELF entry {entry_path}: {e}");
            continue;
        }

        let elf = match goblin::elf::Elf::parse(&buf) {
            Ok(elf) => elf,
            Err(e) => {
                tracing::warn!("Failed to parse ELF {entry_path}: {e}");
                continue;
            },
        };

        for lib in &elf.libraries {
            needs.insert(lib.to_string());
        }

        if let Some(soname) = &elf.soname {
            provides.push(soname.to_string());
        }
    }

    Ok(PackageAnalysis { name, version, provides, needs })
}

fn find_broken(analyses: Vec<PackageAnalysis>, available: &HashSet<String>) -> Vec<BrokenPackage> {
    let mut broken = Vec::new();
    for analysis in analyses {
        let mut missing: Vec<String> =
            analysis.needs.iter().filter(|lib| !available.contains(*lib)).cloned().collect();

        if !missing.is_empty() {
            missing.sort();
            broken.push(BrokenPackage {
                name: analysis.name,
                version: analysis.version,
                missing_libs: missing,
            });
        }
    }
    broken
}

pub fn check_broken_packages(
    repo_dir: &Path,
    pacman_conf: Option<&Path>,
) -> Result<Vec<BrokenPackage>> {
    let arch_sofiles = get_arch_repo_sofiles(pacman_conf)?;
    check_broken_packages_with(repo_dir, arch_sofiles)
}

fn check_broken_packages_with(
    repo_dir: &Path,
    external_sofiles: HashSet<String>,
) -> Result<Vec<BrokenPackage>> {
    let pkg_paths = pkg_utils::find_packages_in_dir(repo_dir)?;

    let total = pkg_paths.len();
    tracing::info!("Analyzing {total} packages in {}", repo_dir.display());

    let mut analyses = Vec::with_capacity(total);
    for (i, pkg_path_str) in pkg_paths.iter().enumerate() {
        let pkg_path = Path::new(pkg_path_str);
        tracing::debug!("[{}/{}] Analyzing {pkg_path_str}", i + 1, total);

        match analyze_package(pkg_path) {
            Ok(analysis) => analyses.push(analysis),
            Err(e) => {
                tracing::error!("Failed to analyze {pkg_path_str}: {e}");
            },
        }
    }

    // Build combined available repo libs + all profile repo provides
    let mut available = external_sofiles;
    for analysis in &analyses {
        for soname in &analysis.provides {
            available.insert(soname.clone());
        }
    }
    tracing::debug!("Total available .so entries: {}", available.len());

    Ok(find_broken(analyses, &available))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_elf(needed: &[&str], soname: Option<&str>) -> Vec<u8> {
        // Build string table: starts with null byte, then each string null-terminated
        let mut strtab = vec![0u8];
        let mut needed_offsets = Vec::new();
        for lib in needed {
            needed_offsets.push(strtab.len() as u64);
            strtab.extend_from_slice(lib.as_bytes());
            strtab.push(0);
        }
        let soname_offset = soname.map(|s| {
            let off = strtab.len() as u64;
            strtab.extend_from_slice(s.as_bytes());
            strtab.push(0);
            off
        });

        // Dynamic entries: STRTAB + STRSZ + N×NEEDED + maybe SONAME + NULL
        let num_dyn_entries = 2 + needed.len() + soname_offset.map_or(0, |_| 1) + 1;
        let dyn_section_size = num_dyn_entries * 16;
        // Layout: ELF header (64) + 2 program headers (2×56=112) + dynamic + strtab
        let dyn_offset: u64 = 64 + 2 * 56;
        let strtab_offset = dyn_offset + dyn_section_size as u64;
        let total_size = strtab_offset + strtab.len() as u64;

        let mut elf = Vec::new();

        // ELF64 Header (64 bytes)
        elf.extend_from_slice(&[0x7f, b'E', b'L', b'F']); // e_ident magic
        elf.push(2); // EI_CLASS: ELFCLASS64
        elf.push(1); // EI_DATA: ELFDATA2LSB
        elf.push(1); // EI_VERSION: EV_CURRENT
        elf.extend_from_slice(&[0; 9]); // EI_OSABI + padding
        elf.extend_from_slice(&3u16.to_le_bytes()); // e_type: ET_DYN
        elf.extend_from_slice(&0x3eu16.to_le_bytes()); // e_machine: EM_X86_64
        elf.extend_from_slice(&1u32.to_le_bytes()); // e_version
        elf.extend_from_slice(&0u64.to_le_bytes()); // e_entry
        elf.extend_from_slice(&64u64.to_le_bytes()); // e_phoff
        elf.extend_from_slice(&0u64.to_le_bytes()); // e_shoff
        elf.extend_from_slice(&0u32.to_le_bytes()); // e_flags
        elf.extend_from_slice(&64u16.to_le_bytes()); // e_ehsize
        elf.extend_from_slice(&56u16.to_le_bytes()); // e_phentsize
        elf.extend_from_slice(&2u16.to_le_bytes()); // e_phnum: 2 (PT_LOAD + PT_DYNAMIC)
        elf.extend_from_slice(&0u16.to_le_bytes()); // e_shentsize
        elf.extend_from_slice(&0u16.to_le_bytes()); // e_shnum
        elf.extend_from_slice(&0u16.to_le_bytes()); // e_shstrndx
        assert_eq!(elf.len(), 64);

        // Program Header 0: PT_LOAD (identity-mapped, covers entire file)
        elf.extend_from_slice(&1u32.to_le_bytes()); // p_type: PT_LOAD
        elf.extend_from_slice(&5u32.to_le_bytes()); // p_flags: PF_R | PF_X
        elf.extend_from_slice(&0u64.to_le_bytes()); // p_offset: 0
        elf.extend_from_slice(&0u64.to_le_bytes()); // p_vaddr: 0
        elf.extend_from_slice(&0u64.to_le_bytes()); // p_paddr: 0
        elf.extend_from_slice(&total_size.to_le_bytes()); // p_filesz
        elf.extend_from_slice(&total_size.to_le_bytes()); // p_memsz
        elf.extend_from_slice(&0x1000u64.to_le_bytes()); // p_align
        assert_eq!(elf.len(), 120);

        // Program Header 1: PT_DYNAMIC
        let dyn_payload = dyn_section_size + strtab.len();
        elf.extend_from_slice(&2u32.to_le_bytes()); // p_type: PT_DYNAMIC
        elf.extend_from_slice(&4u32.to_le_bytes()); // p_flags: PF_R
        elf.extend_from_slice(&dyn_offset.to_le_bytes()); // p_offset
        elf.extend_from_slice(&dyn_offset.to_le_bytes()); // p_vaddr
        elf.extend_from_slice(&dyn_offset.to_le_bytes()); // p_paddr
        elf.extend_from_slice(&(dyn_payload as u64).to_le_bytes()); // p_filesz
        elf.extend_from_slice(&(dyn_payload as u64).to_le_bytes()); // p_memsz
        elf.extend_from_slice(&8u64.to_le_bytes()); // p_align
        assert_eq!(elf.len(), 176);

        // DT_STRTAB (tag=5)
        elf.extend_from_slice(&5u64.to_le_bytes());
        elf.extend_from_slice(&strtab_offset.to_le_bytes());
        // DT_STRSZ (tag=10)
        elf.extend_from_slice(&10u64.to_le_bytes());
        elf.extend_from_slice(&(strtab.len() as u64).to_le_bytes());
        // DT_NEEDED (tag=1) for each library
        for &offset in &needed_offsets {
            elf.extend_from_slice(&1u64.to_le_bytes());
            elf.extend_from_slice(&offset.to_le_bytes());
        }
        // DT_SONAME (tag=14) if specified
        if let Some(offset) = soname_offset {
            elf.extend_from_slice(&14u64.to_le_bytes());
            elf.extend_from_slice(&offset.to_le_bytes());
        }
        // DT_NULL (tag=0)
        elf.extend_from_slice(&0u64.to_le_bytes());
        elf.extend_from_slice(&0u64.to_le_bytes());
        assert_eq!(elf.len(), 176 + dyn_section_size);

        // String Table
        elf.extend_from_slice(&strtab);

        elf
    }

    /// Create a .pkg.tar.zst file in the given directory with specified internal files.
    fn create_test_package(dir: &Path, pkg_filename: &str, files: &[(&str, &[u8])]) {
        let pkg_path = dir.join(pkg_filename);
        let file = File::create(&pkg_path).unwrap();
        let encoder = zstd::Encoder::new(file, 1).unwrap();
        let mut builder = tar::Builder::new(encoder);

        for (name, content) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_entry_type(tar::EntryType::Regular);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, *name, &content[..]).unwrap();
        }

        let encoder = builder.into_inner().unwrap();
        encoder.finish().unwrap();
    }

    fn create_temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_build_elf_is_parseable() {
        let elf_bytes = build_test_elf(&["libfoo.so.1", "libbar.so.2"], Some("libbaz.so.3"));
        let elf = goblin::elf::Elf::parse(&elf_bytes).expect("should parse as valid ELF");

        assert_eq!(elf.libraries, vec!["libfoo.so.1", "libbar.so.2"]);
        assert_eq!(elf.soname, Some("libbaz.so.3"));
    }

    #[test]
    fn test_build_elf_no_soname() {
        let elf_bytes = build_test_elf(&["libc.so.6"], None);
        let elf = goblin::elf::Elf::parse(&elf_bytes).expect("should parse as valid ELF");

        assert_eq!(elf.libraries, vec!["libc.so.6"]);
        assert_eq!(elf.soname, None);
    }

    #[test]
    fn test_analyze_package_extracts_needed_and_soname() {
        let dir = create_temp_dir();

        let lib_elf = build_test_elf(&["libc.so.6"], Some("libtest.so.1"));
        let bin_elf = build_test_elf(&["libtest.so.1", "libc.so.6"], None);

        create_test_package(dir.path(), "testpkg-1.0.0-1-x86_64.pkg.tar.zst", &[
            ("usr/lib/libtest.so.1.0.0", &lib_elf),
            ("usr/bin/testbin", &bin_elf),
            ("usr/share/doc/README", b"just a text file"),
        ]);

        let analysis =
            analyze_package(&dir.path().join("testpkg-1.0.0-1-x86_64.pkg.tar.zst")).unwrap();

        assert_eq!(analysis.name, "testpkg");
        assert_eq!(analysis.version, "1.0.0-1");
        assert!(analysis.provides.contains(&"libtest.so.1".to_string()));
        assert!(analysis.needs.contains(&"libc.so.6".to_string()));
        assert!(analysis.needs.contains(&"libtest.so.1".to_string()));
    }

    #[test]
    fn test_analyze_package_no_elf_files() {
        let dir = create_temp_dir();

        create_test_package(dir.path(), "textpkg-2.0.0-1-any.pkg.tar.zst", &[
            ("usr/share/textpkg/config.conf", b"key=value"),
            ("usr/share/doc/textpkg/README", b"hello"),
        ]);

        let analysis =
            analyze_package(&dir.path().join("textpkg-2.0.0-1-any.pkg.tar.zst")).unwrap();

        assert_eq!(analysis.name, "textpkg");
        assert!(analysis.provides.is_empty());
        assert!(analysis.needs.is_empty());
    }

    #[test]
    fn test_find_broken_detects_missing_deps() {
        let make_analyses = || {
            vec![
                PackageAnalysis {
                    name: "app".into(),
                    version: "1.0-1".into(),
                    provides: vec![],
                    needs: ["libfoo.so.1".into(), "libc.so.6".into()].into(),
                },
                PackageAnalysis {
                    name: "libfoo".into(),
                    version: "1.0-1".into(),
                    provides: vec!["libfoo.so.1".into()],
                    needs: ["libc.so.6".into()].into(),
                },
            ]
        };

        let available: HashSet<String> =
            ["libc.so.6".to_string(), "libfoo.so.1".to_string()].into();

        let broken = find_broken(make_analyses(), &available);
        assert!(broken.is_empty(), "all deps are satisfied");

        // Now remove libfoo.so.1 from available — app should break
        let available_without_foo: HashSet<String> = ["libc.so.6".to_string()].into();
        let broken = find_broken(make_analyses(), &available_without_foo);
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].name, "app");
        assert_eq!(broken[0].missing_libs, vec!["libfoo.so.1"]);
    }

    #[test]
    fn test_find_broken_multiple_missing() {
        let analyses = vec![PackageAnalysis {
            name: "bigapp".into(),
            version: "2.0-1".into(),
            provides: vec![],
            needs: ["libgone.so.1".into(), "libc.so.6".into(), "libalso_gone.so.3".into()].into(),
        }];

        let available: HashSet<String> = ["libc.so.6".to_string()].into();
        let broken = find_broken(analyses, &available);

        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].name, "bigapp");
        assert!(broken[0].missing_libs.contains(&"libgone.so.1".to_string()));
        assert!(broken[0].missing_libs.contains(&"libalso_gone.so.3".to_string()));
        assert!(!broken[0].missing_libs.contains(&"libc.so.6".to_string()));
    }

    #[test]
    fn test_find_broken_all_satisfied() {
        let analyses = vec![
            PackageAnalysis {
                name: "pkg-a".into(),
                version: "1.0-1".into(),
                provides: vec!["liba.so.1".into()],
                needs: ["libc.so.6".into()].into(),
            },
            PackageAnalysis {
                name: "pkg-b".into(),
                version: "1.0-1".into(),
                provides: vec![],
                needs: ["liba.so.1".into(), "libc.so.6".into()].into(),
            },
        ];

        let available: HashSet<String> = ["libc.so.6".to_string(), "liba.so.1".to_string()].into();
        let broken = find_broken(analyses, &available);
        assert!(broken.is_empty());
    }

    #[test]
    fn test_check_broken_packages_integration() {
        let dir = create_temp_dir();

        // Package "mylib" provides libmy.so.1, needs libc.so.6
        let lib_elf = build_test_elf(&["libc.so.6"], Some("libmy.so.1"));
        create_test_package(dir.path(), "mylib-1.0.0-1-x86_64.pkg.tar.zst", &[(
            "usr/lib/libmy.so.1.0.0",
            &lib_elf,
        )]);

        // Package "myapp" needs libmy.so.1 and libc.so.6
        let app_elf = build_test_elf(&["libmy.so.1", "libc.so.6"], None);
        create_test_package(dir.path(), "myapp-2.0.0-1-x86_64.pkg.tar.zst", &[(
            "usr/bin/myapp",
            &app_elf,
        )]);

        // Simulate Arch repos providing libc.so.6
        let arch_sofiles: HashSet<String> = ["libc.so.6".to_string()].into();
        let broken = check_broken_packages_with(dir.path(), arch_sofiles).unwrap();

        // libc.so.6 comes from arch repos, libmy.so.1 is provided by mylib in the profile
        assert!(broken.is_empty(), "all deps should be satisfied: {broken:?}");
    }

    #[test]
    fn test_check_broken_packages_detects_missing() {
        let dir = create_temp_dir();

        // Package needs libgone.so.1 which is not in arch repos or profile
        let app_elf = build_test_elf(&["libgone.so.1", "libc.so.6"], None);
        create_test_package(dir.path(), "brokenapp-1.0.0-1-x86_64.pkg.tar.zst", &[(
            "usr/bin/brokenapp",
            &app_elf,
        )]);

        let arch_sofiles: HashSet<String> = ["libc.so.6".to_string()].into();
        let broken = check_broken_packages_with(dir.path(), arch_sofiles).unwrap();

        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].name, "brokenapp");
        assert_eq!(broken[0].missing_libs, vec!["libgone.so.1"]);
    }

    #[test]
    fn test_check_broken_packages_empty_repo() {
        let dir = create_temp_dir();
        let broken = check_broken_packages_with(dir.path(), HashSet::new()).unwrap();
        assert!(broken.is_empty());
    }
}
