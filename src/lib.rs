use tokio::io::{AsyncRead, AsyncReadExt,AsyncWrite,AsyncWriteExt};
use std::io::Result;

#[derive(Debug)]
pub enum Frame {
    ReadRequest(ReadRequest),
    ReadResponse(ReadResponse),
    WriteRequest(WriteRequest),
    WriteResponse(WriteResponse),
}

impl Frame {
    pub async fn serialize<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> Result<()> {
        let kind = match self {
            Frame::ReadRequest(_) => 0,
            Frame::ReadResponse(_) => 1,
            Frame::WriteRequest(_) => 2,
            Frame::WriteResponse(_) => 3,
        };

        writer.write_u32(kind).await?;

        match self {
            Frame::ReadRequest(req) => req.serialize(writer).await,
            Frame::ReadResponse(req) => req.serialize(writer).await,
            Frame::WriteRequest(req) => req.serialize(writer).await,
            Frame::WriteResponse(req) => req.serialize(writer).await,
        }
    }

    pub async fn deserialize<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Self> {
        let kind = reader.read_u32().await?;
        Ok(match kind {
            0 => Frame::ReadRequest(ReadRequest::deserialize(reader).await?),
            1 => Frame::ReadResponse(ReadResponse::deserialize(reader).await?),
            2 => Frame::WriteRequest(WriteRequest::deserialize(reader).await?),
            3 => Frame::WriteResponse(WriteResponse::deserialize(reader).await?),
            _ => panic!("Unknown frame kind"),
        })
    }
}

#[derive(Debug)]
pub struct ReadRequest {
    pub address: u32,
    pub count: u32,
}

impl ReadRequest {
    pub async fn serialize<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32(self.address).await?;
        writer.write_u32(self.count).await?;
        Ok(())
    }

    pub async fn deserialize<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Self> {
        let address = reader.read_u32().await?;
        let count = reader.read_u32().await?;
        Ok(Self { address, count })
    }
}

#[derive(Debug)]
pub struct ReadResponse {
    pub bytes: Vec<u8>,
}

impl ReadResponse {
    pub async fn serialize<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32(self.bytes.len() as u32).await?;
        writer.write_all(&self.bytes).await?;
        Ok(())
    }

    pub async fn deserialize<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Self> {
        let count = reader.read_u32().await?;
        let mut bytes = Vec::with_capacity(count as usize);
        bytes.resize(count as usize, 0);
        reader.read_exact(&mut bytes).await?;
        Ok(Self { bytes })
    }
}

#[derive(Debug)]
pub struct WriteRequest {
    pub address: u32,
    pub bytes: Vec<u8>,
}

impl WriteRequest {
    pub async fn serialize<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32(self.address).await?;
        writer.write_u32(self.bytes.len() as u32).await?;
        writer.write_all(&self.bytes).await?;
        Ok(())
    }

    pub async fn deserialize<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Self> {
        let address = reader.read_u32().await?;
        let count = reader.read_u32().await?;
        let mut bytes = Vec::with_capacity(count as usize);
        bytes.resize(count as usize, 0);
        reader.read_exact(&mut bytes).await?;
        Ok(Self { address, bytes })
    }
}

#[derive(Debug)]
pub struct WriteResponse { }

impl WriteResponse {
    pub async fn serialize<W: AsyncWrite + Unpin>(&self, _writer: &mut W) -> Result<()> {
        Ok(())
    }
    
    pub async fn deserialize<R: AsyncRead + Unpin>(_reader: &mut R) -> Result<Self> {
        Ok(Self {})
    }
}
