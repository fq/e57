use crate::bitpack::BitPack;
use crate::bs_read::ByteStreamReadBuffer;
use crate::cv_section::CompressedVectorSectionHeader;
use crate::error::Converter;
use crate::packet::PacketHeader;
use crate::paged_reader::PagedReader;
use crate::Error;
use crate::PointCloud;
use crate::RawValues;
use crate::RecordDataType;
use crate::RecordValue;
use crate::Result;
use std::collections::VecDeque;
use std::io::{Read, Seek};

/// Iterate over all raw points of a point cloud for reading.
pub struct PointCloudReaderRaw<'a, T: Read + Seek> {
    pc: PointCloud,
    reader: &'a mut PagedReader<T>,
    byte_streams: Vec<ByteStreamReadBuffer>,
    read: u64,
    queues: Vec<VecDeque<RecordValue>>,
    buffer_sizes: Vec<usize>,
    buffer: Vec<u8>,
}

impl<'a, T: Read + Seek> PointCloudReaderRaw<'a, T> {
    pub(crate) fn new(pc: &PointCloud, reader: &'a mut PagedReader<T>) -> Result<Self> {
        reader
            .seek_physical(pc.file_offset)
            .read_err("Cannot seek to compressed vector header")?;
        let section_header = CompressedVectorSectionHeader::read(reader)?;
        reader
            .seek_physical(section_header.data_offset)
            .read_err("Cannot seek to packet header")?;

        Ok(Self {
            pc: pc.clone(),
            reader,
            read: 0,
            byte_streams: vec![ByteStreamReadBuffer::new(); pc.prototype.len()],
            queues: vec![VecDeque::new(); pc.prototype.len()],
            buffer_sizes: vec![0; pc.prototype.len()],
            buffer: Vec::new(),
        })
    }

    fn available_in_queue(&self) -> usize {
        if self.queues.is_empty() {
            return 0;
        }

        let mut av = usize::MAX;
        for q in &self.queues {
            let len = q.len();
            if len < av {
                av = len;
            }
        }
        av
    }

    fn pop_queue_point(&mut self) -> Result<RawValues> {
        let mut point = RawValues::with_capacity(self.pc.prototype.len());
        for i in 0..self.pc.prototype.len() {
            let value = self.queues[i]
                .pop_front()
                .internal_err("Failed to pop value for next point")?;
            point.push(value);
        }
        Ok(point)
    }

    fn advance(&mut self) -> Result<()> {
        let packet_header = PacketHeader::read(self.reader)?;
        match packet_header {
            PacketHeader::Index(_) => {
                Error::not_implemented("Index packets are not yet supported")?
            }
            PacketHeader::Ignored(_) => {
                Error::not_implemented("Ignored packets are not yet supported")?
            }
            PacketHeader::Data(header) => {
                if header.bytestream_count as usize != self.byte_streams.len() {
                    Error::invalid("Bytestream count does not match prototype size")?
                }

                for i in 0..header.bytestream_count as usize {
                    let mut buf = [0_u8; 2];
                    self.reader
                        .read_exact(&mut buf)
                        .read_err("Failed to read data packet buffer sizes")?;
                    let len = u16::from_le_bytes(buf) as usize;
                    self.buffer_sizes[i] = len;
                }

                for (i, bs) in self.buffer_sizes.iter().enumerate() {
                    self.buffer.resize(*bs, 0_u8);
                    self.reader
                        .read_exact(&mut self.buffer)
                        .read_err("Failed to read data packet buffers")?;
                    self.byte_streams[i].append(&self.buffer);
                }

                for (i, r) in self.pc.prototype.iter().enumerate() {
                    match r.data_type {
                        RecordDataType::Single { .. } => {
                            BitPack::unpack_singles(&mut self.byte_streams[i], &mut self.queues[i])?
                        }
                        RecordDataType::Double { .. } => {
                            BitPack::unpack_doubles(&mut self.byte_streams[i], &mut self.queues[i])?
                        }
                        RecordDataType::ScaledInteger { min, max, .. } => {
                            BitPack::unpack_scaled_ints(
                                &mut self.byte_streams[i],
                                min,
                                max,
                                &mut self.queues[i],
                            )?
                        }
                        RecordDataType::Integer { min, max } => BitPack::unpack_ints(
                            &mut self.byte_streams[i],
                            min,
                            max,
                            &mut self.queues[i],
                        )?,
                    };
                }
            }
        };

        self.reader
            .align()
            .read_err("Failed to align reader on next 4-byte offset after reading packet")?;

        Ok(())
    }
}

impl<'a, T: Read + Seek> Iterator for PointCloudReaderRaw<'a, T> {
    /// Each iterator item is a result for an extracted point.
    type Item = Result<RawValues>;

    /// Returns the next available point or None if the end was reached.
    fn next(&mut self) -> Option<Self::Item> {
        // Already read all points?
        if self.read >= self.pc.records {
            return None;
        }

        // Refill property queues if required
        if self.available_in_queue() < 1 {
            if let Err(err) = self.advance() {
                return Some(Err(err));
            }
        }

        // Try to read next point from properties queues
        if self.available_in_queue() < 1 {
            return None;
        }

        // Extract next point
        match self.pop_queue_point() {
            Ok(point) => {
                self.read += 1;
                Some(Ok(point))
            }
            Err(err) => Some(Err(err)),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let overall = self.pc.records;
        let remaining = overall - self.read;
        (remaining as usize, Some(remaining as usize))
    }
}
