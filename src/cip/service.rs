#[derive(Debug, Copy, Clone)]
pub enum CipService {
    ReadData = 0x4C,
    WriteData = 0x4D,
    MultipleService = 0x0A,
}
