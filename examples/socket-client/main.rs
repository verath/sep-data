extern crate sep_data;

use anyhow::{bail, Context, Result};
use sep_data::{
    client::{Client, ClientError, Packet, TCPClient, UDPClient},
    se_types::{SEOutputData, SEVariant},
};
use std::{
    env,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

enum Protocol {
    Tcp,
    Udp,
}

fn print_usage() -> Result<()> {
    let current_exe = env::current_exe()?
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    println!("socket-client");
    println!("Usage: {} <UDP|TCP> [port] [hostname]", current_exe);
    Ok(())
}

fn print_packet(packet: Packet) {
    for item in packet {
        use SEOutputData::*;
        match item {
            SETimeStamp(v) => println!("TimeStamp = {}", v),
            SEFrameNumber(v) => println!("FrameNumber = {}", v),
            SECameraPositions(positions) => {
                let positions: Vec<String> = positions
                    .iter()
                    .map(|var: &SEVariant| match var {
                        SEVariant::Point3D(point) => format!("{:?}", point),
                        v => panic!("unexpected item type '{:?}'", v),
                    })
                    .collect();
                println!("CameraPositions = [{}]", positions.join(", "))
            }
            _ => (),
        }
    }
    println!("----")
}

fn set_ctrlc_handler() -> Result<Arc<AtomicBool>> {
    let abort = Arc::new(AtomicBool::new(false));
    let abort_clone = Arc::clone(&abort);
    ctrlc::set_handler(move || abort.store(true, Ordering::Relaxed))?;
    Ok(abort_clone)
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);

    let protocol = match args.next().map(|s| s.to_lowercase()) {
        Some(ref s) if s == "tcp" => Protocol::Tcp,
        Some(ref s) if s == "udp" => Protocol::Udp,
        _ => return print_usage(),
    };
    let port = match args.next() {
        Some(port) => port.parse().context("Failed to parse port")?,
        _ => match protocol {
            Protocol::Tcp => 5002,
            Protocol::Udp => 5001,
        },
    };

    let nonblocking = true;
    let mut client: Box<dyn Client> = match protocol {
        Protocol::Udp => {
            println!("Listening for UDP data (port={})", port);
            Box::new(UDPClient::new(port, nonblocking))
        }
        Protocol::Tcp => {
            let hostname = args.next().unwrap_or_else(|| String::from("localhost"));
            println!("Connecting to TCP (hostname={}, port={})", hostname, port);
            Box::new(TCPClient::new(&hostname, port, nonblocking))
        }
    };
    client.connect()?;

    let abort = set_ctrlc_handler()?;
    while !abort.load(Ordering::Relaxed) {
        match client.next() {
            Ok(packet) => print_packet(packet),
            Err(ClientError::ReadWouldBlock) => thread::yield_now(),
            Err(err) => bail!(err),
        }
    }

    client.disconnect()?;
    Ok(())
}
