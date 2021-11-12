use crate::se_types::*;
use nom::{
    bytes::streaming::{tag, take},
    combinator::{all_consuming, eof, map, map_parser, map_res},
    multi::{count, many_till},
    number::complete::{be_f32, be_f64, be_i32, be_u16, be_u32, be_u64, be_u8},
    sequence::tuple,
    IResult,
};
use std::convert::{TryFrom, TryInto};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
#[error("Parse failed")]
pub struct ParseFailedError {}

pub const PACKET_HEADER_SIZE: usize = 4 + 2 + 2;

#[derive(Debug, PartialEq)]
pub struct PacketHeader {
    pub length: u16,
}

#[derive(Debug, PartialEq)]
struct SubPacketHeader {
    id: SEOutputDataId,
    length: u16,
}

fn parse_u8(i: &[u8]) -> IResult<&[u8], u8> {
    be_u8(i)
}

fn parse_u16(i: &[u8]) -> IResult<&[u8], u16> {
    be_u16(i)
}

fn parse_u32(i: &[u8]) -> IResult<&[u8], u32> {
    be_u32(i)
}

fn parse_s32(i: &[u8]) -> IResult<&[u8], i32> {
    be_i32(i)
}

fn parse_u64(i: &[u8]) -> IResult<&[u8], u64> {
    be_u64(i)
}

fn parse_f32(i: &[u8]) -> IResult<&[u8], f32> {
    be_f32(i)
}

fn parse_f64(i: &[u8]) -> IResult<&[u8], f64> {
    be_f64(i)
}

fn parse_point_2d(i: &[u8]) -> IResult<&[u8], Point2D> {
    let (i, x) = parse_f64(i)?;
    let (i, y) = parse_f64(i)?;
    Ok((i, Point2D(x, y)))
}

fn parse_vect_2d(i: &[u8]) -> IResult<&[u8], Vect2D> {
    let (i, x) = parse_f64(i)?;
    let (i, y) = parse_f64(i)?;
    Ok((i, Vect2D(x, y)))
}

fn parse_point_3d(i: &[u8]) -> IResult<&[u8], Point3D> {
    let (i, x) = parse_f64(i)?;
    let (i, y) = parse_f64(i)?;
    let (i, z) = parse_f64(i)?;
    Ok((i, Point3D(x, y, z)))
}

fn parse_vect_3d(i: &[u8]) -> IResult<&[u8], Vect3D> {
    let (i, x) = parse_f64(i)?;
    let (i, y) = parse_f64(i)?;
    let (i, z) = parse_f64(i)?;
    Ok((i, Vect3D(x, y, z)))
}

fn parse_string(i: &[u8]) -> IResult<&[u8], String> {
    let (i, length) = parse_u16(i)?;
    let length = length as usize;
    map_res(count(parse_u8, length), |chars: Vec<u8>| {
        String::from_utf8(chars)
    })(i)
}

#[allow(clippy::many_single_char_names)]
fn parse_quaternion(i: &[u8]) -> IResult<&[u8], Quaternion> {
    let (i, w) = parse_f64(i)?;
    let (i, x) = parse_f64(i)?;
    let (i, y) = parse_f64(i)?;
    let (i, z) = parse_f64(i)?;
    Ok((i, Quaternion(w, x, y, z)))
}

fn parse_world_intersection_item(i: &[u8]) -> IResult<&[u8], WorldIntersection> {
    let (i, world_point) = parse_point_3d(i)?;
    let (i, object_point) = parse_point_3d(i)?;
    let (i, object_name) = parse_string(i)?;
    Ok((
        i,
        WorldIntersection {
            world_point,
            object_point,
            object_name,
        },
    ))
}

fn parse_world_intersection(i: &[u8]) -> IResult<&[u8], Option<WorldIntersection>> {
    let (i, exists) = parse_u16(i)?;
    match exists {
        0 => Ok((i, None)),
        1 => {
            let (i, world_intersection) = parse_world_intersection_item(i)?;
            Ok((i, Some(world_intersection)))
        }
        _ => unimplemented!(),
    }
}

fn parse_world_intersections(i: &[u8]) -> IResult<&[u8], Vec<WorldIntersection>> {
    let (i, num_intersections) = parse_u16(i)?;
    let num_intersections = num_intersections as usize;
    count(parse_world_intersection_item, num_intersections)(i)
}

fn parse_user_marker_item(i: &[u8]) -> IResult<&[u8], UserMarker> {
    let (i, error) = parse_s32(i)?;
    let (i, time_stamp) = parse_u64(i)?;
    let (i, camera_clock) = parse_u64(i)?;
    let (i, camera_idx) = parse_u8(i)?;
    let (i, data) = parse_u64(i)?;
    Ok((
        i,
        UserMarker {
            error,
            time_stamp,
            camera_clock,
            camera_idx,
            data,
        },
    ))
}

fn parse_user_marker(i: &[u8]) -> IResult<&[u8], Option<UserMarker>> {
    let (i, exists) = parse_u16(i)?;
    match exists {
        0 => Ok((i, None)),
        1 => {
            let (i, user_marker) = parse_user_marker_item(i)?;
            Ok((i, Some(user_marker)))
        }
        _ => unimplemented!(),
    }
}

fn parse_variant(i: &[u8]) -> IResult<&[u8], SEVariant> {
    let (i, type_id): (&[u8], SETypeId) = map_res(parse_u16, |id: u16| id.try_into())(i)?;
    match type_id {
        SETypeId::U8 => {
            let (i, v) = parse_u8(i)?;
            Ok((i, SEVariant::U8(v)))
        }
        SETypeId::U16 => {
            let (i, v) = parse_u16(i)?;
            Ok((i, SEVariant::U16(v)))
        }
        SETypeId::U32 => {
            let (i, v) = parse_u32(i)?;
            Ok((i, SEVariant::U32(v)))
        }
        SETypeId::S32 => {
            let (i, v) = parse_s32(i)?;
            Ok((i, SEVariant::S32(v)))
        }
        SETypeId::U64 => {
            let (i, v) = parse_u64(i)?;
            Ok((i, SEVariant::U64(v)))
        }
        SETypeId::F64 => {
            let (i, v) = parse_f64(i)?;
            Ok((i, SEVariant::F64(v)))
        }
        SETypeId::Point2D => {
            let (i, v) = parse_point_2d(i)?;
            Ok((i, SEVariant::Point2D(v)))
        }
        SETypeId::Vect2D => {
            let (i, v) = parse_vect_2d(i)?;
            Ok((i, SEVariant::Vect2D(v)))
        }
        SETypeId::Point3D => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, SEVariant::Point3D(v)))
        }
        SETypeId::Vect3D => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, SEVariant::Vect3D(v)))
        }
        SETypeId::String => {
            let (i, v) = parse_string(i)?;
            Ok((i, SEVariant::String(v)))
        }
        SETypeId::Vector => {
            // TODO: limit recursion?
            let (i, v) = parse_vector(i)?;
            Ok((i, SEVariant::Vector(v)))
        }
        SETypeId::Struct => {
            // TODO: limit recursion?
            let (i, v) = parse_struct(i)?;
            Ok((i, SEVariant::Struct(v)))
        }
        SETypeId::WorldIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, SEVariant::WorldIntersection(v)))
        }
        SETypeId::WorldIntersections => {
            let (i, v) = parse_world_intersections(i)?;
            Ok((i, SEVariant::WorldIntersections(v)))
        }
        SETypeId::PacketHeader => unimplemented!(),
        SETypeId::SubPacketHeader => unimplemented!(),
        SETypeId::F32 => {
            let (i, v) = parse_f32(i)?;
            Ok((i, SEVariant::F32(v)))
        }
        SETypeId::Matrix3X3 => todo!(),
        SETypeId::Matrix2x2 => todo!(),
        SETypeId::Quaternion => {
            let (i, v) = parse_quaternion(i)?;
            Ok((i, SEVariant::Quaternion(v)))
        }
        SETypeId::UserMarker => {
            let (i, v) = parse_user_marker(i)?;
            Ok((i, SEVariant::UserMarker(v)))
        }
    }
}

fn parse_vector_item(i: &[u8]) -> IResult<&[u8], SEVectorItem> {
    parse_variant(i)
}

fn parse_vector(i: &[u8]) -> IResult<&[u8], Vec<SEVectorItem>> {
    let (i, length) = parse_u16(i)?;
    let length = length as usize;
    count(parse_vector_item, length)(i)
}

fn parse_struct_item(i: &[u8]) -> IResult<&[u8], SEStructItem> {
    let (i, key) = parse_string(i)?;
    let (i, value) = parse_variant(i)?;
    Ok((i, SEStructItem(key, value)))
}

fn parse_struct(i: &[u8]) -> IResult<&[u8], Vec<SEStructItem>> {
    let (i, length) = parse_u16(i)?;
    let length = length as usize;
    count(parse_struct_item, length)(i)
}

fn parse_sub_packet_header(i: &[u8]) -> IResult<&[u8], SubPacketHeader> {
    let (i, header_data) = take(4usize)(i)?;
    let (_, (id, length)) = all_consuming(tuple((
        map_res(parse_u16, SEOutputDataId::try_from),
        parse_u16,
    )))(header_data)?;
    Ok((i, SubPacketHeader { id, length }))
}

fn parse_sub_packet_data(
    data_id: SEOutputDataId,
) -> impl Fn(&[u8]) -> IResult<&[u8], SEOutputData> {
    type Id = SEOutputDataId;
    type Data = SEOutputData;
    move |i: &[u8]| match data_id {
        Id::SEFrameNumber => {
            let (i, v) = parse_u32(i)?;
            Ok((i, Data::SEFrameNumber(v)))
        }
        Id::SEEstimatedDelay => {
            let (i, v) = parse_u32(i)?;
            Ok((i, Data::SEEstimatedDelay(v)))
        }
        Id::SETimeStamp => {
            let (i, v) = parse_u64(i)?;
            Ok((i, Data::SETimeStamp(v)))
        }
        Id::SEUserTimeStamp => {
            let (i, v) = parse_u64(i)?;
            Ok((i, Data::SEUserTimeStamp(v)))
        }
        Id::SEFrameRate => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFrameRate(v)))
        }
        Id::SECameraPositions => {
            let (i, v) = parse_vector(i)?;
            Ok((i, Data::SECameraPositions(v)))
        }
        Id::SECameraRotations => {
            let (i, v) = parse_vector(i)?;
            Ok((i, Data::SECameraRotations(v)))
        }
        Id::SEUserDefinedData => {
            let (i, v) = parse_u64(i)?;
            Ok((i, Data::SEUserDefinedData(v)))
        }
        Id::SERealTimeClock => {
            let (i, v) = parse_u64(i)?;
            Ok((i, Data::SERealTimeClock(v)))
        }
        Id::SEKeyboardState => {
            let (i, v) = parse_string(i)?;
            Ok((i, Data::SEKeyboardState(v)))
        }
        Id::SEASCIIKeyboardState => {
            let (i, v) = parse_u16(i)?;
            Ok((i, Data::SEASCIIKeyboardState(v)))
        }
        Id::SEUserMarker => {
            let (i, v) = parse_user_marker(i)?;
            Ok((i, Data::SEUserMarker(v)))
        }
        Id::SECameraClocks => {
            let (i, v) = parse_vector(i)?;
            Ok((i, Data::SECameraClocks(v)))
        }
        Id::SEHeadPosition => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SEHeadPosition(v)))
        }
        Id::SEHeadPositionQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEHeadPositionQ(v)))
        }
        Id::SEHeadRotationRodrigues => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SEHeadRotationRodrigues(v)))
        }
        Id::SEHeadRotationQuaternion => {
            let (i, v) = parse_quaternion(i)?;
            Ok((i, Data::SEHeadRotationQuaternion(v)))
        }
        Id::SEHeadLeftEarDirection => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SEHeadLeftEarDirection(v)))
        }
        Id::SEHeadUpDirection => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SEHeadUpDirection(v)))
        }
        Id::SEHeadNoseDirection => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SEHeadNoseDirection(v)))
        }
        Id::SEHeadHeading => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEHeadHeading(v)))
        }
        Id::SEHeadPitch => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEHeadPitch(v)))
        }
        Id::SEHeadRoll => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEHeadRoll(v)))
        }
        Id::SEHeadRotationQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEHeadRotationQ(v)))
        }
        Id::SEGazeOrigin => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SEGazeOrigin(v)))
        }
        Id::SELeftGazeOrigin => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SELeftGazeOrigin(v)))
        }
        Id::SERightGazeOrigin => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SERightGazeOrigin(v)))
        }
        Id::SEEyePosition => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SEEyePosition(v)))
        }
        Id::SEGazeDirection => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SEGazeDirection(v)))
        }
        Id::SEGazeDirectionQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEGazeDirectionQ(v)))
        }
        Id::SELeftEyePosition => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SELeftEyePosition(v)))
        }
        Id::SELeftGazeDirection => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SELeftGazeDirection(v)))
        }
        Id::SELeftGazeDirectionQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SELeftGazeDirectionQ(v)))
        }
        Id::SERightEyePosition => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SERightEyePosition(v)))
        }
        Id::SERightGazeDirection => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SERightGazeDirection(v)))
        }
        Id::SERightGazeDirectionQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SERightGazeDirectionQ(v)))
        }
        Id::SEGazeHeading => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEGazeHeading(v)))
        }
        Id::SEGazePitch => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEGazePitch(v)))
        }
        Id::SELeftGazeHeading => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SELeftGazeHeading(v)))
        }
        Id::SELeftGazePitch => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SELeftGazePitch(v)))
        }
        Id::SERightGazeHeading => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SERightGazeHeading(v)))
        }
        Id::SERightGazePitch => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SERightGazePitch(v)))
        }
        Id::SEFilteredGazeDirection => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SEFilteredGazeDirection(v)))
        }
        Id::SEFilteredGazeDirectionQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredGazeDirectionQ(v)))
        }
        Id::SEFilteredLeftGazeDirection => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SEFilteredLeftGazeDirection(v)))
        }
        Id::SEFilteredLeftGazeDirectionQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredLeftGazeDirectionQ(v)))
        }
        Id::SEFilteredRightGazeDirection => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SEFilteredRightGazeDirection(v)))
        }
        Id::SEFilteredRightGazeDirectionQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredRightGazeDirectionQ(v)))
        }
        Id::SEFilteredGazeHeading => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredGazeHeading(v)))
        }
        Id::SEFilteredGazePitch => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredGazePitch(v)))
        }
        Id::SEFilteredLeftGazeHeading => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredLeftGazeHeading(v)))
        }
        Id::SEFilteredLeftGazePitch => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredLeftGazePitch(v)))
        }
        Id::SEFilteredRightGazeHeading => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredRightGazeHeading(v)))
        }
        Id::SEFilteredRightGazePitch => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredRightGazePitch(v)))
        }
        Id::SESaccade => {
            let (i, v) = parse_u32(i)?;
            Ok((i, Data::SESaccade(v)))
        }
        Id::SEFixation => {
            let (i, v) = parse_u32(i)?;
            Ok((i, Data::SEFixation(v)))
        }
        Id::SEBlink => {
            let (i, v) = parse_u32(i)?;
            Ok((i, Data::SEBlink(v)))
        }
        Id::SETrackingState => unimplemented!("SETrackingState"),
        Id::SEEyeglassesStatus => unimplemented!("SEEyeglassesStatus"),
        Id::SEReflexReductionStateDEPRECATED => unimplemented!("SEReflexReductionState"),
        Id::SELeftBlinkClosingMidTime => {
            let (i, v) = parse_u64(i)?;
            Ok((i, Data::SELeftBlinkClosingMidTime(v)))
        }
        Id::SELeftBlinkOpeningMidTime => {
            let (i, v) = parse_u64(i)?;
            Ok((i, Data::SELeftBlinkOpeningMidTime(v)))
        }
        Id::SELeftBlinkClosingAmplitude => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SELeftBlinkClosingAmplitude(v)))
        }
        Id::SELeftBlinkOpeningAmplitude => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SELeftBlinkOpeningAmplitude(v)))
        }
        Id::SELeftBlinkClosingSpeed => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SELeftBlinkClosingSpeed(v)))
        }
        Id::SELeftBlinkOpeningSpeed => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SELeftBlinkOpeningSpeed(v)))
        }
        Id::SERightBlinkClosingMidTime => {
            let (i, v) = parse_u64(i)?;
            Ok((i, Data::SERightBlinkClosingMidTime(v)))
        }
        Id::SERightBlinkOpeningMidTime => {
            let (i, v) = parse_u64(i)?;
            Ok((i, Data::SERightBlinkOpeningMidTime(v)))
        }
        Id::SERightBlinkClosingAmplitude => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SERightBlinkClosingAmplitude(v)))
        }
        Id::SERightBlinkOpeningAmplitude => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SERightBlinkOpeningAmplitude(v)))
        }
        Id::SERightBlinkClosingSpeed => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SERightBlinkClosingSpeed(v)))
        }
        Id::SERightBlinkOpeningSpeed => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SERightBlinkOpeningSpeed(v)))
        }
        Id::SEClosestWorldIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, Data::SEClosestWorldIntersection(v)))
        }
        Id::SEFilteredClosestWorldIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, Data::SEFilteredClosestWorldIntersection(v)))
        }
        Id::SEAllWorldIntersections => {
            let (i, v) = parse_world_intersections(i)?;
            Ok((i, Data::SEAllWorldIntersections(v)))
        }
        Id::SEFilteredAllWorldIntersections => {
            let (i, v) = parse_world_intersections(i)?;
            Ok((i, Data::SEFilteredAllWorldIntersections(v)))
        }
        Id::SEZoneId => {
            let (i, v) = parse_u16(i)?;
            Ok((i, Data::SEZoneId(v)))
        }
        Id::SEEstimatedClosestWorldIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, Data::SEEstimatedClosestWorldIntersection(v)))
        }
        Id::SEEstimatedAllWorldIntersections => {
            let (i, v) = parse_world_intersections(i)?;
            Ok((i, Data::SEEstimatedAllWorldIntersections(v)))
        }
        Id::SEHeadClosestWorldIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, Data::SEHeadClosestWorldIntersection(v)))
        }
        Id::SEHeadAllWorldIntersections => {
            let (i, v) = parse_world_intersections(i)?;
            Ok((i, Data::SEHeadAllWorldIntersections(v)))
        }
        Id::SECalibrationGazeIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, Data::SECalibrationGazeIntersection(v)))
        }
        Id::SETaggedGazeIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, Data::SETaggedGazeIntersection(v)))
        }
        Id::SELeftClosestWorldIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, Data::SELeftClosestWorldIntersection(v)))
        }
        Id::SELeftAllWorldIntersections => {
            let (i, v) = parse_world_intersections(i)?;
            Ok((i, Data::SELeftAllWorldIntersections(v)))
        }
        Id::SERightClosestWorldIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, Data::SERightClosestWorldIntersection(v)))
        }
        Id::SERightAllWorldIntersections => {
            let (i, v) = parse_world_intersections(i)?;
            Ok((i, Data::SERightAllWorldIntersections(v)))
        }
        Id::SEFilteredLeftClosestWorldIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, Data::SEFilteredLeftClosestWorldIntersection(v)))
        }
        Id::SEFilteredLeftAllWorldIntersections => {
            let (i, v) = parse_world_intersections(i)?;
            Ok((i, Data::SEFilteredLeftAllWorldIntersections(v)))
        }
        Id::SEFilteredRightClosestWorldIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, Data::SEFilteredRightClosestWorldIntersection(v)))
        }
        Id::SEFilteredRightAllWorldIntersections => {
            let (i, v) = parse_world_intersections(i)?;
            Ok((i, Data::SEFilteredRightAllWorldIntersections(v)))
        }
        Id::SEEstimatedLeftClosestWorldIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, Data::SEEstimatedLeftClosestWorldIntersection(v)))
        }
        Id::SEEstimatedLeftAllWorldIntersections => {
            let (i, v) = parse_world_intersections(i)?;
            Ok((i, Data::SEEstimatedLeftAllWorldIntersections(v)))
        }
        Id::SEEstimatedRightClosestWorldIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, Data::SEEstimatedRightClosestWorldIntersection(v)))
        }
        Id::SEEstimatedRightAllWorldIntersections => {
            let (i, v) = parse_world_intersections(i)?;
            Ok((i, Data::SEEstimatedRightAllWorldIntersections(v)))
        }
        Id::SEFilteredEstimatedClosestWorldIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, Data::SEFilteredEstimatedClosestWorldIntersection(v)))
        }
        Id::SEFilteredEstimatedAllWorldIntersections => {
            let (i, v) = parse_world_intersections(i)?;
            Ok((i, Data::SEFilteredEstimatedAllWorldIntersections(v)))
        }
        Id::SEFilteredEstimatedLeftClosestWorldIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, Data::SEFilteredEstimatedLeftClosestWorldIntersection(v)))
        }
        Id::SEFilteredEstimatedLeftAllWorldIntersections => {
            let (i, v) = parse_world_intersections(i)?;
            Ok((i, Data::SEFilteredEstimatedLeftAllWorldIntersections(v)))
        }
        Id::SEFilteredEstimatedRightClosestWorldIntersection => {
            let (i, v) = parse_world_intersection(i)?;
            Ok((i, Data::SEFilteredEstimatedRightClosestWorldIntersection(v)))
        }
        Id::SEFilteredEstimatedRightAllWorldIntersections => {
            let (i, v) = parse_world_intersections(i)?;
            Ok((i, Data::SEFilteredEstimatedRightAllWorldIntersections(v)))
        }
        Id::SEEyelidOpening => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEEyelidOpening(v)))
        }
        Id::SEEyelidOpeningQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEEyelidOpeningQ(v)))
        }
        Id::SELeftEyelidOpening => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SELeftEyelidOpening(v)))
        }
        Id::SELeftEyelidOpeningQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SELeftEyelidOpeningQ(v)))
        }
        Id::SERightEyelidOpening => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SERightEyelidOpening(v)))
        }
        Id::SERightEyelidOpeningQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SERightEyelidOpeningQ(v)))
        }
        Id::SELeftLowerEyelidExtremePoint => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SELeftLowerEyelidExtremePoint(v)))
        }
        Id::SELeftUpperEyelidExtremePoint => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SELeftUpperEyelidExtremePoint(v)))
        }
        Id::SERightLowerEyelidExtremePoint => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SERightLowerEyelidExtremePoint(v)))
        }
        Id::SERightUpperEyelidExtremePoint => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SERightUpperEyelidExtremePoint(v)))
        }
        Id::SELeftEyelidState => {
            let (i, v) = parse_u8(i)?;
            Ok((i, Data::SELeftEyelidState(v)))
        }
        Id::SERightEyelidState => {
            let (i, v) = parse_u8(i)?;
            Ok((i, Data::SERightEyelidState(v)))
        }
        Id::SEPupilDiameter => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEPupilDiameter(v)))
        }
        Id::SEPupilDiameterQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEPupilDiameterQ(v)))
        }
        Id::SELeftPupilDiameter => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SELeftPupilDiameter(v)))
        }
        Id::SELeftPupilDiameterQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SELeftPupilDiameterQ(v)))
        }
        Id::SERightPupilDiameter => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SERightPupilDiameter(v)))
        }
        Id::SERightPupilDiameterQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SERightPupilDiameterQ(v)))
        }
        Id::SEFilteredPupilDiameter => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredPupilDiameter(v)))
        }
        Id::SEFilteredPupilDiameterQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredPupilDiameterQ(v)))
        }
        Id::SEFilteredLeftPupilDiameter => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredLeftPupilDiameter(v)))
        }
        Id::SEFilteredLeftPupilDiameterQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredLeftPupilDiameterQ(v)))
        }
        Id::SEFilteredRightPupilDiameter => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredRightPupilDiameter(v)))
        }
        Id::SEFilteredRightPupilDiameterQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredRightPupilDiameterQ(v)))
        }
        Id::SEGPSPosition => {
            let (i, v) = parse_point_2d(i)?;
            Ok((i, Data::SEGPSPosition(v)))
        }
        Id::SEGPSGroundSpeed => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEGPSGroundSpeed(v)))
        }
        Id::SEGPSCourse => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEGPSCourse(v)))
        }
        Id::SEGPSTime => {
            let (i, v) = parse_u64(i)?;
            Ok((i, Data::SEGPSTime(v)))
        }
        Id::SEEstimatedGazeOrigin => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SEEstimatedGazeOrigin(v)))
        }
        Id::SEEstimatedLeftGazeOrigin => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SEEstimatedLeftGazeOrigin(v)))
        }
        Id::SEEstimatedRightGazeOrigin => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SEEstimatedRightGazeOrigin(v)))
        }
        Id::SEEstimatedEyePosition => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SEEstimatedEyePosition(v)))
        }
        Id::SEEstimatedGazeDirection => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SEEstimatedGazeDirection(v)))
        }
        Id::SEEstimatedGazeDirectionQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEEstimatedGazeDirectionQ(v)))
        }
        Id::SEEstimatedGazeHeading => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEEstimatedGazeHeading(v)))
        }
        Id::SEEstimatedGazePitch => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEEstimatedGazePitch(v)))
        }
        Id::SEEstimatedLeftEyePosition => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SEEstimatedLeftEyePosition(v)))
        }
        Id::SEEstimatedLeftGazeDirection => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SEEstimatedLeftGazeDirection(v)))
        }
        Id::SEEstimatedLeftGazeDirectionQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEEstimatedLeftGazeDirectionQ(v)))
        }
        Id::SEEstimatedLeftGazeHeading => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEEstimatedLeftGazeHeading(v)))
        }
        Id::SEEstimatedLeftGazePitch => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEEstimatedLeftGazePitch(v)))
        }
        Id::SEEstimatedRightEyePosition => {
            let (i, v) = parse_point_3d(i)?;
            Ok((i, Data::SEEstimatedRightEyePosition(v)))
        }
        Id::SEEstimatedRightGazeDirection => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SEEstimatedRightGazeDirection(v)))
        }
        Id::SEEstimatedRightGazeDirectionQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEEstimatedRightGazeDirectionQ(v)))
        }
        Id::SEEstimatedRightGazeHeading => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEEstimatedRightGazeHeading(v)))
        }
        Id::SEEstimatedRightGazePitch => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEEstimatedRightGazePitch(v)))
        }
        Id::SEFilteredEstimatedGazeDirection => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SEFilteredEstimatedGazeDirection(v)))
        }
        Id::SEFilteredEstimatedGazeDirectionQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredEstimatedGazeDirectionQ(v)))
        }
        Id::SEFilteredEstimatedGazeHeading => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredEstimatedGazeHeading(v)))
        }
        Id::SEFilteredEstimatedGazePitch => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredEstimatedGazePitch(v)))
        }
        Id::SEFilteredEstimatedLeftGazeDirection => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SEFilteredEstimatedLeftGazeDirection(v)))
        }
        Id::SEFilteredEstimatedLeftGazeDirectionQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredEstimatedLeftGazeDirectionQ(v)))
        }
        Id::SEFilteredEstimatedLeftGazeHeading => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredEstimatedLeftGazeHeading(v)))
        }
        Id::SEFilteredEstimatedLeftGazePitch => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredEstimatedLeftGazePitch(v)))
        }
        Id::SEFilteredEstimatedRightGazeDirection => {
            let (i, v) = parse_vect_3d(i)?;
            Ok((i, Data::SEFilteredEstimatedRightGazeDirection(v)))
        }
        Id::SEFilteredEstimatedRightGazeDirectionQ => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredEstimatedRightGazeDirectionQ(v)))
        }
        Id::SEFilteredEstimatedRightGazeHeading => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredEstimatedRightGazeHeading(v)))
        }
        Id::SEFilteredEstimatedRightGazePitch => {
            let (i, v) = parse_f64(i)?;
            Ok((i, Data::SEFilteredEstimatedRightGazePitch(v)))
        }
    }
}

fn parse_sub_packet(i: &[u8]) -> IResult<&[u8], SEOutputData> {
    let (i, header) = parse_sub_packet_header(i)?;
    let (i, data) = take(header.length)(i)?;
    let (_, sub_packet) = all_consuming(parse_sub_packet_data(header.id))(data)?;
    Ok((i, sub_packet))
}

pub fn parse_packet_header(i: &[u8]) -> Result<PacketHeader, ParseFailedError> {
    let (_, (_sync_id, _type, length)) =
        tuple((tag(b"SEPD"), tag(b"\x00\x04"), parse_u16))(i).map_err(|_| ParseFailedError {})?;
    Ok(PacketHeader { length })
}

pub fn parse_packet_data(
    header: PacketHeader,
    i: &[u8],
) -> Result<Vec<SEOutputData>, ParseFailedError> {
    let mut parser = map(
        map_parser(take(header.length), many_till(parse_sub_packet, eof)),
        |(sub_packets, _eof)| sub_packets,
    );
    match parser(i) {
        Ok((_, sub_packets)) => Ok(sub_packets),
        _ => Err(ParseFailedError {}),
    }
}

pub fn parse_packet(i: &[u8]) -> Result<Vec<SEOutputData>, ParseFailedError> {
    let header = parse_packet_header(i)?;
    parse_packet_data(header, &i[PACKET_HEADER_SIZE..])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUB_PACKET_HEADER_SIZE: usize = 2 + 2;

    const PACKET_EMPTY: &[u8] = &[
        0x53, 0x45, 0x50, 0x44, // Sync Id
        0x00, 0x04, // Packet type
        0x00, 0x00, // Packet length
    ];

    const PACKET_FRAME_NUMBER: &[u8] = &[
        // Packet Header
        0x53, 0x45, 0x50, 0x44, // Sync Id
        0x00, 0x04, // Packet type
        0x00, 0x08, // Packet length
        // Subpacket header
        0x00, 0x01, // Id (0x0001 = SEFrameNumber)
        0x00, 0x04, // Length
        // Subpacket data
        0x00, 0x00, 0x45, 0x9B,
    ];

    const PACKET_TIME_STAMP_FRAME_NUMBER: &[u8] = &[
        // Packet Header
        0x53, 0x45, 0x50, 0x44, // Sync Id
        0x00, 0x04, // Packet type
        0x00, 0x14, // Packet length
        // Subpacket header
        0x00, 0x03, // Id (0x0003 = SETimeStamp)
        0x00, 0x08, // Length
        // Subpacket data
        0x00, 0x00, 0x04, 0x12, 0xDE, 0x00, 0x01, 0x00, // Subpacket header
        0x00, 0x01, // Id (0x0001 = SEFrameNumber)
        0x00, 0x04, // Length
        // Subpacket data
        0x00, 0x00, 0x45, 0x9B,
    ];

    const INCOMPLETE_PACKET_FRAME_NUMBER: &[u8] = &[
        // Packet Header
        0x53, 0x45, 0x50, 0x44, // Sync Id
        0x00, 0x04, // Packet type
        0x00, 0x08, // Packet length
        // Subpacket header
        0x00, 0x01, // Id (0x0001 = SEFrameNumber)
        0x00, 0x04, // Length
              // Missing: Subpacket data
    ];

    #[test]
    fn test_parse_point_2d() {
        let point_2d: &[u8] = &[
            0x40, 0x5E, 0xDD, 0x2F, 0x1A, 0x9F, 0xBE, 0x77, // x (123.456)
            0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // y (1.0)
        ];
        assert_eq!(
            parse_point_2d(point_2d),
            Ok((&b""[..], Point2D(123.456, 1.0)))
        );
    }

    #[test]
    fn test_parse_vect_2d() {
        let vect_2d: &[u8] = &[
            0x40, 0x5E, 0xDD, 0x2F, 0x1A, 0x9F, 0xBE, 0x77, // x (123.456)
            0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // y (1.0)
        ];
        assert_eq!(parse_vect_2d(vect_2d), Ok((&b""[..], Vect2D(123.456, 1.0))));
    }

    #[test]
    fn test_parse_point_3d() {
        let point_3d: &[u8] = &[
            0x40, 0x5E, 0xDD, 0x2F, 0x1A, 0x9F, 0xBE, 0x77, // x (123.456)
            0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // y (1.0)
            0xBF, 0xB9, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A, // z (-0.1)
        ];
        assert_eq!(
            parse_point_3d(point_3d),
            Ok((&b""[..], Point3D(123.456, 1.0, -0.1)))
        );
    }

    #[test]
    fn test_parse_vect_3d() {
        let vect_3d: &[u8] = &[
            0x40, 0x5E, 0xDD, 0x2F, 0x1A, 0x9F, 0xBE, 0x77, // x (123.456)
            0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // y (1.0)
            0xBF, 0xB9, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A, // z (-0.1)
        ];
        assert_eq!(
            parse_vect_3d(vect_3d),
            Ok((&b""[..], Vect3D(123.456, 1.0, -0.1)))
        );
    }

    #[test]
    fn test_parse_string() {
        let string: &[u8] = &[
            0x00, 0x06, // Length
            // "AbC!?~"
            0x41, 0x62, 0x43, 0x21, 0x3F, 0x7E,
        ];
        assert_eq!(parse_string(string), Ok((&b""[..], String::from("AbC!?~"))));
    }

    #[test]
    fn test_parse_quaternion() {
        let quaternion: &[u8] = &[
            0x40, 0xC3, 0x88, 0x00, 0x00, 0x00, 0x00, 0x00, // w (10000.0)
            0x40, 0x5E, 0xDD, 0x2F, 0x1A, 0x9F, 0xBE, 0x77, // x (123.456)
            0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // y (1.0)
            0xBF, 0xB9, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A, // z (-0.1)
        ];
        assert_eq!(
            parse_quaternion(quaternion),
            Ok((&b""[..], Quaternion(10000.0, 123.456, 1.0, -0.1)))
        );
    }

    #[test]
    fn test_parse_world_intersection_item() {
        let world_intersection_item: &[u8] = &[
            0x40, 0x5E, 0xDD, 0x2F, 0x1A, 0x9F, 0xBE, 0x77, // worldPoint.x (123.456)
            0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // worldPoint.y (1.0)
            0xBF, 0xB9, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A, // worldPoint.z (-0.1)
            0x40, 0xB6, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, // objectPoint.x (5656.0)
            0x40, 0x24, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A, // objectPoint.y (10.3)
            0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // objectPoint.z (-2.0)
            0x00, 0x06, // objectName.Length
            0x41, 0x62, 0x43, 0x21, 0x3F, 0x7E, // objectName "AbC!?~"
        ];

        assert_eq!(
            parse_world_intersection_item(world_intersection_item),
            Ok((
                &b""[..],
                WorldIntersection {
                    world_point: Point3D(123.456, 1.0, -0.1),
                    object_point: Point3D(5656.0, 10.3, -2.0),
                    object_name: String::from("AbC!?~")
                }
            ))
        );
    }

    #[test]
    fn test_parse_world_intersection() {
        let world_intersection: &[u8] = &[
            0x00, 0x01, // Exists
            0x40, 0x5E, 0xDD, 0x2F, 0x1A, 0x9F, 0xBE, 0x77, // worldPoint.x (123.456)
            0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // worldPoint.y (1.0)
            0xBF, 0xB9, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A, // worldPoint.z (-0.1)
            0x40, 0xB6, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, // objectPoint.x (5656.0)
            0x40, 0x24, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A, // objectPoint.y (10.3)
            0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // objectPoint.z (-2.0)
            0x00, 0x06, // objectName.Length
            0x41, 0x62, 0x43, 0x21, 0x3F, 0x7E, // objectName "AbC!?~"
        ];
        assert_eq!(
            parse_world_intersection(world_intersection),
            Ok((
                &b""[..],
                Some(WorldIntersection {
                    world_point: Point3D(123.456, 1.0, -0.1),
                    object_point: Point3D(5656.0, 10.3, -2.0),
                    object_name: String::from("AbC!?~")
                })
            ))
        );

        let world_intersection: &[u8] = &[0x00, 0x00];
        assert_eq!(
            parse_world_intersection(world_intersection),
            Ok((&b""[..], None))
        );
    }

    #[test]
    fn test_parse_world_intersections() {
        let world_intersections: &[u8] = &[
            0x00, 0x01, // Num intersections
            0x40, 0x5E, 0xDD, 0x2F, 0x1A, 0x9F, 0xBE, 0x77, // worldPoint.x (123.456)
            0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // worldPoint.y (1.0)
            0xBF, 0xB9, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A, // worldPoint.z (-0.1)
            0x40, 0xB6, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, // objectPoint.x (5656.0)
            0x40, 0x24, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A, // objectPoint.y (10.3)
            0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // objectPoint.z (-2.0)
            0x00, 0x06, // objectName.Length
            0x41, 0x62, 0x43, 0x21, 0x3F, 0x7E, // objectName "AbC!?~"
        ];
        assert_eq!(
            parse_world_intersections(world_intersections),
            Ok((
                &b""[..],
                vec![WorldIntersection {
                    world_point: Point3D(123.456, 1.0, -0.1),
                    object_point: Point3D(5656.0, 10.3, -2.0),
                    object_name: String::from("AbC!?~")
                }]
            ))
        );

        let world_intersections: &[u8] = &[0x00, 0x00];
        assert_eq!(
            parse_world_intersections(world_intersections),
            Ok((&b""[..], vec![]))
        );
    }

    #[test]
    fn test_parse_user_marker_item() {
        let user_marker_item: &[u8] = &[
            0x00, 0x00, 0x00, 0x00, // userMarker.error (0)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xF2, // userMarker.time_stamp (1010)
            0x00, 0x00, 0x00, 0x14, 0xF4, 0x6B, 0x04,
            0x00, // userMarker.camera_clock (90000000000)
            0x02, // userMarker.camera_idx (2)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x39, // userMarker.data (1337)
        ];

        assert_eq!(
            parse_user_marker_item(user_marker_item),
            Ok((
                &b""[..],
                UserMarker {
                    error: 0,
                    time_stamp: 1010,
                    camera_clock: 90000000000,
                    camera_idx: 2,
                    data: 1337,
                }
            ))
        );
    }

    #[test]
    fn test_parse_user_marker() {
        let user_marker: &[u8] = &[
            0x00, 0x01, // Exists
            0x00, 0x00, 0x00, 0x00, // userMarker.error (0)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xF2, // userMarker.time_stamp (1010)
            0x00, 0x00, 0x00, 0x14, 0xF4, 0x6B, 0x04,
            0x00, // userMarker.camera_clock (90000000000)
            0x02, // userMarker.camera_idx (2)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x39, // userMarker.data (1337)
        ];
        assert_eq!(
            parse_user_marker(user_marker),
            Ok((
                &b""[..],
                Some(UserMarker {
                    error: 0,
                    time_stamp: 1010,
                    camera_clock: 90000000000,
                    camera_idx: 2,
                    data: 1337,
                })
            ))
        );

        let user_marker: &[u8] = &[0x00, 0x00];
        assert_eq!(parse_user_marker(user_marker), Ok((&b""[..], None)));
    }

    #[test]
    fn test_parse_variant() {
        let variant: &[u8] = &[
            0x00, 0x00, // typeId (=SEType_u8)
            0x01, // Element (=1)
        ];
        assert_eq!(parse_variant(variant), Ok((&b""[..], SEVariant::U8(1))));
    }

    #[test]
    fn test_parse_vector_item() {
        let vector_item: &[u8] = &[
            0x00, 0x00, // typeId (=SEType_u8)
            0x01, // Element (=1)
        ];
        assert_eq!(
            parse_vector_item(vector_item),
            Ok((&b""[..], SEVectorItem::U8(1)))
        );
    }

    #[test]
    fn test_parse_vector() {
        let vector: &[u8] = &[
            0x00, 0x04, // numElements
            0x00, 0x00, // elem[0].typeId (=SEType_u8)
            0x01, // elem[0] (=1)
            0x00, 0x01, // elem[1].typeId (=SEType_u16)
            0x10, 0x01, // elem[1] (=4097)
            0x00, 0x0B, // elem[2].typeId (=SEType_Vector)
            0x00, 0x01, // elem[2].numElements
            0x00, 0x00, // elem[2].elem[0].typeId (=SEType_u8)
            0x04, // elem[2].elem[0] (=4)
            0x00, 0x0C, // elem[3].typeId (SEType_Struct)
            0x00, 0x01, // elem[3].numElements
            0x00, 0x03, // elem[3].elem[0].id.length
            0x41, 0x62, 0x43, // elem[3].elem[0].id.chars ("AbC")
            0x00, 0x01, // elem[3].elem[0].typeId (=SEType_u16)
            0x05, 0x39, // elem[3].elem[0] (=1337)
        ];
        assert_eq!(
            parse_vector(vector),
            Ok((
                &b""[..],
                vec![
                    SEVectorItem::U8(1),
                    SEVectorItem::U16(4097),
                    SEVectorItem::Vector(vec![SEVectorItem::U8(4)]),
                    SEVectorItem::Struct(vec![SEStructItem(
                        String::from("AbC"),
                        SEVariant::U16(1337)
                    )])
                ]
            ))
        );
    }

    #[test]
    fn test_parse_struct_item() {
        let struct_item: &[u8] = &[
            0x00, 0x03, // elem[0].id.length
            0x41, 0x62, 0x43, // elem[0].id.chars ("AbC")
            0x00, 0x01, // elem[0].typeId (=SEType_u16)
            0x05, 0x39, // elem[0] (=1337)
        ];
        assert_eq!(
            parse_struct_item(struct_item),
            Ok((
                &b""[..],
                SEStructItem(String::from("AbC"), SEVariant::U16(1337))
            ))
        );
    }

    #[test]
    fn test_parse_struct() {
        let s: &[u8] = &[
            0x00, 0x01, // numElements
            0x00, 0x03, // elem[0].id.length
            0x41, 0x62, 0x43, // elem[0].id.chars ("AbC")
            0x00, 0x01, // elem[0].typeId (=SEType_u16)
            0x05, 0x39, // elem[0] (=1337)
        ];
        assert_eq!(
            parse_struct(s),
            Ok((
                &b""[..],
                vec![SEStructItem(String::from("AbC"), SEVariant::U16(1337))]
            ))
        );
    }

    #[test]
    fn test_parse_packet_header() {
        assert_eq!(
            parse_packet_header(PACKET_EMPTY),
            Ok(PacketHeader { length: 0 })
        );

        assert_eq!(
            parse_packet_header(PACKET_FRAME_NUMBER),
            Ok(PacketHeader { length: 8 })
        );

        let invalid_sync_id: &[u8] = &[
            0x53, 0x45, 0x66, 0x66, // Sync Id
            0x00, 0x04, // Packet type
            0x00, 0x00, // Packet length
        ];
        assert!(parse_packet_header(invalid_sync_id).is_err());

        let invalid_type: &[u8] = &[
            0x53, 0x45, 0x50, 0x44, // Sync Id
            0x00, 0x03, // Packet type
            0x00, 0x00, // Packet length
        ];
        assert!(parse_packet_header(invalid_type).is_err());

        let empty = &b""[..];
        assert_eq!(parse_packet_header(empty), Err(ParseFailedError {}))
    }

    #[test]
    fn test_parse_sub_packet_header() {
        let sub_packet = &PACKET_FRAME_NUMBER[PACKET_HEADER_SIZE..];
        assert_eq!(
            parse_sub_packet_header(sub_packet),
            Ok((
                &sub_packet[SUB_PACKET_HEADER_SIZE..],
                SubPacketHeader {
                    id: SEOutputDataId::SEFrameNumber,
                    length: 4
                }
            ))
        );

        let sub_packet = &PACKET_TIME_STAMP_FRAME_NUMBER[PACKET_HEADER_SIZE..];
        assert_eq!(
            parse_sub_packet_header(sub_packet),
            Ok((
                &sub_packet[SUB_PACKET_HEADER_SIZE..],
                SubPacketHeader {
                    id: SEOutputDataId::SETimeStamp,
                    length: 8
                }
            ))
        );

        let empty = &b""[..];
        assert_eq!(
            parse_sub_packet_header(empty),
            Err(nom::Err::Incomplete(nom::Needed::new(
                SUB_PACKET_HEADER_SIZE
            )))
        )
    }

    #[test]
    fn test_parse_sub_packet_data() {
        let sub_packet = &PACKET_FRAME_NUMBER[PACKET_HEADER_SIZE..];
        let sub_packet_data = &sub_packet[SUB_PACKET_HEADER_SIZE..];
        assert_eq!(
            parse_sub_packet_data(SEOutputDataId::SEFrameNumber)(sub_packet_data),
            Ok((&sub_packet_data[4..], SEOutputData::SEFrameNumber(17819)))
        );

        let sub_packet = &PACKET_TIME_STAMP_FRAME_NUMBER[PACKET_HEADER_SIZE..];
        let sub_packet_data = &sub_packet[SUB_PACKET_HEADER_SIZE..];
        assert_eq!(
            parse_sub_packet_data(SEOutputDataId::SETimeStamp)(sub_packet_data),
            Ok((
                &sub_packet_data[8..],
                SEOutputData::SETimeStamp(4479080464640)
            ))
        );
    }

    #[test]
    fn test_parse_sub_packet() {
        let sub_packet = &PACKET_FRAME_NUMBER[PACKET_HEADER_SIZE..];
        assert_eq!(
            parse_sub_packet(sub_packet),
            Ok((&b""[..], SEOutputData::SEFrameNumber(17819)))
        );

        let sub_packet = &PACKET_TIME_STAMP_FRAME_NUMBER[PACKET_HEADER_SIZE..];
        assert_eq!(
            parse_sub_packet(sub_packet),
            Ok((
                &sub_packet[SUB_PACKET_HEADER_SIZE + 8..],
                SEOutputData::SETimeStamp(4479080464640)
            ))
        );

        let sub_packet = &INCOMPLETE_PACKET_FRAME_NUMBER[PACKET_HEADER_SIZE..];
        assert_eq!(
            parse_sub_packet(sub_packet),
            Err(nom::Err::Incomplete(nom::Needed::new(4)))
        );
    }

    #[test]
    fn test_parse_packet_data() {
        let packet = &PACKET_FRAME_NUMBER;
        let header = parse_packet_header(packet).unwrap();
        assert_eq!(
            parse_packet_data(header, &packet[PACKET_HEADER_SIZE..]),
            Ok(vec![SEOutputData::SEFrameNumber(17819)])
        );

        let packet = &PACKET_TIME_STAMP_FRAME_NUMBER;
        let header = parse_packet_header(packet).unwrap();
        assert_eq!(
            parse_packet_data(header, &packet[PACKET_HEADER_SIZE..]),
            Ok(vec![
                SEOutputData::SETimeStamp(4479080464640),
                SEOutputData::SEFrameNumber(17819)
            ])
        );

        let packet = &INCOMPLETE_PACKET_FRAME_NUMBER;
        let header = parse_packet_header(packet).unwrap();
        assert_eq!(
            parse_packet_data(header, &packet[PACKET_HEADER_SIZE..]),
            Err(ParseFailedError {})
        );
    }
}

#[cfg(test)]
mod capture_tests;
