//! Constants/functions for interpreting results of os.stat() and os.lstat().
#![allow(dead_code)]
#![allow(non_snake_case)]

// Indices for stat struct members in the tuple returned by os.stat()
pub const ST_MODE: usize  = 0;
pub const ST_INO: usize   = 1;
pub const ST_DEV: usize   = 2;
pub const ST_NLINK: usize = 3;
pub const ST_UID: usize   = 4;
pub const ST_GID: usize   = 5;
pub const ST_SIZE: usize  = 6;
pub const ST_ATIME: usize = 7;
pub const ST_MTIME: usize = 8;
pub const ST_CTIME: usize = 9;


// Extract bits from the mode

/// Return the portion of the file's mode that can be set by os.chmod().
pub fn S_IMODE(mode: u32) -> u32 {
    mode & 0o7777
}

/// Return the portion of the file's mode that describes the file type.
pub fn S_IFMT(mode: u32) -> u32 {
    mode & 0o170000
}

// Constants used as S_IFMT() for various file types
// (not all are implemented on all systems)

pub const S_IFDIR: u32 = 0o040000; // directory
pub const S_IFCHR: u32 = 0o020000; // character device
pub const S_IFBLK: u32 = 0o060000; // block device
pub const S_IFREG: u32 = 0o100000; // regular file
pub const S_IFIFO: u32 = 0o010000; // fifo (named pipe)
pub const S_IFLNK: u32 = 0o120000; // symbolic link
pub const S_IFSOCK: u32 = 0o140000; // socket file
                                // Fallbacks for uncommon platform-specific constants
pub const S_IFDOOR: u32 = 0;
pub const S_IFPORT: u32 = 0;
pub const S_IFWHT: u32 = 0;

// Functions to test for each file type

/// Return True if mode is from a directory.
pub fn S_ISDIR(mode: u32) -> bool {
    S_IFMT(mode) == S_IFDIR
}

/// Return True if mode is from a character special device file.
pub fn S_ISCHR(mode: u32) -> bool {
    S_IFMT(mode) == S_IFCHR
}

/// Return True if mode is from a block special device file.
pub fn S_ISBLK(mode: u32) -> bool {
    S_IFMT(mode) == S_IFBLK
}

/// Return True if mode is from a regular file.
pub fn S_ISREG(mode: u32) -> bool {
    S_IFMT(mode) == S_IFREG
}

/// Return True if mode is from a FIFO (named pipe).
pub fn S_ISFIFO(mode: u32) -> bool {
    S_IFMT(mode) == S_IFIFO
}

/// Return True if mode is from a symbolic link.
pub fn S_ISLNK(mode: u32) -> bool {
    S_IFMT(mode) == S_IFLNK
}

/// Return True if mode is from a socket.
pub fn S_ISSOCK(mode: u32) -> bool {
    S_IFMT(mode) == S_IFSOCK
}

/// Return True if mode is from a door.
pub fn S_ISDOOR(_mode: u32) -> bool {
    false
}

/// Return True if mode is from an event port.
pub fn S_ISPORT(_mode: u32) -> bool {
    false
}

/// Return True if mode is from a whiteout.
pub fn S_ISWHT(_mode: u32) -> bool {
    false
}

// Names for permission bits
pub const S_ISUID: u32 = 0o4000; // set UID bit
pub const S_ISGID: u32 = 0o2000; // set GID bit
pub const S_ENFMT: u32 = S_ISGID; // file locking enforcement
pub const S_ISVTX: u32 = 0o1000; // sticky bit
pub const S_IREAD: u32 = 0o0400; // Unix V7 synonym for S_IRUSR
pub const S_IWRITE: u32 = 0o0200; // Unix V7 synonym for S_IWUSR
pub const S_IEXEC: u32 = 0o0100; // Unix V7 synonym for S_IXUSR
pub const S_IRWXU: u32 = 0o0700; // mask for owner permissions
pub const S_IRUSR: u32 = 0o0400; // read by owner
pub const S_IWUSR: u32 = 0o0200; // write by owner
pub const S_IXUSR: u32 = 0o0100; // execute by owner
pub const S_IRWXG: u32 = 0o0070; // mask for group permissions
pub const S_IRGRP: u32 = 0o0040; // read by group
pub const S_IWGRP: u32 = 0o0020; // write by group
pub const S_IXGRP: u32 = 0o0010; // execute by group
pub const S_IRWXO: u32 = 0o0007; // mask for others (not in group) permissions
pub const S_IROTH: u32 = 0o0004; // read by others
pub const S_IWOTH: u32 = 0o0002; // write by others
pub const S_IXOTH: u32 = 0o0001; // execute by others

// Names for file flags
pub const UF_NODUMP: u32 = 0x00000001; // do not dump file
pub const UF_IMMUTABLE: u32 = 0x00000002; // file may not be changed
pub const UF_APPEND: u32 = 0x00000004; // file may only be appended to
pub const UF_OPAQUE: u32 = 0x00000008; // directory is opaque when viewed through a union stack
pub const UF_NOUNLINK: u32 = 0x00000010; // file may not be renamed or deleted
pub const UF_COMPRESSED: u32 = 0x00000020; // OS X: file is hfs-compressed
pub const UF_HIDDEN: u32 = 0x00008000; // OS X: file should not be displayed
pub const SF_ARCHIVED: u32 = 0x00010000; // file may be archived
pub const SF_IMMUTABLE: u32 = 0x00020000; // file may not be changed
pub const SF_APPEND: u32 = 0x00040000; // file may only be appended to
pub const SF_NOUNLINK: u32 = 0x00100000; // file may not be renamed or deleted
pub const SF_SNAPSHOT: u32 = 0x00200000; // file is a snapshot file


/// Convert a file's mode to a string of the form '-rwxrwxrwx'.
pub fn filemode(mode: u32) -> String {
    let filemode_table: Vec<Vec<(u32, char)>> = vec![
        vec![
            (S_IFLNK, 'l'),
            (S_IFSOCK, 's'), // Must appear before IFREG and IFDIR as IFSOCK == IFREG | IFDIR
            (S_IFREG, '-'),
            (S_IFBLK, 'b'),
            (S_IFDIR, 'd'),
            (S_IFCHR, 'c'),
            (S_IFIFO, 'p'),
        ],
        vec![(S_IRUSR, 'r')],
        vec![(S_IWUSR, 'w')],
        vec![(S_IXUSR | S_ISUID, 's'), (S_ISUID, 'S'), (S_IXUSR, 'x')],
        vec![(S_IRGRP, 'r')],
        vec![(S_IWGRP, 'w')],
        vec![(S_IXGRP | S_ISGID, 's'), (S_ISGID, 'S'), (S_IXGRP, 'x')],
        vec![(S_IROTH, 'r')],
        vec![(S_IWOTH, 'w')],
        vec![(S_IXOTH | S_ISVTX, 't'), (S_ISVTX, 'T'), (S_IXOTH, 'x')],
    ];

    let mut perm: Vec<char> = Vec::new();
    for table in filemode_table {
        for (bit, char) in table {
            if mode & bit == bit {
                perm.push(char);
                break;
            } else {
                perm.push('-');
            }
        }
    }

    perm.iter().collect()
}


// Windows FILE_ATTRIBUTE constants for interpreting os.stat()'s
// "st_file_attributes" member
pub const FILE_ATTRIBUTE_ARCHIVE: u32 = 32;
pub const FILE_ATTRIBUTE_COMPRESSED: u32 = 2048;
pub const FILE_ATTRIBUTE_DEVICE: u32 = 64;
pub const FILE_ATTRIBUTE_DIRECTORY: u32 = 16;
pub const FILE_ATTRIBUTE_ENCRYPTED: u32 = 16384;
pub const FILE_ATTRIBUTE_HIDDEN: u32 = 2;
pub const FILE_ATTRIBUTE_INTEGRITY_STREAM: u32 = 32768;
pub const FILE_ATTRIBUTE_NORMAL: u32 = 128;
pub const FILE_ATTRIBUTE_NOT_CONTENT_INDEXED: u32 = 8192;
pub const FILE_ATTRIBUTE_NO_SCRUB_DATA: u32 = 131072;
pub const FILE_ATTRIBUTE_OFFLINE: u32 = 4096;
pub const FILE_ATTRIBUTE_READONLY: u32 = 1;
pub const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 1024;
pub const FILE_ATTRIBUTE_SPARSE_FILE: u32 = 512;
pub const FILE_ATTRIBUTE_SYSTEM: u32 = 4;
pub const FILE_ATTRIBUTE_TEMPORARY: u32 = 256;
pub const FILE_ATTRIBUTE_VIRTUAL: u32 = 65536;
