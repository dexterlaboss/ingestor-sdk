use {
    enum_iterator::{all, Sequence},
    std::io::{
        self,
        Write
    },
};

#[derive(Debug, Serialize, Deserialize, Sequence)]
pub enum CompressionMethod {
    NoCompression,
    Bzip2,
    Gzip,
    Zstd,
}

pub fn compress(method: CompressionMethod, data: &[u8]) -> Result<Vec<u8>, io::Error> {
    let mut compressed_data = bincode::serialize(&method).unwrap();
    compressed_data.extend(
        match method {
            CompressionMethod::Bzip2 => {
                let mut e = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::best());
                e.write_all(data)?;
                e.finish()?
            }
            CompressionMethod::Gzip => {
                let mut e =
                    flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
                e.write_all(data)?;
                e.finish()?
            }
            CompressionMethod::Zstd => {
                let mut e = zstd::stream::write::Encoder::new(Vec::new(), 0).unwrap();
                e.write_all(data)?;
                e.finish()?
            }
            CompressionMethod::NoCompression => data.to_vec(),
        }
            .into_iter(),
    );

    Ok(compressed_data)
}

pub fn compress_best(data: &[u8]) -> Result<Vec<u8>, io::Error> {
    let mut candidates = vec![];
    for method in all::<CompressionMethod>() {
        candidates.push(compress(method, data)?);
    }

    Ok(candidates
        .into_iter()
        .min_by(|a, b| a.len().cmp(&b.len()))
        .unwrap())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_compress() {
        let data = vec![0; 256];
        assert!(compress_best(&data).expect("compress_best").len() < data.len());
    }
}
