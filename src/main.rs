use std::io::Write;
use pnet::datalink::{self, Channel::Ethernet};
use pnet::datalink::NetworkInterface;
use pnet::packet::{Packet, ethernet::MutableEthernetPacket};

use clap::Parser;

#[derive(Debug, Parser)]
pub struct CliArgs {
	#[arg(short, long)]
	pub target: pnet::util::MacAddr,

	#[arg(short, long)]
	pub interface: Option<String>,
}

fn main() -> Result<(), String> {
	let args = CliArgs::parse();

	let interface: NetworkInterface;
	match args.interface {
		Some(iface) => {
			let mut matching_ifaces = datalink::interfaces()
				.into_iter()
				.filter(|r#if| r#if.name.as_str() == iface.as_str());
			
			interface = match matching_ifaces.next() {
				Some(r#if) => r#if,
				None => return Err(format!("Interface '{}' does not exist!", iface))
			};
		},
		None => {
			interface = match datalink::interfaces()
				.into_iter()
				.find(|e| e.is_up() && !e.is_loopback() && !e.ips.is_empty()) 
				{
					Some(r#if) => r#if,
					None => return Err(format!("Could not find a default interface that is up, has an IP address and is not a loopback interface!")),
				};
		}
	}
	println!("Sending to {} on iface {}", args.target, interface.name);

	let mut pkt: [u8; 120] = [0; 120];
	let mut pkt_ref = &mut pkt[14..];
	pkt_ref.write([ 0xff, 0xff, 0xff, 0xff, 0xff, 0xff ].as_slice()).unwrap();
	for _ in 0..16 {
		pkt_ref.write(args.target.octets().as_slice()).unwrap();
	}

	let (mut tx, _) = match datalink::channel(&interface, Default::default()) {
		Ok(Ethernet(tx, rx)) => (tx, rx),
		Ok(_) => panic!("Unhandled channel type"),
		Err(e) => panic!("An error occurred when creating the datalink channel: {}", e)
	};

	let mut eth_pkt = MutableEthernetPacket::new(&mut pkt[..]).unwrap();
	eth_pkt.set_destination(args.target);
	eth_pkt.set_source(interface.mac.unwrap());
	eth_pkt.set_ethertype(pnet::packet::ethernet::EtherTypes::WakeOnLan);

	tx.send_to(eth_pkt.packet(), None)
		.unwrap() /* Currently, send_to always return Some(_) */
		.map_err(|e| e.to_string())
}
