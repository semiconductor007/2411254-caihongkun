use std::{fs::File, io::Read, path::Path};

use crate::{
    errors::GitError,
    hash::get_hash_kind,
    internal::{object::types::ObjectType, pack::Pack},
};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PackStats {
    pub total: usize,
    pub commits: usize,
    pub trees: usize,
    pub blobs: usize,
    pub tags: usize,
    pub deltas: usize,
}

fn record_type(stats: &mut PackStats, t: ObjectType) {
    match t {
        ObjectType::Commit => stats.commits += 1,
        ObjectType::Tree => stats.trees += 1,
        ObjectType::Blob => stats.blobs += 1,
        ObjectType::Tag => stats.tags += 1,
        ObjectType::OffsetDelta | ObjectType::OffsetZstdelta | ObjectType::HashDelta => {
            stats.deltas += 1
        }
        _ => {}
    }
}

pub fn stat_pack<P: AsRef<Path>>(path: P) -> Result<PackStats, GitError> {
    let mut reader = std::io::BufReader::new(File::open(path.as_ref())?);
    let (object_num, _) = Pack::check_header(&mut reader)?;
    let mut stats = PackStats {
        total: object_num as usize,
        ..Default::default()
    };
    let mut offset = 12;
    for _ in 0..stats.total {
        if let Some(obj) = Pack::decode_pack_object(&mut reader, &mut offset)? {
            record_type(&mut stats, obj.object_type());
        }
    }
    let mut trailer = vec![0u8; get_hash_kind().size()];
    reader.read_exact(&mut trailer)?;
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::Write,
        path::PathBuf,
    };

    use crate::{
        errors::GitError,
        hash::{HashKind, set_hash_kind_for_test},
        internal::pack::stat::stat_pack,
    };

    fn small_sha1_pack() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests/data/packs/small-sha1.pack");
        p
    }

    #[test]
    fn stat_pack_ok() {
        let _guard = set_hash_kind_for_test(HashKind::Sha1);
        let stats = stat_pack(small_sha1_pack()).expect("stat_pack failed");
        assert!(stats.total > 0);
        assert_eq!(
            stats.commits + stats.trees + stats.blobs + stats.tags + stats.deltas,
            stats.total
        );
    }

    #[test]
    fn stat_pack_missing_file() {
        let err = stat_pack("/nonexistent/pack/file.pack").unwrap_err();
        assert!(matches!(err, GitError::IOError(_)));
    }

    #[test]
    fn stat_pack_bad_header() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("bad-pack-{}", std::process::id()));
        let mut f = File::create(&path).unwrap();
        f.write_all(b"BAD!").unwrap();
        drop(f);
        let err = stat_pack(&path).unwrap_err();
        let _ = fs::remove_file(&path);
        assert!(matches!(err, GitError::InvalidPackHeader(_)));
    }
}
