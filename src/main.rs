#![warn(clippy::all, clippy::nursery, clippy::pedantic, rust_2018_idioms)]

use std::fs;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

const GET: &[u8] = b"GET / HTTP/1.1\r\n";
const NUM_CPUS: usize = 10;

pub struct ThreadPool {
    workers: Vec<Worker>,
    tx: mpsc::Sender<Job>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        assert!(size > 0);
        let mut workers = Vec::with_capacity(size);
        let (tx, rx) = mpsc::channel();
        let rx = Arc::new(Mutex::new(rx));

        for i in 0..size {
            workers.push(Worker::new(i, Arc::clone(&rx)));
        }

        Self { workers, tx }
    }

    pub fn exec<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.tx.send(job).unwrap();
    }
}

pub struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, rx: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let thread = thread::spawn(move || loop {
            let job = rx.lock().unwrap().recv().unwrap();
            job();
        });

        Self { id, thread }
    }
}

fn main() {
    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8080);
    let socket = TcpListener::bind(addr).unwrap();

    let pool = ThreadPool::new(NUM_CPUS);

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
