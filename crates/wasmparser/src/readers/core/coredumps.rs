use crate::{BinaryReader, FromReader, Result};

/// The data portion of a custom section representing a core dump. Per the
/// tool-conventions repo, this section just specifies the executable name that
/// the core dump came from while the rest of the core dump information is
/// contained in a corestack custom section
///
/// # Examples
///
/// ```
/// use wasmparser::{ BinaryReader, CoreDumpSection, FromReader, Result };
/// let data: &[u8] = &[0x00, 0x09, 0x74, 0x65, 0x73, 0x74, 0x2e, 0x77, 0x61,
///      0x73, 0x6d];
/// let mut reader = BinaryReader::new(data);
/// let core = CoreDumpSection::from_reader(&mut reader).unwrap();
/// assert!(core.name == "test.wasm")
/// ```
pub struct CoreDumpSection<'a> {
    /// The name of the process that created the core dump
    pub name: &'a str,
}

impl<'a> FromReader<'a> for CoreDumpSection<'a> {
    fn from_reader(reader: &mut BinaryReader<'a>) -> Result<Self> {
        let pos = reader.original_position();
        if reader.read_u8()? != 0 {
            bail!(pos, "invalid start byte for core dump name");
        }
        let name = reader.read_string()?;
        Ok(CoreDumpSection { name })
    }
}

/// The data portion of a custom section representing a core dump stack. The
/// structure of this follows the coredump spec in the tool-conventions repo
///
/// # Examples
///
/// ```
/// let data: &[u8] = &[0x00, 0x04, 0x6d, 0x61, 0x69, 0x6e, 0x01, 0x00, 0x2a,
///     0x33, 0x01, 0x7f, 0x01, 0x01, 0x7f, 0x02];
/// use wasmparser::{ BinaryReader, CoreDumpStackSection, FromReader };
/// let mut reader = BinaryReader::new(data);
/// let corestack = CoreDumpStackSection::from_reader(&mut reader).unwrap();
/// assert!(corestack.name == "main");
/// assert!(corestack.frames.len() == 1);
/// let frame = &corestack.frames[0];
/// assert!(frame.funcidx == 42);
/// assert!(frame.codeoffset == 51);
/// assert!(frame.locals.len() == 1);
/// assert!(frame.stack.len() == 1);
/// ```
pub struct CoreDumpStackSection<'a> {
    /// The thread name
    pub name: &'a str,
    /// The stack frames for the core dump
    pub frames: Vec<CoreDumpStackFrame>,
}

impl<'a> FromReader<'a> for CoreDumpStackSection<'a> {
    fn from_reader(reader: &mut BinaryReader<'a>) -> Result<Self> {
        let pos = reader.original_position();
        if reader.read_u8()? != 0 {
            bail!(pos, "invalid start byte for core dump stack name");
        }
        let name = reader.read_string()?;
        let mut frames = vec![];
        for _ in 0..reader.read_var_u32()? {
            frames.push(CoreDumpStackFrame::from_reader(reader)?);
        }
        Ok(CoreDumpStackSection {
            name: name,
            frames: frames,
        })
    }
}

/// A single stack frame from a core dump
#[derive(Debug)]
pub struct CoreDumpStackFrame {
    /// The function index in the module
    pub funcidx: u32,
    /// The instruction's offset relative to the function's start
    pub codeoffset: u32,
    /// The locals for this stack frame (including function parameters)
    pub locals: Vec<CoreDumpValue>,
    /// The values on the stack
    pub stack: Vec<CoreDumpValue>,
}

impl<'a> FromReader<'a> for CoreDumpStackFrame {
    fn from_reader(reader: &mut BinaryReader<'a>) -> Result<Self> {
        let pos = reader.original_position();
        if reader.read_u8()? != 0 {
            bail!(pos, "invalid start byte for core dump stack frame");
        }
        let funcidx = reader.read_var_u32()?;
        let codeoffset = reader.read_var_u32()?;
        let mut locals = vec![];
        for _ in 0..reader.read_var_u32()? {
            locals.push(CoreDumpValue::from_reader(reader)?);
        }
        let mut stack = vec![];
        for _ in 0..reader.read_var_u32()? {
            stack.push(CoreDumpValue::from_reader(reader)?);
        }

        Ok(CoreDumpStackFrame {
            funcidx,
            codeoffset,
            locals,
            stack,
        })
    }
}

/// Local and stack values are encoded using one byte for the type (similar to
/// Wasm's Number Types) followed by bytes representing the actual value
/// See the tool-conventions repo for more details.
#[derive(Clone, Debug)]
pub enum CoreDumpValue {
    /// A missing value (usually missing because it was optimized out)
    Missing,
    /// An i32 value
    I32(i32),
    /// An i64 value
    I64(i64),
    /// An f32 value
    F32(f32),
    /// An f64 value
    F64(f64),
}

impl<'a> FromReader<'a> for CoreDumpValue {
    fn from_reader(reader: &mut BinaryReader<'a>) -> Result<Self> {
        let pos = reader.original_position();
        match reader.read_u8()? {
            0x01 => Ok(CoreDumpValue::Missing),
            0x7F => Ok(CoreDumpValue::I32(reader.read_var_i32()?)),
            0x7E => Ok(CoreDumpValue::I64(reader.read_var_i64()?)),
            0x7D => Ok(CoreDumpValue::F32(f32::from_bits(
                reader.read_f32()?.bits(),
            ))),
            0x7C => Ok(CoreDumpValue::F64(f64::from_bits(
                reader.read_f64()?.bits(),
            ))),
            _ => bail!(pos, "invalid CoreDumpValue type"),
        }
    }
}
