//! Utilities for comparing files and directories(WIP).
//!
//! Struct like in Python3 std-lib:
//!  - DirCmp
//!
//! Functions like in Python3 std-lib:
//!  - cmp(f1, f2, shallow: bool) -> int
//!  - cmpfiles(a, b, common) -> ([], [], [])
//!  - clear_cache()
//!
//! # Example
//!
//! Check out [Example for cmp()](cmp#example)

mod os;
mod stat;

use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;

const BUFSIZE: usize = 8 * 1024;
const FOLLOW_SYMLINKS_DEFAULT: bool = true;
const MAX_CACHE_SIZE: usize = 100;

lazy_static! {
    /// Cache for File Comparison
    static ref CACHE: Arc<Mutex<HashMap<(PathBuf, PathBuf, Signature, Signature), bool>>> = Arc::new(Mutex::new(HashMap::new()));
}

/// Clear the filecmp cache.
pub fn clear_cache() {
    CACHE.lock().unwrap().clear();
}

/// Compare two files.
///
/// Arguments:
///  - f1 -- First file name
///  - f2 -- Second file name
///  - shallow -- Just check stat signature (do not read the files).
///
/// Return value:
///  - True if the files are the same, False otherwise.
///
/// This function uses a cache for past comparisons and the results,
/// with cache entries invalidated if their stat information
/// changes.  The cache may be cleared by calling clear_cache().
///
/// # Example
///
/// ```rust
/// use std::io::Write;
/// use std::fs::File;
/// use tempfile;
/// use filecmp;
///
/// let temp_dir = tempfile::tempdir().unwrap().into_path();
/// let foo_path = temp_dir.join("foo.txt");
/// let bar_path = temp_dir.join("bar.txt");
/// let baz_path = temp_dir.join("baz.txt");
///
/// { // Create files in temporary directory
///     let mut foo = File::create(&foo_path).unwrap();
///     let mut bar = File::create(&bar_path).unwrap();
///     let mut baz = File::create(&baz_path).unwrap();
///
///     foo.write_all(b"hello filecmp!").unwrap();
///     bar.write_all(b"hello filecmp!").unwrap();
///     baz.write_all(b"hello world!").unwrap();
/// } // Close them
///
/// let a = filecmp::cmp(&foo_path, &bar_path, true).unwrap();
/// let b = filecmp::cmp(&foo_path, &baz_path, true).unwrap();
/// let c = filecmp::cmp(&bar_path, &baz_path, true).unwrap();
///
/// assert!(a);
/// assert!(!b);
/// assert!(!c);
/// ```
///
pub fn cmp(f1: impl AsRef<Path>, f2: impl AsRef<Path>, shallow: bool) -> io::Result<bool> {
    let s1 = sig(os::stat(f1.as_ref(), FOLLOW_SYMLINKS_DEFAULT)?);
    let s2 = sig(os::stat(f2.as_ref(), FOLLOW_SYMLINKS_DEFAULT)?);

    if s1.s_ifmt != stat::S_IFREG || s2.s_ifmt != stat::S_IFREG {
        return Ok(false);
    }
    if shallow && s1 == s2 {
        return Ok(true);
    }
    if s1.st_size != s2.st_size {
        return Ok(false);
    }

    let key = (f1.as_ref().into(), f2.as_ref().into(), s1, s2);
    let c_cache = Arc::clone(&CACHE);
    let outcome = c_cache.lock().unwrap().get(&key).copied();
    let outcome = if let Some(outcome) = outcome {
        outcome
    } else {
        let outcome = do_cmp(f1, f2)?;
        if c_cache.lock().unwrap().len() > MAX_CACHE_SIZE {
            // limit the maximum size of the cache
            clear_cache();
        }
        c_cache.lock().unwrap().insert(key, outcome);
        outcome
    };

    Ok(outcome)
}

/// Compare common files in two directories. (WIP)
///
/// Arguments:
///  - dir1 -- First directory name
///  - dir2 -- Second directory name
///  - common -- list of file names found in both directories
///  - shallow -- if true, do comparison based solely on stat() information
///
/// Returns a tuple of three lists:
///  - filepaths that compare equal
///  - filepaths that are different
///  - filepaths that aren't regular files.
pub fn cmpfiles<A, B, C, D>(
    _dir1: A,
    _dir2: B,
    _common: D,
    _shallow: bool,
) -> io::Result<(Vec<PathBuf>, Vec<PathBuf>, Vec<PathBuf>)>
where
    A: AsRef<Path>,
    B: AsRef<Path>,
    C: AsRef<Path>,
    D: AsRef<[C]>,
{
    unimplemented!() // TODO
}

/// A struct that manages the comparison of 2 directories. (WIP)
///
/// dircmp(a, b, ignore, hide)
/// A and B are directories.
/// IGNORE is a list of names to ignore, defaults to DEFAULT_IGNORES.
/// HIDE is a list of names to hide, defaults to [os.curdir, os.pardir].
///
/// High level usage:
///  + x = dircmp(dir1, dir2)
///    - x.report() -> prints a report on the differences between dir1 and dir2
///      or
///    - x.report_partial_closure() -> prints report on differences between dir1
///      and dir2, and reports on common immediate subdirectories.
///    - x.report_full_closure() -> like report_partial_closure, but fully recursive.
///
/// Attributes:
///  - left_list, right_list: The files in dir1 and dir2, filtered by hide and ignore.
///  - common: a list of names in both dir1 and dir2.
///  - left_only, right_only: names only in dir1, dir2.
///  - common_dirs: subdirectories in both dir1 and dir2.
///  - common_files: files in both dir1 and dir2.
///  - common_funny: names in both dir1 and dir2 where the type differs between dir1 and dir2, or the name is not stat-able.
///  - same_files: list of identical files.
///  - diff_files: list of filenames which differ.
///  - funny_files: list of files which could not be compared.
///  - subdirs: a dictionary of dircmp objects, keyed by names in common_dirs.
pub struct DirCmp;

impl DirCmp {
    // TODO
    pub fn new(_a: impl AsRef<Path>, _b: impl AsRef<Path>) -> Self {
        unimplemented!() // TODO
    }

    pub fn report_full_closure(&self) {
        unimplemented!() // TODO
    }

    pub fn report(&self) {
        unimplemented!() // TODO
    }
}

fn sig(st: os::StatResult) -> Signature {
    Signature {
        s_ifmt: stat::S_IFMT(st.st_mode),
        st_size: st.st_size,
        st_mtime: st.st_mtime,
    }
}

fn do_cmp(f1: impl AsRef<Path>, f2: impl AsRef<Path>) -> io::Result<bool> {
    let mut f1 = File::open(f1.as_ref())?;
    let mut f2 = File::open(f2.as_ref())?;
    loop {
        let mut buf1: [u8; BUFSIZE] = [0; BUFSIZE];
        let mut buf2: [u8; BUFSIZE] = [0; BUFSIZE];
        let len1 = f1.read(&mut buf1)?;
        let len2 = f2.read(&mut buf2)?;
        if len1 != len2 {
            return Ok(false);
        }
        let read_size = len1;
        if read_size == 0 {
            return Ok(true);
        }
        if &buf1[..read_size] != &buf2[..read_size] {
            return Ok(false);
        }
    }
}

#[derive(Debug)]
struct Signature {
    s_ifmt: u32,
    st_size: u64,
    st_mtime: f64,
}

impl Signature {
    fn canonicalize(&self) -> (u32, u64, [u8; 8]) {
        (self.s_ifmt, self.st_size, self.st_mtime.to_ne_bytes())
    }
}

impl PartialEq for Signature {
    fn eq(&self, other: &Self) -> bool {
        self.canonicalize() == other.canonicalize()
    }
}

impl Eq for Signature {}

impl Hash for Signature {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.canonicalize().hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile;

    #[test]
    fn test_stat() {
        let temp_dir = tempfile::tempdir().unwrap().into_path();
        let test_dir = dbg!(temp_dir.join("test_filecmp").join("test_stat"));

        if !test_dir.exists() {
            fs::create_dir_all(&test_dir).unwrap();
        }

        assert!(
            test_dir.is_dir(),
            "Test directory {} must be an existing folder",
            test_dir.display()
        );

        let foo_path = test_dir.join("foo.txt");
        let bar_path = test_dir.join("bar.txt");
        let baz_path = test_dir.join("baz.txt");

        let mut foo = File::create(&foo_path).unwrap(); // b"0123456789abcdeg"
        let mut bar = File::create(&bar_path).unwrap(); // b"0123456789abcdeg"
        let mut baz = File::create(&baz_path).unwrap(); // b"0123456789"

        let buf_digit: &[u8; 10] = b"0123456789";
        let buf_alphabet: &[u8; 6] = b"abcdeg";
        let buf: &[u8] = &[&buf_digit[..], &buf_alphabet[..]].concat();

        foo.write(buf).unwrap();
        bar.write(buf).unwrap();
        baz.write(buf_digit).unwrap();

        let shallow = true;
        assert!(cmp(&foo_path, &foo_path, shallow).unwrap());
        assert!(cmp(&bar_path, &bar_path, shallow).unwrap());
        assert!(cmp(&baz_path, &baz_path, shallow).unwrap());

        assert!(cmp(&foo_path, &bar_path, shallow).unwrap());
        assert!(!cmp(&foo_path, &baz_path, shallow).unwrap());
        assert!(!cmp(&bar_path, &baz_path, shallow).unwrap());

        let shallow = false;
        assert!(cmp(&foo_path, &foo_path, shallow).unwrap());
        assert!(cmp(&bar_path, &bar_path, shallow).unwrap());
        assert!(cmp(&baz_path, &baz_path, shallow).unwrap());

        assert!(cmp(&foo_path, &bar_path, shallow).unwrap());
        assert!(!cmp(&foo_path, &baz_path, shallow).unwrap());
        assert!(!cmp(&bar_path, &baz_path, shallow).unwrap());
    }
}
