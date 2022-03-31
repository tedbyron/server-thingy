#![warn(clippy::all, clippy::nursery, clippy::pedantic, rust_2018_idioms)]

use std::fs;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};

use server_thingy::ThreadPool;

const GET: &[u8] = b"GET / HTTP/1.1\r\n";

fn main() {
    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8080);
    let socket = TcpListener::bind(addr).unwrap();

    let pool = ThreadPool::new(num_cpus::get());

    socket.incoming().filter_map(Result::ok).for_each(|stream| {
        pool.exec(|| {
            handle_stream(stream);
        });
    });
}

fn handle_stream(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let (status, html_path) = if buffer.starts_with(GET) {
        ("HTTP/1.1 200 OK", "src/index.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND", "src/404.html")
    };

    let html = fs::read_to_string(html_path).unwrap();

    let res = format!(
        "{status}\r\nContent-Length: {len}\r\n\r\n{html}",
        len = html.len(),
    );

    stream.write(res.as_bytes()).unwrap();
    stream.flush().unwrap();
}
