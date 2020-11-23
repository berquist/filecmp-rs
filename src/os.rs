use std::fs::{self};
use std::io::{self};
use std::path::Path;
use std::time::SystemTime;

#[cfg(windows)]
pub use nt::stat;

#[cfg(unix)]
pub use posix::stat;

#[derive(Debug)]
pub struct StatResult {
    pub st_mode: u32,
    st_ino: u64,
    st_dev: u64,
    st_nlink: u64,
    st_uid: u32,
    st_gid: u32,
    pub st_size: u64,
    pub st_atime: f64,
    pub st_mtime: f64,
    pub st_ctime: f64,
}

#[cfg(windows)]
mod nt {
    use super::*;

    pub fn stat(path: impl AsRef<Path>, follow_symlinks: bool) -> io::Result<StatResult> {
        use std::os::windows::fs::MetadataExt;

        let meta = fs_metadata(path, follow_symlinks)?;

        // // When use #![feature(windows_by_handle)] in nightly
        // let st_ino = meta.file_index().unwrap();
        // let st_dev = meta.volume_serial_number().unwrap() as u64;
        // let st_nlink = meta.number_of_links().unwrap() as u64;

        let st_ino = 0; // TODO: Not implemented in stable std::os::windows::fs::MetadataExt.
        let st_dev = 0; // TODO: Not implemented in stable std::os::windows::fs::MetadataExt.
        let st_nlink = 0; // TODO: Not implemented in stable std::os::windows::fs::MetadataExt.

        Ok(StatResult {
            st_mode: attributes_to_mode(meta.file_attributes()),
            st_ino,
            st_dev,
            st_nlink,
            st_uid: 0, // 0 on windows
            st_gid: 0, // 0 on windows
            st_size: meta.file_size(),
            st_atime: to_seconds_from_unix_epoch(meta.accessed()?),
            st_mtime: to_seconds_from_unix_epoch(meta.modified()?),
            st_ctime: to_seconds_from_unix_epoch(meta.created()?),
        })
    }

    fn attributes_to_mode(attr: u32) -> u32 {
        const FILE_ATTRIBUTE_DIRECTORY: u32 = 16;
        const FILE_ATTRIBUTE_READONLY: u32 = 1;
        const S_IFDIR: u32 = 0o040000;
        const S_IFREG: u32 = 0o100000;
        let mut m: u32 = 0;
        if attr & FILE_ATTRIBUTE_DIRECTORY == FILE_ATTRIBUTE_DIRECTORY {
            m |= S_IFDIR | 0o111; /* IFEXEC for user,group,other */
        } else {
            m |= S_IFREG;
        }
        if attr & FILE_ATTRIBUTE_READONLY == FILE_ATTRIBUTE_READONLY {
            m |= 0o444;
        } else {
            m |= 0o666;
        }
        m
    }
}

#[cfg(unix)]
mod posix {
    use super::*;
    use std::time::Duration;

    pub fn stat(path: impl AsRef<Path>, follow_symlinks: bool) -> io::Result<StatResult> {
        #[cfg(target_os = "android")]
        use std::os::android::fs::MetadataExt;
        #[cfg(target_os = "linux")]
        use std::os::linux::fs::MetadataExt;
        #[cfg(target_os = "macos")]
        use std::os::macos::fs::MetadataExt;
        #[cfg(target_os = "openbsd")]
        use std::os::openbsd::fs::MetadataExt;
        #[cfg(target_os = "redox")]
        use std::os::redox::fs::MetadataExt;

        let meta = fs_metadata(path, follow_symlinks)?;

        Ok(StatResult {
            st_mode: meta.st_mode(),
            st_ino: meta.st_ino(),
            st_dev: meta.st_dev(),
            st_nlink: meta.st_nlink(),
            st_uid: meta.st_uid(),
            st_gid: meta.st_gid(),
            st_size: meta.st_size(),
            st_atime: to_seconds_from_unix_epoch(meta.accessed()?),
            st_mtime: to_seconds_from_unix_epoch(meta.modified()?),
            st_ctime: to_seconds_from_nanos(meta.st_ctime(), meta.st_ctime_nsec()),
        })
    }

    fn to_seconds_from_nanos(secs: i64, nanos: i64) -> f64 {
        let duration = Duration::new(secs as u64, nanos as u32);
        duration.as_secs_f64()
    }
}

fn fs_metadata(path: impl AsRef<Path>, follow_symlinks: bool) -> io::Result<fs::Metadata> {
    if follow_symlinks {
        fs::metadata(path.as_ref())
    } else {
        fs::symlink_metadata(path.as_ref())
    }
}

fn to_seconds_from_unix_epoch(sys_time: SystemTime) -> f64 {
    match sys_time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs_f64(),
        Err(err) => -err.duration().as_secs_f64(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;

    #[test]
    fn test_stat() {
        let temp_dir = env::temp_dir();
        let test_dir = dbg!(temp_dir.join("test_filecmp").join("test_stat"));

        if !test_dir.exists() {
            fs::create_dir_all(&test_dir).unwrap();
        }

        assert!(
            test_dir.is_dir(),
            "Test directory {} must be an existed folder",
            test_dir.display()
        );

        let foo_path = test_dir.join("foo.txt");
        let bar_path = test_dir.join("bar.txt");
        let baz_path = test_dir.join("baz.txt");

        let mut foo = File::create(&foo_path).unwrap();
        let mut bar = File::create(&bar_path).unwrap();
        let mut baz = File::create(&baz_path).unwrap();

        let buf_digit: &[u8; 10] = b"0123456789";
        let buf_alphabet: &[u8; 6] = b"abcdeg";
        let buf: &[u8] = &[&buf_digit[..], &buf_alphabet[..]].concat();

        foo.write(buf).unwrap();
        bar.write(buf).unwrap();
        baz.write(buf_digit).unwrap();

        let _test_dir_stat = dbg!(stat(&test_dir, false).unwrap());
        let foo_stat = dbg!(stat(&foo_path, false).unwrap());
        let bar_stat = dbg!(stat(&bar_path, false).unwrap());
        let baz_stat = dbg!(stat(&baz_path, false).unwrap());

        // for st_mode
        assert_eq!(foo_stat.st_mode, bar_stat.st_mode);
        assert_eq!(foo_stat.st_mode, baz_stat.st_mode);

        // for st_size
        assert_eq!(foo_stat.st_size, bar_stat.st_size);
        assert_ne!(foo_stat.st_size, baz_stat.st_size);
    }
}
