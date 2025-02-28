use {
    enum_iterator::{Sequence},
    std::io::{self, BufReader, Read},
};

#[derive(Debug, Serialize, Deserialize, Sequence)]
pub enum CompressionMethod {
    NoCompression,
    Bzip2,
    Gzip,
    Zstd,
}

fn decompress_reader<'a, R: Read + 'a>(
    method: CompressionMethod,
    stream: R,
) -> Result<Box<dyn Read + 'a>, io::Error> {
    let buf_reader = BufReader::new(stream);
    let decompress_reader: Box<dyn Read> = match method {
        CompressionMethod::Bzip2 => Box::new(bzip2::bufread::BzDecoder::new(buf_reader)),
        CompressionMethod::Gzip => Box::new(flate2::read::GzDecoder::new(buf_reader)),
        CompressionMethod::Zstd => Box::new(zstd::stream::read::Decoder::new(buf_reader)?),
        CompressionMethod::NoCompression => Box::new(buf_reader),
    };
    Ok(decompress_reader)
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, io::Error> {
    let method_size = bincode::serialized_size(&CompressionMethod::NoCompression).unwrap();
    if (data.len() as u64) < method_size {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("data len too small: {}", data.len()),
        ));
    }
    let method = bincode::deserialize(&data[..method_size as usize]).map_err(|err| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("method deserialize failed: {err}"),
        )
    })?;

    let mut reader = decompress_reader(method, &data[method_size as usize..])?;
    let mut uncompressed_data = vec![];
    reader.read_to_end(&mut uncompressed_data)?;
    Ok(uncompressed_data)
}
