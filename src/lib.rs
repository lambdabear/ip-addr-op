use std::net::{IpAddr, Ipv4Addr};
use std::thread::spawn;

use futures::stream::Stream;
use futures::Future;
use tokio_core::reactor::Core;

use rtnetlink::new_connection;
use rtnetlink::packet::{AddressNla, LinkNla};

pub fn make_ip_addr_setter() -> impl Fn(String, Ipv4Addr, Ipv4Addr, u8) -> Result<(), String> {
    let (connection, handle) = new_connection().unwrap();

    spawn(move || Core::new().unwrap().run(connection));

    move |ifname, reserved_addr, new_addr, prefix_len| {
        // get all address messages
        let addrs = handle.address().get().execute().collect().wait().unwrap();

        // get address messages which's label equal to ifname
        let addrs_iter = addrs.into_iter().filter(|a| {
            a.nlas.iter().fold(false, |acc, nla| {
                acc || match nla {
                    AddressNla::Label(s) => *s == ifname,
                    _ => false,
                }
            })
        });

        let mut index: Option<u32> = None;

        // del all ip address which is not reserved address
        for addr in addrs_iter {
            index = Some(addr.header.index);
            for nla in addr.nlas {
                if let AddressNla::Address(a) = nla {
                    if a != reserved_addr.octets() {
                        handle
                            .address()
                            .del(
                                addr.header.index,
                                IpAddr::V4(Ipv4Addr::new(a[0], a[1], a[2], a[3])),
                                addr.header.prefix_len,
                            )
                            .execute()
                            .wait()
                            .unwrap();
                    }
                }
            }
        }

        // add new ip address
        match index {
            Some(i) => {
                handle
                    .address()
                    .add(i, IpAddr::V4(new_addr), prefix_len)
                    .execute()
                    .wait()
                    .unwrap();
                Ok(())
            }
            // the ifname is not exist or have no ip address
            None => {
                let links = handle.link().get().execute().collect().wait().unwrap();
                for link in links {
                    for nla in link.nlas() {
                        if let LinkNla::IfName(s) = nla {
                            if *s == ifname {
                                // DEBUG
                                // println!("ifname: {} index: {}", s, link.header().index());
                                handle
                                    .address()
                                    .add(link.header().index(), IpAddr::V4(new_addr), prefix_len)
                                    .execute()
                                    .wait()
                                    .unwrap();
                                return Ok(());
                            }
                        }
                    }
                }
                Err(String::from(format!("not find the ifname: {}", ifname)))
            }
        }
    }
}
