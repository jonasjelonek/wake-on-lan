use std::str::FromStr;
use pnet::datalink::{self, Channel::Ethernet};
use pnet::datalink::NetworkInterface;
use pnet::packet::{Packet, ethernet::{MutableEthernetPacket, EtherTypes}};
use arrayvec::{self, ArrayVec};

use clap::Parser;

#[derive(Debug, Clone)]
pub enum WolPassword {
	SixByte(pnet::util::MacAddr),
	FourByte(std::net::Ipv4Addr),
	String(String),
}
impl From<&str> for WolPassword {
	fn from(value: &str) -> Self {
		if let Ok(ip) = std::net::Ipv4Addr::from_str(value) {
			return Self::FourByte(ip);
		}
		if let Ok(mac) = pnet::util::MacAddr::from_str(value) {
			return Self::SixByte(mac);
		}
		
		Self::String(value.to_string())
	}
}

#[derive(Debug, Parser)]
pub struct CliArgs {
	#[arg(short, long, help = "The MAC address of the target host.")]
	pub target: pnet::util::MacAddr,

	#[arg(short, long)]
	pub interface: Option<String>,

	#[arg(
		short, long,
		help = "Password (SecureOn) to be used. This can be in IP notation (XXX.XXX.XXX.XXX; 4-byte), MAC notation (FF:FF:FF:FF:FF:FF; 6-byte) or an ASCII-string (length 1-6 characters).",
	)]
	pub password: Option<WolPassword>,
}

fn main() -> Result<(), String> {
	let args = CliArgs::parse();
	let interface: NetworkInterface;

	match args.interface {
		Some(iface) => {
			interface = datalink::interfaces()
				.into_iter()
				.find(|r#if| r#if.name.as_str() == iface.as_str())
				.ok_or("Specified interface does not exist!".to_string())?;
		},
		None => {
			interface = datalink::interfaces()
				.into_iter()
				.find(|e| e.is_up() && !e.is_loopback() && !e.ips.is_empty())
				.ok_or("Could not find a default interface that is up, has an IP address and is not a loopback interface!".to_string())?;
		}
	}

	let mut password: [u8; 6] = [0; 6];
	let mut pw_short: Option<bool> = None;
	if let Some(pw) = args.password {
		match pw {
			WolPassword::FourByte(pw_f) => {
				pw_short = Some(true);
				password[0..=3].copy_from_slice(pw_f.octets().as_slice());
			},
			WolPassword::SixByte(pw_m) => {
				pw_short = Some(false);
				password.copy_from_slice(pw_m.octets().as_slice());
			},
			WolPassword::String(pw_s) => {
				if pw_s.len() > 6 { return Err("Password is limited to 6 characters!".into()); }
				if !pw_s.is_ascii() { return Err("Password must not contain non-ASCII characters!".into()) }

				pw_short = Some(false);
				password.copy_from_slice(&pw_s[0..=5].as_bytes());
			},
		}
	}

	println!("Sending to {} on iface {}", args.target, interface.name);

	let mut pkt = ArrayVec::<u8, 122>::new(); 
	pkt.extend([0u8; 14]);
	pkt.extend([ 0xff, 0xff, 0xff, 0xff, 0xff, 0xff ]);
	for _ in 0..16 {
		pkt.extend(args.target.octets());
	}
	match pw_short {
		Some(true) => pkt.try_extend_from_slice(&password[..=3]).unwrap(),
		Some(false) => pkt.extend(password),
		None => (),
	}

	let (mut tx, _) = match datalink::channel(&interface, Default::default()) {
		Ok(Ethernet(tx, rx)) => (tx, rx),
		Ok(_) => return Err("Interface is not an Ethernet device!".into()),
		Err(e) => return Err(format!("An error occurred when creating the datalink channel: {}", e)),
	};

	let mut eth_pkt = MutableEthernetPacket::new(&mut pkt[..]).unwrap();
	eth_pkt.set_destination(args.target);
	eth_pkt.set_source(interface.mac.unwrap());
	eth_pkt.set_ethertype(EtherTypes::WakeOnLan);

	tx.send_to(eth_pkt.packet(), None)
		.unwrap() /* Currently send_to always returns Some(_) */
		.map_err(|e| e.to_string())
}
