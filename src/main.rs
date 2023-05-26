use std::str::FromStr;
use pnet::datalink::{self, Channel::Ethernet};
use pnet::datalink::{NetworkInterface, EtherType};
use pnet::packet::{Packet, ethernet::MutableEthernetPacket};
use pnet::util::MacAddr;

fn main() {
	let mut args = std::env::args();
	let mut mac_addr: MacAddr = MacAddr(0xff, 0xff, 0xff, 0xff, 0xff, 0xff);
	let mut interface: Option<String> = None;
	while let Some(arg) = args.next() {
		if &arg[..] == "-t" || &arg[..] == "--target" {
			mac_addr = MacAddr::from_str(&(args.next().unwrap())[..]).unwrap();
		} else if &arg[..] == "-i" || &arg[..] == "--interface" {
			interface = Some(args.next().unwrap());
		}
	}
	if let None = interface {
		println!("No interface specified!");
	}

	let mut iface: Option<NetworkInterface> = None;
	if let Some(name) = interface.as_ref() {
		let mut matching_ifaces = datalink::interfaces()
			.into_iter()
			.filter(|iface| iface.name.as_str() == name.as_str());
		iface =  matching_ifaces.next();
	} 
	if let None = iface.as_ref() {
		println!("Using default interface!");
		iface = datalink::interfaces()
			.into_iter()
			.find(|e| e.is_up() && !e.is_loopback() && !e.ips.is_empty());
	}
	if let None = iface.as_ref() {
		println!("Found no interface to use for transmission!");
		return;
	}
	println!("Sending to {} on iface {}", mac_addr, iface.as_ref().unwrap().name);
	/* From here on it's safe to just unwrap 'iface' */

	let mut pkt = vec![ 0xff, 0xff, 0xff, 0xff, 0xff, 0xff ];
	for _ in 0..16 {
		pkt.extend(mac_addr.octets());
	}

	let (mut tx, _) = match datalink::channel(iface.as_ref().unwrap(), Default::default()) {
		Ok(Ethernet(tx, rx)) => (tx, rx),
		Ok(_) => panic!("Unhandled channel type"),
		Err(e) => panic!("An error occurred when creating the datalink channel: {}", e)
	};

	let mut pkt_buf: [u8; 128] = [0; 128];
	let mut eth_pkt = MutableEthernetPacket::new(&mut pkt_buf[..]).unwrap();
	eth_pkt.set_payload(&pkt[..]);
	eth_pkt.set_destination(mac_addr);
	eth_pkt.set_source(iface.unwrap().mac.unwrap());
	eth_pkt.set_ethertype(pnet::packet::ethernet::EtherTypes::WakeOnLan);

	tx.send_to(eth_pkt.packet(), None).unwrap();
}
