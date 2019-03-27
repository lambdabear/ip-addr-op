use set_ip_addr;
use std::env;
use std::net::Ipv4Addr;

fn main() {
    let args: Vec<String> = env::args().collect();

    let ifname = args[1].to_owned();
    let reserved_addr = args[2].parse::<Ipv4Addr>().unwrap();
    let new_addr = args[3].parse::<Ipv4Addr>().unwrap();
    let prefix_len = args[4].parse::<u8>().unwrap();

    let set = set_ip_addr::make_ip_addr_setter();
    match set(ifname, reserved_addr, new_addr, prefix_len) {
        Ok(_) => println!("set ip address succeed"),
        Err(e) => eprintln!("{:?}", e),
    };
}
