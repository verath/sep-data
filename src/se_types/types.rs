pub type SETypeU8 = u8;
pub type SETypeU16 = u16;
pub type SETypeU32 = u32;
pub type SETypeS32 = i32;
pub type SETypeU64 = u64;
pub type SETypeF32 = f32;
pub type SETypeF64 = f64;
pub type SETypeVector = Vec<SEVectorItem>;
pub type SETypeStruct = Vec<SEStructItem>;
pub type SETypePoint2D = Point2D;
pub type SETypeVect2D = Vect2D;
pub type SETypePoint3D = Point3D;
pub type SETypeVect3D = Vect3D;
pub type SETypeString = String;
pub type SETypeQuaternion = Quaternion;
pub type SETypeUserMarker = Option<UserMarker>;
pub type SETypeWorldIntersection = Option<WorldIntersection>;
pub type SETypeWorldIntersections = Vec<WorldIntersection>;
pub type SETypeFloat = SETypeF64;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u16)]
pub enum SETypeId {
    U8 = 0x00,
    U16 = 0x01,
    U32 = 0x02,
    S32 = 0x03,
    U64 = 0x04,
    F64 = 0x05,
    Point2D = 0x06,
    Vect2D = 0x07,
    Point3D = 0x08,
    Vect3D = 0x09,
    String = 0x0A,
    Vector = 0x0B,
    Struct = 0x0C,
    WorldIntersection = 0x0D,
    WorldIntersections = 0x0E,
    PacketHeader = 0x0F,
    SubPacketHeader = 0x10,
    F32 = 0x11,
    Matrix3X3 = 0x12,
    Matrix2x2 = 0x13,
    Quaternion = 0x14,
    UserMarker = 0x15,
}

// x, y
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Point2D(pub f64, pub f64);

// x, y
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Vect2D(pub f64, pub f64);

// x, y, z
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Point3D(pub f64, pub f64, pub f64);

// x, y, z
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Vect3D(pub f64, pub f64, pub f64);

// w, x, y, z
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Quaternion(pub f64, pub f64, pub f64, pub f64);

#[derive(Debug, PartialEq, Clone)]
pub struct WorldIntersection {
    pub world_point: Point3D,
    pub object_point: Point3D,
    pub object_name: String,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct UserMarker {
    pub error: i32,
    pub time_stamp: u64,
    pub camera_clock: u64,
    pub camera_idx: u8,
    pub data: u64,
}

#[derive(Debug, PartialEq, Clone)]
pub enum SEVariant {
    U8(SETypeU8),
    U16(SETypeU16),
    U32(SETypeU32),
    S32(SETypeS32),
    U64(SETypeU64),
    F64(SETypeF64),
    Point2D(SETypePoint2D),
    Vect2D(SETypeVect2D),
    Point3D(SETypePoint3D),
    Vect3D(SETypeVect3D),
    String(SETypeString),
    Vector(SETypeVector),
    Struct(SETypeStruct),
    WorldIntersection(SETypeWorldIntersection),
    WorldIntersections(SETypeWorldIntersections),
    // PacketHeader(SETypePacketHeader),
    // SubPacketHeader(SETypeSubPacketHeader),
    F32(SETypeF32),
    // Matrix3X3(SETypeMatrix3X3),
    // Matrix2x2(SETypeMatrix2x2),
    Quaternion(SETypeQuaternion),
    UserMarker(SETypeUserMarker),
}

pub type SEVectorItem = SEVariant;

// key, value
#[derive(Debug, PartialEq, Clone)]
pub struct SEStructItem(pub String, pub SEVariant);

impl std::convert::TryFrom<u16> for SETypeId {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            x if x == SETypeId::U8 as u16 => Ok(SETypeId::U8),
            x if x == SETypeId::U16 as u16 => Ok(SETypeId::U16),
            x if x == SETypeId::U32 as u16 => Ok(SETypeId::U32),
            x if x == SETypeId::S32 as u16 => Ok(SETypeId::S32),
            x if x == SETypeId::U64 as u16 => Ok(SETypeId::U64),
            x if x == SETypeId::F64 as u16 => Ok(SETypeId::F64),
            x if x == SETypeId::Point2D as u16 => Ok(SETypeId::Point2D),
            x if x == SETypeId::Vect2D as u16 => Ok(SETypeId::Vect2D),
            x if x == SETypeId::Point3D as u16 => Ok(SETypeId::Point3D),
            x if x == SETypeId::Vect3D as u16 => Ok(SETypeId::Vect3D),
            x if x == SETypeId::String as u16 => Ok(SETypeId::String),
            x if x == SETypeId::Vector as u16 => Ok(SETypeId::Vector),
            x if x == SETypeId::Struct as u16 => Ok(SETypeId::Struct),
            x if x == SETypeId::WorldIntersection as u16 => Ok(SETypeId::WorldIntersection),
            x if x == SETypeId::WorldIntersections as u16 => Ok(SETypeId::WorldIntersections),
            x if x == SETypeId::PacketHeader as u16 => Ok(SETypeId::PacketHeader),
            x if x == SETypeId::SubPacketHeader as u16 => Ok(SETypeId::SubPacketHeader),
            x if x == SETypeId::F32 as u16 => Ok(SETypeId::F32),
            x if x == SETypeId::Matrix3X3 as u16 => Ok(SETypeId::Matrix3X3),
            x if x == SETypeId::Matrix2x2 as u16 => Ok(SETypeId::Matrix2x2),
            x if x == SETypeId::Quaternion as u16 => Ok(SETypeId::Quaternion),
            x if x == SETypeId::UserMarker as u16 => Ok(SETypeId::UserMarker),
            _ => Err(()),
        }
    }
}
