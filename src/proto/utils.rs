use futures::prelude::*;

async fn read_bytes<R, const N: usize>(mut reader: R) -> std::io::Result<[u8; N]>
where
    R: AsyncRead + Unpin,
{
    let mut buf = [0u8; N];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

pub async fn read_u8<R>(reader: R) -> std::io::Result<u8>
where
    R: AsyncRead + Unpin,
{
    let bytes = read_bytes(reader).await?;
    Ok(u8::from_ne_bytes(bytes))
}

pub async fn read_u32_le<R>(reader: R) -> std::io::Result<u32>
where
    R: AsyncRead + Unpin,
{
    let bytes = read_bytes(reader).await?;
    Ok(u32::from_le_bytes(bytes))
}

pub async fn read_u16_le<R>(reader: R) -> std::io::Result<u16>
where
    R: AsyncRead + Unpin,
{
    let bytes = read_bytes(reader).await?;
    Ok(u16::from_le_bytes(bytes))
}

pub async fn read_u32_be<R>(reader: R) -> std::io::Result<u32>
where
    R: AsyncRead + Unpin,
{
    let bytes = read_bytes(reader).await?;
    Ok(u32::from_be_bytes(bytes))
}
