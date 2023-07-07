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
use stat::{S_IFMT, S_ISDIR, S_ISREG};

const BUFSIZE: usize = 8 * 1024;
const FOLLOW_SYMLINKS_DEFAULT: bool = true;
const MAX_CACHE_SIZE: usize = 100;
const DEFAULT_IGNORES: [&str; 8] = [
    "RCS",
    "CVS",
    "tags",
    ".git",
    ".hg",
    ".bzr",
    "_darcs",
    "__pycache__",
];
const CURDIR: &str = ".";
const PARDIR: &str = "..";

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
/// let td = tempfile::tempdir().unwrap();
/// let temp_dir = td.path().to_path_buf();
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
///
/// td.close().unwrap();
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

/// Compare common files in two directories.
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
    dir1: A,
    dir2: B,
    common: D,
    shallow: bool,
) -> io::Result<(Vec<PathBuf>, Vec<PathBuf>, Vec<PathBuf>)>
where
    A: AsRef<Path>,
    B: AsRef<Path>,
    C: AsRef<Path>,
    D: AsRef<[C]>,
{
    let mut same = Vec::new();
    let mut diff = Vec::new();
    let mut funny = Vec::new();
    for x in common.as_ref().iter() {
        let ax = dir1.as_ref().join(x);
        let bx = dir2.as_ref().join(x);
        match cmp(&ax, &bx, shallow) {
            Ok(true) => same.push(x.as_ref().to_path_buf()),
            Ok(false) => diff.push(x.as_ref().to_path_buf()),
            Err(_) => funny.push(x.as_ref().to_path_buf()),
        }
    }
    Ok((same, diff, funny))
}

/// A struct that manages the comparison of 2 directories.
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
pub struct DirCmp {
    left: PathBuf,
    right: PathBuf,
    // hide: Vec<PathBuf>,
    // ignore: Vec<PathBuf>,
    left_list: Vec<PathBuf>,
    right_list: Vec<PathBuf>,
    common: Vec<String>,
    left_only: Vec<String>,
    right_only: Vec<String>,
    common_dirs: Vec<String>,
    common_files: Vec<String>,
    common_funny: Vec<String>,
    same_files: Vec<String>,
    diff_files: Vec<String>,
    funny_files: Vec<String>,
    // subdirs: Vec<PathBuf>,
}

impl DirCmp {
    pub fn new(
        left: impl AsRef<Path>,
        right: impl AsRef<Path>,
        // ignore: Option<Vec<String>>,
        // hide: Option<Vec<String>>,
    ) -> Self {
        let left = left.as_ref().to_path_buf();
        let right = right.as_ref().to_path_buf();
        // let ignore = if ignore.is_some() {
        //     ignore.unwrap().iter().map(|x| PathBuf::from(x)).collect()
        // } else {
        //     DEFAULT_IGNORES.iter().map(|&x| PathBuf::from(x)).collect()
        // };
        // let hide = if hide.is_some() {
        //     hide.unwrap().iter().map(|x| PathBuf::from(x)).collect()
        // } else {
        //     vec![CURDIR, PARDIR]
        //         .iter()
        //         .map(|&x| PathBuf::from(x))
        //         .collect()
        // };
        // let ignore = DEFAULT_IGNORES.iter().map(|&x| PathBuf::from(x)).collect();
        // let hide = vec![CURDIR, PARDIR]
        //     .iter()
        //     .map(|&x| PathBuf::from(x))
        //     .collect();
        let ignore = DEFAULT_IGNORES.iter().map(|&x| x).collect::<Vec<_>>();
        let hide = vec![CURDIR, PARDIR];
        let mut left_list_full: Vec<_> = left
            .read_dir()
            .unwrap()
            .map(|der| der.unwrap().path())
            .filter(|der| !ignore.contains(der.file_name().unwrap().to_str().as_ref().unwrap()))
            .filter(|der| !hide.contains(der.file_name().unwrap().to_str().as_ref().unwrap()))
            .collect();
        left_list_full.sort();
        let mut right_list_full: Vec<_> = right
            .read_dir()
            .unwrap()
            .map(|der| der.unwrap().path())
            .filter(|der| !ignore.contains(der.file_name().unwrap().to_str().as_ref().unwrap()))
            .filter(|der| !hide.contains(der.file_name().unwrap().to_str().as_ref().unwrap()))
            .collect();
        right_list_full.sort();
        let left_names = left_list_full
            .iter()
            .map(|pb| String::from(pb.strip_prefix(&left).unwrap().to_str().unwrap()))
            .collect::<Vec<_>>();
        let right_names = right_list_full
            .iter()
            .map(|pb| String::from(pb.strip_prefix(&right).unwrap().to_str().unwrap()))
            .collect::<Vec<_>>();
        let common = left_names
            .iter()
            .filter(|&ln| right_names.contains(ln))
            .map(|n| n.clone())
            .collect::<Vec<_>>();
        let left_only = left_names
            .iter()
            .filter(|name| !common.contains(name))
            .map(|n| n.clone())
            .collect::<Vec<_>>();
        let right_only = right_names
            .iter()
            .filter(|name| !common.contains(name))
            .map(|n| n.clone())
            .collect::<Vec<_>>();
        let mut common_dirs = Vec::new();
        let mut common_files = Vec::new();
        let mut common_funny = Vec::new();
        for x in &common {
            match (
                os::stat(&left.join(x), FOLLOW_SYMLINKS_DEFAULT),
                os::stat(&right.join(x), FOLLOW_SYMLINKS_DEFAULT),
            ) {
                (Ok(left_stat), Ok(right_stat)) => {
                    let left_type = S_IFMT(left_stat.st_mode);
                    let right_type = S_IFMT(right_stat.st_mode);
                    if left_type != right_type {
                        common_funny.push(x.clone());
                    } else if S_ISDIR(left_type) {
                        common_dirs.push(x.clone());
                    } else if S_ISREG(left_type) {
                        common_files.push(x.clone());
                    } else {
                        common_funny.push(x.clone());
                    }
                }
                _ => {
                    common_funny.push(x.clone());
                }
            }
        }
        let xx = cmpfiles(&left, &right, &common_files, true).unwrap();
        let same_files =
            xx.0.iter()
                .map(|pb| pb.clone().into_os_string().into_string().unwrap())
                .collect();
        let diff_files =
            xx.1.iter()
                .map(|pb| pb.clone().into_os_string().into_string().unwrap())
                .collect();
        let funny_files =
            xx.2.iter()
                .map(|pb| pb.clone().into_os_string().into_string().unwrap())
                .collect();
        DirCmp {
            left,
            right,
            // hide,
            // ignore,
            left_list: left_list_full,
            right_list: right_list_full,
            common,
            left_only,
            right_only,
            common_dirs,
            common_files,
            common_funny,
            same_files,
            diff_files,
            funny_files,
        }
    }

    // fn strip_prefix(v: &Vec<PathBuf>, prefix: &Path) -> Vec<&str> {
    //     v.iter()
    //         .map(|&pb| pb.strip_prefix(prefix).unwrap().to_str().unwrap())
    //         .collect::<Vec<_>>();
    // }

    pub fn report_full_closure(&self) {
        unimplemented!()
    }

    pub fn report(&self) {
        unimplemented!()
    }
}

fn filter<T: Eq + Clone>(flist: &Vec<T>, skip: &Vec<T>) -> Vec<T> {
    flist
        .iter()
        .filter(|item| skip.contains(item))
        .cloned()
        .collect()
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

    fn create_and_verify(root: &PathBuf, name: &str) -> PathBuf {
        let new_dir = dbg!(root.join(name));
        if !new_dir.exists() {
            fs::create_dir_all(&new_dir).unwrap();
        }

        assert!(
            new_dir.is_dir(),
            "New directory {} must be an existing folder",
            new_dir.display()
        );

        new_dir
    }

    #[test]
    fn test_cmp() {
        let td = tempfile::tempdir().unwrap();
        let temp_dir = td.path().to_path_buf();
        let test_dir = temp_dir.join("test_filecmp");
        let test_dir = create_and_verify(&test_dir, "test_stat");

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

        td.close().unwrap();
    }

    fn get_sorted_names(v: &Vec<PathBuf>) -> Vec<&str> {
        let mut lst = v
            .iter()
            .map(|pbuf| pbuf.file_name().unwrap().to_str().unwrap())
            .collect::<Vec<_>>();
        lst.sort();
        lst
    }

    #[test]
    fn test_dircmp() {
        let td = tempfile::tempdir().unwrap();
        let temp_dir = td.path().to_path_buf();
        let dir = create_and_verify(&temp_dir, "dir");
        let dir_same = create_and_verify(&temp_dir, "dir_same");
        let dir_diff = create_and_verify(&temp_dir, "dir_diff");
        // Another dir is created under dir_same, but it has a name from the
        // ignored list so it should not affect testing results.
        let dir_ignored = create_and_verify(&dir_same, ".git");

        {
            let data = "Contents of file go here.\n";
            let dirs = vec![
                dir.clone(),
                dir_same.clone(),
                dir_diff.clone(),
                dir_ignored.clone(),
            ];
            let name_d = "subdir";
            let name_f = "file";
            for d in dirs {
                std::fs::create_dir(d.join(&name_d)).unwrap();
                let fp = d.join(&name_f);
                let mut f = File::create(&fp).unwrap();
                write!(f, "{}", data).expect("write failed");
            }
            let data2 = "An extra file.\n";
            let fp = dir_diff.join("file2");
            let mut f = File::create(&fp).unwrap();
            write!(f, "{}", data2).expect("write failed");
        }

        // Check attributes for comparison of two identical directories
        let left_dir = dir.clone();
        let right_dir = dir_same.clone();
        let result = DirCmp::new(left_dir.clone(), right_dir.clone());
        assert_eq!(result.left, left_dir);
        assert_eq!(result.right, right_dir);
        assert_eq!(get_sorted_names(&result.left_list), vec!["file", "subdir"]);
        assert_eq!(get_sorted_names(&result.right_list), vec!["file", "subdir"]);
        let mut result_common = result.common;
        result_common.sort();
        assert_eq!(result_common, vec!["file", "subdir"]);
        let mut result_left_only = result.left_only;
        result_left_only.sort();
        assert_eq!(result_left_only, Vec::<String>::new());
        let mut result_right_only = result.right_only;
        result_right_only.sort();
        assert_eq!(result_right_only, Vec::<String>::new());
        let mut result_common_dirs = result.common_dirs;
        result_common_dirs.sort();
        assert_eq!(result_common_dirs, vec!["subdir"]);

        td.close().unwrap();
    }
}
