use std::error::Error;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr};
use std::thread::spawn;

use futures::stream::Stream;
use futures::Future;
use tokio_core::reactor::Core;

use rtnetlink::new_connection;
use rtnetlink::packet::{AddressNla, LinkNla};

#[derive(Debug)]
pub struct IpSettingError {
    dev: String,
}

impl fmt::Display for IpSettingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "set ip address failed on dev {}", self.dev)
    }
}

impl Error for IpSettingError {
    fn description(&self) -> &str {
        "set ip address failed"
    }
}

pub fn make_ip_addr_operaters() -> (
    impl Fn(String) -> Result<Vec<Ipv4Addr>, Box<dyn Error>>,
    impl Fn(String, Ipv4Addr, Ipv4Addr, u8) -> Result<(), IpSettingError>,
) {
    let (connection, handle) = new_connection().unwrap();
    let handle1 = handle.clone();

    spawn(move || Core::new().unwrap().run(connection));

    let getter = move |ifname| {
        // get all address messages
        let addrs = handle1
            .address()
            .get()
            .execute()
            .collect()
            .wait()
            .expect("get ip address failed");

        // get address messages which's label equal to ifname
        let addrs_iter = addrs.into_iter().filter(|a| {
            a.nlas.iter().fold(false, |acc, nla| {
                acc || match nla {
                    AddressNla::Label(s) => *s == ifname,
                    _ => false,
                }
            })
        });

        let mut addrs = vec![];

        for addr_msg in addrs_iter {
            for nla in addr_msg.nlas {
                match nla {
                    AddressNla::Address(addr) => {
                        addrs.push(Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3]))
                    }
                    _ => (),
                }
            }
        }

        Ok(addrs)
    };

    let setter = move |ifname, reserved_addr: Ipv4Addr, new_addr, prefix_len| {
        // get all address messages
        let addrs = handle
            .address()
            .get()
            .execute()
            .collect()
            .wait()
            .expect("get ip address failed");

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
                            .expect("del ip address failed");
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
                    .expect("add ip address failed");
                Ok(())
            }
            // the ifname is not exist or have no ip address
            None => {
                let links = handle.link().get().execute().collect().wait().unwrap();
                for link in links {
                    for nla in link.nlas() {
                        if let LinkNla::IfName(s) = nla {
                            if *s == ifname {
                                handle
                                    .address()
                                    .add(link.header().index(), IpAddr::V4(new_addr), prefix_len)
                                    .execute()
                                    .wait()
                                    .expect("add ip address failed");
                                return Ok(());
                            }
                        }
                    }
                }
                Err(IpSettingError { dev: ifname })
            }
        }
    };

    (getter, setter)
}
